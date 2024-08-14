/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::prelude::*;

pub(crate) fn cached_field(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let debug = !attr.find_all(parse("debug")).is_empty();

    let pat = "pub fn __NAME (&self) -> Result< ___RET_TYPE > { ___BODY Ok( ___RET_VALUE ) }";
    let pat = pat.to_items().disallow_group_match("___RET_TYPE");
    let count = AtomicUsize::default();

    let tokens = tokens.replace_with(pat, |m: &Match<TokenInfo>| {
        let _ = count.fetch_add(1, Ordering::Release);

        let name = m.captured_tokens("__NAME");
        let name_str = name.to_string();
        let ret_type = m.captured_tokens("___RET_TYPE");
        let body = m.captured_tokens("___BODY");
        let ret_value = m.captured_tokens("___RET_VALUE");
        let load_name = format_ident!("load_{}", name_str);

        // If RET_TYPE is Arc<RwLock<T>>, and RET_VALUE is Arc::new(RwLock::new(T)), there is a way
        // to generate `invalidate_NAME`. `load_NAME` returns `T` in this case.
        if let (Some(ret_type_match), Some(ret_value_match)) = (
            ret_type.matches_full("Arc<RwLock<___INNER>>"),
            ret_value.matches_full("Arc::new(RwLock::new(___INNER))"),
        ) {
            let ret_type_inner = ret_type_match.captured_tokens("___INNER");
            let ret_value_inner = ret_value_match.captured_tokens("___INNER");
            let invalidate_name = format_ident!("invalidate_{}", name_str);
            quote! {
                pub fn #name(&self) -> Result<#ret_type> {
                    Ok(self.#name.get_or_try_init(|| self.#load_name().map(|v| Arc::new(RwLock::new(v))))?.clone())
                }
                fn #load_name(&self) -> Result<#ret_type_inner> {
                    #body
                    Ok(#ret_value_inner)
                }
                pub fn #invalidate_name(&self) -> Result<()> {
                    if let Some(v) = self.#name.get() {
                        *v.write() = self.#load_name()?;
                    }
                    Ok(())
                }
            }
        } else {
            quote! {
                pub fn #name(&self) -> Result<#ret_type> {
                    Ok(self.#name.get_or_try_init(|| self.#load_name())?.clone())
                }
                fn #load_name(&self) -> Result<#ret_type> {
                    #body
                    Ok(#ret_value)
                }
            }
        }.to_items()
    });

    if debug {
        eprintln!("{}", unparse(&tokens));
    }

    if count.load(Ordering::Acquire) == 0 {
        panic!(concat!(
            "#[cached_field] does not find matched patterns. check:\n",
            "- return type: Result<Arc<RwLock<T>>> or Result<T>\n",
            "- (last) return statement: Ok(Arc::new(RwLock::new(expr))) or Ok(expr)\n"
        ));
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use "debug" to extra logs for debugging.
    const ATTR: &str = "";

    #[test]
    fn test_cached_arc_rwlock() {
        let attr = parse(ATTR);
        let code = parse(
            r#"
            pub fn foo(&self) -> Result<Arc<RwLock<Data>>> {
                let data = calculate_data(self)?;
                Ok(Arc::new(RwLock::new(data)))
            }
"#,
        );
        assert_eq!(
            unparse(&cached_field(attr, code)),
            r#"
            pub fn foo (& self) -> Result < Arc < RwLock < Data >>> {
                Ok (self . foo . get_or_try_init (|| self . load_foo () . map (| v | Arc :: new (RwLock :: new (v)))) ? . clone ())
            }
            fn load_foo (& self) -> Result < Data > {
                let data = calculate_data (self) ?;
                Ok (data)
            }
            pub fn invalidate_foo (& self) -> Result < () > {
                if let Some (v) = self . foo . get () {
                    * v . write () = self . load_foo () ?;
                }
                Ok (())
            }"#
        );
    }

    #[test]
    fn test_cached_arc_without_rwlock() {
        let attr = parse(ATTR);
        let code = parse(
            r#"
            pub fn foo(&self) -> Result<Arc<dyn Trait>> {
                let data = calculate_data(self)?;
                Ok(Arc::new(data))
            }
"#,
        );
        assert_eq!(
            unparse(&cached_field(attr, code)),
            r#"
            pub fn foo (& self) -> Result < Arc < dyn Trait >> {
                Ok (self . foo . get_or_try_init (|| self . load_foo ()) ? . clone ())
            }
            fn load_foo (& self) -> Result < Arc < dyn Trait >> {
                let data = calculate_data (self) ?;
                Ok (Arc :: new (data))
            }"#
        );
    }

    #[test]
    fn test_impl_block() {
        let attr = parse(ATTR);
        let code = parse(
            r#"
            impl MyStruct {
                pub fn baz(&self, x: usize) -> usize {
                    x + 1
                }
                pub fn foo(&self) -> Result<String> {
                    Ok(String::new())
                }
                pub fn bar(&self) -> Result<Arc<RwLock<usize>>> {
                    Ok(Arc::new(RwLock::new(42)))
                }
            }
"#,
        );
        assert_eq!(
            unparse(&cached_field(attr, code)),
            r#"
            impl MyStruct {
                pub fn baz (& self , x : usize) -> usize { x + 1 } pub fn foo (& self) -> Result < String > {
                    Ok (self . foo . get_or_try_init (|| self . load_foo ()) ? . clone ())
                }
                fn load_foo (& self) -> Result < String > {
                    Ok (String :: new ())
                }
                pub fn bar (& self) -> Result < Arc < RwLock < usize >>> {
                    Ok (self . bar . get_or_try_init (|| self . load_bar () . map (| v | Arc :: new (RwLock :: new (v)))) ? . clone ())
                }
                fn load_bar (& self) -> Result < usize > { Ok (42) } pub fn invalidate_bar (& self) -> Result < () > {
                    if let Some (v) = self . bar . get () {
                        * v . write () = self . load_bar () ?;
                    }
                    Ok (())
                }
            }"#
        );
    }

    #[test]
    #[should_panic]
    fn test_unsupported_pattern() {
        let attr = parse(ATTR);
        let code = parse(
            r#"
            pub fn foo(&self, x: String) -> String {
                x
            }
"#,
        );
        let _ = cached_field(attr, code);
    }
}