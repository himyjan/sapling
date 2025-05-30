/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::Result;
use anyhow::bail;
use async_trait::async_trait;
use clap::Parser;
use executor_lib::RepoShardedProcess;
use executor_lib::RepoShardedProcessExecutor;
use mononoke_app::MononokeApp;
use mononoke_app::args::SourceRepoArgs;
use sharding_ext::RepoShard;

use crate::ModernSyncArgs;
use crate::Repo;
use crate::sync::ExecutionType;

const SM_CLEANUP_TIMEOUT_SECS: u64 = 120;

/// Replays bookmark's moves
#[derive(Parser)]
pub struct CommandArgs {
    #[clap(long = "start-id", help = "Start id for the sync [default: 0]")]
    start_id: Option<u64>,
    #[clap(long, help = "Print sent items without actually syncing")]
    dry_run: bool,
}

pub struct ModernSyncProcess {
    app: Arc<MononokeApp>,
    sync_args: Arc<CommandArgs>,
}

impl ModernSyncProcess {
    fn new(app: Arc<MononokeApp>, sync_args: Arc<CommandArgs>) -> Self {
        Self { app, sync_args }
    }
}

#[async_trait]
impl RepoShardedProcess for ModernSyncProcess {
    async fn setup(&self, repo: &RepoShard) -> anyhow::Result<Arc<dyn RepoShardedProcessExecutor>> {
        let source_repo_name = repo.repo_name.clone();
        let target_repo_name = match repo.target_repo_name.clone() {
            Some(repo_name) => repo_name,
            None => {
                let details = format!(
                    "Only source repo name {} provided, target repo name missing in {}",
                    source_repo_name, repo
                );
                bail!("{}", details)
            }
        };

        tracing::info!(
            "Setting up sharded sync from repo {} to repo {}",
            source_repo_name,
            target_repo_name,
        );

        let source_repo_args = SourceRepoArgs::with_name(source_repo_name.clone());

        Ok(Arc::new(ModernSyncProcessExecutor {
            app: self.app.clone(),
            sync_args: self.sync_args.clone(),
            source_repo_args,
            dest_repo_name: target_repo_name,
        }))
    }
}

pub struct ModernSyncProcessExecutor {
    app: Arc<MononokeApp>,
    sync_args: Arc<CommandArgs>,
    source_repo_args: SourceRepoArgs,
    dest_repo_name: String,
}

#[async_trait]
impl RepoShardedProcessExecutor for ModernSyncProcessExecutor {
    async fn execute(&self) -> Result<()> {
        let cancellation_requested = Arc::new(AtomicBool::new(false));
        crate::sync::sync(
            self.app.clone(),
            self.sync_args.start_id,
            self.source_repo_args.clone(),
            self.dest_repo_name.clone(),
            ExecutionType::Tail,
            self.sync_args.dry_run,
            self.app.args::<ModernSyncArgs>()?.exit_file.clone(),
            None,
            None,
            cancellation_requested,
        )
        .await?;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

pub async fn run(app: MononokeApp, args: CommandArgs) -> Result<()> {
    let app_args = &app.args::<ModernSyncArgs>()?;

    let process = Arc::new(ModernSyncProcess::new(Arc::new(app), Arc::new(args)));
    let logger = process.app.logger().clone();

    let exit_file = app_args.exit_file.clone();

    if let Some(mut executor) = app_args.sharded_executor_args.clone().build_executor(
        process.app.fb,
        process.app.runtime().clone(),
        &logger,
        || process.clone(),
        true, // enable shard (repo) level healing
        SM_CLEANUP_TIMEOUT_SECS,
    )? {
        tracing::info!("Running sharded sync loop");
        executor
            .block_and_execute(&logger, Arc::new(AtomicBool::new(false)))
            .await?;
    } else {
        tracing::info!("Running unsharded sync loop");

        let source_repo: Repo = process.app.clone().open_repo(&app_args.repo).await?;
        let source_repo_name = source_repo.repo_identity.name().to_string();
        let target_repo_name = app_args
            .dest_repo_name
            .clone()
            .unwrap_or(source_repo_name.clone());

        let source_repo_args = SourceRepoArgs::with_name(source_repo_name.clone());

        tracing::info!(
            "Setting up unsharded sync from repo {:?} to repo {:?}",
            source_repo_name,
            target_repo_name,
        );

        let cancellation_requested = Arc::new(AtomicBool::new(false));
        crate::sync::sync(
            process.app.clone(),
            process.sync_args.start_id.clone(),
            source_repo_args,
            target_repo_name,
            ExecutionType::Tail,
            process.sync_args.dry_run.clone(),
            exit_file.clone(),
            None,
            None,
            cancellation_requested,
        )
        .await?;
    }
    Ok(())
}
