// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CaesarProtocol settlement orchestration methods.
//!
//! Settle, retry, dissolve, and conservation audit.

use anyhow::Result;
use rust_decimal_macros::dec;

use hypermesh_lib::economic::GoldGrams;

use crate::{conservation, governor, holding, models, settlement, upi, CaesarProtocol};

impl CaesarProtocol {
    /// Dissolve abandoned packet via gravity dissolution.
    pub async fn dissolve_packet(
        &mut self,
        packet_id: &hypermesh_lib::economic::PacketId,
        qualified_nodes: &[settlement::gravity::GravityQualification],
        shard_holders: &[hypermesh_lib::NodeId],
    ) -> Result<settlement::gravity::DissolutionResult> {
        let record = self
            .storage
            .get_packet(packet_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("packet not found"))?;

        // Verify packet is in Held or Stalled state
        let valid = record.state == hypermesh_lib::economic::PacketState::Held
            || record.state == hypermesh_lib::economic::PacketState::Stalled;
        if !valid {
            return Err(anyhow::anyhow!(
                "packet must be in Held or Stalled state, got {:?}",
                record.state
            ));
        }

        if !settlement::gravity::GravityDissolution::is_eligible_for_dissolution(record.updated_at)
        {
            return Err(anyhow::anyhow!("not eligible"));
        }

        let result = settlement::gravity::GravityDissolution::dissolve(
            *packet_id,
            record.current_value,
            qualified_nodes,
            shard_holders,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut packet = models::packet_from_record(&record);
        packet.dissolve().map_err(|e| anyhow::anyhow!("{e}"))?;

        self.storage
            .update_packet_state(
                packet_id,
                hypermesh_lib::economic::PacketState::Dissolved,
                packet.initial_value,
            )
            .await?;

        Ok(result)
    }

    /// Settle a Delivered packet through an egress adapter.
    ///
    /// Transitions the packet through Delivered -> Settling -> Settled on
    /// success, or to Dispersed (held for retry) on egress failure.
    pub async fn settle_packet(
        &mut self,
        packet_id: &hypermesh_lib::economic::PacketId,
        egress_adapter: &dyn upi::EgressAdapter,
        criteria: settlement::acceptance::AcceptanceCriteria,
    ) -> Result<settlement::protocol::ExecutedSettlement> {
        let record = self
            .storage
            .get_packet(packet_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("packet not found"))?;

        let mut packet = models::packet_from_record(&record);

        // Transition to Delivered (InTransit -> Delivered)
        packet.deliver().map_err(|e| anyhow::anyhow!("{e}"))?;

        // Transition to Settling (Delivered -> Settling)
        packet
            .begin_settling()
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // Calculate Governor-adjusted fee
        let gov_params = governor::GovernanceParams::default();
        let governor_fee = self.governor.calculate_fee(
            &gov_params,
            record.tier,
            dec!(0.01),
            record.current_value.0,
        );

        let request = settlement::protocol::SettlementRequest {
            packet_id: *packet_id,
            packet_state: hypermesh_lib::economic::PacketState::Delivered,
            packet_tier: record.tier,
            packet_value: record.current_value,
            fee: GoldGrams::from_decimal(governor_fee),
            settler_node: record.recipient.clone(),
            adapter_id: egress_adapter.adapter_id().to_string(),
            recipient_criteria: criteria,
        };

        let transit_nodes: Vec<(hypermesh_lib::NodeId, u64)> =
            record.route.iter().map(|n| (n.clone(), 1000_u64)).collect();

        match settlement::protocol::SettlementProtocol::execute_settlement(
            request,
            egress_adapter,
            &self.fee_distributor,
            &transit_nodes,
            self.config.gold_price_usd,
        )
        .await
        {
            Ok(result) => {
                packet.settle().map_err(|e| anyhow::anyhow!("{e}"))?;

                // Conservation check: initial = settled + fee + demurrage
                let _ = self.conservation.verify_settlement(
                    record.initial_value,
                    GoldGrams::from_decimal(result.settlement_result.settled_value.0),
                    result.settlement_result.fee_collected,
                    record.demurrage_cost,
                );

                self.storage
                    .store_settlement(models::SettlementRecord {
                        settlement_id: format!("settle-{packet_id}"),
                        packet_id: *packet_id,
                        egress_node: record.recipient.clone(),
                        finality_type: "instant".to_string(),
                        fee_collected: result.settlement_result.fee_collected,
                        settled_at: result.settlement_result.settled_at,
                    })
                    .await?;

                self.storage
                    .update_packet_state(
                        packet_id,
                        hypermesh_lib::economic::PacketState::Settled,
                        record.current_value,
                    )
                    .await?;

                Ok(result)
            }
            Err(settlement::protocol::SettlementError::EgressFailed { reason }) => {
                packet.disperse().map_err(|e| anyhow::anyhow!("{e}"))?;

                self.holding_buffer
                    .hold(*packet_id, holding::HoldReason::EgressFailure);

                self.storage
                    .update_packet_state(
                        packet_id,
                        hypermesh_lib::economic::PacketState::Dispersed,
                        record.current_value,
                    )
                    .await?;

                Err(anyhow::anyhow!("egress adapter failed: {reason}"))
            }
            Err(other) => Err(anyhow::anyhow!("{other}")),
        }
    }

    /// Retry settlement for a Dispersed packet.
    ///
    /// Releases the packet from the holding buffer and re-executes the
    /// settlement flow through the egress adapter.
    pub async fn retry_settlement(
        &mut self,
        packet_id: &hypermesh_lib::economic::PacketId,
        egress_adapter: &dyn upi::EgressAdapter,
        criteria: settlement::acceptance::AcceptanceCriteria,
    ) -> Result<settlement::protocol::ExecutedSettlement> {
        let record = self
            .storage
            .get_packet(packet_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("packet not found"))?;

        if record.state != hypermesh_lib::economic::PacketState::Dispersed {
            return Err(anyhow::anyhow!(
                "packet must be in Dispersed state, got {:?}",
                record.state
            ));
        }

        let mut packet = models::packet_from_record(&record);

        // Transition Dispersed -> Settling
        packet
            .retry_settlement()
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // Release from holding buffer
        self.holding_buffer.release(packet_id);

        // Calculate Governor-adjusted fee
        let gov_params = governor::GovernanceParams::default();
        let governor_fee = self.governor.calculate_fee(
            &gov_params,
            record.tier,
            dec!(0.01),
            record.current_value.0,
        );

        let request = settlement::protocol::SettlementRequest {
            packet_id: *packet_id,
            packet_state: hypermesh_lib::economic::PacketState::Delivered,
            packet_tier: record.tier,
            packet_value: record.current_value,
            fee: GoldGrams::from_decimal(governor_fee),
            settler_node: record.recipient.clone(),
            adapter_id: egress_adapter.adapter_id().to_string(),
            recipient_criteria: criteria,
        };

        let transit_nodes: Vec<(hypermesh_lib::NodeId, u64)> =
            record.route.iter().map(|n| (n.clone(), 1000_u64)).collect();

        match settlement::protocol::SettlementProtocol::execute_settlement(
            request,
            egress_adapter,
            &self.fee_distributor,
            &transit_nodes,
            self.config.gold_price_usd,
        )
        .await
        {
            Ok(result) => {
                packet.settle().map_err(|e| anyhow::anyhow!("{e}"))?;

                let _ = self.conservation.verify_settlement(
                    record.initial_value,
                    GoldGrams::from_decimal(result.settlement_result.settled_value.0),
                    result.settlement_result.fee_collected,
                    record.demurrage_cost,
                );

                self.storage
                    .store_settlement(models::SettlementRecord {
                        settlement_id: format!("retry-settle-{packet_id}"),
                        packet_id: *packet_id,
                        egress_node: record.recipient.clone(),
                        finality_type: "instant".to_string(),
                        fee_collected: result.settlement_result.fee_collected,
                        settled_at: result.settlement_result.settled_at,
                    })
                    .await?;

                self.storage
                    .update_packet_state(
                        packet_id,
                        hypermesh_lib::economic::PacketState::Settled,
                        record.current_value,
                    )
                    .await?;

                Ok(result)
            }
            Err(settlement::protocol::SettlementError::EgressFailed { reason }) => {
                packet.disperse().map_err(|e| anyhow::anyhow!("{e}"))?;

                self.holding_buffer
                    .hold(*packet_id, holding::HoldReason::EgressFailure);

                self.storage
                    .update_packet_state(
                        packet_id,
                        hypermesh_lib::economic::PacketState::Dispersed,
                        record.current_value,
                    )
                    .await?;

                Err(anyhow::anyhow!("egress adapter failed: {reason}"))
            }
            Err(other) => Err(anyhow::anyhow!("{other}")),
        }
    }

    /// Run full conservation audit.
    pub async fn audit_conservation(&mut self) -> Result<conservation::ConservationAudit> {
        self.conservation.audit(&self.storage).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::{holding, models, settlement, storage, CaesarConfig, CaesarProtocol};
    use hypermesh_lib::economic::GoldGrams;
    use rust_decimal_macros::dec;
    use tempfile::TempDir;

    fn test_config(dir: &TempDir) -> CaesarConfig {
        CaesarConfig {
            storage: storage::StorageConfig {
                path: dir.path().to_str().expect("test: tempdir path").to_string(),
            },
            ..CaesarConfig::default()
        }
    }

    fn make_criteria() -> settlement::acceptance::AcceptanceCriteria {
        settlement::acceptance::AcceptanceCriteria::new(hypermesh_lib::NodeId::from("recipient"))
    }

    /// Helper: mint a packet and force it to a specific state with a route.
    async fn mint_and_force_state(
        protocol: &mut CaesarProtocol,
        state: hypermesh_lib::economic::PacketState,
    ) -> hypermesh_lib::economic::PacketId {
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

        // Add a route node to the record so transit_nodes is non-empty
        let record = protocol
            .storage()
            .get_packet(&packet_id)
            .await
            .expect("test: get")
            .expect("test: exists");

        let updated = models::PacketRecord {
            state,
            route: vec![hypermesh_lib::NodeId::from("relay-1")],
            ..record
        };
        protocol
            .storage()
            .replace_packet(updated)
            .await
            .expect("test: replace");

        packet_id
    }

    #[tokio::test]
    async fn test_settle_packet() {
        use crate::upi::egress::testing::MockEgressAdapter;

        let dir = TempDir::new().expect("test: tempdir");
        let mut protocol = CaesarProtocol::new(test_config(&dir))
            .await
            .expect("test: protocol init");

        let packet_id = mint_and_force_state(
            &mut protocol,
            hypermesh_lib::economic::PacketState::InTransit,
        )
        .await;

        let adapter = MockEgressAdapter::new(GoldGrams::from_decimal(dec!(10000)));
        let criteria = make_criteria();

        let result = protocol
            .settle_packet(&packet_id, &adapter, criteria)
            .await
            .expect("test: settle should succeed");

        assert_eq!(result.settlement_result.packet_id, packet_id);

        let stored = protocol
            .storage()
            .get_packet(&packet_id)
            .await
            .expect("test: get")
            .expect("test: exists");
        assert_eq!(
            stored.state,
            hypermesh_lib::economic::PacketState::Settled,
            "packet should be in Settled state"
        );
    }

    #[tokio::test]
    async fn test_settle_dispersed_on_egress_failure() {
        use crate::upi::egress::testing::MockEgressAdapter;

        let dir = TempDir::new().expect("test: tempdir");
        let mut protocol = CaesarProtocol::new(test_config(&dir))
            .await
            .expect("test: protocol init");

        let packet_id = mint_and_force_state(
            &mut protocol,
            hypermesh_lib::economic::PacketState::InTransit,
        )
        .await;

        // Zero-capacity adapter causes egress failure
        let adapter = MockEgressAdapter::new(GoldGrams::zero());
        let criteria = make_criteria();

        let result = protocol.settle_packet(&packet_id, &adapter, criteria).await;

        assert!(
            result.is_err(),
            "settle should fail with zero-capacity adapter"
        );

        let stored = protocol
            .storage()
            .get_packet(&packet_id)
            .await
            .expect("test: get")
            .expect("test: exists");
        assert_eq!(
            stored.state,
            hypermesh_lib::economic::PacketState::Dispersed,
            "packet should be in Dispersed state after egress failure"
        );
    }

    #[tokio::test]
    async fn test_retry_settlement() {
        use crate::upi::egress::testing::MockEgressAdapter;

        let dir = TempDir::new().expect("test: tempdir");
        let mut protocol = CaesarProtocol::new(test_config(&dir))
            .await
            .expect("test: protocol init");

        let packet_id = mint_and_force_state(
            &mut protocol,
            hypermesh_lib::economic::PacketState::Dispersed,
        )
        .await;

        // Put it in the holding buffer so release works
        protocol
            .holding_buffer
            .hold(packet_id, holding::HoldReason::EgressFailure);

        let adapter = MockEgressAdapter::new(GoldGrams::from_decimal(dec!(10000)));
        let criteria = make_criteria();

        let result = protocol
            .retry_settlement(&packet_id, &adapter, criteria)
            .await
            .expect("test: retry should succeed");

        assert_eq!(result.settlement_result.packet_id, packet_id);

        let stored = protocol
            .storage()
            .get_packet(&packet_id)
            .await
            .expect("test: get")
            .expect("test: exists");
        assert_eq!(
            stored.state,
            hypermesh_lib::economic::PacketState::Settled,
            "packet should be in Settled state after retry"
        );
    }
}
