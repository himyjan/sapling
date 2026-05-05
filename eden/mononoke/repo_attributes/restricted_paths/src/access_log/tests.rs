/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use context::CoreContext;
use fbinit::FacebookInit;
use metaconfig_types::AclManifestMode;
use mononoke_macros::mononoke;
use mononoke_types::NonRootMPath;
use mononoke_types::RepositoryId;
use permission_checker::MononokeIdentity;
use scuba_ext::MononokeScubaSampleBuilder;
use serde_json::Value;

use super::RestrictedPathAccessData;
use super::SourceRestrictionCheckResult;
use super::SourceRestrictionResult;
use crate::ManifestId;
use crate::ManifestType;

struct ShadowComparisonFieldFixture {
    ctx: CoreContext,
    repo_id: RepositoryId,
    acl_manifest_mode: AclManifestMode,
    config_result: Option<SourceRestrictionResult>,
    acl_manifest_result: Option<SourceRestrictionResult>,
    access_data: RestrictedPathAccessData,
    scuba: MononokeScubaSampleBuilder,
    log_path: PathBuf,
}

impl ShadowComparisonFieldFixture {
    fn new(
        fb: FacebookInit,
        config_result: Option<SourceRestrictionResult>,
        acl_manifest_result: Option<SourceRestrictionResult>,
        access_data: RestrictedPathAccessData,
    ) -> Result<Self> {
        let temp_log_file = tempfile::NamedTempFile::new()?;
        let log_path = temp_log_file.into_temp_path().keep()?;
        let scuba = MononokeScubaSampleBuilder::with_discard().with_log_file(&log_path)?;

        Ok(Self {
            ctx: CoreContext::test_mock(fb),
            repo_id: RepositoryId::new(1),
            acl_manifest_mode: AclManifestMode::Shadow,
            config_result,
            acl_manifest_result,
            access_data,
            scuba,
            log_path,
        })
    }

    #[expect(
        dead_code,
        reason = "shadow comparison assertion tests start logging samples in the next diff"
    )]
    fn log_with(
        self,
        log_results: impl FnOnce(
            &CoreContext,
            RepositoryId,
            Option<&SourceRestrictionResult>,
            Option<&SourceRestrictionResult>,
            AclManifestMode,
            RestrictedPathAccessData,
            MononokeScubaSampleBuilder,
        ) -> Result<()>,
    ) -> Result<Vec<serde_json::Map<String, Value>>> {
        let Self {
            ctx,
            repo_id,
            acl_manifest_mode,
            config_result,
            acl_manifest_result,
            access_data,
            scuba,
            log_path,
        } = self;

        log_results(
            &ctx,
            repo_id,
            config_result.as_ref(),
            acl_manifest_result.as_ref(),
            acl_manifest_mode,
            access_data,
            scuba,
        )?;
        read_logged_samples(&log_path)
    }
}

// What it tests: Shadow logging can surface compact comparison fields for
// config and AclManifest results.
// Expected: source attribution and mismatch summary fields are emitted.
#[mononoke::fbinit_test]
async fn test_shadow_mismatch_summary_fields_are_logged(fb: FacebookInit) -> Result<()> {
    let fixture = ShadowComparisonFieldFixture::new(
        fb,
        Some(restricted_result(
            false,
            false,
            "config_acl",
            Some("config/restricted"),
        )?),
        Some(restricted_result(
            true,
            true,
            "acl_manifest_acl",
            Some("acl_manifest/restricted"),
        )?),
        full_path_access_data()?,
    )?;

    observe_fixture_shape(fixture);
    // TODO(T248660053): assert shadow_mismatch, shadow_mismatch_detail,
    // acl_manifest_mode, and considered_restricted_by.
    Ok(())
}

// What it tests: Shadow logging records the parity case where both sources
// report the same restricted result.
// Expected: a row is emitted without mismatch detail when both sources agree.
#[mononoke::fbinit_test]
async fn test_shadow_matching_restricted_sources_log_row_without_mismatch(
    fb: FacebookInit,
) -> Result<()> {
    let fixture = ShadowComparisonFieldFixture::new(
        fb,
        Some(restricted_result(
            false,
            false,
            "shared_acl",
            Some("shared/restricted"),
        )?),
        Some(restricted_result(
            false,
            false,
            "shared_acl",
            Some("shared/restricted"),
        )?),
        full_path_access_data()?,
    )?;

    observe_fixture_shape(fixture);
    // TODO(T248660053): assert matching restricted sources log a row with
    // shadow_mismatch=false and no shadow_mismatch_detail.
    Ok(())
}

// What it tests: Shadow aggregate fields stay config-authoritative while
// AclManifest contributes comparison-only telemetry.
// Expected: top-level aggregate fields are derived from config, while
// AclManifest disagreement is recorded in the mismatch summary.
#[mononoke::fbinit_test]
async fn test_shadow_aggregate_fields_stay_config_authoritative(fb: FacebookInit) -> Result<()> {
    let fixture = ShadowComparisonFieldFixture::new(
        fb,
        Some(restricted_result(
            false,
            false,
            "config_acl",
            Some("config/restricted"),
        )?),
        Some(unrestricted_result(Some(vec![]))),
        manifest_access_data(ManifestType::HgAugmented),
    )?;

    observe_fixture_shape(fixture);
    // TODO(T248660053): assert config remains authoritative for top-level
    // authorization and restriction fields.
    Ok(())
}

// What it tests: Shadow comparison errors are logged without changing the
// config-authoritative aggregate result.
// Expected: the AclManifest error and mismatch summary fields are populated
// while top-level authorization still comes from the config source.
#[mononoke::fbinit_test]
async fn test_shadow_comparison_errors_are_logged(fb: FacebookInit) -> Result<()> {
    let fixture = ShadowComparisonFieldFixture::new(
        fb,
        Some(unrestricted_result(Some(vec![]))),
        Some(error_result("acl manifest lookup failed")),
        full_path_access_data()?,
    )?;

    observe_fixture_shape(fixture);
    // TODO(T248660053): assert acl_manifest_error and shadow_mismatch_detail
    // are present without an authorization failure.
    Ok(())
}

// What it tests: Shadow emits error-only rows when both sources fail.
// Expected: both source-specific error fields and mismatch summary are present
// without top-level aggregate authorization fields.
#[mononoke::fbinit_test]
async fn test_shadow_error_only_rows_are_logged(fb: FacebookInit) -> Result<()> {
    let fixture = ShadowComparisonFieldFixture::new(
        fb,
        Some(error_result("config lookup failed")),
        Some(error_result("acl manifest lookup failed")),
        full_path_access_data()?,
    )?;

    observe_fixture_shape(fixture);
    // TODO(T248660053): assert both source errors are logged without aggregate
    // authorization fields.
    Ok(())
}

// What it tests: a skipped comparison source stays distinct from an
// unrestricted source.
// Expected: skipped AclManifest comparison does not produce an error, mismatch
// detail, or AclManifest source attribution.
#[mononoke::fbinit_test]
async fn test_shadow_skipped_comparison_source_is_not_logged(fb: FacebookInit) -> Result<()> {
    let fixture = ShadowComparisonFieldFixture::new(
        fb,
        Some(restricted_result(
            false,
            false,
            "config_acl",
            Some("config/restricted"),
        )?),
        None,
        manifest_access_data(ManifestType::Hg),
    )?;

    observe_fixture_shape(fixture);
    // TODO(T248660053): assert skipped sources do not populate acl_manifest_error,
    // shadow_mismatch, or AclManifest in considered_restricted_by.
    Ok(())
}

// What it tests: successful unrestricted source results do not emit a row.
// Expected: no Scuba row is written when both sources are unrestricted.
#[mononoke::fbinit_test]
async fn test_shadow_unrestricted_sources_do_not_log_rows(fb: FacebookInit) -> Result<()> {
    let fixture = ShadowComparisonFieldFixture::new(
        fb,
        Some(unrestricted_result(Some(vec![]))),
        Some(unrestricted_result(Some(vec![]))),
        full_path_access_data()?,
    )?;

    observe_fixture_shape(fixture);
    // TODO(T248660053): assert unrestricted source results do not log a row.
    Ok(())
}

fn restricted_result(
    has_authorization: bool,
    has_acl_access: bool,
    acl_name: &str,
    restriction_path: Option<&str>,
) -> Result<SourceRestrictionResult> {
    Ok(Ok(Arc::new(SourceRestrictionCheckResult::new(
        has_authorization,
        has_acl_access,
        vec![MononokeIdentity::new("REPO_REGION", acl_name)],
        restriction_path
            .map(|path| NonRootMPath::new(path).map(|path| vec![path]))
            .transpose()?,
        false,
        false,
    ))))
}

fn unrestricted_result(restriction_paths: Option<Vec<NonRootMPath>>) -> SourceRestrictionResult {
    Ok(Arc::new(SourceRestrictionCheckResult::unrestricted(
        restriction_paths,
    )))
}

fn error_result(message: &str) -> SourceRestrictionResult {
    Err(Arc::new(anyhow!("{message}")))
}

fn full_path_access_data() -> Result<RestrictedPathAccessData> {
    Ok(RestrictedPathAccessData::FullPath {
        full_path: NonRootMPath::new("requested/path")?,
    })
}

fn manifest_access_data(manifest_type: ManifestType) -> RestrictedPathAccessData {
    RestrictedPathAccessData::Manifest(
        ManifestId::from("1111111111111111111111111111111111111111"),
        manifest_type,
    )
}

fn observe_fixture_shape(fixture: ShadowComparisonFieldFixture) {
    let ShadowComparisonFieldFixture {
        ctx,
        repo_id,
        acl_manifest_mode,
        config_result,
        acl_manifest_result,
        access_data,
        scuba,
        log_path,
    } = fixture;

    let _ = (ctx, repo_id, acl_manifest_mode, scuba, log_path);
    observe_source_result(config_result);
    observe_source_result(acl_manifest_result);
    observe_access_data(access_data);
}

fn observe_source_result(result: Option<SourceRestrictionResult>) {
    match result {
        Some(Ok(result)) => {
            let _ = (
                result.has_authorization,
                result.has_acl_access,
                &result.restriction_acls,
                &result.restriction_paths,
                result.is_allowlisted_tooling,
                result.is_rollout_allowlisted,
            );
        }
        Some(Err(err)) => {
            let _ = err;
        }
        None => {}
    }
}

fn observe_access_data(access_data: RestrictedPathAccessData) {
    match access_data {
        RestrictedPathAccessData::Manifest(manifest_id, manifest_type) => {
            let _ = (manifest_id, manifest_type);
        }
        RestrictedPathAccessData::FullPath { full_path } => {
            let _ = full_path;
        }
    }
}

fn read_logged_samples(log_path: &std::path::Path) -> Result<Vec<serde_json::Map<String, Value>>> {
    let contents = std::fs::read_to_string(log_path)
        .with_context(|| format!("failed to read scuba log {}", log_path.display()))?;
    contents
        .lines()
        .map(|line| {
            let json: Value = serde_json::from_str(line)
                .with_context(|| format!("failed to parse scuba row: {line}"))?;
            flatten_scuba_sample(&json)
        })
        .collect()
}

fn flatten_scuba_sample(json: &Value) -> Result<serde_json::Map<String, Value>> {
    let top_level = json
        .as_object()
        .ok_or_else(|| anyhow!("top-level scuba row should be a JSON object"))?;
    Ok(top_level
        .values()
        .filter_map(Value::as_object)
        .flat_map(|category| category.iter())
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect())
}

fn sample_field(sample: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    match sample.get(key) {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Bool(value)) => Some(value.to_string()),
        Some(other) => Some(other.to_string()),
        None => None,
    }
}

#[expect(
    dead_code,
    reason = "shadow comparison assertion tests start using logged samples in the next diff"
)]
fn sample_json_field(sample: &serde_json::Map<String, Value>, key: &str) -> Result<Option<Value>> {
    sample_field(sample, key)
        .map(|value| {
            serde_json::from_str(&value)
                .with_context(|| format!("failed to parse {key} as JSON: {value}"))
        })
        .transpose()
}

#[expect(
    dead_code,
    reason = "shadow comparison assertion tests start using logged samples in the next diff"
)]
fn sample_array(sample: &serde_json::Map<String, Value>, key: &str) -> Vec<String> {
    match sample.get(key).and_then(Value::as_array) {
        Some(values) => values
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect(),
        None => Vec::new(),
    }
}
