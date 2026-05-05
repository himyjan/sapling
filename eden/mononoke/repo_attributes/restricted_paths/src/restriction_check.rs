/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! Restriction check helpers that turn restriction lookup results into
//! authorization results.

use std::sync::Arc;

use anyhow::Result;
use context::CoreContext;
use futures::future;
use mononoke_types::ChangesetId;
use mononoke_types::NonRootMPath;
use permission_checker::AclProvider;
use permission_checker::MononokeIdentity;

use crate::ManifestId;
use crate::ManifestType;
use crate::RestrictedPaths;
use crate::RestrictionCheckResult;
use crate::access_log::has_read_access_to_repo_region_acls;
use crate::access_log::is_part_of_group;

/// Source to use for path-side restriction checks.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PathRestrictionSource {
    /// Config-backed path ACLs.
    Config,
    /// AclManifest at a specific changeset.
    AclManifest(ChangesetId),
}

/// Source to use for manifest-side restriction checks.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ManifestRestrictionSource {
    /// Config-backed manifest-id store.
    Config,
    /// AclManifest pointer attached to the manifest.
    AclManifest,
}

/// Authorization flags produced by evaluating the caller against one source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AuthorizationCheckResult {
    /// Whether the caller has direct read access to every matching path ACL.
    pub(crate) has_acl_access: bool,
    /// Whether the caller is in the tooling allowlist group.
    pub(crate) is_allowlisted_tooling: bool,
    /// Whether the caller is in the rollout allowlist group.
    pub(crate) is_rollout_allowlisted: bool,
}

impl AuthorizationCheckResult {
    /// Whether the caller has read authorization through ACLs or allowlists.
    pub(crate) fn has_authorization(&self) -> bool {
        self.has_acl_access || self.is_allowlisted_tooling || self.is_rollout_allowlisted
    }
}

/// Evaluate ACL and allowlist authorization for a restricted-path access.
pub(crate) async fn check_authorization(
    ctx: &CoreContext,
    acl_provider: &Arc<dyn AclProvider>,
    acls: &[&MononokeIdentity],
    tooling_allowlist_group: Option<&str>,
    rollout_allowlist_group: Option<&str>,
) -> Result<AuthorizationCheckResult> {
    let has_acl_access = has_read_access_to_repo_region_acls(ctx, acl_provider, acls).await?;
    let is_allowlisted_tooling = tooling_allowlist_group
        .map_or(future::Either::Left(future::ok(false)), |group_name| {
            future::Either::Right(is_part_of_group(ctx, acl_provider, group_name))
        })
        .await?;
    let is_rollout_allowlisted = rollout_allowlist_group
        .map_or(future::Either::Left(future::ok(false)), |group_name| {
            future::Either::Right(is_part_of_group(ctx, acl_provider, group_name))
        })
        .await?;

    Ok(AuthorizationCheckResult {
        has_acl_access,
        is_allowlisted_tooling,
        is_rollout_allowlisted,
    })
}

/// Build the legacy restricted-path check result shape.
pub(crate) fn build_restriction_check_result(
    has_authorization: bool,
    restriction_roots: Vec<NonRootMPath>,
) -> RestrictionCheckResult {
    RestrictionCheckResult {
        has_authorization,
        restriction_roots,
    }
}

/// Check a path against one restriction source.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
pub(crate) async fn check_path_restriction(
    _ctx: &CoreContext,
    _restricted_paths: &RestrictedPaths,
    _path: NonRootMPath,
    _source: PathRestrictionSource,
) -> Result<RestrictionCheckResult> {
    unimplemented!("implemented by the follow-up extraction diff")
}

/// Check a manifest against one restriction source.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
pub(crate) async fn check_manifest_restriction(
    _ctx: &CoreContext,
    _restricted_paths: &RestrictedPaths,
    _manifest_id: ManifestId,
    _manifest_type: ManifestType,
    _source: ManifestRestrictionSource,
) -> Result<RestrictionCheckResult> {
    unimplemented!("implemented by the follow-up extraction diff")
}
