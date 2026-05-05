/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! Find all restriction roots under specified paths (streaming).

use std::io::Write;

use anyhow::Result;
use clap::Parser;
use commit_id_types::CommitIdArgs;
use futures::TryStreamExt;
use scs_client_raw::thrift;
use serde::Serialize;

use crate::ScscApp;
use crate::args::commit_id::resolve_commit_id;
use crate::args::repo::RepoArgs;
use crate::errors::SelectionErrorExt;
use crate::render::Render;

/// Find all restriction roots under the specified roots.
/// Returns the restriction root paths and their ACLs.
#[derive(Parser)]
pub(super) struct FindArgs {
    #[clap(flatten)]
    repo_args: RepoArgs,
    #[clap(flatten)]
    commit_id_args: CommitIdArgs,
    #[clap(long)]
    /// Check access permissions (populates `has_access` in the streamed output)
    check_permissions: bool,
    #[clap(long)]
    /// Return only restriction roots the caller can access
    return_only_accessible: bool,
    #[clap(long, short)]
    /// Root paths to search under (empty for entire repository)
    root: Option<Vec<String>>,
}

#[derive(Serialize)]
struct FindOutput {
    path: String,
    acls: Vec<String>,
    has_access: Option<bool>,
}

impl Render for FindOutput {
    type Args = ();

    fn render(&self, _args: &Self::Args, w: &mut dyn Write) -> Result<()> {
        let access = match self.has_access {
            Some(true) => ", access: allowed",
            Some(false) => ", access: denied",
            None => "",
        };
        writeln!(
            w,
            "{} (ACLs: {}{})",
            self.path,
            self.acls.join(", "),
            access
        )?;
        Ok(())
    }

    fn render_json(&self, _args: &Self::Args, w: &mut dyn Write) -> Result<()> {
        Ok(serde_json::to_writer(w, self)?)
    }
}

pub(super) async fn run(app: ScscApp, args: FindArgs) -> Result<()> {
    let repo = args.repo_args.clone().into_repo_specifier();
    let commit_id = args.commit_id_args.clone().into_commit_id();
    let conn = app.get_connection(Some(&repo.name)).await?;
    let id = resolve_commit_id(&conn, &repo, &commit_id).await?;

    let commit = thrift::CommitSpecifier {
        repo: repo.clone(),
        id,
        ..Default::default()
    };

    let roots = args.root.unwrap_or_default().into_iter().collect();
    let params = thrift::CommitFindRestrictedPathsParams {
        roots,
        check_permissions: Some(args.check_permissions),
        return_only_accessible: Some(args.return_only_accessible),
        ..Default::default()
    };

    let (_initial_response, response_stream) = conn
        .commit_find_restricted_paths(&commit, &params)
        .await
        .map_err(|e| e.handle_selection_error(&repo))?;

    let response = response_stream
        .map_ok(|item| FindOutput {
            path: item.path,
            acls: item.acls,
            has_access: item.has_access,
        })
        .map_err(Into::into);

    app.target.render(&(), response).await
}
