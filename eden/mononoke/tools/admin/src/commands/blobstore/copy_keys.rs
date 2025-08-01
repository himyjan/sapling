/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::path::PathBuf;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use blobstore::Blobstore;
use clap::Parser;
use context::CoreContext;
use context::SessionClass;
use futures::StreamExt;
use futures::TryStreamExt;
use futures::future;
use futures::stream;
use mononoke_app::MononokeApp;
use mononoke_app::args::AsRepoArg;
use mononoke_app::args::SourceAndTargetRepoArgs;
use repo_blobstore::RepoBlobstore;
use repo_blobstore::RepoBlobstoreRef;
use slog::debug;
use slog::info;
use slog::warn;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio_stream::wrappers::LinesStream;

#[facet::container]
#[derive(Clone)]
struct Repo {
    #[facet]
    repo_blobstore: RepoBlobstore,
}

struct OutputFiles {
    error_file: File,
    missing_file: File,
    successful_file: File,
}

impl OutputFiles {
    pub async fn new(args: &BlobstoreCopyKeysArgs) -> Result<Self, Error> {
        let (error_file, missing_file, successful_file) = future::try_join3(
            File::create(&args.error_keys_output),
            File::create(&args.missing_keys_output),
            File::create(&args.success_keys_output),
        )
        .await?;

        Ok(Self {
            error_file,
            missing_file,
            successful_file,
        })
    }

    pub async fn record_copy_result(
        &mut self,
        key: &str,
        res: Result<(), CopyError>,
    ) -> Result<(), Error> {
        let file = match res {
            Ok(()) => &mut self.successful_file,
            Err(CopyError::NotFound) => &mut self.missing_file,
            Err(CopyError::Error(_)) => &mut self.error_file,
        };

        file.write_all(key.as_bytes()).await?;
        file.write_all(b"\n").await?;

        res.with_context(|| format!("failed to copy {}", key))?;

        Ok(())
    }
}

#[derive(Error, Debug)]
enum CopyError {
    #[error("Not found")]
    NotFound,
    #[error(transparent)]
    Error(#[from] Error),
}

#[derive(Parser, Debug)]
#[clap(about = "Copy a list of blobs from one blobstore to another")]
pub struct BlobstoreCopyKeysArgs {
    /// Input filename with a list of keys
    #[clap(long)]
    input_file: PathBuf,
    /// How many blobs to copy at once
    #[clap(long, default_value_t = 100)]
    concurrency: usize,
    /// Don't terminate if failed to process a key
    #[clap(long)]
    ignore_errors: bool,
    /// If a key starts with 'repoXXXX' prefix (where XXXX matches source repository) then strip this prefix before copying
    #[clap(long)]
    strip_source_repo_prefix: bool,
    /// A file to write successfully copied keys to
    #[clap(long)]
    success_keys_output: PathBuf,
    /// A file to write missing keys to
    #[clap(long)]
    missing_keys_output: PathBuf,
    /// A file to write error fetching keys to
    #[clap(long)]
    error_keys_output: PathBuf,
    /// In case of multiplexed blobstore this will be source id of inner blobstore
    #[clap(long)]
    source_inner_blobstore_id: Option<u64>,
    /// In case of multiplexed blobstore this will be target id of inner blobstore
    #[clap(long)]
    target_inner_blobstore_id: Option<u64>,
    /// Identifiers or names for the source and target repos
    #[clap(flatten, next_help_heading = "CROSS REPO OPTIONS")]
    repo_args: SourceAndTargetRepoArgs,
}

pub async fn copy_keys(app: MononokeApp, args: BlobstoreCopyKeysArgs) -> Result<()> {
    if let (Some(_), None) = (
        &args.source_inner_blobstore_id,
        &args.target_inner_blobstore_id,
    ) {
        bail!("Missing --target-inner-blobstore-id for --source-inner-blobstore-id");
    }

    // Background session class tells multiplexed blobstore to wait
    // for all blobstores to finish.
    let mut ctx = CoreContext::new_with_logger(app.fb, app.logger().clone());
    ctx.session_mut()
        .override_session_class(SessionClass::Background);

    let repo_args = &args.repo_args;
    let (source_repo, target_repo): (Repo, Repo) = future::try_join(
        app.create_repo_unredacted(&repo_args.source_repo, args.source_inner_blobstore_id),
        app.create_repo_unredacted(&repo_args.target_repo, args.target_inner_blobstore_id),
    )
    .await?;

    let mut keys = vec![];
    let source_repo_id = app.repo_id(repo_args.source_repo.as_repo_arg())?;
    let source_repo_prefix = source_repo_id.prefix();

    let mut inputfile = File::open(&args.input_file).await?;
    let file = BufReader::new(&mut inputfile);
    let mut lines = LinesStream::new(file.lines());
    while let Some(line) = lines.try_next().await? {
        if args.strip_source_repo_prefix {
            match line.strip_prefix(&source_repo_prefix) {
                Some(key) => {
                    keys.push(key.to_string());
                }
                None => {
                    return Err(anyhow!(
                        "key {} doesn't start with prefix {}",
                        line,
                        source_repo_prefix
                    ));
                }
            }
        } else {
            keys.push(line);
        }
    }

    info!(ctx.logger(), "{} keys to copy", keys.len());
    let log_step = std::cmp::max(1, keys.len() / 10);

    let mut s = stream::iter(keys)
        .map(|key| async {
            let copy_key = key.clone();
            let res = async {
                let source_blobstore = source_repo.repo_blobstore().clone();
                let target_blobstore = target_repo.repo_blobstore().clone();
                let maybe_value = source_blobstore.get(&ctx, &key).await?;
                let value = maybe_value.ok_or(CopyError::NotFound)?;
                debug!(ctx.logger(), "copying {}", key);
                target_blobstore.put(&ctx, key, value.into_bytes()).await?;
                Result::<_, CopyError>::Ok(())
            }
            .await;

            (copy_key, res)
        })
        .buffered(args.concurrency);

    let mut copied = 0;
    let mut processed = 0;
    let mut output_files = OutputFiles::new(&args).await?;
    while let Some((key, res)) = s.next().await {
        let res = output_files.record_copy_result(&key, res).await;
        match res {
            Ok(()) => {
                copied += 1;
            }
            Err(err) => {
                if args.ignore_errors {
                    warn!(ctx.logger(), "key: {} {:#}", key, err);
                } else {
                    return Err(err);
                }
            }
        };
        processed += 1;
        if processed % log_step == 0 {
            info!(ctx.logger(), "{} keys processed", processed);
        }
    }

    info!(ctx.logger(), "{} keys were copied", copied);
    Ok(())
}
