// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar Storage Layer - Packet-centric EVP operations
//!
//! Stores ephemeral value packet records and settlement history.
//! No wallets, no persistent balances — packets are the only unit of value.

#[allow(unused_imports)]
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use hypermesh_lib::economic::{GoldGrams, PacketId, PacketState};

use crate::models::{PacketRecord, SettlementRecord};

// ============ Configuration ============

/// Storage configuration for Caesar EVP persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Path to the storage directory
    pub path: String,
}

// ============ Storage ============

/// Packet-centric storage for the Caesar EVP system.
///
/// All data is held in-memory with JSON file persistence.
/// No wallet tables, no balance ledgers — only packets and settlements.
pub struct CaesarStorage {
    /// Storage directory path
    storage_path: PathBuf,
    /// In-memory packet records (keyed by PacketId)
    packets: Arc<RwLock<HashMap<PacketId, PacketRecord>>>,
    /// In-memory settlement records (keyed by settlement_id, ordered)
    settlements: Arc<RwLock<BTreeMap<String, SettlementRecord>>>,
    /// In-memory economic metrics
    metrics: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl CaesarStorage {
    /// Create a new storage instance, loading any existing data from disk.
    pub async fn new(config: StorageConfig) -> Result<Self> {
        info!("Initializing Caesar EVP storage layer");

        let storage_path = PathBuf::from(&config.path);
        tokio::fs::create_dir_all(&storage_path).await?;

        let storage = Self {
            storage_path,
            packets: Arc::new(RwLock::new(HashMap::new())),
            settlements: Arc::new(RwLock::new(BTreeMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
        };

        storage.load_from_disk().await?;

        Ok(storage)
    }

    // ---- Disk persistence ----

    /// Load packets and settlements from JSON files on disk.
    ///
    /// Packets are stored as a Vec (PacketId is `[u8; 32]` and cannot be
    /// used as a JSON map key), then reconstructed into a HashMap on load.
    async fn load_from_disk(&self) -> Result<()> {
        let packets_file = self.storage_path.join("packets.json");
        if packets_file.exists() {
            let data = tokio::fs::read_to_string(&packets_file).await?;
            if let Ok(loaded) = serde_json::from_str::<Vec<PacketRecord>>(&data)
            {
                let map: HashMap<PacketId, PacketRecord> =
                    loaded.into_iter().map(|p| (p.packet_id, p)).collect();
                *self.packets.write().await = map;
                debug!(
                    "Loaded {} packets from disk",
                    self.packets.read().await.len()
                );
            }
        }

        let settlements_file = self.storage_path.join("settlements.json");
        if settlements_file.exists() {
            let data = tokio::fs::read_to_string(&settlements_file).await?;
            if let Ok(loaded) =
                serde_json::from_str::<BTreeMap<String, SettlementRecord>>(&data)
            {
                *self.settlements.write().await = loaded;
                debug!(
                    "Loaded {} settlements from disk",
                    self.settlements.read().await.len()
                );
            }
        }

        Ok(())
    }

    /// Persist current state to JSON files on disk.
    ///
    /// Packets are serialized as a Vec (not a HashMap) because PacketId
    /// (`[u8; 32]`) cannot be used as a JSON object key.
    async fn persist_to_disk(&self) -> Result<()> {
        let packets_file = self.storage_path.join("packets.json");
        let packets_data = {
            let guard = self.packets.read().await;
            let packets_vec: Vec<&PacketRecord> = guard.values().collect();
            serde_json::to_string_pretty(&packets_vec)?
        };
        tokio::fs::write(&packets_file, packets_data).await?;

        let settlements_file = self.storage_path.join("settlements.json");
        let settlements_data =
            serde_json::to_string_pretty(&*self.settlements.read().await)?;
        tokio::fs::write(&settlements_file, settlements_data).await?;

        Ok(())
    }

    // ---- Packet operations ----

    /// Store a new packet record and persist to disk.
    pub async fn store_packet(&self, record: PacketRecord) -> Result<()> {
        let id = record.packet_id;
        self.packets.write().await.insert(id, record);
        self.persist_to_disk().await?;
        debug!("Stored packet {}", id);
        Ok(())
    }

    /// Look up a packet by its identifier.
    pub async fn get_packet(
        &self,
        id: &PacketId,
    ) -> Result<Option<PacketRecord>> {
        Ok(self.packets.read().await.get(id).cloned())
    }

    /// Update a packet's state and current value.
    ///
    /// Also bumps `updated_at` to now.
    pub async fn update_packet_state(
        &self,
        id: &PacketId,
        new_state: PacketState,
        new_value: GoldGrams,
    ) -> Result<()> {
        let mut packets = self.packets.write().await;
        let record = packets
            .get_mut(id)
            .ok_or_else(|| anyhow!("Packet {} not found", id))?;
        record.state = new_state;
        record.current_value = new_value;
        record.updated_at = Utc::now();
        if new_state.is_terminal() {
            record.settled_at = Some(Utc::now());
        }
        drop(packets);
        self.persist_to_disk().await?;
        debug!("Updated packet {} to state {:?}", id, new_state);
        Ok(())
    }

    /// Return all packets that are currently active (non-terminal).
    pub async fn list_active_packets(&self) -> Result<Vec<PacketRecord>> {
        let packets = self.packets.read().await;
        let active: Vec<PacketRecord> = packets
            .values()
            .filter(|p| p.state.is_active())
            .cloned()
            .collect();
        Ok(active)
    }

    /// Return all packets in a specific state.
    pub async fn list_packets_by_state(
        &self,
        state: PacketState,
    ) -> Result<Vec<PacketRecord>> {
        let packets = self.packets.read().await;
        let filtered: Vec<PacketRecord> = packets
            .values()
            .filter(|p| p.state == state)
            .cloned()
            .collect();
        Ok(filtered)
    }

    // ---- Settlement operations ----

    /// Store a settlement record and persist to disk.
    pub async fn store_settlement(
        &self,
        record: SettlementRecord,
    ) -> Result<()> {
        let id = record.settlement_id.clone();
        self.settlements.write().await.insert(id.clone(), record);
        self.persist_to_disk().await?;
        debug!("Stored settlement {}", id);
        Ok(())
    }

    /// Look up a settlement by its identifier.
    pub async fn get_settlement(
        &self,
        id: &str,
    ) -> Result<Option<SettlementRecord>> {
        Ok(self.settlements.read().await.get(id).cloned())
    }

    /// Return the most recent N settlements (ordered by BTreeMap key).
    pub async fn list_recent_settlements(
        &self,
        limit: usize,
    ) -> Result<Vec<SettlementRecord>> {
        let settlements = self.settlements.read().await;
        let recent: Vec<SettlementRecord> = settlements
            .values()
            .rev()
            .take(limit)
            .cloned()
            .collect();
        Ok(recent)
    }

    // ---- Metrics operations ----

    /// Save a metrics snapshot, keeping only the most recent 1000 entries.
    pub async fn save_metrics(
        &self,
        metrics: serde_json::Value,
    ) -> Result<()> {
        let timestamp = Utc::now().to_rfc3339();
        self.metrics
            .write()
            .await
            .insert(timestamp, metrics);

        let mut store = self.metrics.write().await;
        if store.len() > 1000 {
            let to_remove: Vec<String> = store
                .keys()
                .take(store.len() - 1000)
                .cloned()
                .collect();
            for key in to_remove {
                store.remove(&key);
            }
        }
        drop(store);

        self.persist_to_disk().await?;
        Ok(())
    }

    /// Get the most recently stored metrics snapshot.
    pub async fn get_latest_metrics(
        &self,
    ) -> Result<Option<serde_json::Value>> {
        let metrics = self.metrics.read().await;
        Ok(metrics.values().last().cloned())
    }

    /// Get up to `limit` metrics snapshots, most recent first.
    pub async fn get_metrics_history(
        &self,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let metrics = self.metrics.read().await;
        let mut history: Vec<serde_json::Value> =
            metrics.values().cloned().collect();
        history.reverse();
        history.truncate(limit);
        Ok(history)
    }

    // ---- Statistics ----

    /// Count of all non-terminal packets.
    pub async fn get_active_packet_count(&self) -> Result<usize> {
        let packets = self.packets.read().await;
        let count = packets.values().filter(|p| p.state.is_active()).count();
        Ok(count)
    }

    /// Sum of `current_value` across all `InTransit` packets.
    pub async fn get_total_in_transit_value(&self) -> Result<GoldGrams> {
        let packets = self.packets.read().await;
        let total = packets
            .values()
            .filter(|p| p.state == PacketState::InTransit)
            .fold(GoldGrams::zero(), |acc, p| acc + p.current_value);
        Ok(total)
    }

    /// Sum of `fee_collected` across settlements completed since `since`.
    pub async fn get_settlement_volume(
        &self,
        since: DateTime<Utc>,
    ) -> Result<GoldGrams> {
        let settlements = self.settlements.read().await;
        let total = settlements
            .values()
            .filter(|s| s.settled_at >= since)
            .fold(GoldGrams::zero(), |acc, s| acc + s.fee_collected);
        Ok(total)
    }
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use hypermesh_lib::economic::MarketTier;
    use hypermesh_lib::NodeId;
    use rust_decimal::Decimal;
    use tempfile::TempDir;

    fn make_config(dir: &TempDir) -> StorageConfig {
        StorageConfig {
            path: dir
                .path()
                .to_str()
                .expect("test: tempdir path should be valid utf-8")
                .to_string(),
        }
    }

    fn make_packet(id_byte: u8, state: PacketState, value: i64) -> PacketRecord {
        let now = Utc::now();
        PacketRecord {
            packet_id: PacketId::new([id_byte; 32]),
            state,
            tier: MarketTier::L0,
            initial_value: GoldGrams::from_decimal(Decimal::new(value, 0)),
            current_value: GoldGrams::from_decimal(Decimal::new(value, 0)),
            fee_budget: GoldGrams::from_decimal(Decimal::new(5, 0)),
            hop_count: 0,
            hop_limit: 10,
            demurrage_cost: GoldGrams::zero(),
            route: vec![NodeId::from("node-a")],
            created_at: now,
            updated_at: now,
            settled_at: None,
        }
    }

    #[tokio::test]
    async fn test_storage_basic() {
        let dir = TempDir::new().expect("test: should create tempdir");
        let config = make_config(&dir);

        let storage = CaesarStorage::new(config)
            .await
            .expect("test: storage init should succeed");

        let packet = make_packet(1, PacketState::Minted, 100);
        storage
            .store_packet(packet.clone())
            .await
            .expect("test: store_packet should succeed");

        let retrieved = storage
            .get_packet(&PacketId::new([1u8; 32]))
            .await
            .expect("test: get_packet should succeed");
        assert!(retrieved.is_some());
        let r = retrieved.expect("test: packet should exist");
        assert_eq!(r.state, PacketState::Minted);
        assert_eq!(r.initial_value.0, Decimal::new(100, 0));
    }

    #[tokio::test]
    async fn test_packet_state_update() {
        let dir = TempDir::new().expect("test: should create tempdir");
        let storage = CaesarStorage::new(make_config(&dir))
            .await
            .expect("test: storage init should succeed");

        let packet = make_packet(2, PacketState::Minted, 200);
        let pid = packet.packet_id;
        storage
            .store_packet(packet)
            .await
            .expect("test: store should succeed");

        let new_value = GoldGrams::from_decimal(Decimal::new(190, 0));
        storage
            .update_packet_state(&pid, PacketState::InTransit, new_value)
            .await
            .expect("test: update should succeed");

        let updated = storage
            .get_packet(&pid)
            .await
            .expect("test: get should succeed")
            .expect("test: packet should exist");
        assert_eq!(updated.state, PacketState::InTransit);
        assert_eq!(updated.current_value.0, Decimal::new(190, 0));
        assert!(updated.settled_at.is_none());
    }

    #[tokio::test]
    async fn test_list_active_packets() {
        let dir = TempDir::new().expect("test: should create tempdir");
        let storage = CaesarStorage::new(make_config(&dir))
            .await
            .expect("test: storage init should succeed");

        // Minted (active)
        storage
            .store_packet(make_packet(10, PacketState::Minted, 100))
            .await
            .expect("test: store minted");
        // InTransit (active)
        storage
            .store_packet(make_packet(11, PacketState::InTransit, 200))
            .await
            .expect("test: store in-transit");
        // Settled (terminal, NOT active)
        storage
            .store_packet(make_packet(12, PacketState::Settled, 300))
            .await
            .expect("test: store settled");

        let active = storage
            .list_active_packets()
            .await
            .expect("test: list_active should succeed");
        assert_eq!(active.len(), 2);

        let by_state = storage
            .list_packets_by_state(PacketState::InTransit)
            .await
            .expect("test: list_by_state should succeed");
        assert_eq!(by_state.len(), 1);
    }

    #[tokio::test]
    async fn test_settlement_storage() {
        let dir = TempDir::new().expect("test: should create tempdir");
        let storage = CaesarStorage::new(make_config(&dir))
            .await
            .expect("test: storage init should succeed");

        let settlement = SettlementRecord {
            settlement_id: "s-001".to_string(),
            packet_id: PacketId::new([42u8; 32]),
            egress_node: NodeId::from("egress-1"),
            finality_type: "instant".to_string(),
            fee_collected: GoldGrams::from_decimal(Decimal::new(50, 1)),
            settled_at: Utc::now(),
        };

        storage
            .store_settlement(settlement)
            .await
            .expect("test: store settlement should succeed");

        let retrieved = storage
            .get_settlement("s-001")
            .await
            .expect("test: get settlement should succeed");
        assert!(retrieved.is_some());
        let s = retrieved.expect("test: settlement should exist");
        assert_eq!(s.finality_type, "instant");
        assert_eq!(s.fee_collected.0, Decimal::new(50, 1));
    }

    #[tokio::test]
    async fn test_persistence() {
        let dir = TempDir::new().expect("test: should create tempdir");
        let config = make_config(&dir);

        let pid = PacketId::new([99u8; 32]);

        // Create and store
        {
            let storage = CaesarStorage::new(config.clone())
                .await
                .expect("test: first init should succeed");
            storage
                .store_packet(make_packet(99, PacketState::Minted, 500))
                .await
                .expect("test: store should succeed");
        }

        // Reload and verify
        {
            let storage = CaesarStorage::new(config)
                .await
                .expect("test: second init should succeed");
            let packet = storage
                .get_packet(&pid)
                .await
                .expect("test: get should succeed");
            assert!(packet.is_some());
            let p = packet.expect("test: packet should survive persistence");
            assert_eq!(p.state, PacketState::Minted);
            assert_eq!(p.initial_value.0, Decimal::new(500, 0));
        }
    }

    #[tokio::test]
    async fn test_in_transit_value() {
        let dir = TempDir::new().expect("test: should create tempdir");
        let storage = CaesarStorage::new(make_config(&dir))
            .await
            .expect("test: storage init should succeed");

        // Two InTransit packets
        storage
            .store_packet(make_packet(20, PacketState::InTransit, 100))
            .await
            .expect("test: store 1");
        storage
            .store_packet(make_packet(21, PacketState::InTransit, 250))
            .await
            .expect("test: store 2");
        // One Minted (not InTransit, should not count)
        storage
            .store_packet(make_packet(22, PacketState::Minted, 999))
            .await
            .expect("test: store 3");

        let total = storage
            .get_total_in_transit_value()
            .await
            .expect("test: get total should succeed");
        assert_eq!(total.0, Decimal::new(350, 0));
    }
}
