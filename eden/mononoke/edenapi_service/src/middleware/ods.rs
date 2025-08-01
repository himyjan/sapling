/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use gotham::prelude::FromState;
use gotham::state::State;
use gotham_ext::middleware::MetadataState;
use gotham_ext::middleware::Middleware;
use gotham_ext::middleware::PostResponseCallbacks;
use gotham_ext::middleware::RequestLoad;
use hyper::Body;
use hyper::Response;
use hyper::StatusCode;
use permission_checker::MononokeIdentitySetExt;
use stats::prelude::*;

use crate::handlers::HandlerInfo;
use crate::handlers::SaplingRemoteApiMethod;
#[cfg(fbcode_build)]
use crate::middleware::facebook::log_ods3;
#[cfg(not(fbcode_build))]
use crate::middleware::oss::log_ods3;

define_stats! {
    prefix = "mononoke.edenapi.request";
    alter_snapshot_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    blame_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    bookmarks_duration_ms: histogram(10, 0, 500, Average, Sum, Count; P 50; P 75; P 95; P 99),
    bookmarks2_duration_ms: histogram(10, 0, 500, Average, Sum, Count; P 50; P 75; P 95; P 99),
    capabilities_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    cloud_historical_versions_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    cloud_other_repo_workspaces_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    cloud_references_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    cloud_rename_workspace_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    cloud_rollback_workspace_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    cloud_share_workspace_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 75; P 95; P 99),
    cloud_smartlog_by_version: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 75; P 95; P 99),
    cloud_smartlog_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 75; P 95; P 99),
    cloud_update_archive_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    cloud_update_references_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 95; P 99),
    cloud_workspace_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    cloud_workspaces_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    commit_graph_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    commit_graph_segments_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    commit_graph_v2_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    commit_hash_lookup_duration_ms: histogram(10, 0, 500, Average, Sum, Count; P 50; P 75; P 95; P 99),
    commit_hash_to_location_duration_ms: histogram(10, 0, 500, Average, Sum, Count; P 50; P 75; P 95; P 99),
    commit_location_to_hash_duration_ms: histogram(10, 0, 500, Average, Sum, Count; P 50; P 75; P 95; P 99),
    commit_mutations_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    commit_revlog_data_duration_ms: histogram(10, 0, 500, Average, Sum, Count; P 50; P 75; P 95; P 99),
    commit_translate_id_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    download_file_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    ephemeral_prepare_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    failure_4xx: dynamic_timeseries("{}.failure_4xx", (method: String); Rate, Sum),
    failure_5xx: dynamic_timeseries("{}.failure_5xx", (method: String); Rate, Sum),
    fetch_snapshot_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    files2_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    git_objects_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    history_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    land_stack_duration_ms: histogram(10, 0, 500, Average, Sum, Count; P 50; P 75; P 95; P 99),
    lookup_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    path_history_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    request_load: histogram(100, 0, 5000, Average; P 50; P 75; P 95; P 99),
    requests: dynamic_timeseries("{}.requests", (method: String); Rate, Sum),
    total_requests: timeseries(Rate, Sum),
    response_bytes_sent: dynamic_histogram("{}.response_bytes_sent", (method: String); 1_500_000, 0, 150_000_000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    set_bookmark_duration_ms: histogram(10, 0, 500, Average, Sum, Count; P 50; P 75; P 95; P 99),
    streaming_clone_duration_ms:  histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    success: dynamic_timeseries("{}.success", (method: String); Rate, Sum),
    suffix_query_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    trees_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    upload_bonsai_changeset_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    upload_file_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    upload_hg_changesets_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    upload_hg_filenodes_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    upload_identical_changesets_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
    upload_trees_duration_ms: histogram(100, 0, 5000, Average, Sum, Count; P 50; P 75; P 95; P 99),
}

fn log_stats(state: &mut State, status: StatusCode) -> Option<()> {
    // Proxygen can be configured to periodically send a preconfigured set of
    // requests to check server health. These requests will look like ordinary
    // user requests, but should be filtered out of the server's metrics.
    match state.try_borrow::<MetadataState>() {
        Some(state) if state.metadata().identities().is_proxygen_test_identity() => {
            return None;
        }
        _ => {}
    }

    let handler_info = state.try_borrow::<HandlerInfo>()?;
    let method = handler_info.method?;
    let request_load = RequestLoad::try_borrow_from(state).map(|r| r.0 as f64);

    let callbacks = state.try_borrow_mut::<PostResponseCallbacks>()?;

    callbacks.add(move |info| {
        if let Some(duration) = info.duration {
            let dur_ms = duration.as_millis() as i64;

            use SaplingRemoteApiMethod::*;
            match method {
                AlterSnapshot => STATS::alter_snapshot_duration_ms.add_value(dur_ms),
                Blame => STATS::blame_duration_ms.add_value(dur_ms),
                Bookmarks => STATS::bookmarks_duration_ms.add_value(dur_ms),
                Bookmarks2 => STATS::bookmarks2_duration_ms.add_value(dur_ms),
                Capabilities => STATS::capabilities_duration_ms.add_value(dur_ms),
                CloudHistoricalVersions => {
                    STATS::cloud_historical_versions_duration_ms.add_value(dur_ms)
                }
                CloudOtherRepoWorkspaces => {
                    STATS::cloud_other_repo_workspaces_duration_ms.add_value(dur_ms)
                }
                CloudReferences => STATS::cloud_references_duration_ms.add_value(dur_ms),
                CloudRenameWorkspace => STATS::cloud_rename_workspace_duration_ms.add_value(dur_ms),
                CloudRollbackWorkspace => {
                    STATS::cloud_rollback_workspace_duration_ms.add_value(dur_ms)
                }
                CloudShareWorkspace => STATS::cloud_share_workspace_duration_ms.add_value(dur_ms),
                CloudSmartlog => STATS::cloud_smartlog_duration_ms.add_value(dur_ms),
                CloudSmartlogByVersion => STATS::cloud_smartlog_by_version.add_value(dur_ms),
                CloudUpdateArchive => STATS::cloud_update_archive_duration_ms.add_value(dur_ms),
                CloudUpdateReferences => {
                    STATS::cloud_update_references_duration_ms.add_value(dur_ms)
                }
                CloudWorkspace => STATS::cloud_workspace_duration_ms.add_value(dur_ms),
                CloudWorkspaces => STATS::cloud_workspaces_duration_ms.add_value(dur_ms),
                CommitGraphSegments => STATS::commit_graph_segments_duration_ms.add_value(dur_ms),
                CommitGraphV2 => STATS::commit_graph_v2_duration_ms.add_value(dur_ms),
                CommitHashLookup => STATS::commit_hash_lookup_duration_ms.add_value(dur_ms),
                CommitHashToLocation => {
                    STATS::commit_hash_to_location_duration_ms.add_value(dur_ms)
                }
                CommitLocationToHash => {
                    STATS::commit_location_to_hash_duration_ms.add_value(dur_ms)
                }
                CommitMutations => STATS::commit_mutations_duration_ms.add_value(dur_ms),
                CommitRevlogData => STATS::commit_revlog_data_duration_ms.add_value(dur_ms),
                CommitTranslateId => STATS::commit_translate_id_duration_ms.add_value(dur_ms),
                DownloadFile => STATS::download_file_duration_ms.add_value(dur_ms),
                EphemeralPrepare => STATS::ephemeral_prepare_duration_ms.add_value(dur_ms),
                FetchSnapshot => STATS::fetch_snapshot_duration_ms.add_value(dur_ms),
                Files2 => STATS::files2_duration_ms.add_value(dur_ms),
                GitObjects => STATS::git_objects_duration_ms.add_value(dur_ms),
                History => STATS::history_duration_ms.add_value(dur_ms),
                LandStack => STATS::land_stack_duration_ms.add_value(dur_ms),
                Lookup => STATS::lookup_duration_ms.add_value(dur_ms),
                PathHistory => STATS::path_history_duration_ms.add_value(dur_ms),
                SetBookmark => STATS::set_bookmark_duration_ms.add_value(dur_ms),
                StreamingClone => STATS::streaming_clone_duration_ms.add_value(dur_ms),
                SuffixQuery => STATS::suffix_query_duration_ms.add_value(dur_ms),
                Trees => STATS::trees_duration_ms.add_value(dur_ms),
                UploadBonsaiChangeset => {
                    STATS::upload_bonsai_changeset_duration_ms.add_value(dur_ms)
                }
                UploadFile => STATS::upload_file_duration_ms.add_value(dur_ms),
                UploadHgChangesets => STATS::upload_hg_changesets_duration_ms.add_value(dur_ms),
                UploadHgFilenodes => STATS::upload_hg_filenodes_duration_ms.add_value(dur_ms),
                UploadIdenticalChangesets => {
                    STATS::upload_identical_changesets_duration_ms.add_value(dur_ms)
                }
                UploadTrees => STATS::upload_trees_duration_ms.add_value(dur_ms),
            }
        }

        let method = method.to_string();
        STATS::requests.add_value(1, (method.clone(),));
        STATS::total_requests.add_value(1);

        if status.is_success() {
            STATS::success.add_value(1, (method.clone(),));
        } else if status.is_client_error() {
            STATS::failure_4xx.add_value(1, (method.clone(),));
        } else if status.is_server_error() {
            STATS::failure_5xx.add_value(1, (method.clone(),));
        }

        if let Some(response_bytes_sent) = info.meta.as_ref().map(|m| m.body().bytes_sent) {
            STATS::response_bytes_sent.add_value(response_bytes_sent as i64, (method.clone(),))
        }
        log_ods3(info, &status, method, request_load);
    });

    if let Some(request_load) = RequestLoad::try_borrow_from(state) {
        STATS::request_load.add_value(request_load.0);
    }
    Some(())
}

pub struct OdsMiddleware;

impl OdsMiddleware {
    pub fn new() -> Self {
        OdsMiddleware
    }
}

#[async_trait::async_trait]
impl Middleware for OdsMiddleware {
    async fn outbound(&self, state: &mut State, response: &mut Response<Body>) {
        log_stats(state, response.status());
    }
}
