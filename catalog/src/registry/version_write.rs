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
mod tests {
    use super::*;
    use crate::registry::catalog_registry::{RegistryConfig, TrustPolicy};
    use crate::registry::AssetTypeDefinition;
    use blockmatrix::blockchain::NodeBlockchain;
    use blockmatrix::matrix::coordinate::MatrixCoordinate;
    use blockmatrix::proof_of_state::proof_of_state_integration::{
        SpaceProof, StakeProof, TimeProof, WorkProof,
    };
    use hypermesh_lib::PrivacyMode;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::Duration;

    /// A real (structurally valid) author proof: PoStake carries a bound
    /// authorization identity (WHO), never a magnitude — the canonical model.
    fn author_proof(who: &str) -> StateProof {
        let stake = StakeProof::new(who.to_string(), format!("{who}-id"));
        let space = SpaceProof::new(who.to_string(), format!("/{who}"), 1024);
        let work = WorkProof::from_work(who.to_string(), "register".to_string(), b"work");
        let time = TimeProof::new(Duration::from_secs(5));
        StateProof::new(stake, time, space, work)
    }

    /// A registry with a fresh in-mem `NodeBlockchain` attached and a relaxed
    /// trust policy (proofs are constructed explicitly per version).
    fn registry_with_chain() -> (CatalogRegistry, Arc<NodeBlockchain>) {
        let coord = MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate");
        let chain = Arc::new(NodeBlockchain::new(coord));
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        )
        .with_chain(chain.clone());
        (registry, chain)
    }

    /// Recover a branch entry's parent pointer from its `StoragePointer::Local`
    /// payload (the recoverable, C1-hash-committed place the pointer rides).
    fn branch_pointer(entry: &BlockAssetEntry) -> serde_json::Value {
        match &entry.storage_pointer {
            StoragePointer::Local { path } => {
                serde_json::from_str(path).expect("test: branch pointer JSON")
            }
            other => unreachable!("test: branch genesis expected Local pointer, got {other:?}"),
        }
    }

    fn type_def(name: &str, schema_id: &str, version: &str) -> AssetTypeDefinition {
        let schema = json!({ "type": "object", "id": schema_id });
        let mut td = AssetTypeDefinition::new(name.to_string(), schema, author_proof(name));
        td.metadata.version = version.to_string();
        td
    }

    /// v1 (genesis) → v2 (progression): the same name written twice lands two
    /// entries on ONE asset-chain, in seq order, and the lineage verifies as an
    /// unbroken chain — exactly what the Phase-2 read view reports.
    #[tokio::test]
    async fn test_register_genesis_then_progression() {
        let (registry, chain) = registry_with_chain();

        // v1 = genesis.
        registry
            .register_type(type_def("foo", "v1", "1.0.0"))
            .await
            .expect("test: register foo v1 (genesis)");

        let asset_hash = registry
            .asset_hash_for_name("foo")
            .await
            .expect("test: foo resolves to a genesis asset_hash");

        let after_v1 = registry.list_versions(&asset_hash).await;
        assert_eq!(after_v1.len(), 1, "genesis is the only version");
        assert_eq!(after_v1[0].seq, 0, "genesis is seq 0");

        // v2 = progression (single-slot relaxed: same name, new version).
        registry
            .register_type(type_def("foo", "v2", "2.0.0"))
            .await
            .expect("test: register foo v2 (progression)");

        let versions = registry.list_versions(&asset_hash).await;
        assert_eq!(versions.len(), 2, "genesis + one successor");
        assert_eq!(
            versions.iter().map(|v| v.seq).collect::<Vec<_>>(),
            vec![0, 1],
            "versions returned in seq order (v1 then v2)"
        );
        assert_eq!(
            versions[0].asset_hash, asset_hash,
            "both versions share the genesis asset_hash identity"
        );
        assert_eq!(versions[1].asset_hash, asset_hash);
        assert_ne!(
            versions[0].lineage_id, versions[1].lineage_id,
            "each version has a distinct per-version content id"
        );

        // The lineage the read view reports is the one that verifies unbroken.
        let lineage = chain.asset_lineage(&asset_hash).await;
        lineage
            .verify()
            .expect("test: linear asset chain verifies green");
        assert_eq!(lineage.sequence(), vec![0, 1]);
    }

    /// A branch mints a NEW independent asset-chain (its own genesis) whose
    /// metadata names the parent version + parent asset. The branch verifies as
    /// its own valid genesis and does NOT extend the parent chain.
    #[tokio::test]
    async fn test_branch_mints_independent_genesis_naming_parent() {
        let (registry, chain) = registry_with_chain();

        registry
            .register_type(type_def("foo", "v1", "1.0.0"))
            .await
            .expect("test: register foo v1");
        registry
            .register_type(type_def("foo", "v2", "2.0.0"))
            .await
            .expect("test: register foo v2");

        let parent_hash = registry
            .asset_hash_for_name("foo")
            .await
            .expect("test: foo resolves");
        let parent_head = registry
            .head_version(&parent_hash)
            .await
            .expect("test: parent head present");

        // Branch from the head (v2).
        let branch = registry
            .branch_version("foo", None, &author_proof("brancher"))
            .await
            .expect("test: branch foo from head");

        // The branch is a fresh, independent identity.
        assert_ne!(
            branch.asset_hash, parent_hash,
            "branch is a NEW asset-chain, not the parent"
        );
        assert_eq!(branch.seq, 0, "branch is its own genesis (seq 0)");

        // The branch genesis is a valid independent genesis.
        let branch_lineage = chain.asset_lineage(&branch.asset_hash).await;
        assert_eq!(branch_lineage.len(), 1, "branch chain has just its genesis");
        branch_lineage
            .verify()
            .expect("test: branch genesis verifies as a valid independent genesis");
        assert!(
            branch_lineage
                .root()
                .expect("test: branch root")
                .is_asset_genesis(),
            "branch root is a true asset genesis (prev=None, seq=0)"
        );

        // The branch genesis pointer NAMES the parent (recoverable, C1-committed).
        let root = branch_lineage.root().expect("test: branch root");
        let meta = branch_pointer(root);
        assert_eq!(
            meta["branch_parent"], parent_head.lineage_id,
            "branch_parent names the parent version's lineage_id"
        );
        assert_eq!(
            meta["branch_of_asset"],
            hex::encode(parent_hash),
            "branch_of_asset names the parent genesis asset_hash"
        );

        // The parent chain is untouched by the branch.
        assert_eq!(
            registry.list_versions(&parent_hash).await.len(),
            2,
            "branching did not append to the parent chain"
        );
    }

    /// Branch from a SPECIFIC earlier version (seq 0) — the pointer names that
    /// version's lineage_id, not the head's.
    #[tokio::test]
    async fn test_branch_from_specific_version() {
        let (registry, chain) = registry_with_chain();
        registry
            .register_type(type_def("bar", "v1", "1.0.0"))
            .await
            .expect("test: bar v1");
        registry
            .register_type(type_def("bar", "v2", "2.0.0"))
            .await
            .expect("test: bar v2");

        let parent_hash = registry.asset_hash_for_name("bar").await.expect("test: bar");
        let versions = registry.list_versions(&parent_hash).await;
        let v1 = &versions[0];

        let branch = registry
            .branch_version("bar", Some(0), &author_proof("brancher"))
            .await
            .expect("test: branch bar from seq 0");

        let root = chain
            .asset_lineage(&branch.asset_hash)
            .await
            .root()
            .cloned()
            .expect("test: branch root");
        let meta = branch_pointer(&root);
        assert_eq!(
            meta["branch_parent"], v1.lineage_id,
            "branch_parent names v1's lineage_id (not the head)"
        );
    }

    /// Without a chain handle Catalog is unchanged: a repeat name still rejects,
    /// and branching is refused.
    #[tokio::test]
    async fn test_standalone_unchanged_without_chain() {
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );
        registry
            .register_type(type_def("solo", "v1", "1.0.0"))
            .await
            .expect("test: first registration succeeds");
        let repeat = registry.register_type(type_def("solo", "v2", "2.0.0")).await;
        assert!(repeat.is_err(), "standalone: repeat name still rejects");
        assert!(repeat
            .unwrap_err()
            .to_string()
            .contains("already registered"));

        let branch = registry
            .branch_version("solo", None, &author_proof("x"))
            .await;
        assert!(branch.is_err(), "branch requires a chain handle");
    }

    /// Branching an unknown name errors rather than minting an orphan chain.
    #[tokio::test]
    async fn test_branch_unknown_name_errors() {
        let (registry, _chain) = registry_with_chain();
        let branch = registry
            .branch_version("ghost", None, &author_proof("x"))
            .await;
        assert!(branch.is_err(), "cannot branch an unregistered name");
    }
}
