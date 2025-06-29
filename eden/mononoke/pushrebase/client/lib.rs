/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#![feature(let_chains)]
#[cfg(fbcode_build)]
mod facebook;
mod hybrid;
mod local;

use std::collections::HashMap;

use bookmarks_movement::BookmarkKindRestrictions;
use bookmarks_movement::BookmarkMovementError;
use bookmarks_types::BookmarkKey;
use bytes::Bytes;
#[cfg(fbcode_build)]
pub use facebook::land_service::LandServicePushrebaseClient;
#[cfg(fbcode_build)]
pub use facebook::land_service::override_certificate_paths as land_service_override_certificate_paths;
use hooks::CrossRepoPushSource;
pub use hybrid::normal_pushrebase;
pub use local::LocalPushrebaseClient;
use mononoke_types::BonsaiChangeset;
use pushrebase::PushrebaseOutcome;

#[async_trait::async_trait]
/// This trait provides an abstraction for pushrebase, which can be used to allow
/// pushrebase to happen remotely.
pub trait PushrebaseClient: Sync + Send {
    /// Pushrebase the given changesets to the given bookmark.
    async fn pushrebase(
        &self,
        bookmark: &BookmarkKey,
        // Must be a stack
        changesets: &[BonsaiChangeset],
        pushvars: Option<&HashMap<String, Bytes>>,
        cross_repo_push_source: CrossRepoPushSource,
        bookmark_restrictions: BookmarkKindRestrictions,
        log_new_public_commits_to_scribe: bool,
    ) -> Result<PushrebaseOutcome, BookmarkMovementError>;
}
