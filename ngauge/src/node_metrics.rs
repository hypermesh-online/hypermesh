// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Self-node vs network-wide metrics separation.
//!
//! [`SelfMetrics`] captures what THIS node measures locally (hardware + transport).
//! [`PeerMetrics`] captures aggregated observations from the network received
//! via the MetricsFrame streaming protocol.
//!
//! Routing intelligence uses both:
//! - Self-metrics for capacity decisions (am I overloaded?)
//! - Peer-metrics for path optimization (which peers are best?)

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

// ---------------------------------------------------------------------------
// SelfMetrics
// ---------------------------------------------------------------------------

/// Metrics about THIS node's own hardware and transport resources.
///
/// Collected locally by reading `/proc` (hardware) and QUIC connection
/// stats (transport). Never transmitted to peers directly -- the
/// [`MetricsPublisher`](crate::streaming::MetricsPublisher) produces
/// privacy-filtered snapshots from these values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfMetrics {
    /// When this snapshot was taken.
    pub timestamp: SystemTime,
    /// Hardware metrics.
    pub hardware: HardwareSummary,
    /// Transport metrics.
    pub transport: TransportSummary,
}

/// Summary of local hardware state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HardwareSummary {
    /// CPU usage ratio (0.0 to 1.0).
    pub cpu_usage: f64,
    /// Number of logical CPU cores.
    pub cpu_cores: usize,
    /// Total memory in bytes.
    pub memory_total_bytes: u64,
    /// Available memory in bytes.
    pub memory_available_bytes: u64,
    /// Memory usage ratio (0.0 to 1.0).
    pub memory_usage: f64,
    /// Root filesystem total in bytes.
    pub storage_total_bytes: u64,
    /// Root filesystem available in bytes.
    pub storage_available_bytes: u64,
    /// Storage usage ratio (0.0 to 1.0).
    pub storage_usage: f64,
    /// Network bytes received (cumulative).
    pub net_rx_bytes: u64,
    /// Network bytes transmitted (cumulative).
    pub net_tx_bytes: u64,
    /// Network receive errors (cumulative).
    pub net_rx_errors: u64,
    /// Network transmit errors (cumulative).
    pub net_tx_errors: u64,
}

/// Summary of local transport (STOQ/QUIC) state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransportSummary {
    /// Current throughput in bits per second.
    pub throughput_bps: f64,
    /// Average RTT in microseconds.
    pub avg_latency_us: u64,
    /// RTT jitter in microseconds.
    pub jitter_us: u64,
    /// P95 latency in microseconds.
    pub p95_latency_us: u64,
    /// Packet loss ratio (0.0 to 1.0).
    pub loss_ratio: f64,
    /// Active QUIC connections.
    pub active_connections: usize,
    /// Total bytes sent.
    pub bytes_sent: u64,
    /// Total bytes received.
    pub bytes_received: u64,
    /// Retransmissions count.
    pub retransmissions: u64,
    /// Transport uptime.
    #[serde(skip)]
    pub uptime: Duration,
}

// ---------------------------------------------------------------------------
// PeerMetrics
// ---------------------------------------------------------------------------

/// Aggregated metrics from the network, received via streaming protocol.
///
/// Built from [`RegionalAggregate`](crate::streaming::RegionalAggregate)
/// data received from peer nodes. Contains NO local hardware data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerMetrics {
    /// When this aggregate was computed.
    pub timestamp: SystemTime,
    /// Number of peers contributing data.
    pub peer_count: usize,
    /// Average buffer fullness across peers (0.0 to 1.0).
    pub avg_congestion: f64,
    /// Average latency across peers in microseconds.
    pub avg_latency_us: f64,
    /// Average throughput across peers in bits per second.
    pub avg_throughput_bps: f64,
    /// Total available bandwidth across peers in bps.
    pub total_bandwidth_bps: u64,
    /// Average capacity score across peers (0.0 to 1.0).
    pub avg_capacity_score: f64,
    /// Number of spatially verified peers.
    pub verified_peer_count: usize,
    /// Average spatial consistency ratio across verified peers.
    pub avg_consistency_ratio: f64,
}

impl PeerMetrics {
    /// Create from a regional aggregate.
    pub fn from_aggregate(
        agg: &crate::streaming::RegionalAggregate,
    ) -> Self {
        Self {
            timestamp: SystemTime::now(),
            peer_count: agg.node_count,
            avg_congestion: agg.avg_buffer_fullness,
            avg_latency_us: agg.avg_latency_us,
            avg_throughput_bps: agg.avg_throughput_bps,
            total_bandwidth_bps: agg.total_bandwidth_bps,
            avg_capacity_score: agg.avg_capacity_score,
            verified_peer_count: agg.verified_node_count,
            avg_consistency_ratio: agg.avg_consistency_ratio,
        }
    }
}

// ---------------------------------------------------------------------------
// Capacity assessment
// ---------------------------------------------------------------------------

/// Assessment of whether this node can accept more work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityLevel {
    /// Node has plenty of headroom.
    Available,
    /// Node is moderately loaded.
    Moderate,
    /// Node is near capacity -- shed load.
    Saturated,
}

/// Thresholds for capacity assessment.
const CPU_MODERATE: f64 = 0.60;
const CPU_SATURATED: f64 = 0.85;
const MEMORY_MODERATE: f64 = 0.70;
const MEMORY_SATURATED: f64 = 0.90;

/// Assess this node's capacity from self-metrics.
pub fn assess_capacity(self_metrics: &SelfMetrics) -> CapacityLevel {
    let hw = &self_metrics.hardware;

    if hw.cpu_usage > CPU_SATURATED || hw.memory_usage > MEMORY_SATURATED {
        CapacityLevel::Saturated
    } else if hw.cpu_usage > CPU_MODERATE || hw.memory_usage > MEMORY_MODERATE {
        CapacityLevel::Moderate
    } else {
        CapacityLevel::Available
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn idle_hardware() -> HardwareSummary {
        HardwareSummary {
            cpu_usage: 0.10,
            cpu_cores: 4,
            memory_total_bytes: 16_000_000_000,
            memory_available_bytes: 12_000_000_000,
            memory_usage: 0.25,
            storage_total_bytes: 500_000_000_000,
            storage_available_bytes: 300_000_000_000,
            storage_usage: 0.40,
            net_rx_bytes: 1_000_000,
            net_tx_bytes: 500_000,
            net_rx_errors: 0,
            net_tx_errors: 0,
        }
    }

    fn busy_hardware() -> HardwareSummary {
        HardwareSummary {
            cpu_usage: 0.92,
            memory_usage: 0.95,
            ..idle_hardware()
        }
    }

    fn moderate_hardware() -> HardwareSummary {
        HardwareSummary {
            cpu_usage: 0.70,
            memory_usage: 0.75,
            ..idle_hardware()
        }
    }

    fn idle_transport() -> TransportSummary {
        TransportSummary {
            throughput_bps: 100_000_000.0,
            avg_latency_us: 5000,
            jitter_us: 200,
            p95_latency_us: 8000,
            loss_ratio: 0.001,
            active_connections: 3,
            bytes_sent: 10_000_000,
            bytes_received: 20_000_000,
            retransmissions: 5,
            uptime: Duration::from_secs(3600),
        }
    }

    fn self_metrics_with(hw: HardwareSummary) -> SelfMetrics {
        SelfMetrics {
            timestamp: SystemTime::now(),
            hardware: hw,
            transport: idle_transport(),
        }
    }

    #[test]
    fn capacity_available_when_idle() {
        let m = self_metrics_with(idle_hardware());
        assert_eq!(assess_capacity(&m), CapacityLevel::Available);
    }

    #[test]
    fn capacity_saturated_when_busy() {
        let m = self_metrics_with(busy_hardware());
        assert_eq!(assess_capacity(&m), CapacityLevel::Saturated);
    }

    #[test]
    fn capacity_moderate_when_loaded() {
        let m = self_metrics_with(moderate_hardware());
        assert_eq!(assess_capacity(&m), CapacityLevel::Moderate);
    }

    #[test]
    fn peer_metrics_from_aggregate() {
        let agg = crate::streaming::RegionalAggregate {
            node_count: 5,
            avg_buffer_fullness: 0.3,
            avg_latency_us: 8000.0,
            avg_throughput_bps: 500_000_000.0,
            total_bandwidth_bps: 2_500_000_000,
            avg_capacity_score: 0.7,
            verified_node_count: 4,
            avg_consistency_ratio: 0.95,
        };

        let pm = PeerMetrics::from_aggregate(&agg);
        assert_eq!(pm.peer_count, 5);
        assert!((pm.avg_congestion - 0.3).abs() < 1e-9);
        assert_eq!(pm.total_bandwidth_bps, 2_500_000_000);
        assert_eq!(pm.verified_peer_count, 4);
    }

    #[test]
    fn self_metrics_serde_roundtrip() {
        let m = self_metrics_with(idle_hardware());
        let json = serde_json::to_string(&m).expect("test: serialize self_metrics");
        let back: SelfMetrics =
            serde_json::from_str(&json).expect("test: deserialize self_metrics");
        assert_eq!(back.hardware.cpu_cores, 4);
        assert_eq!(back.transport.active_connections, 3);
    }

    #[test]
    fn peer_metrics_serde_roundtrip() {
        let agg = crate::streaming::RegionalAggregate {
            node_count: 2,
            avg_buffer_fullness: 0.5,
            avg_latency_us: 1000.0,
            avg_throughput_bps: 100_000.0,
            total_bandwidth_bps: 200_000,
            avg_capacity_score: 0.8,
            verified_node_count: 1,
            avg_consistency_ratio: 0.99,
        };
        let pm = PeerMetrics::from_aggregate(&agg);
        let json = serde_json::to_string(&pm).expect("test: serialize peer_metrics");
        let back: PeerMetrics =
            serde_json::from_str(&json).expect("test: deserialize peer_metrics");
        assert_eq!(back.peer_count, 2);
    }

    #[test]
    fn hardware_summary_default_is_zero() {
        let hw = HardwareSummary::default();
        assert_eq!(hw.cpu_cores, 0);
        assert_eq!(hw.memory_total_bytes, 0);
        assert!(hw.cpu_usage.abs() < 1e-9);
    }

    #[test]
    fn transport_summary_default_is_zero() {
        let ts = TransportSummary::default();
        assert_eq!(ts.active_connections, 0);
        assert!(ts.throughput_bps.abs() < 1e-9);
    }
}
