// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! InboxStore -- blockchain-backed storage for received share invitations.
//!
//! Invites are registered as blockchain assets (like DNS entries and dashboards)
//! and persisted as YAML for human readability and backup.
//!
//! When a blockchain reference is available, `add()` registers the invite as
//! an on-chain asset (category `BaseSystem(Invitation)`) following the same
//! pattern used by DNS registration in [`crate::blockchain::mutations`].
//! YAML persistence acts as a local backup regardless of blockchain state.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, warn};

use super::invite::ShareInvite;
use crate::assets::core::asset_id::{
    AssetCategory, AssetData, BaseSystemType, NetworkScope,
};
use crate::assets::core::AssetRegistration;
use crate::blockchain::block::{BlockAssetEntry, StoragePointer};
use crate::blockchain::chain::NodeBlockchain;
use trustchain::proof_of_state::StateProof;

/// Blockchain-backed inbox store for share invitations.
///
/// Invites are registered as blockchain assets (like DNS entries and
/// dashboards). Persisted as YAML for human readability and local backup.
///
/// Operates in two modes:
/// - **Standalone** (`new`): YAML-only persistence, no blockchain.
///   Suitable for tests and standalone operation.
/// - **Blockchain-backed** (`with_blockchain`): Registers invites on-chain
///   AND persists to YAML.
pub struct InboxStore {
    /// Reference to the node's blockchain for asset registration.
    blockchain: Option<Arc<RwLock<NodeBlockchain>>>,
    /// In-memory cache of pending invites.
    invites: Arc<RwLock<HashMap<String, ShareInvite>>>,
    /// Data directory for YAML backup file.
    data_dir: Option<PathBuf>,
}

impl InboxStore {
    /// Create a new empty inbox (standalone mode, no blockchain).
    ///
    /// If `data_dir` is `Some`, [`persist`] and [`load`] will read/write
    /// `{data_dir}/invites.yaml`.
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        Self {
            blockchain: None,
            invites: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
        }
    }

    /// Create a new inbox backed by a blockchain.
    ///
    /// Invites added via [`add`] will be registered as on-chain assets
    /// in addition to YAML persistence.
    pub fn with_blockchain(
        data_dir: Option<PathBuf>,
        blockchain: Arc<RwLock<NodeBlockchain>>,
    ) -> Self {
        Self {
            blockchain: Some(blockchain),
            invites: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
        }
    }

    /// Add (or replace) an invite in the inbox.
    ///
    /// 1. Stores in memory cache.
    /// 2. Registers as blockchain asset (if blockchain available).
    /// 3. Persists YAML backup.
    pub async fn add(&self, invite: ShareInvite) -> anyhow::Result<()> {
        let id = invite.invite_id.clone();
        self.invites.write().await.insert(id, invite.clone());

        // Register on blockchain (best-effort for alpha)
        if let Some(ref bc) = self.blockchain {
            if let Err(e) = self.register_invite_asset(&invite, bc).await {
                warn!(
                    invite_id = %invite.invite_id,
                    "Failed to register invite on blockchain (continuing with YAML): {e}"
                );
            }
        }

        // Persist YAML backup
        self.persist_yaml().await?;

        Ok(())
    }

    /// List all invites, sorted by `created_at` descending (newest first).
    pub async fn list(&self) -> Vec<ShareInvite> {
        let map = self.invites.read().await;
        let mut items: Vec<ShareInvite> = map.values().cloned().collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items
    }

    /// Get a single invite by ID.
    pub async fn get(&self, invite_id: &str) -> Option<ShareInvite> {
        self.invites.read().await.get(invite_id).cloned()
    }

    /// Remove and return an invite by ID.
    pub async fn remove(&self, invite_id: &str) -> Option<ShareInvite> {
        let removed = self.invites.write().await.remove(invite_id);
        if removed.is_some() {
            // Best-effort persist after removal
            if let Err(e) = self.persist_yaml().await {
                warn!("Failed to persist YAML after invite removal: {e}");
            }
        }
        removed
    }

    /// Number of invites in the inbox.
    pub async fn count(&self) -> usize {
        self.invites.read().await.len()
    }

    /// Persist the current inbox to `{data_dir}/invites.yaml`.
    ///
    /// No-op if `data_dir` was not provided.
    pub async fn persist(&self) -> anyhow::Result<()> {
        self.persist_yaml().await
    }

    /// Load invites from `{data_dir}/invites.yaml`, merging into current state.
    ///
    /// Also attempts to load from legacy `inbox.json` if YAML file does not
    /// exist (one-time migration).
    ///
    /// No-op if `data_dir` was not provided or no file exists.
    pub async fn load(&self) -> anyhow::Result<()> {
        let dir = match &self.data_dir {
            Some(d) => d,
            None => return Ok(()),
        };

        let yaml_path = dir.join("invites.yaml");
        let json_path = dir.join("inbox.json");

        if yaml_path.exists() {
            // Load from YAML (preferred).
            // Roundtrip through serde_json::Value to handle serde_bytes fields.
            let data = std::fs::read_to_string(&yaml_path)
                .map_err(|e| anyhow::anyhow!("Failed to read YAML inbox: {e}"))?;
            let json_value: serde_json::Value = serde_yaml::from_str(&data)
                .map_err(|e| anyhow::anyhow!("Failed to parse YAML inbox: {e}"))?;
            let items: Vec<ShareInvite> = serde_json::from_value(json_value)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize YAML inbox: {e}"))?;
            let mut map = self.invites.write().await;
            for invite in items {
                map.insert(invite.invite_id.clone(), invite);
            }
        } else if json_path.exists() {
            // Legacy migration: load from JSON, then persist as YAML
            let data = std::fs::read(&json_path)
                .map_err(|e| anyhow::anyhow!("Failed to read legacy JSON inbox: {e}"))?;
            let items: Vec<ShareInvite> = serde_json::from_slice(&data)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize legacy JSON inbox: {e}"))?;
            let mut map = self.invites.write().await;
            for invite in items {
                map.insert(invite.invite_id.clone(), invite);
            }
            drop(map);
            // Migrate to YAML
            self.persist_yaml().await?;
            info!("Migrated inbox from JSON to YAML format");
        }

        Ok(())
    }

    /// Persist invites to YAML backup file.
    ///
    /// ShareInvite contains `#[serde(with = "serde_bytes")]` fields that
    /// `serde_yaml` cannot serialize directly. We convert through
    /// `serde_json::Value` first (which handles bytes as arrays), then
    /// emit YAML from that intermediate representation.
    async fn persist_yaml(&self) -> anyhow::Result<()> {
        let dir = match &self.data_dir {
            Some(d) => d,
            None => return Ok(()),
        };
        std::fs::create_dir_all(dir)
            .map_err(|e| anyhow::anyhow!("Failed to create inbox dir: {e}"))?;
        let path = dir.join("invites.yaml");
        let map = self.invites.read().await;
        let items: Vec<&ShareInvite> = map.values().collect();
        // Roundtrip through serde_json::Value to avoid serde_bytes incompatibility
        let json_value = serde_json::to_value(&items)
            .map_err(|e| anyhow::anyhow!("Failed to serialize inbox: {e}"))?;
        let yaml = serde_yaml::to_string(&json_value)
            .map_err(|e| anyhow::anyhow!("Failed to convert inbox to YAML: {e}"))?;
        std::fs::write(&path, yaml.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to write YAML inbox file: {e}"))?;
        Ok(())
    }

    /// Register an invite as a blockchain asset, following the DNS pattern.
    ///
    /// Creates a `BlockAssetEntry` with `BaseSystemType::Invitation` and
    /// stores the JSON-serialized invite in `StoragePointer::Local`
    /// (matching the DNS registration pattern in `mutations.rs`).
    async fn register_invite_asset(
        &self,
        invite: &ShareInvite,
        blockchain: &Arc<RwLock<NodeBlockchain>>,
    ) -> anyhow::Result<()> {
        // Serialize invite to JSON for the storage pointer payload
        // (matches DNS pattern which stores JSON in StoragePointer::Local)
        let invite_json = serde_json::to_string(invite)
            .map_err(|e| anyhow::anyhow!("Failed to serialize invite: {e}"))?;
        let invite_bytes = invite_json.as_bytes();

        // Build AssetRegistration with Invitation category
        let asset_data = AssetData {
            config: Vec::new(),
            definition: invite_bytes.to_vec(),
            metadata: format!("ShareInvite:{}", invite.invite_id).into_bytes(),
        };
        let registration = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Invitation),
        );

        // Compute hashes (following mutations.rs pattern)
        let asset_hash = registration.content_hash;
        let state_proof = StateProof::new_for_testing();
        let proof_bytes = serde_json::to_vec(&state_proof).unwrap_or_default();
        let proof_hash = *blake3::hash(&proof_bytes).as_bytes();

        // Store serialized invite JSON in the path field (same as DNS pattern)
        let entry = BlockAssetEntry {
            asset_hash,
            proof_hash,
            state_proof,
            storage_pointer: StoragePointer::Local {
                path: invite_json,
            },
            registration,
        };

        // Add the block to the blockchain
        let chain = blockchain.read().await;
        match chain.add_block(vec![entry]).await {
            Ok(block) => {
                info!(
                    invite_id = %invite.invite_id,
                    block_index = block.index,
                    "Registered invite as blockchain asset"
                );
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!(
                "Failed to add invite block: {e}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_invite(id: &str, created_at: i64) -> ShareInvite {
        ShareInvite::new(
            id.into(),
            "asset-1".into(),
            "sender".into(),
            None,
            "recipient".into(),
            "file.bin".into(),
            1024,
            4,
            b"[]".to_vec(),
            vec![0xAA; 32],
            vec![0xBB; 64],
            created_at,
        )
    }

    #[tokio::test]
    async fn test_inbox_add_list_remove() {
        let store = InboxStore::new(None);

        store.add(make_invite("a", 100)).await.expect("test: add a");
        store.add(make_invite("b", 200)).await.expect("test: add b");

        let list = store.list().await;
        assert_eq!(list.len(), 2);
        // Newest first
        assert_eq!(list[0].invite_id, "b");
        assert_eq!(list[1].invite_id, "a");

        let got = store.get("a").await;
        assert!(got.is_some());
        assert_eq!(got.expect("test: get a").invite_id, "a");

        let removed = store.remove("a").await;
        assert!(removed.is_some());
        assert_eq!(store.count().await, 1);

        assert!(store.get("a").await.is_none());
    }

    #[tokio::test]
    async fn test_inbox_count() {
        let store = InboxStore::new(None);
        assert_eq!(store.count().await, 0);

        store.add(make_invite("x", 10)).await.expect("test: add");
        assert_eq!(store.count().await, 1);

        store.add(make_invite("y", 20)).await.expect("test: add");
        assert_eq!(store.count().await, 2);

        store.remove("x").await;
        assert_eq!(store.count().await, 1);
    }

    #[tokio::test]
    async fn test_inbox_persistence_yaml() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let inbox_dir = dir.path().join("inbox");

        // Create and persist
        let store1 = InboxStore::new(Some(inbox_dir.clone()));
        store1
            .add(make_invite("p1", 300))
            .await
            .expect("test: add");
        store1
            .add(make_invite("p2", 400))
            .await
            .expect("test: add");
        store1.persist().await.expect("test: persist");

        // Verify YAML file exists and is YAML format
        let yaml_path = inbox_dir.join("invites.yaml");
        assert!(yaml_path.exists(), "YAML file must exist");
        let content = std::fs::read_to_string(&yaml_path).expect("test: read yaml");
        assert!(
            content.contains("invite_id:"),
            "File must be YAML format, got: {content}"
        );

        // Load into a new store
        let store2 = InboxStore::new(Some(inbox_dir));
        store2.load().await.expect("test: load");

        assert_eq!(store2.count().await, 2);
        assert!(store2.get("p1").await.is_some());
        assert!(store2.get("p2").await.is_some());
    }

    #[tokio::test]
    async fn test_inbox_legacy_json_migration() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let inbox_dir = dir.path().join("inbox");
        std::fs::create_dir_all(&inbox_dir).expect("test: mkdir");

        // Write a legacy JSON file
        let invites = vec![make_invite("legacy1", 100), make_invite("legacy2", 200)];
        let json = serde_json::to_vec_pretty(&invites).expect("test: serialize json");
        std::fs::write(inbox_dir.join("inbox.json"), &json).expect("test: write json");

        // Load should migrate from JSON to YAML
        let store = InboxStore::new(Some(inbox_dir.clone()));
        store.load().await.expect("test: load legacy");

        assert_eq!(store.count().await, 2);
        assert!(store.get("legacy1").await.is_some());
        assert!(store.get("legacy2").await.is_some());

        // YAML file should now exist (migrated)
        let yaml_path = inbox_dir.join("invites.yaml");
        assert!(yaml_path.exists(), "YAML file must be created during migration");
    }

    #[tokio::test]
    async fn test_inbox_load_nonexistent_is_noop() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let store = InboxStore::new(Some(dir.path().join("nonexistent")));
        // Should succeed silently
        store.load().await.expect("test: load nonexistent");
        assert_eq!(store.count().await, 0);
    }

    #[tokio::test]
    async fn test_inbox_with_blockchain_constructor() {
        // Verify with_blockchain constructor works
        use crate::matrix::coordinate::MatrixCoordinate;
        let coord = MatrixCoordinate::new(1, 1, 1).expect("test: coord");
        let chain = Arc::new(RwLock::new(NodeBlockchain::new(coord)));
        let store = InboxStore::with_blockchain(None, chain);
        assert_eq!(store.count().await, 0);
        assert!(store.blockchain.is_some());
    }

    #[tokio::test]
    async fn test_inbox_blockchain_registration() {
        use crate::matrix::coordinate::MatrixCoordinate;
        let coord = MatrixCoordinate::new(2, 2, 2).expect("test: coord");
        let chain = Arc::new(RwLock::new(NodeBlockchain::new(coord)));

        let store = InboxStore::with_blockchain(None, chain.clone());
        store.add(make_invite("bc1", 500)).await.expect("test: add with blockchain");

        // Verify the invite was registered on-chain
        let bc = chain.read().await;
        let height = bc.get_height().await;
        // Genesis block is at index 0, invite should be at index 1
        assert!(height >= 1, "Blockchain should have at least 1 block after invite registration");

        let block = bc.get_block(1).await;
        assert!(block.is_some(), "Block 1 should exist");
        let block = block.expect("test: block exists");
        assert_eq!(block.entries.len(), 1);
        assert_eq!(
            block.entries[0].registration.category,
            AssetCategory::BaseSystem(BaseSystemType::Invitation),
        );
    }
}
