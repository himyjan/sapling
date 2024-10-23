/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::bail;
use anyhow::ensure;
use anyhow::Context as _;
use anyhow::Result;
use minibytes::Text;
use once_cell::sync::OnceCell;
use storemodel::SerializationFormat;
use types::Id20;

pub use crate::CommitFields;

/// Holds the Git commit text. Fields can be lazily fields.
pub struct GitCommitLazyFields {
    text: Text,
    fields: OnceCell<Box<GitCommitFields>>,
}

/// Fields of a git commit. Enough information to serialize to text.
#[derive(Default)]
pub struct GitCommitFields {
    tree: Id20,
    parents: Vec<Id20>,
    author: Text,
    date: Date,
    committer: Text,
    committer_date: Date,
    message: Text,
}

type Date = (u64, i32);

impl GitCommitFields {
    fn from_text(text: &Text) -> Result<Self> {
        // tree {tree_sha}
        // {parents}
        // author {author_name} <{author_email}> {author_date_seconds} {author_date_timezone}
        // committer {committer_name} <{committer_email}> {committer_date_seconds} {committer_date_timezone}
        //
        // {commit message}
        let mut result = Self::default();
        let mut last_pos = 0;
        for pos in memchr::memchr_iter(b'\n', text.as_bytes()) {
            let line = &text[last_pos..pos];
            if let Some(hex) = line.strip_prefix("tree ") {
                result.tree = Id20::from_hex(hex.as_bytes())?;
            } else if let Some(hex) = line.strip_prefix("parent ") {
                result.parents.push(Id20::from_hex(hex.as_bytes())?);
            } else if let Some(line) = line.strip_prefix("author ") {
                (result.author, result.date) = parse_name_date(text.slice_to_bytes(line))?;
            } else if let Some(line) = line.strip_prefix("committer ") {
                (result.committer, result.committer_date) =
                    parse_name_date(text.slice_to_bytes(line))?;
            } else {
                // Treat the rest as "message".
                result.message = text.slice(pos + 1..);
                break;
            }
            last_pos = pos + 1;
        }
        Ok(result)
    }
}

impl GitCommitLazyFields {
    pub fn new(text: Text) -> Self {
        Self {
            text,
            fields: Default::default(),
        }
    }

    pub fn fields(&self) -> Result<&GitCommitFields> {
        let fields = self
            .fields
            .get_or_try_init(|| GitCommitFields::from_text(&self.text).map(Box::new))?;
        Ok(fields)
    }
}

impl CommitFields for GitCommitLazyFields {
    fn root_tree(&self) -> Result<Id20> {
        if let Some(fields) = self.fields.get() {
            return Ok(fields.tree);
        }
        // Extract tree without parsing all fields.
        if let Some(rest) = self.text.strip_prefix("tree ") {
            if let Some(hex) = rest.get(..Id20::hex_len()) {
                return Ok(Id20::from_hex(hex.as_bytes())?);
            }
        }
        bail!("invalid git commit format: {}", &self.text);
    }

    fn author_name(&self) -> Result<&str> {
        Ok(self.fields()?.author.as_ref())
    }

    fn committer_name(&self) -> Result<Option<&str>> {
        Ok(Some(self.fields()?.committer.as_ref()))
    }

    fn author_date(&self) -> Result<(u64, i32)> {
        Ok(self.fields()?.date)
    }

    fn committer_date(&self) -> Result<Option<(u64, i32)>> {
        Ok(Some(self.fields()?.committer_date))
    }

    fn parents(&self) -> Result<Option<&[Id20]>> {
        Ok(Some(&self.fields()?.parents))
    }

    fn description(&self) -> Result<&str> {
        Ok(&self.fields()?.message)
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::Git
    }

    fn raw_text(&self) -> &[u8] {
        self.text.as_bytes()
    }
}

fn parse_name_date(line: Text) -> Result<(Text, Date)> {
    // {name} <{email}> {date_seconds} {date_timezone}
    let mut parts = line.rsplitn(3, ' ');
    let tz_seconds = {
        // +HHMM or -HHMM
        let tz_str = parts.next().context("missing timezone")?;
        ensure!(tz_str.len() == 5, "invalid git timezone: {}", tz_str);
        // Git's "-0700" = Hg's "25200"
        let sign = if tz_str.starts_with('-') { 1 } else { -1 };
        let hours = tz_str[1..3].parse::<i32>()?;
        let minutes = tz_str[3..5].parse::<i32>()?;
        (hours * 3600 + minutes * 60) * sign
    };
    let date_seconds = {
        let date_str = parts.next().context("missing date")?;
        date_str.parse::<u64>()?
    };
    let name = {
        let name_str = parts.next().context("missing name")?;
        line.slice_to_bytes(name_str)
    };
    Ok((name, (date_seconds, tz_seconds)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_commit_basic() {
        let text = r#"tree 98edb6a9c7a48cae7a1ed9a39600952547daaebb
parent 8e1b0fe96dc24617d192af955208ddd485888bd6
parent 7769e9429c9c3563110d296e745b8e45581bbe1f
author Alice 1 <a@example.com> 1714100000 -0700
committer Bob 2 <b@example.com> 1714200000 +0800

This is the commit message.

Signed-off-by: Alice <a@example.com>
"#;
        let fields = GitCommitLazyFields::new(text.into());
        assert_eq!(
            fields.root_tree().unwrap().to_hex(),
            "98edb6a9c7a48cae7a1ed9a39600952547daaebb"
        );
        assert_eq!(fields.author_name().unwrap(), "Alice 1 <a@example.com>");
        assert_eq!(
            fields.committer_name().unwrap().unwrap(),
            "Bob 2 <b@example.com>"
        );
        assert_eq!(fields.author_date().unwrap(), (1714100000, 25200));
        assert_eq!(
            fields.committer_date().unwrap().unwrap(),
            (1714200000, -28800)
        );
        assert_eq!(
            format!("{:?}", fields.parents().unwrap().unwrap()),
            "[HgId(\"8e1b0fe96dc24617d192af955208ddd485888bd6\"), HgId(\"7769e9429c9c3563110d296e745b8e45581bbe1f\")]"
        );
        assert_eq!(
            fields.description().unwrap(),
            "This is the commit message.\n\nSigned-off-by: Alice <a@example.com>\n"
        );
        assert_eq!(fields.raw_text(), text.as_bytes());
        assert_eq!(
            fields.root_tree().unwrap().to_hex(),
            "98edb6a9c7a48cae7a1ed9a39600952547daaebb"
        );
    }
}