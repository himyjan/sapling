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
use cmdutil::Config;
#[cfg(feature = "fb")]
use cmdutil::ConfigExt;
use cmdutil::Repo;
use cmdutil::Result;
use cmdutil::define_flags;

define_flags! {
    pub struct DebugDynamicConfigOpts {
        /// Host name to fetch a canary config from.
        canary: Option<String>,
    }
}

pub fn run(ctx: ReqCtx<DebugDynamicConfigOpts>, repo: Option<&Repo>) -> Result<u8> {
    #[cfg(feature = "fb")]
    {
        use configloader::fb::FbConfigMode;
        use repo_minimal_info::RepoMinimalInfo;
        let (info, repo_name) = match repo {
            Some(repo) => (
                Some(RepoMinimalInfo::from_repo_root(
                    repo.shared_path().to_path_buf(),
                )?),
                repo.repo_name(),
            ),
            None => (None, None),
        };

        let config = ctx.config().clone();
        let username = config
            .get("ui", "username")
            .map_or_else(|| "".to_string(), |u| u.to_string());

        let mode = FbConfigMode::default();

        configloader::hg::maybe_refresh_internalconfig_on_disk(
            mode,
            info.as_ref(),
            repo_name,
            ctx.opts.canary,
            username,
            config.get_opt("auth_proxy", "unix_socket_path")?,
            // We are explicitly requesting a config refresh - do not allow using baked in
            // remote config snapshot in case remote fetch fails.
            false,
            config
                .get("experimental", "dynamic-config-domain-override")
                .and_then(|d| configloader::fb::Domain::from_str(d.as_ref()).ok()),
        )?;
    }
    #[cfg(not(feature = "fb"))]
    let _ = (ctx, repo);

    Ok(0)
}

pub fn aliases() -> &'static str {
    "debugrefreshconfig|debugdynamicconfig"
}

pub fn doc() -> &'static str {
    "refresh the internal configuration"
}

pub fn synopsis() -> Option<&'static str> {
    None
}

pub fn enable_cas() -> bool {
    false
}
