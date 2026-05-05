/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::str::FromStr;
use std::sync::Arc;

use anyhow::Context;
use context::CoreContext;
use futures::StreamExt;
use futures::TryStreamExt;
use futures::stream;
use mononoke_types::NonRootMPath;
use permission_checker::AclProvider;
use permission_checker::MononokeIdentity;
use restricted_paths::PathRestrictionInfo;
use restricted_paths::has_read_access_to_repo_region_acls;

use crate::errors::MononokeError;

/// Access check result for a restricted path.
#[derive(Clone, Debug, PartialEq)]
pub struct PathAccessInfo {
    /// Core restriction info from the restricted_paths crate.
    pub restriction: PathRestrictionInfo,

    /// Whether the caller has access. None if not checked.
    pub has_access: Option<bool>,
}

impl PathAccessInfo {
    /// Convenience accessor for the restriction root.
    pub fn restriction_root(&self) -> &NonRootMPath {
        &self.restriction.restriction_root
    }

    /// Convenience accessor for the repo region ACL.
    pub fn repo_region_acl(&self) -> &str {
        &self.restriction.repo_region_acl
    }

    /// Convenience accessor for the request ACL.
    pub fn request_acl(&self) -> &str {
        &self.restriction.request_acl
    }
}

pub(crate) async fn build_path_access_info(
    ctx: &CoreContext,
    acl_provider: &Arc<dyn AclProvider>,
    restriction_infos: Vec<PathRestrictionInfo>,
    check_permissions: bool,
) -> Result<Vec<PathAccessInfo>, MononokeError> {
    if !check_permissions {
        return Ok(restriction_infos
            .into_iter()
            .map(|restriction| PathAccessInfo {
                restriction,
                has_access: None,
            })
            .collect());
    }

    stream::iter(restriction_infos)
        .map(|restriction| {
            let acl_provider = Arc::clone(acl_provider);
            async move {
                let acl = MononokeIdentity::from_str(&restriction.repo_region_acl)
                    .context("Failed to parse repo_region_acl")?;
                let has_access =
                    has_read_access_to_repo_region_acls(ctx, &acl_provider, &[&acl]).await?;
                Ok::<_, MononokeError>(PathAccessInfo {
                    restriction,
                    has_access: Some(has_access),
                })
            }
        })
        .buffered(100)
        .try_collect()
        .await
}

/// Information about restricted path changes in a changeset.
#[derive(Clone, Debug, PartialEq)]
pub struct RestrictedPathsChangesInfo {
    /// Changed paths that fall under restrictions, grouped by restriction root.
    pub restricted_changes: Vec<RestrictedChangeGroup>,
}

/// A group of changed paths that share the same restriction root.
#[derive(Clone, Debug, PartialEq)]
pub struct RestrictedChangeGroup {
    /// The restriction root and access info covering these changes.
    pub restriction_info: PathAccessInfo,
    // TODO(T248660146): remove this field and `RestrictedChangeGroup` if there's
    // no need to use it for now.
    /// The changed paths under this restriction root.
    pub changed_paths: Vec<NonRootMPath>,
}
