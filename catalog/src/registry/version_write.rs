// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog VCS write side — Catalog WRITES versions to the asset-chain.
//!
//! Phase 3 of the Catalog-VCS / NGauge-DMS roadmap. Where Phase 2
//! ([`version_view`]) only READS an asset's version graph from the single
//! lineage substrate ([`blockmatrix::blockchain::AssetLineage`]), Phase 3 makes
//! [`CatalogRegistry`] the on-chain VCS WRITER. It leverages the EXISTING core
//! primitives — it does not build a new proof or chain path:
//!
//! - **Genesis (first version of a name):** the name-derived
//!   `type_def.asset_id` is minted via
//!   [`NodeBlockchain::register_asset_record`], whose `asset_hash` is the
//!   registration's `content_hash` — the stable genesis identity that Phase-2's
//!   [`asset_hash_for_name`](CatalogRegistry::asset_hash_for_name) resolves.
//! - **Successor (progression):** a per-version [`BlockAssetEntry::new_bound`]
//!   REUSES the genesis `asset_hash` as its chain key while carrying distinct
//!   per-version content (its schema + version) in its own registration. The
//!   single-write chokepoint [`NodeBlockchain::add_block`] stamps the asset
//!   lineage (`prev = head.lineage_id`, `asset_seq = head + 1`) and, on a chain
//!   with a signer, FALCON-signs the entry.
//! - **Branch:** [`branch_version`](CatalogRegistry::branch_version) mints a NEW
//!   asset-chain (a fresh genesis, new `asset_hash`) whose genesis
//!   `AssetData.metadata` JSON carries `{"branch_parent", "branch_of_asset"}` —
//!   content-bound, NON-format (no `Block`/`StateProof` field is added).
//!
//! The signed `StateProof` is NOT invented here: the write reuses the proof the
//! caller already supplied on the [`AssetTypeDefinition`]. `new_bound` /
//! `register_asset_record` re-bind it to the target `asset_hash`
//! (signed-to-content, P1); the chain's own signer attaches the FALCON envelope
//! at `add_block` when present. The version author is that signer, and PoStake
//! remains authorization (WHO) — never a magnitude.
//!
//! Every method here is a no-op-degrading when no chain handle is attached: the
//! caller only reaches the on-chain writers after checking `self.chain.is_some()`
//! (or through [`branch_version`], which returns an error without a chain).
//!
//! [`version_view`]: super::version_view

use anyhow::Result;

use blockmatrix::assets::core::AssetData;
use blockmatrix::assets::{AssetRegistration, StateProof};
use blockmatrix::blockchain::block::{BlockAssetEntry, StoragePointer};

use super::asset_type::AssetTypeDefinition;
use super::catalog_registry::CatalogRegistry;
use super::version_view::VersionRef;

/// Build a per-version [`AssetRegistration`] whose content is DISTINCT per
/// version (schema + version string) but whose scope/category match the genesis.
///
/// This is the registration that rides inside a SUCCESSOR entry: the entry's
/// `asset_hash` stays the stable genesis identity, while this registration gives
/// each version its own content so successive entries are not byte-identical.
fn per_version_registration(
    type_def: &AssetTypeDefinition,
    genesis_reg: &AssetRegistration,
) -> AssetRegistration {
    let schema_bytes = serde_json::to_vec(&type_def.schema).unwrap_or_default();
    let data = AssetData {
        config: type_def.type_name.as_bytes().to_vec(),
        definition: schema_bytes,
        metadata: type_def.metadata.version.as_bytes().to_vec(),
    };
    AssetRegistration::from_asset_data(
        &data,
        genesis_reg.network_scope.clone(),
        genesis_reg.category.clone(),
    )
}

impl CatalogRegistry {
    /// Mint a name's GENESIS version on the asset-chain.
    ///
    /// Delegates to [`NodeBlockchain::register_asset_record`], the existing
    /// genesis-minting path — `asset_hash = reg.content_hash` (the name-derived
    /// identity), `asset_seq = 0`. Requires a chain handle (callers gate on
    /// `self.chain.is_some()`).
    pub(crate) async fn write_genesis_on_chain(
        &self,
        reg: &AssetRegistration,
        proof: &StateProof,
    ) -> Result<()> {
        let chain = self
            .chain
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("write_genesis_on_chain requires a chain handle"))?;
        chain
            .register_asset_record(reg.clone(), proof)
            .await
            .map_err(|e| anyhow::anyhow!("on-chain genesis registration failed: {e}"))?;
        Ok(())
    }

    /// Append a SUCCESSOR version (progression) to an existing name's
    /// asset-chain and return the newly-written [`VersionRef`].
    ///
    /// Builds a per-version entry bound to the stable genesis `asset_hash`, then
    /// appends it via [`NodeBlockchain::add_block`] — the single write chokepoint
    /// that stamps the lineage pointer (`prev`/`asset_seq`) onto the current
    /// head. The in-mem head definition + a per-version registration record are
    /// updated so standalone reads stay in step with the chain.
    pub(crate) async fn append_version(
        &self,
        type_def: AssetTypeDefinition,
    ) -> Result<AssetRegistration> {
        let genesis_reg = self.find_type(&type_def.type_name).await?;
        let genesis_asset_hash = genesis_reg.content_hash;
        let per_version = per_version_registration(&type_def, &genesis_reg);

        let entry = BlockAssetEntry::new_bound(
            genesis_asset_hash,
            &type_def.state_proof,
            StoragePointer::Genesis,
            per_version,
        );

        let chain = self
            .chain
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("append_version requires a chain handle"))?;
        chain
            .add_block(vec![entry])
            .await
            .map_err(|e| anyhow::anyhow!("on-chain version append failed: {e}"))?;

        // Keep the in-mem view current: head definition + a version record.
        self.record_type_registration(&type_def).await?;
        Ok(genesis_reg)
    }

    /// Branch a name into a NEW independent asset-chain that names its parent.
    ///
    /// A branch is not a version of the parent chain — it is a fresh genesis
    /// (new `asset_hash`) recording `{"branch_parent": <parent version's
    /// lineage_id>, "branch_of_asset": <hex parent genesis asset_hash>}`. That
    /// pointer rides in two existing, content-bound places (NON-format — no
    /// `Block`/`StateProof` field is added):
    /// - the branch genesis `AssetData.metadata`, so it feeds the branch's
    ///   `content_hash` (a branch of a different parent is a different asset), and
    /// - the entry's `StoragePointer::Local` payload, so it is RECOVERABLE from
    ///   the on-chain entry (an `AssetRegistration` retains only its
    ///   `content_hash`, not the source `AssetData`). `StoragePointer` is bound
    ///   into the block hash (C1), so the pointer is tamper-evident — the same
    ///   auxiliary-payload pattern as `register_dns_asset` / key rotation.
    ///
    /// `from_version` selects which version of the parent to branch from by its
    /// `asset_seq` (`None` = the parent's head). The version author supplies the
    /// branch genesis `state_proof`; no proof is fabricated here.
    ///
    /// Requires a chain handle and a registered parent `name`.
    pub async fn branch_version(
        &self,
        name: &str,
        from_version: Option<u64>,
        state_proof: &StateProof,
    ) -> Result<VersionRef> {
        let chain = self
            .chain
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("branch_version requires a chain handle"))?;

        let parent_asset_hash = self
            .asset_hash_for_name(name)
            .await
            .ok_or_else(|| anyhow::anyhow!("cannot branch unknown type '{name}'"))?;

        let parent_lineage_id = self
            .parent_branch_point(&parent_asset_hash, from_version, name)
            .await?;
        let genesis_reg = self.find_type(name).await?;
        let (branch_reg, pointer_json) =
            branch_parts(name, &parent_lineage_id, &parent_asset_hash, &genesis_reg)?;
        let branch_asset_hash = branch_reg.content_hash;

        // Recoverable, C1-hash-committed pointer in StoragePointer::Local; the
        // fresh (parent-bound) asset_hash makes add_block stamp this seq-0
        // genesis of an independent chain.
        let entry = BlockAssetEntry::new_bound(
            branch_asset_hash,
            state_proof,
            StoragePointer::Local { path: pointer_json },
            branch_reg,
        );
        chain
            .add_block(vec![entry])
            .await
            .map_err(|e| anyhow::anyhow!("on-chain branch genesis failed: {e}"))?;

        let lineage = chain.asset_lineage(&branch_asset_hash).await;
        let head = lineage
            .head()
            .ok_or_else(|| anyhow::anyhow!("branch genesis not found on chain after write"))?;
        Ok(VersionRef {
            asset_hash: branch_asset_hash,
            seq: head.asset_seq(),
            lineage_id: head.lineage_id(),
        })
    }

    /// The `lineage_id` of the parent version a branch forks from: the version
    /// at `from_version` (`asset_seq`), or the parent head when `None`.
    async fn parent_branch_point(
        &self,
        parent_asset_hash: &[u8; 32],
        from_version: Option<u64>,
        name: &str,
    ) -> Result<String> {
        let versions = self.list_versions(parent_asset_hash).await;
        let parent = match from_version {
            Some(seq) => versions
                .iter()
                .find(|v| v.seq == seq)
                .ok_or_else(|| anyhow::anyhow!("type '{name}' has no version at seq {seq}"))?,
            None => versions
                .last()
                .ok_or_else(|| anyhow::anyhow!("type '{name}' has no versions to branch from"))?,
        };
        Ok(parent.lineage_id.clone())
    }
}

/// Build the branch genesis registration + its recoverable pointer JSON.
///
/// The JSON is placed in the registration's `AssetData.metadata` (so it feeds
/// the branch `content_hash`) AND returned for the entry's
/// `StoragePointer::Local` payload (so it is recoverable on-chain).
fn branch_parts(
    name: &str,
    parent_lineage_id: &str,
    parent_asset_hash: &[u8; 32],
    genesis_reg: &AssetRegistration,
) -> Result<(AssetRegistration, String)> {
    let pointer_json = serde_json::to_string(&serde_json::json!({
        "branch_parent": parent_lineage_id,
        "branch_of_asset": hex::encode(parent_asset_hash),
    }))
    .map_err(|e| anyhow::anyhow!("failed to serialize branch pointer: {e}"))?;

    let data = AssetData {
        config: name.as_bytes().to_vec(),
        definition: b"catalog_branch".to_vec(),
        metadata: pointer_json.as_bytes().to_vec(),
    };
    let branch_reg = AssetRegistration::from_asset_data(
        &data,
        genesis_reg.network_scope.clone(),
        genesis_reg.category.clone(),
    );
    Ok((branch_reg, pointer_json))
}

#[cfg(test)]
#[path = "version_write_tests.rs"]
mod tests;
