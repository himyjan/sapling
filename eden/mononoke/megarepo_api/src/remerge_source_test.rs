/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::sync::Arc;

use anyhow::Error;
use commit_graph::CommitGraphRef;
use context::CoreContext;
use fbinit::FacebookInit;
use maplit::btreemap;
use maplit::hashmap;
use megarepo_config::MononokeMegarepoConfigs;
use megarepo_config::Target;
use megarepo_mapping::CommitRemappingState;
use megarepo_mapping::REMAPPING_STATE_FILE;
use megarepo_mapping::SourceName;
use metaconfig_types::RepoConfigArc;
use mononoke_macros::mononoke;
use mononoke_types::NonRootMPath;
use tests_utils::CreateCommitContext;
use tests_utils::bookmark;
use tests_utils::list_working_copy_utf8;
use tests_utils::resolve_cs_id;

use crate::add_sync_target::AddSyncTarget;
use crate::common::MegarepoOp;
use crate::common::SYNC_TARGET_CONFIG_FILE;
use crate::megarepo_test_utils::MegarepoTest;
use crate::megarepo_test_utils::SyncTargetConfigBuilder;
use crate::remerge_source::RemergeSource;

#[mononoke::fbinit_test]
async fn test_remerge_source_simple(fb: FacebookInit) -> Result<(), Error> {
    let ctx = CoreContext::test_mock(fb);
    let mut test = MegarepoTest::new(&ctx).await?;
    let target: Target = test.target("target".to_string());

    let first_source_name = SourceName::new("source_1");
    let second_source_name = SourceName::new("source_2");
    let version = "version_1".to_string();
    SyncTargetConfigBuilder::new(test.repo_id(), target.clone(), version.clone())
        .source_builder(first_source_name.clone())
        .set_prefix_bookmark_to_source_name()
        .build_source()?
        .source_builder(second_source_name.clone())
        .set_prefix_bookmark_to_source_name()
        .build_source()?
        .build(&mut test.configs_storage);

    println!("Create initial source commits and bookmarks");
    let first_source_cs_id = CreateCommitContext::new_root(&ctx, &test.repo)
        .add_file("first", "first")
        .commit()
        .await?;

    bookmark(&ctx, &test.repo, first_source_name.to_string())
        .set_to(first_source_cs_id)
        .await?;

    let second_source_root = CreateCommitContext::new_root(&ctx, &test.repo)
        .add_file("second", "root")
        .commit()
        .await?;
    let second_source_cs_id = CreateCommitContext::new(&ctx, &test.repo, vec![second_source_root])
        .add_file("second", "second")
        .commit()
        .await?;

    bookmark(&ctx, &test.repo, second_source_name.to_string())
        .set_to(second_source_cs_id)
        .await?;

    let configs_storage: Arc<dyn MononokeMegarepoConfigs> = Arc::new(test.configs_storage.clone());

    let add_sync_target = AddSyncTarget::new(&configs_storage, &test.mononoke);
    let repo = add_sync_target
        .find_repo_by_id(&ctx, target.repo_id)
        .await?;
    let repo_config = repo.repo().repo_config_arc();

    let sync_target_config = test
        .configs_storage
        .get_config_by_version(ctx.clone(), repo_config, target.clone(), version.clone())
        .await?;
    add_sync_target
        .run(
            &ctx,
            sync_target_config,
            btreemap! {
                first_source_name.clone() => first_source_cs_id,
                second_source_name.clone() => second_source_cs_id,
            },
            None,
        )
        .await?;

    let mut target_cs_id = resolve_cs_id(&ctx, &test.repo, "target").await?;
    let mut wc = list_working_copy_utf8(&ctx, &test.repo, target_cs_id).await?;

    let state =
        CommitRemappingState::read_state_from_commit(&ctx, &test.repo, target_cs_id).await?;
    assert_eq!(
        state.get_latest_synced_changeset(&first_source_name),
        Some(&first_source_cs_id),
    );
    assert_eq!(
        state.get_latest_synced_changeset(&second_source_name),
        Some(&second_source_cs_id),
    );
    assert_eq!(state.sync_config_version(), &version);

    // Remove file with commit remapping state because it's never present in source
    assert!(
        wc.remove(&NonRootMPath::new(REMAPPING_STATE_FILE)?)
            .is_some()
    );
    assert!(
        wc.remove(&NonRootMPath::new(SYNC_TARGET_CONFIG_FILE)?)
            .is_some()
    );

    assert_eq!(
        wc,
        hashmap! {
            NonRootMPath::new("source_1/first")? => "first".to_string(),
            NonRootMPath::new("source_2/second")? => "second".to_string(),
        }
    );

    let remerge_source = RemergeSource::new(&configs_storage, &test.mononoke);
    let old_target_cs_id = target_cs_id;
    remerge_source
        .run(
            &ctx,
            &second_source_name,
            second_source_root,
            None, // message
            &target,
            old_target_cs_id,
        )
        .await?;

    let remerge_source = RemergeSource::new(&configs_storage, &test.mononoke);
    // Retry, make sure it doesn't fail
    target_cs_id = remerge_source
        .run(
            &ctx,
            &second_source_name,
            second_source_root,
            None, // message
            &target,
            old_target_cs_id,
        )
        .await?;

    let parents = test
        .repo
        .commit_graph()
        .changeset_parents(&ctx, target_cs_id)
        .await?;
    assert_eq!(parents[0], old_target_cs_id);

    let state =
        CommitRemappingState::read_state_from_commit(&ctx, &test.repo, target_cs_id).await?;
    assert_eq!(
        state.get_latest_synced_changeset(&first_source_name),
        Some(&first_source_cs_id),
    );
    assert_eq!(
        state.get_latest_synced_changeset(&second_source_name),
        Some(&second_source_root),
    );
    assert_eq!(state.sync_config_version(), &version);

    let mut wc = list_working_copy_utf8(&ctx, &test.repo, target_cs_id).await?;
    // Remove file with commit remapping state because it's never present in source
    assert!(
        wc.remove(&NonRootMPath::new(REMAPPING_STATE_FILE)?)
            .is_some()
    );
    assert!(
        wc.remove(&NonRootMPath::new(SYNC_TARGET_CONFIG_FILE)?)
            .is_some()
    );

    assert_eq!(
        wc,
        hashmap! {
            NonRootMPath::new("source_1/first")? => "first".to_string(),
            NonRootMPath::new("source_2/second")? => "root".to_string(),
        }
    );

    let resolved_target_cs_id = resolve_cs_id(&ctx, &test.repo, "target").await?;
    assert_eq!(target_cs_id, resolved_target_cs_id);

    Ok(())
}
