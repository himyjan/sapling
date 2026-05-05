/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! Low-level restriction lookup primitives.

use anyhow::Result;
use context::CoreContext;
use mononoke_types::ChangesetId;
use mononoke_types::MPath;
use mononoke_types::NonRootMPath;

use crate::ManifestId;
use crate::ManifestType;
use crate::PathRestrictionInfo;
use crate::RestrictedPaths;

/// Core restriction information for a manifest access.
#[derive(Clone, Debug, PartialEq)]
pub struct ManifestRestrictionInfo {
    /// The matched restriction root when it is known from the source.
    pub restriction_root: Option<NonRootMPath>,
    /// The repo region ACL string, e.g. "REPO_REGION:repos/hg/fbsource/=project1".
    pub repo_region_acl: String,
    /// ACL for requesting access. Defaults to repo_region_acl if not configured.
    pub request_acl: String,
}

/// Get exact path restriction info for one or more paths.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
pub(crate) async fn get_exact_path_restriction(
    _restricted_paths: &RestrictedPaths,
    _ctx: &CoreContext,
    _cs_id: Option<ChangesetId>,
    _paths: &[NonRootMPath],
) -> Result<Vec<PathRestrictionInfo>> {
    unimplemented!("implemented by the follow-up extraction diff")
}

/// Get restriction info for one or more paths, considering ancestor restrictions.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
pub(crate) async fn get_path_restriction_info(
    _restricted_paths: &RestrictedPaths,
    _ctx: &CoreContext,
    _cs_id: Option<ChangesetId>,
    _paths: &[NonRootMPath],
) -> Result<Vec<PathRestrictionInfo>> {
    unimplemented!("implemented by the follow-up extraction diff")
}

/// Check if a path is itself a restriction root.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
pub(crate) async fn is_restriction_root(
    _restricted_paths: &RestrictedPaths,
    _ctx: &CoreContext,
    _cs_id: Option<ChangesetId>,
    _path: &NonRootMPath,
) -> Result<bool> {
    unimplemented!("implemented by the follow-up extraction diff")
}

/// Check if a path is restricted, considering ancestor directories.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
pub(crate) async fn is_restricted_path(
    _restricted_paths: &RestrictedPaths,
    _ctx: &CoreContext,
    _cs_id: Option<ChangesetId>,
    _path: &NonRootMPath,
) -> Result<bool> {
    unimplemented!("implemented by the follow-up extraction diff")
}

/// Find all restriction roots that are descendants of any of the given root paths.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
pub(crate) async fn find_restricted_descendants(
    _restricted_paths: &RestrictedPaths,
    _ctx: &CoreContext,
    _cs_id: Option<ChangesetId>,
    _roots: Vec<MPath>,
) -> Result<Vec<PathRestrictionInfo>> {
    unimplemented!("implemented by the follow-up extraction diff")
}

/// Lookup restriction info for a manifest access through the config-backed source.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
pub(crate) async fn get_manifest_restriction_info_from_config(
    _restricted_paths: &RestrictedPaths,
    _ctx: &CoreContext,
    _manifest_id: &ManifestId,
    _manifest_type: &ManifestType,
) -> Result<Vec<ManifestRestrictionInfo>> {
    unimplemented!("implemented by the follow-up extraction diff")
}

/// Lookup restriction info for a manifest access through the AclManifest source.
#[expect(dead_code, reason = "interface skeleton for the split stack")]
pub(crate) async fn get_manifest_restriction_info_from_acl_manifest(
    _restricted_paths: &RestrictedPaths,
    _ctx: &CoreContext,
    _manifest_id: &ManifestId,
    _manifest_type: &ManifestType,
) -> Result<Vec<ManifestRestrictionInfo>> {
    unimplemented!("implemented by the follow-up extraction diff")
}
