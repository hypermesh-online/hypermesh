// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar EVP Data Models
//!
//! Ephemeral Value Protocol models for packet-centric economics.
//! Value exists only in-flight as packets — no wallets, no persistent balances.

#[allow(unused_imports)]
use chrono::{DateTime, Utc};
use hypermesh_lib::economic::{DemurrageRate, GoldGrams, MarketTier, PacketId, PacketState};
use hypermesh_lib::NodeId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ============ Packet Models ============

/// Stored representation of an EVP packet.
///
/// Tracks the full lifecycle from minting through settlement/dissolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketRecord {
    /// Unique packet identifier (32-byte hash)
    pub packet_id: PacketId,
    /// Current lifecycle state
    pub state: PacketState,
    /// Market tier (L0-L3)
    pub tier: MarketTier,
    /// Value at minting
    pub initial_value: GoldGrams,
    /// Current value (after demurrage decay)
    pub current_value: GoldGrams,
    /// Fee budget allocated for transit
    pub fee_budget: GoldGrams,
    /// Number of hops traversed
    pub hop_count: u16,
    /// Maximum allowed hops
    pub hop_limit: u16,
    /// Accumulated demurrage cost
    pub demurrage_cost: GoldGrams,
    /// Nodes traversed in order
    pub route: Vec<NodeId>,
    /// When this packet was minted
    pub created_at: DateTime<Utc>,
    /// Last state change timestamp
    pub updated_at: DateTime<Utc>,
    /// When settlement completed (terminal states only)
    pub settled_at: Option<DateTime<Utc>>,
    /// Sender node (for CaesPacket reconstruction)
    pub sender: NodeId,
    /// Recipient node (for CaesPacket reconstruction)
    pub recipient: NodeId,
    /// Demurrage rate (for CaesPacket reconstruction)
    pub demurrage_rate: DemurrageRate,
}

/// Reconstruct a CaesPacket from a stored PacketRecord.
///
/// Used by CaesarProtocol orchestration methods to apply state transitions
/// to packets loaded from storage.
pub fn packet_from_record(record: &PacketRecord) -> crate::evp::CaesPacket {
    crate::evp::CaesPacket {
        id: record.packet_id,
        state: record.state,
        tier: record.tier,
        initial_value: record.initial_value,
        demurrage_rate: record.demurrage_rate,
        sender: record.sender.clone(),
        recipient: record.recipient.clone(),
        created_at: record.created_at,
        last_transition: record.updated_at,
        fee: GoldGrams::zero(), // fee already deducted at mint
        route: record.route.clone(),
        hold_retries: 0,
        hop_count: record.hop_count,
        hop_limit: record.hop_limit,
        fee_budget: record.fee_budget,
    }
}

// ============ Settlement Models ============

/// Record of a completed settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecord {
    /// Unique settlement identifier
    pub settlement_id: String,
    /// The packet that was settled
    pub packet_id: PacketId,
    /// Node that performed egress
    pub egress_node: NodeId,
    /// Settlement finality type name (e.g. "instant", "deferred")
    pub finality_type: String,
    /// Total fee collected during transit
    pub fee_collected: GoldGrams,
    /// When settlement completed
    pub settled_at: DateTime<Utc>,
}

// ============ Node Models ============

/// Status of a Caesar node in the EVP network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    /// Node identity
    pub node_id: NodeId,
    /// Number of packets currently in-flight through this node
    pub active_packets: u64,
    /// Total packets settled through this node
    pub settled_count: u64,
    /// Cumulative fees earned by this node
    pub total_fees_earned: GoldGrams,
    /// Operator's soft routing preferences
    pub operator_preferences: OperatorPreferences,
    /// Last time this node processed a packet
    pub last_activity: DateTime<Utc>,
}

/// Node operator soft preferences (whitepaper section 8.4).
///
/// These are preferences, not hard routing rules. A node in auto_mode
/// accepts all traffic regardless of other preference settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorPreferences {
    /// Soft preference weights per market tier
    pub tier_weights: TierWeights,
    /// Prefer packets above this size
    pub preferred_min_packet: GoldGrams,
    /// Prefer packets below this size
    pub preferred_max_packet: GoldGrams,
    /// When true, accept all traffic and ignore preferences
    pub auto_mode: bool,
}

impl Default for OperatorPreferences {
    fn default() -> Self {
        Self {
            tier_weights: TierWeights::default(),
            preferred_min_packet: GoldGrams::zero(),
            preferred_max_packet: GoldGrams::from_decimal(Decimal::MAX),
            auto_mode: true,
        }
    }
}

/// Soft preference weights for each market tier.
///
/// A weight of 1.0 is neutral (no preference). Higher weights indicate
/// preference for that tier; lower weights indicate deprioritization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierWeights {
    /// Weight for L0 (retail) packets
    pub l0: Decimal,
    /// Weight for L1 (professional) packets
    pub l1: Decimal,
    /// Weight for L2 (institutional) packets
    pub l2: Decimal,
    /// Weight for L3 (sovereign) packets
    pub l3: Decimal,
}

impl Default for TierWeights {
    fn default() -> Self {
        Self {
            l0: Decimal::ONE,
            l1: Decimal::ONE,
            l2: Decimal::ONE,
            l3: Decimal::ONE,
        }
    }
}

// ============ Transaction Type ============

/// EVP transaction types (replaces old wallet-based TransactionType).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    /// External value entering the network
    Ingress,
    /// Value leaving the network
    Egress,
    /// Packet handoff between nodes
    Transit,
    /// Settlement completion
    Settlement,
    /// Refund to sender
    Refund,
    /// Gravity dissolution distribution
    Dissolution,
    /// Fee payment to transit/egress nodes
    FeeDistribution,
}

// ============ System Models (kept from original) ============

/// System health status for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub status: SystemStatus,
    pub latency_ms: f64,
    pub transactions_per_second: f64,
    pub active_connections: u64,
    pub memory_usage_mb: f64,
    pub last_block_height: u64,
    pub last_block_time: DateTime<Utc>,
}

/// System operational status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemStatus {
    Healthy,
    Degraded,
    Critical,
    Maintenance,
}

/// API error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error_code: String,
    pub message: String,
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_record_creation() {
        let now = Utc::now();
        let record = PacketRecord {
            packet_id: PacketId::zero(),
            state: PacketState::Minted,
            tier: MarketTier::L0,
            initial_value: GoldGrams::from_decimal(Decimal::new(100, 0)),
            current_value: GoldGrams::from_decimal(Decimal::new(100, 0)),
            fee_budget: GoldGrams::from_decimal(Decimal::new(5, 0)),
            hop_count: 0,
            hop_limit: 10,
            demurrage_cost: GoldGrams::zero(),
            route: vec![NodeId::from("ingress-node")],
            created_at: now,
            updated_at: now,
            settled_at: None,
            sender: NodeId::from("sender-node"),
            recipient: NodeId::from("recipient-node"),
            demurrage_rate: MarketTier::L0.default_demurrage_rate(),
        };

        assert_eq!(record.state, PacketState::Minted);
        assert_eq!(record.tier, MarketTier::L0);
        assert_eq!(record.initial_value.0, Decimal::new(100, 0));
        assert_eq!(record.current_value.0, Decimal::new(100, 0));
        assert_eq!(record.hop_count, 0);
        assert_eq!(record.hop_limit, 10);
        assert!(record.demurrage_cost.is_zero());
        assert_eq!(record.route.len(), 1);
        assert!(record.settled_at.is_none());
    }

    #[test]
    fn settlement_record_creation() {
        let now = Utc::now();
        let record = SettlementRecord {
            settlement_id: "settle-001".to_string(),
            packet_id: PacketId::new([1u8; 32]),
            egress_node: NodeId::from("egress-node-42"),
            finality_type: "instant".to_string(),
            fee_collected: GoldGrams::from_decimal(Decimal::new(250, 2)),
            settled_at: now,
        };

        assert_eq!(record.settlement_id, "settle-001");
        assert_eq!(record.egress_node.0, "egress-node-42");
        assert_eq!(record.finality_type, "instant");
        assert_eq!(record.fee_collected.0, Decimal::new(250, 2));
    }

    #[test]
    fn node_status_defaults() {
        let status = NodeStatus {
            node_id: NodeId::from("test-node"),
            active_packets: 0,
            settled_count: 0,
            total_fees_earned: GoldGrams::zero(),
            operator_preferences: OperatorPreferences::default(),
            last_activity: Utc::now(),
        };

        assert_eq!(status.active_packets, 0);
        assert_eq!(status.settled_count, 0);
        assert!(status.total_fees_earned.is_zero());
        assert!(status.operator_preferences.auto_mode);
    }

    #[test]
    fn tier_weights_default_neutral() {
        let weights = TierWeights::default();

        assert_eq!(weights.l0, Decimal::ONE);
        assert_eq!(weights.l1, Decimal::ONE);
        assert_eq!(weights.l2, Decimal::ONE);
        assert_eq!(weights.l3, Decimal::ONE);
    }

    #[test]
    fn operator_preferences_auto_mode() {
        let prefs = OperatorPreferences::default();

        assert!(prefs.auto_mode);
        assert!(prefs.preferred_min_packet.is_zero());
        // auto_mode means all traffic accepted
        assert_eq!(prefs.tier_weights.l0, Decimal::ONE);
    }

    #[test]
    fn transaction_type_serialization() {
        let variants = [
            (TransactionType::Ingress, "\"ingress\""),
            (TransactionType::Egress, "\"egress\""),
            (TransactionType::Transit, "\"transit\""),
            (TransactionType::Settlement, "\"settlement\""),
            (TransactionType::Refund, "\"refund\""),
            (TransactionType::Dissolution, "\"dissolution\""),
            (TransactionType::FeeDistribution, "\"fee_distribution\""),
        ];

        for (variant, expected_json) in &variants {
            let serialized =
                serde_json::to_string(variant).expect("test: serialization should succeed");
            assert_eq!(
                &serialized, expected_json,
                "TransactionType::{variant:?} serialized to {serialized} but expected {expected_json}"
            );
        }
    }
}
