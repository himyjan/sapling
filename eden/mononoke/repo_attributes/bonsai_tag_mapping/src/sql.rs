/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use ::sql_ext::mononoke_queries;
use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use context::CoreContext;
use mononoke_types::ChangesetId;
use mononoke_types::RepositoryId;
use mononoke_types::hash::GitSha1;
use sql_construct::SqlConstruct;
use sql_construct::SqlConstructFromMetadataDatabaseConfig;
use sql_ext::SqlConnections;

use super::BonsaiTagMapping;
use super::BonsaiTagMappingEntry;
use crate::Freshness;

mononoke_queries! {
    write AddOrUpdateBonsaiTagMapping(values: (
        repo_id: RepositoryId,
        tag_name: String,
        changeset_id: ChangesetId,
        tag_hash: GitSha1,
        target_is_tag: bool,
    )) {
        none,
        "REPLACE INTO bonsai_tag_mapping (repo_id, tag_name, changeset_id, tag_hash, target_is_tag) VALUES {values}"
    }

    write DeleteBonsaiTagMappingsByName(repo_id: RepositoryId,
        >list tag_names: String) {
        none,
        "DELETE FROM bonsai_tag_mapping WHERE repo_id = {repo_id} AND tag_name IN {tag_names}"
    }

    read SelectAllMappings(
        repo_id: RepositoryId,
    ) -> (String, ChangesetId, GitSha1, bool) {
        "SELECT tag_name, changeset_id, tag_hash, target_is_tag
         FROM bonsai_tag_mapping
         WHERE repo_id = {repo_id}"
    }

    read SelectMappingByChangeset(
        repo_id: RepositoryId,
        >list changeset_id: ChangesetId
    ) -> (String, ChangesetId, GitSha1, bool) {
        "SELECT tag_name, changeset_id, tag_hash, target_is_tag
         FROM bonsai_tag_mapping
         WHERE repo_id = {repo_id} AND changeset_id IN {changeset_id}"
    }

    read SelectMappingByTagName(
        repo_id: RepositoryId,
        tag_name: String,
    ) -> (String, ChangesetId, GitSha1, bool) {
        "SELECT tag_name, changeset_id, tag_hash, target_is_tag
         FROM bonsai_tag_mapping
         WHERE repo_id = {repo_id} AND tag_name = {tag_name}"
    }

    read SelectMappingByTagHash(
        repo_id: RepositoryId,
        >list tag_hash: GitSha1
    ) -> (String, ChangesetId, GitSha1, bool) {
        "SELECT tag_name, changeset_id, tag_hash, target_is_tag
         FROM bonsai_tag_mapping
         WHERE repo_id = {repo_id} AND tag_hash IN {tag_hash}"
    }
}

pub struct SqlBonsaiTagMapping {
    connections: SqlConnections,
    repo_id: RepositoryId,
}

#[derive(Clone)]
pub struct SqlBonsaiTagMappingBuilder {
    connections: SqlConnections,
}

impl SqlConstruct for SqlBonsaiTagMappingBuilder {
    const LABEL: &'static str = "bonsai_tag_mapping";

    const CREATION_QUERY: &'static str = include_str!("../schemas/sqlite-bonsai-tag-mapping.sql");

    fn from_sql_connections(connections: SqlConnections) -> Self {
        Self { connections }
    }
}

impl SqlConstructFromMetadataDatabaseConfig for SqlBonsaiTagMappingBuilder {}

impl SqlBonsaiTagMappingBuilder {
    pub fn build(self, repo_id: RepositoryId) -> SqlBonsaiTagMapping {
        SqlBonsaiTagMapping {
            connections: self.connections,
            repo_id,
        }
    }
}

#[async_trait]
impl BonsaiTagMapping for SqlBonsaiTagMapping {
    fn repo_id(&self) -> RepositoryId {
        self.repo_id
    }

    async fn get_all_entries(&self, ctx: &CoreContext) -> Result<Vec<BonsaiTagMappingEntry>> {
        let results = SelectAllMappings::query(
            &self.connections.read_connection,
            ctx.sql_query_telemetry(),
            &self.repo_id,
        )
        .await
        .with_context(|| format!("Failure in fetching all entries for repo {}", self.repo_id))?;

        let values = results
            .into_iter()
            .map(|(tag_name, changeset_id, tag_hash, target_is_tag)| {
                BonsaiTagMappingEntry::new(changeset_id, tag_name, tag_hash, target_is_tag)
            })
            .collect::<Vec<_>>();
        return Ok(values);
    }

    async fn get_entry_by_tag_name(
        &self,
        ctx: &CoreContext,
        tag_name: String,
        freshness: Freshness,
    ) -> Result<Option<BonsaiTagMappingEntry>> {
        let connection = if freshness == Freshness::Latest {
            &self.connections.read_master_connection
        } else {
            &self.connections.read_connection
        };
        let results = SelectMappingByTagName::query(
            connection,
            ctx.sql_query_telemetry(),
            &self.repo_id,
            &tag_name,
        )
        .await
        .with_context(|| {
            format!(
                "Failure in fetching entry for tag {} in repo {}",
                tag_name, self.repo_id
            )
        })?;
        // This should not happen but since this is new code, extra checks dont hurt.
        if results.len() > 1 {
            anyhow::bail!(
                "Multiple entries returned for tag {} in repo {}",
                tag_name,
                self.repo_id
            )
        }
        Ok(results
            .into_iter()
            .next()
            .map(|(tag_name, changeset_id, tag_hash, target_is_tag)| {
                BonsaiTagMappingEntry::new(changeset_id, tag_name, tag_hash, target_is_tag)
            }))
    }

    async fn get_entries_by_changesets(
        &self,
        ctx: &CoreContext,
        changeset_ids: Vec<ChangesetId>,
    ) -> Result<Vec<BonsaiTagMappingEntry>> {
        let results = SelectMappingByChangeset::query(
            &self.connections.read_connection,
            ctx.sql_query_telemetry(),
            &self.repo_id,
            changeset_ids.as_slice(),
        )
        .await
        .with_context(|| {
            format!(
                "Failure in fetching entry for changesets {:?} in repo {}",
                changeset_ids, self.repo_id
            )
        })?;

        let values = results
            .into_iter()
            .map(|(tag_name, changeset_id, tag_hash, target_is_tag)| {
                BonsaiTagMappingEntry::new(changeset_id, tag_name, tag_hash, target_is_tag)
            })
            .collect::<Vec<_>>();
        return Ok(values);
    }

    async fn get_entries_by_tag_hashes(
        &self,
        ctx: &CoreContext,
        tag_hashes: Vec<GitSha1>,
    ) -> Result<Vec<BonsaiTagMappingEntry>> {
        let results = SelectMappingByTagHash::query(
            &self.connections.read_connection,
            ctx.sql_query_telemetry(),
            &self.repo_id,
            tag_hashes.as_slice(),
        )
        .await
        .with_context(|| {
            format!(
                "Failure in fetching entry for tag hashes {:?} in repo {}",
                tag_hashes, self.repo_id
            )
        })?;

        let values = results
            .into_iter()
            .map(|(tag_name, changeset_id, tag_hash, target_is_tag)| {
                BonsaiTagMappingEntry::new(changeset_id, tag_name, tag_hash, target_is_tag)
            })
            .collect::<Vec<_>>();
        return Ok(values);
    }

    async fn add_or_update_mappings(
        &self,
        ctx: &CoreContext,
        entries: Vec<BonsaiTagMappingEntry>,
    ) -> Result<()> {
        let converted_entries: Vec<_> = entries
            .iter()
            .map(|entry| {
                (
                    &self.repo_id,
                    &entry.tag_name,
                    &entry.changeset_id,
                    &entry.tag_hash,
                    &entry.target_is_tag,
                )
            })
            .collect();
        AddOrUpdateBonsaiTagMapping::query(
            &self.connections.write_connection,
            ctx.sql_query_telemetry(),
            converted_entries.as_slice(),
        )
        .await
        .with_context(|| {
            format!(
                "Failed to add mappings in repo {} for entries {:?}",
                self.repo_id, entries,
            )
        })?;
        Ok(())
    }

    async fn delete_mappings_by_name(
        &self,
        ctx: &CoreContext,
        tag_names: Vec<String>,
    ) -> Result<()> {
        DeleteBonsaiTagMappingsByName::query(
            &self.connections.write_connection,
            ctx.sql_query_telemetry(),
            &self.repo_id,
            tag_names.as_slice(),
        )
        .await
        .with_context(|| {
            format!(
                "Failed to delete mappings in repo {} for tag names {:?}",
                self.repo_id, tag_names,
            )
        })?;
        Ok(())
    }
}
