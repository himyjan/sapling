/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::sync::Arc;

use anyhow::Result;
use bonsai_hg_mapping::BonsaiHgMapping;
use bookmarks::Bookmarks;
use commit_graph::CommitGraph;
use commit_graph::CommitGraphWriter;
use context::CoreContext;
use fbinit::FacebookInit;
use filestore::FilestoreConfig;
use mononoke_macros::mononoke;
use mononoke_types::ChangesetId;
use mononoke_types::RepositoryId;
use permission_checker::dummy::DummyAclProvider;
use repo_blobstore::RepoBlobstore;
use repo_derived_data::RepoDerivedData;
use repo_derived_data::RepoDerivedDataArc;
use repo_identity::RepoIdentity;
use scuba_ext::MononokeScubaSampleBuilder;
use sql_construct::SqlConstruct;
use tests_utils::CreateCommitContext;

use crate::RestrictedPaths;
use crate::RestrictedPathsConfig;
use crate::RestrictedPathsConfigBased;
use crate::SqlRestrictedPathsManifestIdStoreBuilder;

#[facet::container]
struct AclManifestLookupTestRepo(
    dyn BonsaiHgMapping,
    dyn Bookmarks,
    CommitGraph,
    dyn CommitGraphWriter,
    RepoDerivedData,
    RepoBlobstore,
    FilestoreConfig,
    RepoIdentity,
);

#[expect(
    dead_code,
    reason = "placeholder tests build this fixture before assertions"
)]
struct AclManifestLookupFixture {
    ctx: CoreContext,
    restricted_paths: RestrictedPaths,
    cs_id: ChangesetId,
}

mod acl_manifest_path_lookup {
    use super::*;

    #[mononoke::fbinit_test]
    async fn test_exact_path_lookup_finds_restriction_root(fb: FacebookInit) -> Result<()> {
        let repo_region_acl = "REPO_REGION:repos/hg/fbsource/=restricted";
        let _fixture = acl_manifest_lookup_fixture(
            fb,
            vec![
                ("restricted/.slacl", slacl(repo_region_acl)),
                ("restricted/file.txt", b"content".to_vec()),
            ],
        )
        .await?;
        // TODO(T248658346): Call the AclManifest exact path lookup and assert it returns the restricted root.
        Ok(())
    }

    #[mononoke::fbinit_test]
    async fn test_ancestor_path_lookup_finds_parent_restriction(fb: FacebookInit) -> Result<()> {
        let lookup_path = "restricted/child/file.txt";
        let repo_region_acl = "REPO_REGION:repos/hg/fbsource/=restricted_parent";
        let _fixture = acl_manifest_lookup_fixture(
            fb,
            vec![
                ("restricted/.slacl", slacl(repo_region_acl)),
                (lookup_path, b"content".to_vec()),
            ],
        )
        .await?;
        // TODO(T248658346): Call the AclManifest ancestor lookup and assert it returns the parent restriction.
        Ok(())
    }

    #[mononoke::fbinit_test]
    async fn test_descendant_lookup_finds_restricted_children(fb: FacebookInit) -> Result<()> {
        let first_repo_region_acl = "REPO_REGION:repos/hg/fbsource/=first";
        let second_repo_region_acl = "REPO_REGION:repos/hg/fbsource/=second";
        let _fixture = acl_manifest_lookup_fixture(
            fb,
            vec![
                ("project/first/.slacl", slacl(first_repo_region_acl)),
                ("project/first/file.txt", b"content".to_vec()),
                ("project/second/.slacl", slacl(second_repo_region_acl)),
                ("project/second/file.txt", b"content".to_vec()),
            ],
        )
        .await?;
        // TODO(T248658346): Call the AclManifest descendant lookup and assert it returns the restricted children.
        Ok(())
    }
}

async fn acl_manifest_lookup_fixture(
    fb: FacebookInit,
    files: Vec<(&'static str, Vec<u8>)>,
) -> Result<AclManifestLookupFixture> {
    let ctx = CoreContext::test_mock(fb);
    let repo: AclManifestLookupTestRepo = test_repo_factory::build_empty(fb).await?;
    let cs_id = files
        .into_iter()
        .fold(
            CreateCommitContext::new_root(&ctx, &repo),
            |commit, (path, content)| commit.add_file(path, content),
        )
        .commit()
        .await?;
    let manifest_id_store = Arc::new(
        SqlRestrictedPathsManifestIdStoreBuilder::with_sqlite_in_memory()?
            .with_repo_id(RepositoryId::new(0)),
    );
    let config_based = Arc::new(RestrictedPathsConfigBased::new(
        RestrictedPathsConfig::default(),
        manifest_id_store,
        None,
    ));
    let restricted_paths = RestrictedPaths::new(
        config_based,
        DummyAclProvider::new(fb)?,
        MononokeScubaSampleBuilder::with_discard(),
        true,
        repo.repo_derived_data_arc(),
    )?;

    Ok(AclManifestLookupFixture {
        ctx,
        restricted_paths,
        cs_id,
    })
}

fn slacl(repo_region_acl: &str) -> Vec<u8> {
    format!("repo_region_acl = \"{repo_region_acl}\"\n").into_bytes()
}
