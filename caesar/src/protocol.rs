// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CaesarProtocol core orchestration methods.
//!
//! Packet lifecycle: new, accessors, mint, route, hold, release, expire,
//! refund. Settlement methods are in `protocol_settlement`.

use anyhow::Result;
use rust_decimal_macros::dec;
use tracing::info;

use hypermesh_lib::economic::GoldGrams;

use crate::{
    conservation, evp, fee_distribution, gold_oracle, governor, holding, models,
    packet_processor, routing, storage, CaesarConfig, CaesarProtocol,
};
use std::sync::Arc;

impl CaesarProtocol {
    /// Create a new Caesar protocol instance.
    pub async fn new(config: CaesarConfig) -> Result<Self> {
        info!("Initializing Caesar Ephemeral Value Protocol");

        let config = Arc::new(config);
        let storage = Arc::new(
            storage::CaesarStorage::new(config.storage.clone()).await?,
        );
        let processor = packet_processor::PacketProcessor::new(
            config.processor.clone(),
        );
        let fee_distributor = fee_distribution::FeeDistributor::default();
        let oracle = gold_oracle::GoldOracle::new(config.gold_price_usd);
        let router = routing::PacketRouter::default();
        let governor = governor::GovernorPid::new();
        let holding_buffer = holding::HoldingBuffer::new(5); // max 5 retries
        let conservation = conservation::ConservationLaw::new(dec!(0.001));

        Ok(Self {
            config,
            storage,
            processor,
            fee_distributor,
            oracle,
            router,
            governor,
            holding_buffer,
            conservation,
        })
    }

    /// Get the Governor PID controller (mutable for recalculation).
    pub fn governor_mut(&mut self) -> &mut governor::GovernorPid {
        &mut self.governor
    }

    /// Get the Governor PID controller (read-only).
    pub fn governor(&self) -> &governor::GovernorPid {
        &self.governor
    }

    /// Get the gold oracle for price queries.
    pub fn oracle(&self) -> &gold_oracle::GoldOracle {
        &self.oracle
    }

    /// Get the storage layer.
    pub fn storage(&self) -> &Arc<storage::CaesarStorage> {
        &self.storage
    }

    /// Get active packet count.
    pub async fn active_packet_count(&self) -> Result<usize> {
        self.storage.get_active_packet_count().await
    }

    /// Get total value currently in transit.
    pub async fn in_transit_value(&self) -> Result<GoldGrams> {
        self.storage.get_total_in_transit_value().await
    }

    /// Get the conservation law status (circuit breaker state).
    pub fn conservation_status(&self) -> bool {
        self.conservation.is_circuit_breaker_tripped()
    }

    /// Get the packet router.
    pub fn router(&self) -> &routing::PacketRouter {
        &self.router
    }

    /// Get the packet processor.
    pub fn processor(&self) -> &packet_processor::PacketProcessor {
        &self.processor
    }

    // -----------------------------------------------------------------------
    // Orchestration methods
    // -----------------------------------------------------------------------

    /// Mint a new EVP. Blocked if circuit breaker tripped.
    pub async fn mint_packet(
        &mut self,
        sender: hypermesh_lib::NodeId,
        recipient: hypermesh_lib::NodeId,
        value: GoldGrams,
        fee: GoldGrams,
        tier: hypermesh_lib::economic::MarketTier,
        hop_limit: u16,
        fee_budget: GoldGrams,
    ) -> Result<hypermesh_lib::economic::PacketId> {
        if self.conservation.is_circuit_breaker_tripped() {
            return Err(anyhow::anyhow!("circuit breaker tripped"));
        }

        let packet = evp::CaesPacket::mint(
            sender.clone(),
            recipient.clone(),
            value,
            fee,
            tier,
            tier.default_demurrage_rate(),
            hop_limit,
            fee_budget,
        );
        let packet_id = packet.id;

        let record = models::PacketRecord {
            packet_id: packet.id,
            state: packet.state,
            tier: packet.tier,
            initial_value: packet.initial_value,
            current_value: packet.initial_value,
            fee_budget: packet.fee_budget,
            hop_count: packet.hop_count,
            hop_limit: packet.hop_limit,
            demurrage_cost: GoldGrams::zero(),
            route: packet.route.clone(),
            created_at: packet.created_at,
            updated_at: packet.last_transition,
            settled_at: None,
            sender: packet.sender.clone(),
            recipient: packet.recipient.clone(),
            demurrage_rate: packet.demurrage_rate,
        };

        self.storage.store_packet(record).await?;
        Ok(packet_id)
    }

    /// Route packet to next hop using capacity-based routing.
    ///
    /// When `metrics` is provided, the Governor recalculates fee parameters
    /// from fresh network data; otherwise the base fee of 0.01g is used.
    pub async fn route_packet(
        &mut self,
        packet_id: &hypermesh_lib::economic::PacketId,
        candidates: &[routing::CapacityMetrics],
        metrics: Option<&governor::pid::NetworkMetrics>,
    ) -> Result<packet_processor::HandoffResult> {
        let record = self
            .storage
            .get_packet(packet_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("packet not found"))?;
        let mut packet = models::packet_from_record(&record);

        let selection = self
            .router
            .find_route(candidates, record.tier)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let handoff_fee = if let Some(m) = metrics {
            let params = self.governor.recalculate(m);
            let fee_decimal = self.governor.calculate_fee(
                &params,
                record.tier,
                dec!(0.01),
                record.current_value.0,
            );
            GoldGrams::from_decimal(fee_decimal)
        } else {
            GoldGrams::from_decimal(dec!(0.01))
        };

        let result = self
            .processor
            .process_handoff(&mut packet, selection.next_hop.clone(), handoff_fee)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let updated = models::PacketRecord {
            state: packet.state,
            current_value: packet.initial_value,
            hop_count: packet.hop_count,
            route: packet.route.clone(),
            updated_at: packet.last_transition,
            ..record
        };
        self.storage.replace_packet(updated).await?;

        Ok(result)
    }

    /// Place packet in holding buffer.
    pub async fn hold_packet(
        &mut self,
        packet_id: &hypermesh_lib::economic::PacketId,
        reason: holding::HoldReason,
    ) -> Result<()> {
        let record = self
            .storage
            .get_packet(packet_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("packet not found"))?;
        let mut packet = models::packet_from_record(&record);

        packet.hold().map_err(|e| anyhow::anyhow!("{}", e))?;
        self.holding_buffer.hold(*packet_id, reason);
        self.storage
            .update_packet_state(packet_id, packet.state, packet.initial_value)
            .await?;

        Ok(())
    }

    /// Release packet from holding buffer.
    pub async fn release_packet(
        &mut self,
        packet_id: &hypermesh_lib::economic::PacketId,
    ) -> Result<()> {
        self.holding_buffer.release(packet_id);

        let record = self
            .storage
            .get_packet(packet_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("packet not found"))?;
        let mut packet = models::packet_from_record(&record);

        packet
            .retry_from_hold()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        self.storage
            .update_packet_state(
                packet_id,
                hypermesh_lib::economic::PacketState::InTransit,
                packet.initial_value,
            )
            .await?;

        Ok(())
    }

    /// Expire packet (TTL exceeded).
    pub async fn expire_packet(
        &mut self,
        packet_id: &hypermesh_lib::economic::PacketId,
    ) -> Result<()> {
        let record = self
            .storage
            .get_packet(packet_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("packet not found"))?;
        let mut packet = models::packet_from_record(&record);

        packet.expire().map_err(|e| anyhow::anyhow!("{}", e))?;
        self.storage
            .update_packet_state(packet_id, packet.state, packet.initial_value)
            .await?;

        Ok(())
    }

    /// Refund expired packet to ingress.
    pub async fn refund_packet(
        &mut self,
        packet_id: &hypermesh_lib::economic::PacketId,
    ) -> Result<()> {
        let record = self
            .storage
            .get_packet(packet_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("packet not found"))?;
        let mut packet = models::packet_from_record(&record);

        packet.refund().map_err(|e| anyhow::anyhow!("{}", e))?;
        self.storage
            .update_packet_state(packet_id, packet.state, packet.initial_value)
            .await?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::{governor, holding, routing, storage, CaesarConfig, CaesarProtocol};
    use hypermesh_lib::economic::GoldGrams;
    use rust_decimal_macros::dec;
    use tempfile::TempDir;

    fn test_config(dir: &TempDir) -> CaesarConfig {
        CaesarConfig {
            storage: storage::StorageConfig {
                path: dir
                    .path()
                    .to_str()
                    .expect("test: tempdir path")
                    .to_string(),
            },
            ..CaesarConfig::default()
        }
    }

    #[tokio::test]
    async fn test_caesar_protocol_initialization() {
        let dir = TempDir::new().expect("test: tempdir");
        let config = test_config(&dir);
        let protocol = CaesarProtocol::new(config).await;
        assert!(
            protocol.is_ok(),
            "CaesarProtocol should initialize: {:?}",
            protocol.err()
        );
    }

    #[tokio::test]
    async fn test_mint_packet() {
        let dir = TempDir::new().expect("test: tempdir");
        let mut protocol = CaesarProtocol::new(test_config(&dir))
            .await
            .expect("test: protocol init");

        let packet_id = protocol
            .mint_packet(
                hypermesh_lib::NodeId::from("sender"),
                hypermesh_lib::NodeId::from("recipient"),
                GoldGrams::from_decimal(dec!(100)),
                GoldGrams::from_decimal(dec!(0.1)),
                hypermesh_lib::economic::MarketTier::L0,
                20,
                GoldGrams::from_decimal(dec!(5)),
            )
            .await
            .expect("test: mint should succeed");

        let count = protocol
            .active_packet_count()
            .await
            .expect("test: count");
        assert_eq!(count, 1);

        let stored = protocol
            .storage()
            .get_packet(&packet_id)
            .await
            .expect("test: get packet")
            .expect("test: packet should exist");
        assert_eq!(
            stored.state,
            hypermesh_lib::economic::PacketState::Minted
        );
    }

    #[tokio::test]
    async fn test_hold_and_release() {
        let dir = TempDir::new().expect("test: tempdir");
        let mut protocol = CaesarProtocol::new(test_config(&dir))
            .await
            .expect("test: protocol init");

        let packet_id = protocol
            .mint_packet(
                hypermesh_lib::NodeId::from("sender"),
                hypermesh_lib::NodeId::from("recipient"),
                GoldGrams::from_decimal(dec!(100)),
                GoldGrams::from_decimal(dec!(0.1)),
                hypermesh_lib::economic::MarketTier::L0,
                20,
                GoldGrams::from_decimal(dec!(5)),
            )
            .await
            .expect("test: mint");

        protocol
            .storage()
            .update_packet_state(
                &packet_id,
                hypermesh_lib::economic::PacketState::InTransit,
                GoldGrams::from_decimal(dec!(100)),
            )
            .await
            .expect("test: update state");

        protocol
            .hold_packet(&packet_id, holding::HoldReason::NetworkCongestion)
            .await
            .expect("test: hold");

        let held = protocol
            .storage()
            .get_packet(&packet_id)
            .await
            .expect("test: get")
            .expect("test: exists");
        assert_eq!(held.state, hypermesh_lib::economic::PacketState::Held);

        protocol
            .release_packet(&packet_id)
            .await
            .expect("test: release");

        let released = protocol
            .storage()
            .get_packet(&packet_id)
            .await
            .expect("test: get")
            .expect("test: exists");
        assert_eq!(
            released.state,
            hypermesh_lib::economic::PacketState::InTransit
        );
    }

    #[tokio::test]
    async fn test_expire_and_refund() {
        let dir = TempDir::new().expect("test: tempdir");
        let mut protocol = CaesarProtocol::new(test_config(&dir))
            .await
            .expect("test: protocol init");

        let packet_id = protocol
            .mint_packet(
                hypermesh_lib::NodeId::from("sender"),
                hypermesh_lib::NodeId::from("recipient"),
                GoldGrams::from_decimal(dec!(50)),
                GoldGrams::from_decimal(dec!(0.05)),
                hypermesh_lib::economic::MarketTier::L1,
                10,
                GoldGrams::from_decimal(dec!(2)),
            )
            .await
            .expect("test: mint");

        protocol
            .expire_packet(&packet_id)
            .await
            .expect("test: expire");
        let expired = protocol
            .storage()
            .get_packet(&packet_id)
            .await
            .expect("test: get")
            .expect("test: exists");
        assert_eq!(
            expired.state,
            hypermesh_lib::economic::PacketState::Expired
        );

        protocol
            .refund_packet(&packet_id)
            .await
            .expect("test: refund");
        let refunded = protocol
            .storage()
            .get_packet(&packet_id)
            .await
            .expect("test: get")
            .expect("test: exists");
        assert_eq!(
            refunded.state,
            hypermesh_lib::economic::PacketState::Refunded
        );
    }

    #[tokio::test]
    async fn test_conservation_status() {
        let dir = TempDir::new().expect("test: tempdir");
        let protocol = CaesarProtocol::new(test_config(&dir))
            .await
            .expect("test: protocol init");
        assert!(
            !protocol.conservation_status(),
            "circuit breaker should not be tripped initially"
        );
    }

    #[tokio::test]
    async fn test_route_with_governor() {
        let dir = TempDir::new().expect("test: tempdir");
        let mut protocol = CaesarProtocol::new(test_config(&dir))
            .await
            .expect("test: protocol init");

        let packet_id = protocol
            .mint_packet(
                hypermesh_lib::NodeId::from("sender"),
                hypermesh_lib::NodeId::from("recipient"),
                GoldGrams::from_decimal(dec!(100)),
                GoldGrams::from_decimal(dec!(0.1)),
                hypermesh_lib::economic::MarketTier::L0,
                20,
                GoldGrams::from_decimal(dec!(5)),
            )
            .await
            .expect("test: mint");

        let candidates = vec![routing::CapacityMetrics {
            node_id: hypermesh_lib::NodeId::from("relay-1"),
            available_bandwidth_mbps: dec!(500),
            buffer_capacity_packets: 200,
            avg_latency_ms: dec!(5),
            active_packet_count: 2,
        }];

        let metrics = governor::pid::NetworkMetrics {
            current_gold_price_usd: dec!(84),
            target_gold_price_usd: dec!(84),
            market_volatility: dec!(0.10),
            transaction_volume: dec!(500000),
            liquidity_depth: dec!(1500000),
            network_velocity: dec!(1.0),
            active_packets_by_tier: governor::pid::TierCounts::default(),
            in_transit_float: dec!(0),
        };

        let result = protocol
            .route_packet(&packet_id, &candidates, Some(&metrics))
            .await
            .expect("test: route with governor should succeed");

        assert_eq!(
            result.to_node,
            hypermesh_lib::NodeId::from("relay-1"),
            "should route to the only candidate"
        );
        assert_eq!(result.hop_count, 1, "hop count should be 1");
    }
}
