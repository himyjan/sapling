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
use permission_checker::MononokeIdentity;

use crate::ManifestId;
use crate::ManifestType;
use crate::RestrictedPaths;

/// Core restriction information for a path.
/// Does not include access check results; that is the API layer's concern
/// (see `mononoke_api::PathAccessInfo`).
#[derive(Clone, Debug, PartialEq)]
pub struct PathRestrictionInfo {
    /// The root path of this restriction (directory containing `.slacl`).
    pub restriction_root: NonRootMPath,
    /// The repo region ACL string, e.g. "REPO_REGION:repos/hg/fbsource/=project1".
    pub repo_region_acl: String,
    /// ACL for requesting access. Defaults to repo_region_acl if not configured.
    pub request_acl: String,
}

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

/// Get exact config-backed restriction info for one or more paths.
pub(crate) fn get_exact_path_restriction_from_config(
    restricted_paths: &RestrictedPaths,
    paths: &[NonRootMPath],
) -> Vec<PathRestrictionInfo> {
    paths
        .iter()
        .filter_map(|path| {
            restricted_paths
                .config_based()
                .get_acl_for_path(path)
                .map(|acl| build_config_path_restriction_info(path.clone(), acl))
        })
        .collect()
}

/// Get config-backed restriction info for one or more paths, considering ancestors.
pub(crate) fn get_path_restriction_info_from_config(
    restricted_paths: &RestrictedPaths,
    paths: &[NonRootMPath],
) -> Vec<PathRestrictionInfo> {
    paths
        .iter()
        .flat_map(|path| get_config_path_restriction_info_for_path(restricted_paths, path))
        .collect()
}

/// Find config-backed restriction roots that are descendants of the given roots.
pub(crate) fn find_restricted_descendants_from_config(
    restricted_paths: &RestrictedPaths,
    roots: &[MPath],
) -> Vec<PathRestrictionInfo> {
    let mut results: Vec<PathRestrictionInfo> = restricted_paths
        .config()
        .path_acls
        .iter()
        .filter(|(root, _)| {
            roots
                .iter()
                .any(|query_root| query_root.is_prefix_of(*root))
        })
        .map(|(root, acl)| build_config_path_restriction_info(root.clone(), acl))
        .collect();
    results.sort_by(|left, right| left.restriction_root.cmp(&right.restriction_root));
    results.dedup_by(|left, right| left.restriction_root == right.restriction_root);
    results
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

fn get_config_path_restriction_info_for_path(
    restricted_paths: &RestrictedPaths,
    path: &NonRootMPath,
) -> Vec<PathRestrictionInfo> {
    restricted_paths
        .config()
        .path_acls
        .iter()
        .filter(|(prefix, _)| prefix.is_prefix_of(path))
        .map(|(prefix, acl)| build_config_path_restriction_info(prefix.clone(), acl))
        .collect()
}

fn build_config_path_restriction_info(
    restriction_root: NonRootMPath,
    acl: &MononokeIdentity,
) -> PathRestrictionInfo {
    let repo_region_acl = acl.to_string();
    PathRestrictionInfo {
        restriction_root,
        request_acl: repo_region_acl.clone(),
        repo_region_acl,
    }
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
