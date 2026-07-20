// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! MessageStore -- blockchain-backed storage for direct messages.
//!
//! Messages are registered as blockchain assets and persisted as YAML
//! for human readability and backup, following the same pattern as
//! [`crate::sharing::inbox::InboxStore`].

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, warn};

use super::message::DirectMessage;
use crate::assets::core::asset_id::{
    AssetCategory, AssetData, BaseSystemType, NetworkScope,
};
use crate::assets::core::AssetRegistration;
use crate::blockchain::block::{BlockAssetEntry, StoragePointer};
use crate::blockchain::chain::NodeBlockchain;
use trustchain::proof_of_state::StateProof;
use trustchain::proof_of_state::StateProofOps;

/// Blockchain-backed message store.
///
/// Operates in two modes:
/// - **Standalone** (`new`): YAML-only persistence, no blockchain.
/// - **Blockchain-backed** (`with_blockchain`): Registers messages on-chain
///   AND persists to YAML.
pub struct MessageStore {
    /// Reference to the node's blockchain for asset registration.
    blockchain: Option<Arc<RwLock<NodeBlockchain>>>,
    /// In-memory cache of messages keyed by message_id.
    messages: Arc<RwLock<HashMap<String, DirectMessage>>>,
    /// Data directory for YAML backup file.
    data_dir: Option<PathBuf>,
}

impl MessageStore {
    /// Create a new empty store (standalone mode, no blockchain).
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        Self {
            blockchain: None,
            messages: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
        }
    }

    /// Create a new store backed by a blockchain.
    pub fn with_blockchain(
        data_dir: Option<PathBuf>,
        blockchain: Arc<RwLock<NodeBlockchain>>,
    ) -> Self {
        Self {
            blockchain: Some(blockchain),
            messages: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
        }
    }

    /// Add a message to the store.
    ///
    /// 1. Stores in memory cache.
    /// 2. Registers as blockchain asset (if blockchain available).
    /// 3. Persists YAML backup.
    pub async fn add(&self, message: DirectMessage) -> anyhow::Result<()> {
        let id = message.message_id.clone();
        self.messages.write().await.insert(id, message.clone());

        if let Some(ref bc) = self.blockchain {
            if let Err(e) = self.register_message_asset(&message, bc).await {
                warn!(
                    message_id = %message.message_id,
                    "Failed to register message on blockchain (continuing with YAML): {e}"
                );
            }
        }

        self.persist_yaml().await?;
        Ok(())
    }

    /// List messages where `recipient_node_id == node_id`, newest first.
    pub async fn list_for_recipient(&self, node_id: &str) -> Vec<DirectMessage> {
        let map = self.messages.read().await;
        let mut items: Vec<DirectMessage> = map
            .values()
            .filter(|m| m.recipient_node_id == node_id)
            .cloned()
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items
    }

    /// Return messages exchanged between `our_id` and `peer_id`, sorted
    /// by `created_at` ascending (oldest first, like a chat log).
    pub async fn history_with_peer(
        &self,
        our_id: &str,
        peer_id: &str,
    ) -> Vec<DirectMessage> {
        let map = self.messages.read().await;
        let mut items: Vec<DirectMessage> = map
            .values()
            .filter(|m| {
                (m.sender_node_id == our_id && m.recipient_node_id == peer_id)
                    || (m.sender_node_id == peer_id && m.recipient_node_id == our_id)
            })
            .cloned()
            .collect();
        items.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        items
    }

    /// Get a single message by ID.
    pub async fn get(&self, message_id: &str) -> Option<DirectMessage> {
        self.messages.read().await.get(message_id).cloned()
    }

    /// Number of messages in the store.
    pub async fn count(&self) -> usize {
        self.messages.read().await.len()
    }

    /// Persist current messages to YAML backup file.
    pub async fn persist(&self) -> anyhow::Result<()> {
        self.persist_yaml().await
    }

    /// Load messages from `{data_dir}/messages.yaml`, merging into state.
    pub async fn load(&self) -> anyhow::Result<()> {
        let dir = match &self.data_dir {
            Some(d) => d,
            None => return Ok(()),
        };

        let yaml_path = dir.join("messages.yaml");
        if !yaml_path.exists() {
            return Ok(());
        }

        let data = std::fs::read_to_string(&yaml_path)
            .map_err(|e| anyhow::anyhow!("Failed to read messages YAML: {e}"))?;
        let json_value: serde_json::Value = serde_yaml::from_str(&data)
            .map_err(|e| anyhow::anyhow!("Failed to parse messages YAML: {e}"))?;
        let items: Vec<DirectMessage> = serde_json::from_value(json_value)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize messages: {e}"))?;
        let mut map = self.messages.write().await;
        for msg in items {
            map.insert(msg.message_id.clone(), msg);
        }
        Ok(())
    }

    /// Persist messages to `{data_dir}/messages.yaml`.
    async fn persist_yaml(&self) -> anyhow::Result<()> {
        let dir = match &self.data_dir {
            Some(d) => d,
            None => return Ok(()),
        };
        std::fs::create_dir_all(dir)
            .map_err(|e| anyhow::anyhow!("Failed to create messages dir: {e}"))?;
        let path = dir.join("messages.yaml");
        let map = self.messages.read().await;
        let items: Vec<&DirectMessage> = map.values().collect();
        let json_value = serde_json::to_value(&items)
            .map_err(|e| anyhow::anyhow!("Failed to serialize messages: {e}"))?;
        let yaml = serde_yaml::to_string(&json_value)
            .map_err(|e| anyhow::anyhow!("Failed to convert messages to YAML: {e}"))?;
        std::fs::write(&path, yaml.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to write messages YAML: {e}"))?;
        Ok(())
    }

    /// Register a message as a blockchain asset (BaseSystem(Message)).
    async fn register_message_asset(
        &self,
        message: &DirectMessage,
        blockchain: &Arc<RwLock<NodeBlockchain>>,
    ) -> anyhow::Result<()> {
        let msg_json = serde_json::to_string(message)
            .map_err(|e| anyhow::anyhow!("Failed to serialize message: {e}"))?;
        let msg_bytes = msg_json.as_bytes();

        let asset_data = AssetData {
            config: Vec::new(),
            definition: msg_bytes.to_vec(),
            metadata: format!("DirectMessage:{}", message.message_id).into_bytes(),
        };
        let registration = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::Application(crate::assets::core::asset_id::ApplicationDomain {
                domain_name: "Message".to_string(),
                domain_hash: *blake3::hash(b"Message-v1-schema").as_bytes(),
            }),
        );

        let chain = blockchain.read().await;

        // Generate a REAL PoS proof from this node's own identity, derived
        // deterministically from its matrix coordinate (R1: hardware-assessed).
        let node_id = crate::bootstrap::node_id(chain.node_coordinate());
        let state_proof = StateProof::generate_from_network(&node_id)
            .await
            .map_err(|e| anyhow::anyhow!("state proof generation: {e}"))?;

        // Bind the proof to the content hash (signed-to-content invariant, P1).
        let asset_hash = registration.content_hash;
        let entry = BlockAssetEntry::new_bound(
            asset_hash,
            &state_proof,
            StoragePointer::Local { path: msg_json },
            registration,
        );

        match chain.add_block(vec![entry]).await {
            Ok(block) => {
                info!(
                    message_id = %message.message_id,
                    block_index = block.index,
                    "Registered message as blockchain asset"
                );
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("Failed to add message block: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(id: &str, sender: &str, recipient: &str, created_at: i64) -> DirectMessage {
        DirectMessage {
            message_id: id.into(),
            sender_node_id: sender.into(),
            sender_name: None,
            recipient_node_id: recipient.into(),
            encrypted_body: vec![0xAA; 16],
            kem_ciphertext: vec![0xBB; 32],
            reply_to: None,
            content_type: "text/plain".into(),
            created_at,
            signature: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_store_add_and_list() {
        let store = MessageStore::new(None);

        let m1 = make_message("m1", "alice", "bob", 100);
        let m2 = make_message("m2", "carol", "bob", 200);
        let m3 = make_message("m3", "alice", "carol", 300);

        store.add(m1).await.expect("test: add m1");
        store.add(m2).await.expect("test: add m2");
        store.add(m3).await.expect("test: add m3");

        let bob_inbox = store.list_for_recipient("bob").await;
        assert_eq!(bob_inbox.len(), 2);
        // Newest first
        assert_eq!(bob_inbox[0].message_id, "m2");
        assert_eq!(bob_inbox[1].message_id, "m1");

        let carol_inbox = store.list_for_recipient("carol").await;
        assert_eq!(carol_inbox.len(), 1);
        assert_eq!(carol_inbox[0].message_id, "m3");
    }

    #[tokio::test]
    async fn test_store_history_with_peer() {
        let store = MessageStore::new(None);

        store
            .add(make_message("h1", "alice", "bob", 100))
            .await
            .expect("test: add");
        store
            .add(make_message("h2", "bob", "alice", 200))
            .await
            .expect("test: add");
        store
            .add(make_message("h3", "alice", "bob", 300))
            .await
            .expect("test: add");
        store
            .add(make_message("h4", "alice", "carol", 400))
            .await
            .expect("test: add");

        let history = store.history_with_peer("alice", "bob").await;
        assert_eq!(history.len(), 3);
        // Oldest first
        assert_eq!(history[0].message_id, "h1");
        assert_eq!(history[1].message_id, "h2");
        assert_eq!(history[2].message_id, "h3");
    }

    #[tokio::test]
    async fn test_store_persistence_yaml() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let msg_dir = dir.path().join("messages");

        let store1 = MessageStore::new(Some(msg_dir.clone()));
        store1
            .add(make_message("p1", "a", "b", 100))
            .await
            .expect("test: add");
        store1
            .add(make_message("p2", "c", "d", 200))
            .await
            .expect("test: add");
        store1.persist().await.expect("test: persist");

        let yaml_path = msg_dir.join("messages.yaml");
        assert!(yaml_path.exists(), "YAML file must exist");
        let content = std::fs::read_to_string(&yaml_path).expect("test: read yaml");
        assert!(
            content.contains("message_id:"),
            "File must be YAML format, got: {content}"
        );

        let store2 = MessageStore::new(Some(msg_dir));
        store2.load().await.expect("test: load");
        assert_eq!(store2.count().await, 2);
        assert!(store2.get("p1").await.is_some());
        assert!(store2.get("p2").await.is_some());
    }

    #[tokio::test]
    async fn test_store_count() {
        let store = MessageStore::new(None);
        assert_eq!(store.count().await, 0);

        store
            .add(make_message("c1", "a", "b", 10))
            .await
            .expect("test: add");
        assert_eq!(store.count().await, 1);

        store
            .add(make_message("c2", "a", "b", 20))
            .await
            .expect("test: add");
        assert_eq!(store.count().await, 2);
    }
}
