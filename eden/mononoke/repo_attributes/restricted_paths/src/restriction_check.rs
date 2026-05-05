/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! Restriction check helpers that turn restriction lookup results into
//! authorization results.

use anyhow::Result;
use context::CoreContext;
use mononoke_types::ChangesetId;
use mononoke_types::NonRootMPath;

use crate::ManifestId;
use crate::ManifestType;
use crate::RestrictedPaths;
use crate::RestrictionCheckResult;

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
