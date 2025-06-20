/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use async_trait::async_trait;
use blobstore::Blobstore;
use blobstore::BlobstoreBytes;
use blobstore::BlobstoreGetData;
use blobstore::Storable;
use bytes::Bytes;
use cloned::cloned;
use context::CoreContext;
use derived_data_manager::BonsaiDerivable;
use derived_data_manager::DerivableType;
use derived_data_manager::DerivationContext;
use derived_data_manager::dependencies;
use derived_data_service_if as thrift;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use futures::StreamExt;
use futures::TryStreamExt;
use futures::stream;
use futures::try_join;
use itertools::Itertools;
use manifest::Diff;
use manifest::Entry;
use manifest::ManifestOps;
use metaconfig_types::GitDeltaManifestV2Config;
use mononoke_macros::mononoke;
use mononoke_types::BlobstoreValue;
use mononoke_types::BonsaiChangeset;
use mononoke_types::ChangesetId;
use mononoke_types::ThriftConvert;
use mononoke_types::path::MPath;

use crate::BaseObject;
use crate::GitLeaf;
use crate::GitPackfileBaseItem;
use crate::GitTreeId;
use crate::MappedGitCommitId;
use crate::delta_manifest_v2::GDMV2DeltaEntry;
use crate::delta_manifest_v2::GDMV2Entry;
use crate::delta_manifest_v2::GDMV2Instructions;
use crate::delta_manifest_v2::GDMV2ObjectEntry;
use crate::delta_manifest_v2::GitDeltaManifestV2;
use crate::delta_manifest_v2::GitDeltaManifestV2Id;
use crate::store::HeaderState;
use crate::store::fetch_git_object_bytes;
use crate::tree::GitEntry;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct RootGitDeltaManifestV2Id(GitDeltaManifestV2Id);

impl RootGitDeltaManifestV2Id {
    pub fn manifest_id(&self) -> &GitDeltaManifestV2Id {
        &self.0
    }
}

pub fn format_key(derivation_ctx: &DerivationContext, changeset_id: ChangesetId) -> String {
    let root_prefix = "derived_root_gdm2.";
    let key_prefix = derivation_ctx.mapping_key_prefix::<RootGitDeltaManifestV2Id>();
    format!("{}{}{}", root_prefix, key_prefix, changeset_id)
}

impl TryFrom<BlobstoreBytes> for RootGitDeltaManifestV2Id {
    type Error = Error;
    fn try_from(blob_bytes: BlobstoreBytes) -> Result<Self> {
        GitDeltaManifestV2Id::from_bytes(blob_bytes.into_bytes()).map(RootGitDeltaManifestV2Id)
    }
}

impl TryFrom<BlobstoreGetData> for RootGitDeltaManifestV2Id {
    type Error = Error;
    fn try_from(blob_val: BlobstoreGetData) -> Result<Self> {
        blob_val.into_bytes().try_into()
    }
}

impl From<RootGitDeltaManifestV2Id> for BlobstoreBytes {
    fn from(root_mf_id: RootGitDeltaManifestV2Id) -> Self {
        BlobstoreBytes::from_bytes(root_mf_id.0.into_bytes())
    }
}

async fn derive_single(
    ctx: &CoreContext,
    derivation_ctx: &DerivationContext,
    bonsai: BonsaiChangeset,
) -> Result<RootGitDeltaManifestV2Id> {
    let blobstore = derivation_ctx.blobstore();
    let config = &derivation_ctx
        .config()
        .git_delta_manifest_v2_config
        .ok_or_else(|| anyhow!("Can't derive GitDeltaManifestV2 without its config"))?;
    let cs_id = bonsai.get_changeset_id();

    let fetch_tree = |bcs_id| async move {
        derivation_ctx
            .fetch_dependency::<MappedGitCommitId>(ctx, bcs_id)
            .await?
            .fetch_root_tree(ctx, blobstore)
            .await
    };

    let (current_tree, parent_trees) = try_join!(
        fetch_tree(cs_id),
        stream::iter(bonsai.parents())
            .map(fetch_tree)
            .buffered(10)
            .try_collect::<Vec<_>>()
    )?;

    let entries = if parent_trees.is_empty() {
        gdm_v2_entries_root(ctx, blobstore, current_tree).await?
    } else {
        gdm_v2_entries_non_root(ctx, blobstore, config, current_tree, parent_trees).await?
    };

    let manifest = GitDeltaManifestV2::from_entries(ctx, blobstore, entries).await?;
    let mf_id = manifest.into_blob().store(ctx, blobstore).await?;

    Ok(RootGitDeltaManifestV2Id(mf_id))
}

async fn gdm_v2_entries_root(
    ctx: &CoreContext,
    blobstore: &Arc<dyn Blobstore>,
    current_tree: GitTreeId,
) -> Result<Vec<(MPath, GDMV2Entry)>> {
    // For root commits we store an entry for each object in the tree with
    // no deltas.
    current_tree
        .list_all_entries(ctx.clone(), blobstore.clone())
        .map_ok(|(path, entry)| async move {
            // If the entry corresponds to a submodule (and shows up as a commit), then we ignore it
            if entry.is_submodule() {
                return Ok(None);
            }
            let full_object =
                GDMV2ObjectEntry::from_tree_entry(ctx, blobstore, &entry, None).await?;
            Ok(Some((
                path,
                GDMV2Entry {
                    full_object,
                    deltas: vec![],
                },
            )))
        })
        .try_buffered(100)
        .try_filter_map(futures::future::ok)
        .try_collect::<Vec<_>>()
        .await
}

async fn gdm_v2_entries_non_root(
    ctx: &CoreContext,
    blobstore: &Arc<dyn Blobstore>,
    config: &GitDeltaManifestV2Config,
    current_tree: GitTreeId,
    parent_trees: Vec<GitTreeId>,
) -> Result<Vec<(MPath, GDMV2Entry)>> {
    let parent_count = parent_trees.len();
    let diffs = group_diffs_by_path(ctx, blobstore, current_tree, parent_trees).await?;

    stream::iter(diffs)
        .map(|(path, diffs)| async move {
            // If this object is not different from all parents then we don't need
            // to store an entry for it.
            if diffs.len() < parent_count {
                return Ok(None);
            }

            let new_entry = match diffs.first() {
                Some((_, new_entry)) => new_entry.clone(),
                None => bail!("Expected at least one diff entry for every grouped path (while deriving GitDeltaManifestV2)")
            };

            // If the entry corresponds to a submodule, then we ignore it
            if new_entry.is_submodule() {
                return Ok(None);
            }

            let deltas = stream::iter(diffs)
                .map(|(old_entry, new_entry)| create_delta_entry(ctx, blobstore, config, path.clone(), old_entry, new_entry))
                .buffered(10)
                .try_filter_map(futures::future::ok)
                .try_collect::<Vec<_>>()
                .await?;

            // If the object doesn't have any deltas and isn't too big, then we can inline it
            // into the GDMV2Entry after converting it into a packfile item.
            let inlined_bytes = if deltas.is_empty() {
                let new_object_bytes = fetch_git_object_bytes(
                    ctx,
                    blobstore.clone(),
                    &new_entry.identifier()?,
                    HeaderState::Included,
                ).await?;

                if new_object_bytes.len() <= config.max_inlined_object_size {
                    let packfile_item: GitPackfileBaseItem = BaseObject::new(new_object_bytes)?.try_into()?;
                    Some(packfile_item.into_blobstore_bytes().into_bytes())
                } else {
                    None
                }
            } else {
                None
            };

            let full_object = GDMV2ObjectEntry::from_tree_entry(ctx, blobstore, &new_entry, inlined_bytes).await?;
            Ok(Some((path, GDMV2Entry {
                full_object,
                deltas,
            })))
        })
        .buffered(100)
        .try_filter_map(futures::future::ok)
        .try_collect::<Vec<_>>()
        .await
}

/// Diffs the current tree against all parent trees and returns a grouping of the diffs by path.
/// For each diff we store a tuple of the old and new tree members.
async fn group_diffs_by_path(
    ctx: &CoreContext,
    blobstore: &Arc<dyn Blobstore>,
    current_tree: GitTreeId,
    parent_trees: Vec<GitTreeId>,
) -> Result<HashMap<MPath, Vec<(Option<Entry<GitTreeId, GitLeaf>>, Entry<GitTreeId, GitLeaf>)>>> {
    Ok(stream::iter(parent_trees)
        .map(|parent_tree| async move {
            parent_tree
                .diff(ctx.clone(), blobstore.clone(), current_tree)
                .try_filter_map(|diff| async move {
                    match diff {
                        Diff::Changed(path, old_entry, new_entry) => {
                            if old_entry.oid() == new_entry.oid() {
                                Ok(None)
                            } else {
                                Ok(Some((path, (Some(old_entry), new_entry))))
                            }
                        }
                        Diff::Added(path, new_entry) => Ok(Some((path, (None, new_entry)))),
                        _ => Ok(None),
                    }
                })
                .try_collect::<Vec<_>>()
                .await
        })
        .buffered(10)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .flatten()
        .into_group_map())
}

async fn create_delta_entry(
    ctx: &CoreContext,
    blobstore: &Arc<dyn Blobstore>,
    config: &GitDeltaManifestV2Config,
    path: MPath,
    old_entry: Option<Entry<GitTreeId, GitLeaf>>,
    new_entry: Entry<GitTreeId, GitLeaf>,
) -> Result<Option<GDMV2DeltaEntry>> {
    let old_entry = match old_entry {
        Some(entry) => {
            // If the entry corresponds to a submodule (and shows up as a commit), then we ignore it
            if entry.is_submodule() {
                return Ok(None);
            }
            entry
        }
        None => {
            return Ok(None);
        }
    };

    let old_git_ident = old_entry.identifier()?;
    let new_git_ident = new_entry.identifier()?;

    let (old_object, new_object) = try_join!(
        fetch_git_object_bytes(
            ctx,
            blobstore.clone(),
            &old_git_ident,
            HeaderState::Excluded,
        ),
        fetch_git_object_bytes(
            ctx,
            blobstore.clone(),
            &new_git_ident,
            HeaderState::Excluded,
        ),
    )?;

    let raw_delta = if let Some(delta) =
        mononoke::spawn_task(compute_raw_delta(old_object, new_object)).await??
    {
        delta
    } else {
        return Ok(None);
    };

    let base_object = GDMV2ObjectEntry::from_tree_entry(ctx, blobstore, &old_entry, None).await?;

    Ok(Some(GDMV2DeltaEntry {
        base_object,
        base_object_path: path,
        instructions: GDMV2Instructions::from_raw_delta(
            ctx,
            blobstore,
            raw_delta,
            config.delta_chunk_size,
            config.max_inlined_delta_size,
        )
        .await?,
    }))
}

async fn compute_raw_delta(old_object: Bytes, new_object: Bytes) -> Result<Option<Vec<u8>>> {
    if old_object.is_empty() || new_object.is_empty() {
        return Ok(None);
    }

    // zlib compress actual object to see how big of a delta makes sense
    let mut encoder = ZlibEncoder::new(vec![], Compression::default());
    encoder
        .write_all(&new_object)
        .context("Failure in writing raw delta instruction bytes to ZLib buffer (while deriving GitDeltaManifestV2)")?;
    let new_object_compressed_len = encoder
        .finish()
        .context(
            "Failure in ZLib encoding delta instruction bytes (while deriving GitDeltaManifestV2)",
        )?
        .len();

    if let Ok(raw_delta) = git_delta::git_delta(&old_object, &new_object, new_object_compressed_len)
    {
        Ok(Some(raw_delta))
    } else {
        // if the delta is larger than max_delta above will fail and we'll fail back to
        // serving the full object
        Ok(None)
    }
}

#[async_trait]
impl BonsaiDerivable for RootGitDeltaManifestV2Id {
    const VARIANT: DerivableType = DerivableType::GitDeltaManifestsV2;

    type Dependencies = dependencies![MappedGitCommitId];
    type PredecessorDependencies = dependencies![];

    async fn derive_single(
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        bonsai: BonsaiChangeset,
        _parents: Vec<Self>,
        _known: Option<&HashMap<ChangesetId, Self>>,
    ) -> Result<Self> {
        derive_single(ctx, derivation_ctx, bonsai).await
    }

    async fn derive_batch(
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        bonsais: Vec<BonsaiChangeset>,
    ) -> Result<HashMap<ChangesetId, Self>> {
        let ctx = Arc::new(ctx.clone());
        let derivation_ctx = Arc::new(derivation_ctx.clone());
        let output = stream::iter(bonsais)
            .map(Ok)
            .map_ok(|bonsai| {
                cloned!(ctx, derivation_ctx);
                async move {
                    let output = mononoke::spawn_task(async move {
                        let bonsai_id = bonsai.get_changeset_id();
                        let gdm_v2 = derive_single(&ctx, &derivation_ctx, bonsai).await?;
                        anyhow::Ok((bonsai_id, gdm_v2))
                    })
                    .await??;
                    anyhow::Ok(output)
                }
            })
            .try_buffer_unordered(100)
            .try_collect::<Vec<_>>()
            .await?;
        Ok(output.into_iter().collect())
    }

    async fn derive_from_predecessor(
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        bonsai: BonsaiChangeset,
    ) -> Result<Self> {
        derive_single(ctx, derivation_ctx, bonsai).await
    }

    async fn store_mapping(
        self,
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        changeset_id: ChangesetId,
    ) -> Result<()> {
        let key = format_key(derivation_ctx, changeset_id);
        derivation_ctx.blobstore().put(ctx, key, self.into()).await
    }

    async fn fetch(
        ctx: &CoreContext,
        derivation_ctx: &DerivationContext,
        changeset_id: ChangesetId,
    ) -> Result<Option<Self>> {
        let key = format_key(derivation_ctx, changeset_id);
        Ok(derivation_ctx
            .blobstore()
            .get(ctx, &key)
            .await?
            .map(TryInto::try_into)
            .transpose()?)
    }

    fn from_thrift(data: thrift::DerivedData) -> Result<Self> {
        if let thrift::DerivedData::git_delta_manifest_v2(
            thrift::DerivedDataGitDeltaManifestV2::root_git_delta_manifest_v2_id(id),
        ) = data
        {
            GitDeltaManifestV2Id::from_thrift(id).map(Self)
        } else {
            Err(anyhow!(
                "Can't convert {} from provided thrift::DerivedData",
                Self::NAME.to_string(),
            ))
        }
    }

    fn into_thrift(data: Self) -> Result<thrift::DerivedData> {
        Ok(thrift::DerivedData::git_delta_manifest_v2(
            thrift::DerivedDataGitDeltaManifestV2::root_git_delta_manifest_v2_id(
                data.0.into_thrift(),
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::str::FromStr;

    use anyhow::Context;
    use anyhow::Result;
    use anyhow::format_err;
    use async_compression::tokio::write::ZlibDecoder;
    use blobstore::Loadable;
    use bonsai_hg_mapping::BonsaiHgMapping;
    use bookmarks::BookmarkKey;
    use bookmarks::Bookmarks;
    use bookmarks::BookmarksRef;
    use commit_graph::CommitGraph;
    use commit_graph::CommitGraphRef;
    use commit_graph::CommitGraphWriter;
    use fbinit::FacebookInit;
    use filestore::FilestoreConfig;
    use fixtures::TestRepoFixture;
    use futures::future;
    use futures_util::stream::TryStreamExt;
    use mononoke_macros::mononoke;
    use mononoke_types::ChangesetIdPrefix;
    use repo_blobstore::RepoBlobstore;
    use repo_blobstore::RepoBlobstoreRef;
    use repo_derived_data::RepoDerivedData;
    use repo_derived_data::RepoDerivedDataRef;
    use repo_identity::RepoIdentity;
    use repo_identity::RepoIdentityRef;
    use tokio::io::AsyncWriteExt;

    use super::*;

    #[facet::container]
    struct Repo(
        dyn BonsaiHgMapping,
        dyn Bookmarks,
        RepoBlobstore,
        RepoDerivedData,
        RepoIdentity,
        CommitGraph,
        dyn CommitGraphWriter,
        FilestoreConfig,
    );

    /// This function generates GitDeltaManifestV2 for each bonsai commit in the fixture starting from
    /// the fixture's master Bonsai bookmark. It validates that the derivation is successful and returns
    /// the GitDeltaManifestV2 and Bonsai Changeset ID corresponding to the master bookmark
    async fn common_gdm_v2_validation(
        repo: &Repo,
        ctx: &CoreContext,
    ) -> Result<(RootGitDeltaManifestV2Id, ChangesetId)> {
        let cs_id = repo
            .bookmarks()
            .get(ctx.clone(), &BookmarkKey::from_str("master")?)
            .await?
            .ok_or_else(|| format_err!("no master"))?;
        // Validate that the derivation of the GitDeltaManifestV2 for the head commit succeeds
        let root_mf_id = repo
            .repo_derived_data()
            .derive::<RootGitDeltaManifestV2Id>(ctx, cs_id)
            .await?;
        // Validate the derivation of all the commits in this repo succeeds
        let all_cs_ids = repo
            .commit_graph()
            .find_by_prefix(ctx, ChangesetIdPrefix::from_bytes("").unwrap(), 1000)
            .await?
            .to_vec();
        repo.commit_graph()
            .process_topologically(ctx, all_cs_ids, |cs_id| async move {
                repo.repo_derived_data()
                    .derive::<RootGitDeltaManifestV2Id>(ctx, cs_id)
                    .await?;
                Ok(())
            })
            .await
            .with_context(|| {
                format!(
                    "Failed to derive GitDeltaManifestV2 for commits in repo {}",
                    repo.repo_identity().name()
                )
            })?;

        Ok((root_mf_id, cs_id))
    }

    #[mononoke::fbinit_test]
    async fn delta_manifest_v2_linear(fb: FacebookInit) -> Result<()> {
        let repo: Repo = fixtures::Linear::get_repo(fb).await;
        let ctx = CoreContext::test_mock(fb);
        let blobstore = repo.repo_blobstore();
        let (master_mf_id, _) = common_gdm_v2_validation(&repo, &ctx).await?;
        let gdm_v2 = master_mf_id.0.load(&ctx, blobstore).await?;
        let expected_paths = vec![MPath::ROOT, MPath::new("10")?] //MPath::ROOT for root directory
            .into_iter()
            .collect::<HashSet<_>>();
        let matched_entries = gdm_v2
            .into_entries(&ctx, blobstore)
            .try_filter(|(path, _)| future::ready(expected_paths.contains(path)))
            .try_collect::<HashMap<_, _>>()
            .await?;
        // Ensure that the delta manifest contains entries for the paths that were added/modified as part of this commit
        assert_eq!(matched_entries.len(), expected_paths.len());
        // Since the file 10 was modified, we should have a delta variant for it. Additionally, the root directory is always modified so it should
        // have a delta variant as well
        assert!(matched_entries.values().all(|entry| entry.has_deltas()));
        Ok(())
    }

    #[mononoke::fbinit_test]
    async fn delta_manifest_v2_branch_even(fb: FacebookInit) -> Result<()> {
        let repo: Repo = fixtures::BranchEven::get_repo(fb).await;
        let ctx = CoreContext::test_mock(fb);
        let blobstore = repo.repo_blobstore();
        let (master_mf_id, _) = common_gdm_v2_validation(&repo, &ctx).await?;
        let gdm_v2 = master_mf_id.0.load(&ctx, blobstore).await?;
        let expected_paths = vec![MPath::ROOT, MPath::new("base")?] //MPath::ROOT for root directory
            .into_iter()
            .collect::<HashSet<_>>();
        let matched_entries = gdm_v2
            .into_entries(&ctx, &blobstore)
            .try_filter(|(path, _)| future::ready(expected_paths.contains(path)))
            .try_collect::<HashMap<_, _>>()
            .await?;
        // Ensure that the delta manifest contains entries for the paths that were added/modified as part of this commit
        assert_eq!(matched_entries.len(), expected_paths.len());
        // Since the file base was modified, we should have a delta variant for it. Additionally, the root directory is always modified so it should
        // have a delta variant as well
        assert!(matched_entries.values().all(|entry| entry.has_deltas()));
        // Since the file "base" was modified, ensure that the delta variant for its entry points to the right changeset
        let entry = matched_entries
            .get(&MPath::new("base")?)
            .expect("Expected entry for path 'base'");
        // There should only be one delta base for the file "base"
        assert_eq!(entry.deltas.len(), 1);
        Ok(())
    }

    #[mononoke::fbinit_test]
    async fn delta_manifest_v2_branch_uneven(fb: FacebookInit) -> Result<()> {
        let repo: Repo = fixtures::BranchUneven::get_repo(fb).await;
        let ctx = CoreContext::test_mock(fb);
        let blobstore = repo.repo_blobstore();
        let (master_mf_id, _) = common_gdm_v2_validation(&repo, &ctx).await?;
        let gdm_v2 = master_mf_id.0.load(&ctx, blobstore).await?;
        let expected_paths = vec![MPath::ROOT, MPath::new("5")?] //MPath::ROOT for root directory
            .into_iter()
            .collect::<HashSet<_>>();
        let matched_entries = gdm_v2
            .into_entries(&ctx, blobstore)
            .try_filter(|(path, _)| future::ready(expected_paths.contains(path)))
            .try_collect::<HashMap<_, _>>()
            .await?;
        // Ensure that the delta manifest contains entries for the paths that were added/modified as part of this commit
        assert_eq!(matched_entries.len(), expected_paths.len());
        // Ensure that the root entry has a delta variant
        assert!(
            matched_entries
                .get(&MPath::ROOT)
                .expect("Expected root entry to exist")
                .has_deltas()
        );
        // Since the file 5 was added in this commit, it should NOT have a delta variant
        assert!(
            !matched_entries
                .get(&MPath::new("5")?)
                .expect("Expected file 5 entry to exist")
                .has_deltas()
        );
        Ok(())
    }

    #[mononoke::fbinit_test]
    async fn delta_manifest_v2_branch_wide(fb: FacebookInit) -> Result<()> {
        let repo: Repo = fixtures::BranchWide::get_repo(fb).await;
        let ctx = CoreContext::test_mock(fb);
        let blobstore = repo.repo_blobstore();
        let (master_mf_id, _) = common_gdm_v2_validation(&repo, &ctx).await?;
        let gdm_v2 = master_mf_id.0.load(&ctx, blobstore).await?;
        let expected_paths = vec![MPath::ROOT, MPath::new("3")?] //MPath::ROOT for root directory
            .into_iter()
            .collect::<HashSet<_>>();
        let matched_entries = gdm_v2
            .into_entries(&ctx, &blobstore)
            .try_filter(|(path, _)| future::ready(expected_paths.contains(path)))
            .try_collect::<HashMap<_, _>>()
            .await?;
        // Ensure that the delta manifest contains entries for the paths that were added/modified as part of this commit
        assert_eq!(matched_entries.len(), expected_paths.len());
        // Ensure that the root entry has a delta variant
        assert!(
            matched_entries
                .get(&MPath::ROOT)
                .expect("Expected root entry to exist")
                .has_deltas()
        );
        // Since the file 3 was added in this commit, it should NOT have a delta variant
        assert!(
            !matched_entries
                .get(&MPath::new("3")?)
                .expect("Expected file 3 entry to exist")
                .has_deltas()
        );
        Ok(())
    }

    #[mononoke::fbinit_test]
    async fn delta_manifest_v2_merge_even(fb: FacebookInit) -> Result<()> {
        let repo: Repo = fixtures::MergeEven::get_repo(fb).await;
        let ctx = CoreContext::test_mock(fb);
        let blobstore = repo.repo_blobstore();
        let (master_mf_id, _) = common_gdm_v2_validation(&repo, &ctx).await?;
        let gdm_v2 = master_mf_id.0.load(&ctx, blobstore).await?;
        let expected_paths = vec![MPath::ROOT, MPath::new("base")?] //MPath::ROOT for root directory
            .into_iter()
            .collect::<HashSet<_>>();
        let matched_entries = gdm_v2
            .clone()
            .into_entries(&ctx, blobstore)
            .try_filter(|(path, _)| future::ready(expected_paths.contains(path)))
            .try_collect::<HashMap<_, _>>()
            .await?;
        assert!(matched_entries.is_empty());
        // The commit has a change for path "branch" as well. However, both parents of the merge commit have the same version
        // of the file, so there should not be any entry for it in the manifest
        let branch_entry = gdm_v2
            .lookup(&ctx, &blobstore, &MPath::new("branch")?)
            .await?;
        assert!(branch_entry.is_none());
        Ok(())
    }

    #[mononoke::fbinit_test]
    async fn delta_manifest_v2_many_files_dirs(fb: FacebookInit) -> Result<()> {
        let repo: Repo = fixtures::ManyFilesDirs::get_repo(fb).await;
        let ctx = CoreContext::test_mock(fb);
        let blobstore = repo.repo_blobstore();
        let (master_mf_id, _) = common_gdm_v2_validation(&repo, &ctx).await?;
        let gdm_v2 = master_mf_id.0.load(&ctx, blobstore).await?;
        let expected_paths = vec![MPath::ROOT, MPath::new("1")?] //MPath::ROOT for root directory
            .into_iter()
            .collect::<HashSet<_>>();
        let matched_entries = gdm_v2
            .into_entries(&ctx, blobstore)
            .try_filter(|(path, _)| future::ready(expected_paths.contains(path)))
            .try_collect::<HashMap<_, _>>()
            .await?;
        // Ensure that the delta manifest contains entries for the paths that were added/modified as part of this commit
        assert_eq!(matched_entries.len(), expected_paths.len());
        // Since the commit is a root commit, i.e. has no parents, all changes introduced by this commit should be considered new additions and should
        // not have any delta variant associated with it
        assert!(matched_entries.values().all(|entry| !entry.has_deltas()));
        Ok(())
    }

    #[mononoke::fbinit_test]
    async fn delta_manifest_v2_merge_uneven(fb: FacebookInit) -> Result<()> {
        let repo: Repo = fixtures::MergeUneven::get_repo(fb).await;
        let ctx = CoreContext::test_mock(fb);
        let blobstore = repo.repo_blobstore();
        let (master_mf_id, _) = common_gdm_v2_validation(&repo, &ctx).await?;
        let gdm_v2 = master_mf_id.0.load(&ctx, blobstore).await?;
        let matched_entries = gdm_v2
            .clone()
            .into_entries(&ctx, blobstore)
            .try_collect::<HashMap<_, _>>()
            .await?;
        assert!(matched_entries.is_empty());
        // The commit has a change for path "branch" as well. However, both parents of the merge commit have the same version
        // of the file, so there should not be any entry for it in the manifest
        let branch_entry = gdm_v2
            .lookup(&ctx, &blobstore, &MPath::new("branch")?)
            .await?;
        assert!(branch_entry.is_none());
        Ok(())
    }

    #[mononoke::fbinit_test]
    async fn delta_manifest_v2_merge_multiple_files(fb: FacebookInit) -> Result<()> {
        let repo: Repo = fixtures::MergeMultipleFiles::get_repo(fb).await;
        let ctx = CoreContext::test_mock(fb);
        let blobstore = repo.repo_blobstore();
        let (master_mf_id, _) = common_gdm_v2_validation(&repo, &ctx).await?;
        let gdm_v2 = master_mf_id.0.load(&ctx, blobstore).await?;
        let expected_paths = vec![
            MPath::ROOT,
            MPath::new("2")?,
            MPath::new("3")?,
            MPath::new("4")?,
        ] //MPath::ROOT for root directory
        .into_iter()
        .collect::<HashSet<_>>();
        let matched_entries = gdm_v2
            .clone()
            .into_entries(&ctx, blobstore)
            .try_filter(|(path, _)| future::ready(expected_paths.contains(path)))
            .try_collect::<HashMap<_, _>>()
            .await?;

        // Ensure that the delta manifest contains entries for the paths that were added/modified as part of this commit
        assert_eq!(matched_entries.len(), expected_paths.len());
        // The commit has a change for path "branch" as well. However, both parents of the merge commit have the same version
        // of the file, so there should not be any entry for it in the manifest
        let branch_entry = gdm_v2
            .lookup(&ctx, blobstore, &MPath::new("branch")?)
            .await?;
        assert!(branch_entry.is_none());
        // Files 1, 2, 4 and 5 should show up as added entries without any delta variants since they are present in one parent branch
        // and not in the other
        let added_paths = vec![
            MPath::new("1")?,
            MPath::new("2")?,
            MPath::new("4")?,
            MPath::new("5")?,
        ]
        .into_iter()
        .collect::<HashSet<_>>();
        assert!(
            matched_entries
                .iter()
                .filter(|(path, _)| added_paths.contains(path))
                .all(|(_, entry)| !entry.has_deltas())
        );
        Ok(())
    }

    #[mononoke::fbinit_test]
    async fn delta_manifest_v2_instructions_encoding(fb: FacebookInit) -> Result<()> {
        let repo: Repo = fixtures::Linear::get_repo(fb).await;
        let ctx = CoreContext::test_mock(fb);
        let blobstore = repo.repo_blobstore();
        let (master_mf_id, _) = common_gdm_v2_validation(&repo, &ctx).await?;
        let gdm_v2 = master_mf_id.0.load(&ctx, &blobstore).await?;
        let entry = gdm_v2
            .lookup(&ctx, &blobstore, &MPath::new("10")?)
            .await?
            .expect("Expected entry for path '10'");
        let delta = entry
            .deltas
            .into_iter()
            .next()
            .expect("Expected a delta variant for path '10'");
        // We can't make any assertions about the size of the delta instructions since they can be larger than the
        // size of the actual object itself if the object is too small
        let bytes = delta
            .instructions
            .instruction_bytes
            .into_raw_bytes(&ctx, blobstore)
            .await?;

        let mut decoder = ZlibDecoder::new(vec![]);
        decoder.write_all(bytes.as_ref()).await?;

        Ok(())
    }
}
