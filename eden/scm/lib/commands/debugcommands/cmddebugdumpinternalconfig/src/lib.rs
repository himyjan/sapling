/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#[cfg(feature = "fb")]
use std::str::FromStr;

use clidispatch::ReqCtx;
#[cfg(feature = "fb")]
use cmdutil::ConfigExt;
use cmdutil::Repo;
use cmdutil::Result;
use cmdutil::define_flags;
#[cfg(feature = "fb")]
use configloader::hg::calculate_internalconfig;

define_flags! {
    pub struct DebugDumpConfigOpts {
        /// repository name
        reponame: Option<String>,

        /// user name
        username: String,

        /// host name to fetch a canary config from
        canary: Option<String>,

        /// config mode
        mode: String,

        #[args]
        args: Vec<String>,
    }
}

pub fn run(ctx: ReqCtx<DebugDumpConfigOpts>, repo: Option<&Repo>) -> Result<u8> {
    #[cfg(feature = "fb")]
    {
        use std::ops::Deref;

        use configloader::fb::FbConfigMode;

        let config = ctx.config().clone();

        let reponame = ctx.opts.reponame;
        let mut username = ctx.opts.username;
        if username.is_empty() {
            username = config.get_opt("ui", "username")?.unwrap_or_default();
        }
        let canary = ctx.opts.canary;

        let temp_dir = std::env::temp_dir();
        let mode = if ctx.opts.mode.is_empty() {
            FbConfigMode::default()
        } else {
            FbConfigMode::from_str(&ctx.opts.mode)?
        };
        let generated = calculate_internalconfig(
            mode,
            temp_dir,
            reponame,
            canary,
            username,
            config.get_opt("auth_proxy", "unix_socket_path")?,
            true,
            config
                .get("experimental", "dynamic-config-domain-override")
                .and_then(|d| configloader::fb::Domain::from_str(d.as_ref()).ok()),
            repo.map(|r| r.deref()),
        )?;

        if ctx.opts.args.is_empty() {
            ctx.core.io.write(generated.to_string())?;
        } else {
            for arg in ctx.opts.args {
                let split: Vec<_> = arg.splitn(2, '.').collect();
                if let [section, name] = split[..] {
                    let value: String = generated.get_opt(section, name)?.unwrap_or_default();
                    ctx.core.io.write(format!("{}\n", value))?;
                }
            }
        }
    }
    #[cfg(not(feature = "fb"))]
    let _ = (ctx, repo);

    Ok(0)
}

pub fn aliases() -> &'static str {
    "debugdumpinternalconfig|debugdumpdynamicconfig"
}

pub fn doc() -> &'static str {
    "print the internal configuration

Without arguments, print the dynamic config in hgrc format.
Otherwise, print config values specified by the arguments.
An argument should be in the format ``section.name``.
"
}

pub fn synopsis() -> Option<&'static str> {
    None
}

pub fn enable_cas() -> bool {
    false
}
