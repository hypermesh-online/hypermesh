// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog VCS view — READ an asset's version graph from the asset-chain.
//!
//! Phase 2 of the Catalog-VCS / NGauge-DMS roadmap. Catalog is the VCS *view*
//! over the single lineage substrate: the asset's OWN chain
//! ([`blockmatrix::blockchain::AssetLineage`]) is the source of truth for its
//! version history. Catalog does NOT own a version store — it READS the chain.
//!
//! # Version-identity model (settled here)
//!
//! - `asset_hash` = the asset's GENESIS identity, carried forward by every later
//!   lineage entry. It is stable across versions — a version is not a new asset,
//!   it is the next entry in the same chain. A Catalog `type_name` maps to this
//!   `asset_hash` via its registration's `content_hash`
//!   ([`asset_hash_for_name`](CatalogRegistry::asset_hash_for_name)).
//! - A VERSION = one lineage entry. Its per-version content id is the entry's
//!   `lineage_id` (`= hex(proof_hash)`), which is exactly what a successor entry
//!   names in `prev_asset_entry`.
//!
//! This module is READ-ONLY. It never writes to any chain (that is Phase 3,
//! gated on cluster G) and never changes existing registration behavior. Every
//! method returns empty/`None` when no chain handle is attached or the asset is
//! unknown — it never panics.

use serde::{Deserialize, Serialize};

use super::catalog_registry::CatalogRegistry;

/// A single version in an asset's chain, as read from its [`AssetLineage`].
///
/// [`AssetLineage`]: blockmatrix::blockchain::AssetLineage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionRef {
    /// The asset's stable GENESIS identity (`hA`), shared by every version in
    /// the chain. Raw 32-byte content hash.
    pub asset_hash: [u8; 32],

    /// Position in the asset chain: `0` = genesis (v1), advancing by one per
    /// version.
    pub seq: u64,

    /// This version's per-version content id — `hex(proof_hash)`, the entry's
    /// `lineage_id`. A successor version names this in `prev_asset_entry`.
    pub lineage_id: String,
}

impl VersionRef {
    /// The asset's genesis identity as lowercase hex — the friendly spelling of
    /// [`asset_hash`](Self::asset_hash).
    pub fn asset_hash_hex(&self) -> String {
        hex::encode(self.asset_hash)
    }
}

impl CatalogRegistry {
    /// Resolve a registered `type_name` to its stable asset-chain identity
    /// (`asset_hash` = the registration's `content_hash`, the genesis identity).
    ///
    /// Returns `None` if the name is not registered. This is the name↔asset_hash
    /// bridge between the Catalog index (keyed by name) and the lineage
    /// substrate (keyed by asset_hash).
    pub async fn asset_hash_for_name(&self, name: &str) -> Option<[u8; 32]> {
        self.find_type(name).await.ok().map(|reg| reg.content_hash)
    }

    /// List every version of an asset, oldest first, from its lineage.
    ///
    /// Returns an empty vector when no chain handle is attached or the asset has
    /// no entries in this container. Never panics.
    pub async fn list_versions(&self, asset_hash: &[u8; 32]) -> Vec<VersionRef> {
        let Some(chain) = self.chain.as_ref() else {
            return Vec::new();
        };
        let lineage = chain.asset_lineage(asset_hash).await;
        lineage
            .entries
            .iter()
            .map(|entry| VersionRef {
                asset_hash: *asset_hash,
                seq: entry.asset_seq(),
                lineage_id: entry.lineage_id(),
            })
            .collect()
    }

    /// The asset's current head (most recent) version, if any.
    ///
    /// Returns `None` when no chain handle is attached or the asset is unknown.
    pub async fn head_version(&self, asset_hash: &[u8; 32]) -> Option<VersionRef> {
        let chain = self.chain.as_ref()?;
        let lineage = chain.asset_lineage(asset_hash).await;
        lineage.head().map(|entry| VersionRef {
            asset_hash: *asset_hash,
            seq: entry.asset_seq(),
            lineage_id: entry.lineage_id(),
        })
    }

    /// The asset's ordered version progression.
    ///
    /// Linear today (the substrate is a single chain per asset); branch support
    /// arrives with the Phase 3+ write side. Currently equivalent to
    /// [`list_versions`](Self::list_versions), kept as a distinct name so the
    /// VCS-graph call site is explicit.
    pub async fn version_graph(&self, asset_hash: &[u8; 32]) -> Vec<VersionRef> {
        self.list_versions(asset_hash).await
    }

    /// [`list_versions`](Self::list_versions) addressed by registered type name.
    pub async fn list_versions_by_name(&self, name: &str) -> Vec<VersionRef> {
        match self.asset_hash_for_name(name).await {
            Some(asset_hash) => self.list_versions(&asset_hash).await,
            None => Vec::new(),
        }
    }

    /// [`head_version`](Self::head_version) addressed by registered type name.
    pub async fn head_version_by_name(&self, name: &str) -> Option<VersionRef> {
        let asset_hash = self.asset_hash_for_name(name).await?;
        self.head_version(&asset_hash).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::catalog_registry::{RegistryConfig, TrustPolicy};
    use blockmatrix::assets::core::{AssetCategory, AssetData, BaseSystemType, NetworkScope};
    use blockmatrix::assets::AssetRegistration;
    use blockmatrix::blockchain::block::{BlockAssetEntry, StoragePointer};
    use blockmatrix::blockchain::NodeBlockchain;
    use blockmatrix::matrix::coordinate::MatrixCoordinate;
    use blockmatrix::proof_of_state::StateProof;
    use hypermesh_lib::PrivacyMode;
    use std::sync::Arc;

    /// Build a distinct registration per version (its own content differs, so it
    /// stands in for a real per-version payload).
    fn registration(tag: &[u8]) -> AssetRegistration {
        let data = AssetData {
            config: Vec::new(),
            definition: tag.to_vec(),
            metadata: b"version".to_vec(),
        };
        AssetRegistration::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Container),
        )
    }

    /// Append one entry for `asset_hash` (a new version). `stamp_asset_lineage`
    /// inside `add_block` links it onto the asset's current head, so successive
    /// calls with the same `asset_hash` build a multi-entry lineage.
    async fn push_version(chain: &NodeBlockchain, asset_hash: [u8; 32], tag: &[u8]) {
        let proof = StateProof::new_for_testing();
        let entry = BlockAssetEntry::new_bound(
            asset_hash,
            &proof,
            StoragePointer::Genesis,
            registration(tag),
        );
        chain
            .add_block(vec![entry])
            .await
            .expect("test: append version entry");
    }

    /// A chain carrying a 3-entry lineage for `asset_hash`, plus a registry with
    /// that chain attached.
    async fn registry_with_three_versions(
        asset_hash: [u8; 32],
    ) -> (CatalogRegistry, Arc<NodeBlockchain>) {
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let chain = NodeBlockchain::new(coord);
        push_version(&chain, asset_hash, b"v1").await;
        push_version(&chain, asset_hash, b"v2").await;
        push_version(&chain, asset_hash, b"v3").await;
        let chain = Arc::new(chain);
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        )
        .with_chain(chain.clone());
        (registry, chain)
    }

    #[tokio::test]
    async fn test_list_versions_returns_entries_in_seq_order() {
        let asset_hash = [7u8; 32];
        let (registry, _chain) = registry_with_three_versions(asset_hash).await;

        let versions = registry.list_versions(&asset_hash).await;
        assert_eq!(versions.len(), 3, "three versions appended");
        assert_eq!(
            versions.iter().map(|v| v.seq).collect::<Vec<_>>(),
            vec![0, 1, 2],
            "seq advances by one from genesis"
        );
        for v in &versions {
            assert_eq!(v.asset_hash, asset_hash);
            assert!(!v.lineage_id.is_empty(), "each version has a content id");
        }
        // Every version's lineage_id is distinct (distinct content).
        let mut ids = versions.iter().map(|v| v.lineage_id.clone()).collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 3, "lineage ids are distinct per version");
    }

    #[tokio::test]
    async fn test_head_version_is_the_tip() {
        let asset_hash = [9u8; 32];
        let (registry, _chain) = registry_with_three_versions(asset_hash).await;

        let versions = registry.list_versions(&asset_hash).await;
        let head = registry
            .head_version(&asset_hash)
            .await
            .expect("test: head present");
        assert_eq!(head.seq, 2, "head is the most recent version");
        assert_eq!(
            head.lineage_id,
            versions.last().expect("test: last version").lineage_id,
            "head_version equals the tip of list_versions"
        );
    }

    #[tokio::test]
    async fn test_version_graph_matches_list_versions() {
        let asset_hash = [3u8; 32];
        let (registry, _chain) = registry_with_three_versions(asset_hash).await;

        assert_eq!(
            registry.version_graph(&asset_hash).await,
            registry.list_versions(&asset_hash).await,
            "linear graph == ordered version list today"
        );
    }

    #[tokio::test]
    async fn test_read_methods_empty_without_chain_handle() {
        // No chain attached — every read is empty/None, never a panic.
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );
        assert!(!registry.has_chain());
        let asset_hash = [1u8; 32];
        assert!(registry.list_versions(&asset_hash).await.is_empty());
        assert!(registry.head_version(&asset_hash).await.is_none());
        assert!(registry.version_graph(&asset_hash).await.is_empty());
    }

    #[tokio::test]
    async fn test_read_methods_empty_for_unknown_asset() {
        // Chain attached but the asset was never registered on it.
        let asset_hash = [5u8; 32];
        let (registry, _chain) = registry_with_three_versions(asset_hash).await;
        let unknown = [42u8; 32];
        assert!(registry.list_versions(&unknown).await.is_empty());
        assert!(registry.head_version(&unknown).await.is_none());
    }

    #[tokio::test]
    async fn test_lineage_verifies_and_matches_view() {
        // The read view is faithful to the substrate: the lineage the registry
        // reports is exactly the one that verifies as an unbroken chain.
        let asset_hash = [11u8; 32];
        let (registry, chain) = registry_with_three_versions(asset_hash).await;
        let lineage = chain.asset_lineage(&asset_hash).await;
        lineage.verify().expect("test: lineage is an unbroken chain");
        assert_eq!(
            registry.list_versions(&asset_hash).await.len(),
            lineage.len()
        );
    }

    #[tokio::test]
    async fn test_asset_hash_for_name_unknown_is_none() {
        let registry = CatalogRegistry::new(
            PrivacyMode::PUBLIC,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );
        assert!(registry.asset_hash_for_name("Nope").await.is_none());
        assert!(registry.list_versions_by_name("Nope").await.is_empty());
        assert!(registry.head_version_by_name("Nope").await.is_none());
    }
}
