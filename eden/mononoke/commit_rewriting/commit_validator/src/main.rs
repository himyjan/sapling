/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#![feature(auto_traits)]

//! Mononoke Cross Repo validator job
//!
//! This is a special job used to validate that cross-repo sync,
//! produced correct results

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::format_err;
use bookmarks::BookmarkKey;
use bookmarks::BookmarkUpdateLogArc;
use bookmarks::BookmarkUpdateLogEntry;
use bookmarks::BookmarkUpdateLogId;
use bookmarks::BookmarkUpdateLogRef;
use bookmarks::Freshness;
use clientinfo::ClientEntryPoint;
use clientinfo::ClientInfo;
use context::CoreContext;
use context::SessionContainer;
use fbinit::FacebookInit;
use futures::future;
use futures::stream;
use futures::stream::Stream;
use futures::stream::StreamExt;
use futures::stream::TryStreamExt;
use metadata::Metadata;
use mononoke_app::MononokeApp;
use mononoke_app::MononokeAppBuilder;
use mononoke_app::args::AsRepoArg;
use mononoke_app::monitoring::AliveService;
use mononoke_app::monitoring::MonitoringAppExtension;
use mutable_counters::MutableCountersRef;
use repo_identity::RepoIdentityRef;
use scuba_ext::MononokeScubaSampleBuilder;
use slog::info;

mod cli;
mod repo;
mod reporting;
mod setup;
mod tail;
mod validation;

use crate::cli::MononokeCommitValidatorArgs;
use crate::cli::SubcommandValidator::Once;
use crate::cli::SubcommandValidator::Tail;
use crate::repo::Repo;
use crate::setup::format_counter;
use crate::setup::get_start_id;
use crate::setup::get_validation_helpers;
use crate::tail::QueueSize;
use crate::tail::tail_entries;
use crate::validation::EntryCommitId;
use crate::validation::ValidationHelpers;
use crate::validation::get_entry_with_small_repo_mappings;
use crate::validation::prepare_entry;
use crate::validation::unfold_bookmarks_update_log_entry;
use crate::validation::validate_entry;

const SERVICE_NAME: &str = "mononoke_x_repo_commit_validator";

fn validate_stream<'a>(
    ctx: &'a CoreContext,
    validation_helpers: &'a ValidationHelpers,
    entries: impl Stream<Item = Result<(BookmarkUpdateLogEntry, QueueSize), Error>> + 'a,
) -> impl Stream<Item = Result<EntryCommitId, Error>> + 'a {
    entries
        .then(move |bookmarks_update_log_entry_res| async move {
            unfold_bookmarks_update_log_entry(
                ctx,
                bookmarks_update_log_entry_res?,
                validation_helpers,
            )
            .await
        })
        .try_flatten()
        .then(move |res_entry| async move {
            get_entry_with_small_repo_mappings(ctx, res_entry?, validation_helpers).await
        })
        .filter_map(|maybe_entry_res| future::ready(maybe_entry_res.transpose()))
        .then(move |res_entry| async move {
            prepare_entry(ctx, res_entry?, validation_helpers).await
        })
        .map(|res_of_option| res_of_option.transpose())
        .filter_map(future::ready)
        .then(move |prepared_entry| async move {
            let prepared_entry = prepared_entry?;
            let entry_id = prepared_entry.entry_id.clone();

            validate_entry(ctx, prepared_entry, validation_helpers).await?;

            Ok(entry_id)
        })
}

async fn run_in_tailing_mode(
    ctx: &CoreContext,
    repo: Repo,
    skip_bookmarks: HashSet<BookmarkKey>,
    validation_helpers: ValidationHelpers,
    start_id: BookmarkUpdateLogId,
    scuba_sample: MononokeScubaSampleBuilder,
) -> Result<(), Error> {
    info!(
        ctx.logger(),
        "Starting to tail commits from id {}", start_id
    );
    let counter_name = format_counter();
    let stream_of_entries = tail_entries(
        ctx.clone(),
        start_id,
        skip_bookmarks,
        repo.repo_identity().id(),
        repo.bookmark_update_log_arc(),
        scuba_sample,
    );

    validate_stream(ctx, &validation_helpers, stream_of_entries)
        .then(
            |validated_entry_id_res: Result<EntryCommitId, Error>| async {
                let entry_id = validated_entry_id_res?;
                if entry_id.last_commit_for_bookmark_move() {
                    let id = entry_id.bookmarks_update_log_entry_id;
                    repo.mutable_counters()
                        .set_counter(ctx, &counter_name, id.try_into()?, None)
                        .await?;
                }

                Result::<_, Error>::Ok(())
            },
        )
        .try_for_each(|_| future::ready(Ok(())))
        .await
}

async fn run_in_once_mode(
    ctx: &CoreContext,
    repo: Repo,
    validation_helpers: ValidationHelpers,
    entry_id: BookmarkUpdateLogId,
) -> Result<(), Error> {
    let bookmark_update_log = repo.bookmark_update_log();
    let entries: Vec<Result<(BookmarkUpdateLogEntry, QueueSize), Error>> = bookmark_update_log
        .read_next_bookmark_log_entries(
            ctx.clone(),
            BookmarkUpdateLogId(u64::from(entry_id) - 1),
            1, /* limit */
            Freshness::MaybeStale,
        )
        .map_ok(|entry: BookmarkUpdateLogEntry| (entry, QueueSize(0)))
        .collect()
        .await;

    if entries.is_empty() {
        return Err(format_err!(
            "No entries for {} with id >{}",
            repo.repo_identity().id(),
            u64::from(entry_id) - 1
        ));
    }

    let stream_of_entries = stream::iter(entries);
    validate_stream(ctx, &validation_helpers, stream_of_entries)
        .try_for_each(|_| future::ready(Ok(())))
        .await
}

async fn run(fb: FacebookInit, ctx: CoreContext, app: MononokeApp) -> Result<(), Error> {
    let env = app.environment();

    let args: MononokeCommitValidatorArgs = app.args::<MononokeCommitValidatorArgs>()?;
    let repo_arg = args.repo.as_repo_arg();
    let (_, repo_config) = app.repo_config(repo_arg)?;

    let repo: Repo = app.open_repo(&args.repo).await?;
    let mysql_options = &env.mysql_options;
    let readonly_storage = env.readonly_storage;
    let scuba_sample = ctx.scuba();
    let skip_bookmarks = repo_config
        .cross_repo_commit_validation_config
        .as_ref()
        .map_or_else(HashSet::new, |conf| conf.skip_bookmarks.clone());
    let validation_helpers = get_validation_helpers(
        fb,
        ctx.clone(),
        &app,
        repo.clone(),
        repo_config,
        mysql_options.clone(),
        readonly_storage.clone(),
        scuba_sample.clone(),
    )
    .await
    .context("While instantiating commit syncers")?;

    let subcommand = args.subcommand;

    match subcommand {
        Once { entry_id } => run_in_once_mode(&ctx, repo, validation_helpers, entry_id).await,
        Tail { start_id } => {
            let start_id = get_start_id(&ctx, &repo, start_id)
                .await
                .context("While fetching the start_id")?;

            run_in_tailing_mode(
                &ctx,
                repo,
                skip_bookmarks,
                validation_helpers,
                start_id,
                scuba_sample.clone(),
            )
            .await
        }
    }
}

async fn async_main(app: MononokeApp) -> Result<()> {
    let fb = app.environment().fb;

    let mut metadata = Metadata::default();
    metadata.add_client_info(ClientInfo::default_with_entry_point(
        ClientEntryPoint::MegarepoCommitValidator,
    ));

    let mut scuba = app.environment().scuba_sample_builder.clone();
    scuba.add_metadata(&metadata);

    let session_container = SessionContainer::builder(fb)
        .metadata(Arc::new(metadata))
        .build();

    let ctx = session_container.new_context(app.logger().clone(), scuba);

    run(fb, ctx, app).await
}

#[fbinit::main]
fn main(fb: FacebookInit) -> Result<()> {
    let app = MononokeAppBuilder::new(fb)
        .with_app_extension(MonitoringAppExtension {})
        .build::<MononokeCommitValidatorArgs>()?;

    app.run_with_monitoring_and_logging(async_main, SERVICE_NAME, AliveService)
}
