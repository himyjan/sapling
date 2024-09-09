/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::Result;
use async_trait::async_trait;
use blobstore::Blobstore;
use context::CoreContext;
use futures::stream::BoxStream;
use futures::stream::StreamExt;
use futures::stream::TryStreamExt;
use mononoke_types::sharded_map_v2::LoadableShardedMapV2Node;
use mononoke_types::skeleton_manifest::SkeletonManifest;
use mononoke_types::skeleton_manifest::SkeletonManifestEntry;
use mononoke_types::skeleton_manifest_v2::SkeletonManifestV2;
use mononoke_types::skeleton_manifest_v2::SkeletonManifestV2Entry;
use mononoke_types::MPathElement;
use mononoke_types::SkeletonManifestId;

use super::AsyncManifest;
use super::AsyncOrderedManifest;
use super::Entry;
use super::Manifest;
use super::OrderedManifest;
use super::Weight;

pub(crate) fn skeleton_manifest_v2_to_mf_entry(
    entry: SkeletonManifestV2Entry,
) -> Entry<SkeletonManifestV2, ()> {
    match entry {
        SkeletonManifestV2Entry::Directory(dir) => Entry::Tree(dir),
        SkeletonManifestV2Entry::File => Entry::Leaf(()),
    }
}

#[async_trait]
impl<Store: Blobstore> AsyncManifest<Store> for SkeletonManifestV2 {
    type TreeId = SkeletonManifestV2;
    type LeafId = ();
    type TrieMapType = LoadableShardedMapV2Node<SkeletonManifestV2Entry>;

    async fn list(
        &self,
        ctx: &CoreContext,
        blobstore: &Store,
    ) -> Result<BoxStream<'async_trait, Result<(MPathElement, Entry<Self::TreeId, Self::LeafId>)>>>
    {
        anyhow::Ok(
            self.clone()
                .into_subentries(ctx, blobstore)
                .map_ok(|(path, entry)| (path, skeleton_manifest_v2_to_mf_entry(entry)))
                .boxed(),
        )
    }

    async fn list_prefix(
        &self,
        ctx: &CoreContext,
        blobstore: &Store,
        prefix: &[u8],
    ) -> Result<BoxStream<'async_trait, Result<(MPathElement, Entry<Self::TreeId, Self::LeafId>)>>>
    {
        anyhow::Ok(
            self.clone()
                .into_prefix_subentries(ctx, blobstore, prefix)
                .map_ok(|(path, entry)| (path, skeleton_manifest_v2_to_mf_entry(entry)))
                .boxed(),
        )
    }

    async fn list_prefix_after(
        &self,
        ctx: &CoreContext,
        blobstore: &Store,
        prefix: &[u8],
        after: &[u8],
    ) -> Result<BoxStream<'async_trait, Result<(MPathElement, Entry<Self::TreeId, Self::LeafId>)>>>
    {
        anyhow::Ok(
            self.clone()
                .into_prefix_subentries_after(ctx, blobstore, prefix, after)
                .map_ok(|(path, entry)| (path, skeleton_manifest_v2_to_mf_entry(entry)))
                .boxed(),
        )
    }

    async fn list_skip(
        &self,
        ctx: &CoreContext,
        blobstore: &Store,
        skip: usize,
    ) -> Result<BoxStream<'async_trait, Result<(MPathElement, Entry<Self::TreeId, Self::LeafId>)>>>
    {
        anyhow::Ok(
            self.clone()
                .into_subentries_skip(ctx, blobstore, skip)
                .map_ok(|(path, entry)| (path, skeleton_manifest_v2_to_mf_entry(entry)))
                .boxed(),
        )
    }

    async fn lookup(
        &self,
        ctx: &CoreContext,
        blobstore: &Store,
        name: &MPathElement,
    ) -> Result<Option<Entry<Self::TreeId, Self::LeafId>>> {
        Ok(self
            .lookup(ctx, blobstore, name)
            .await?
            .map(skeleton_manifest_v2_to_mf_entry))
    }

    async fn into_trie_map(
        self,
        _ctx: &CoreContext,
        _blobstore: &Store,
    ) -> Result<Self::TrieMapType> {
        Ok(LoadableShardedMapV2Node::Inlined(self.subentries))
    }
}

impl Manifest for SkeletonManifest {
    type TreeId = SkeletonManifestId;
    type LeafId = ();

    fn lookup(&self, name: &MPathElement) -> Option<Entry<Self::TreeId, Self::LeafId>> {
        self.lookup(name).map(convert_skeleton_manifest)
    }

    fn list(&self) -> Box<dyn Iterator<Item = (MPathElement, Entry<Self::TreeId, Self::LeafId>)>> {
        let v: Vec<_> = self
            .list()
            .map(|(basename, entry)| (basename.clone(), convert_skeleton_manifest(entry)))
            .collect();
        Box::new(v.into_iter())
    }
}

fn convert_skeleton_manifest(
    skeleton_entry: &SkeletonManifestEntry,
) -> Entry<SkeletonManifestId, ()> {
    match skeleton_entry {
        SkeletonManifestEntry::File => Entry::Leaf(()),
        SkeletonManifestEntry::Directory(skeleton_directory) => {
            Entry::Tree(skeleton_directory.id().clone())
        }
    }
}

fn convert_skeleton_manifest_v2_to_weighted(
    entry: Entry<SkeletonManifestV2, ()>,
) -> Entry<(Weight, SkeletonManifestV2), ()> {
    match entry {
        Entry::Tree(dir) => Entry::Tree((
            dir.rollup_count()
                .into_inner()
                .try_into()
                .unwrap_or(usize::MAX),
            dir,
        )),
        Entry::Leaf(()) => Entry::Leaf(()),
    }
}

#[async_trait]
impl<Store: Blobstore> AsyncOrderedManifest<Store> for SkeletonManifestV2 {
    async fn list_weighted(
        &self,
        ctx: &CoreContext,
        blobstore: &Store,
    ) -> Result<
        BoxStream<
            'async_trait,
            Result<(MPathElement, Entry<(Weight, Self::TreeId), Self::LeafId>)>,
        >,
    > {
        self.list(ctx, blobstore).await.map(|stream| {
            stream
                .map_ok(|(p, entry)| (p, convert_skeleton_manifest_v2_to_weighted(entry)))
                .boxed()
        })
    }

    async fn lookup_weighted(
        &self,
        ctx: &CoreContext,
        blobstore: &Store,
        name: &MPathElement,
    ) -> Result<Option<Entry<(Weight, Self::TreeId), Self::LeafId>>> {
        AsyncManifest::lookup(self, ctx, blobstore, name)
            .await
            .map(|opt| opt.map(convert_skeleton_manifest_v2_to_weighted))
    }
}

impl OrderedManifest for SkeletonManifest {
    fn lookup_weighted(
        &self,
        name: &MPathElement,
    ) -> Option<Entry<(Weight, <Self as Manifest>::TreeId), <Self as Manifest>::LeafId>> {
        self.lookup(name).map(convert_skeleton_manifest_weighted)
    }

    fn list_weighted(
        &self,
    ) -> Box<
        dyn Iterator<
            Item = (
                MPathElement,
                Entry<(Weight, <Self as Manifest>::TreeId), <Self as Manifest>::LeafId>,
            ),
        >,
    > {
        let v: Vec<_> = self
            .list()
            .map(|(basename, entry)| (basename.clone(), convert_skeleton_manifest_weighted(entry)))
            .collect();
        Box::new(v.into_iter())
    }
}

fn convert_skeleton_manifest_weighted(
    skeleton_entry: &SkeletonManifestEntry,
) -> Entry<(Weight, SkeletonManifestId), ()> {
    match skeleton_entry {
        SkeletonManifestEntry::File => Entry::Leaf(()),
        SkeletonManifestEntry::Directory(skeleton_directory) => {
            let summary = skeleton_directory.summary();
            let weight = summary.descendant_files_count + summary.descendant_dirs_count;
            Entry::Tree((weight as Weight, skeleton_directory.id().clone()))
        }
    }
}