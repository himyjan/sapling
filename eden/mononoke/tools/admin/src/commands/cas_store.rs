/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

mod info;
mod upload;

use anyhow::Result;
use bonsai_hg_mapping::BonsaiHgMapping;
use bookmarks::Bookmarks;
use clap::Parser;
use clap::Subcommand;
use info::CasStoreInfoArgs;
use metaconfig_types::RepoConfig;
use mononoke_app::MononokeApp;
use mononoke_app::args::RepoArgs;
use repo_blobstore::RepoBlobstore;
use repo_derived_data::RepoDerivedData;
use repo_identity::RepoIdentity;
use upload::CasStoreUploadArgs;

/// Examine and maintain the contents of the cas store.
#[derive(Parser)]
pub struct CommandArgs {
    /// The repository name or ID. Any changesets provided for
    /// subcommands will use this repoID for scoping.
    #[clap(flatten)]
    repo: RepoArgs,

    /// The subcommand for cas store.
    #[clap(subcommand)]
    subcommand: CasStoreSubcommand,
}

#[facet::container]
pub struct Repo {
    #[facet]
    repo_identity: RepoIdentity,

    #[facet]
    repo_config: RepoConfig,

    #[facet]
    repo_blobstore: RepoBlobstore,

    #[facet]
    bonsai_hg_mapping: dyn BonsaiHgMapping,

    #[facet]
    bookmarks: dyn Bookmarks,

    #[facet]
    repo_derived_data: RepoDerivedData,
}

#[derive(Subcommand)]
pub enum CasStoreSubcommand {
    /// Describe data associated with a digest within the cas store.
    Info(CasStoreInfoArgs),
    /// Upload a specific (augmented) tree, file or data for a given commit recursively into the cas store.
    Upload(CasStoreUploadArgs),
}

pub async fn run(app: MononokeApp, args: CommandArgs) -> Result<()> {
    let ctx = app.new_basic_context();
    let repo: Repo = app.open_repo(&args.repo).await?;

    match args.subcommand {
        CasStoreSubcommand::Upload(upload_args) => {
            upload::cas_store_upload(&ctx, &repo, upload_args).await
        }
        CasStoreSubcommand::Info(info_args) => info::cas_store_info(&ctx, &repo, info_args).await,
    }
}
