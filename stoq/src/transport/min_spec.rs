// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! R13 minimum-spec transport validation for STOQ.
//!
//! Enforces that STOQ operates within the hard requirement R13 budget:
//! 1 Mb/s network, 4 GB RAM, 2-core 1 GHz CPU.

use hypermesh_lib::protocol::HardwareCapabilities;

/// Per-QUIC-connection memory overhead estimate in bytes.
///
/// Each QUIC connection requires TLS state, stream buffers, congestion
/// control state, and send/receive windows. Conservative estimate.
pub const QUIC_CONNECTION_OVERHEAD_BYTES: u64 = 2 * 1024 * 1024; // ~2 MB

/// Minimum RAM reserved for OS and non-transport processes (bytes).
pub const RESERVED_RAM_BYTES: u64 = 1_500_000_000; // 1.5 GB

/// R13 transport budget derived from hardware minimum spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportMinSpec {
    /// Maximum network bandwidth in bits per second (R13: 1 Mb/s).
    pub max_bandwidth_bps: u64,
    /// Maximum RAM available for transport (total - reserved).
    pub max_transport_ram_bytes: u64,
    /// Number of CPU cores available.
    pub cpu_cores: u16,
}

impl TransportMinSpec {
    /// Create from R13 hard limits.
    pub fn from_r13() -> Self {
        Self {
            max_bandwidth_bps: HardwareCapabilities::MIN_NETWORK_BPS,
            max_transport_ram_bytes: HardwareCapabilities::MIN_RAM_BYTES
                .saturating_sub(RESERVED_RAM_BYTES),
            cpu_cores: HardwareCapabilities::MIN_CPU_CORES,
        }
    }

    /// Create from actual hardware capabilities.
    pub fn from_hardware(hw: &HardwareCapabilities) -> Self {
        Self {
            max_bandwidth_bps: hw.network_bandwidth_bps,
            max_transport_ram_bytes: hw.ram_bytes.saturating_sub(RESERVED_RAM_BYTES),
            cpu_cores: hw.cpu_cores,
        }
    }
}

/// Per-connection resource budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionBudget {
    /// Estimated memory per connection in bytes.
    pub memory_bytes: u64,
    /// Estimated CPU fraction per connection (0.0-1.0 approximated as basis points).
    pub cpu_basis_points: u32,
}

impl Default for ConnectionBudget {
    fn default() -> Self {
        Self {
            memory_bytes: QUIC_CONNECTION_OVERHEAD_BYTES,
            cpu_basis_points: 50, // ~0.5% per connection
        }
    }
}

/// Check whether the node can sustain the given number of active QUIC
/// connections within the available RAM.
///
/// Each connection is estimated at [`QUIC_CONNECTION_OVERHEAD_BYTES`].
pub fn validate_connection_budget(active_connections: usize, ram_available: u64) -> bool {
    let transport_ram = ram_available.saturating_sub(RESERVED_RAM_BYTES);
    let required = (active_connections as u64).saturating_mul(QUIC_CONNECTION_OVERHEAD_BYTES);
    required <= transport_ram
}

/// Check whether the target throughput fits within the available bandwidth.
///
/// `target_throughput_bps` is in bits per second. Returns `true` if it fits
/// within the R13 minimum of 1 Mb/s (or the provided bandwidth).
pub fn validate_bandwidth_budget(target_throughput_bps: u64) -> bool {
    target_throughput_bps <= HardwareCapabilities::MIN_NETWORK_BPS
}

/// Estimate per-connection resource overhead.
pub fn estimate_connection_overhead() -> ConnectionBudget {
    ConnectionBudget::default()
}

/// Calculate the maximum number of concurrent QUIC connections that can
/// fit in the given available RAM (after reserving OS overhead).
pub fn max_concurrent_connections(available_ram: u64) -> usize {
    let transport_ram = available_ram.saturating_sub(RESERVED_RAM_BYTES);
    if QUIC_CONNECTION_OVERHEAD_BYTES == 0 {
        return 0;
    }
    (transport_ram / QUIC_CONNECTION_OVERHEAD_BYTES) as usize
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_min_spec_from_r13() {
        let spec = TransportMinSpec::from_r13();
        assert_eq!(spec.max_bandwidth_bps, 1_000_000);
        assert_eq!(spec.cpu_cores, 2);
        // 4 GB - 1.5 GB reserved = 2.5 GB for transport
        assert_eq!(spec.max_transport_ram_bytes, 2_500_000_000);
    }

    #[test]
    fn validate_connection_budget_within_limits() {
        // 4 GB RAM, 10 connections = 20 MB << 2.5 GB available
        assert!(validate_connection_budget(10, 4_000_000_000));
    }

    #[test]
    fn validate_connection_budget_exceeds_limits() {
        // 2 GB RAM (below min spec), 1500 connections would need 3 GB
        assert!(!validate_connection_budget(1500, 2_000_000_000));
    }

    #[test]
    fn validate_connection_budget_zero_connections() {
        assert!(validate_connection_budget(0, 4_000_000_000));
    }

    #[test]
    fn validate_bandwidth_within_budget() {
        assert!(validate_bandwidth_budget(500_000));  // 500 Kbps < 1 Mbps
        assert!(validate_bandwidth_budget(1_000_000)); // exactly 1 Mbps
    }

    #[test]
    fn validate_bandwidth_exceeds_budget() {
        assert!(!validate_bandwidth_budget(2_000_000)); // 2 Mbps > 1 Mbps
    }

    #[test]
    fn estimate_connection_overhead_returns_defaults() {
        let budget = estimate_connection_overhead();
        assert_eq!(budget.memory_bytes, QUIC_CONNECTION_OVERHEAD_BYTES);
        assert!(budget.cpu_basis_points > 0);
    }

    #[test]
    fn max_concurrent_connections_r13() {
        // R13 min: 4 GB RAM -> 2.5 GB transport -> 2.5 GB / 2 MiB per conn
        let transport_ram = 4_000_000_000u64 - RESERVED_RAM_BYTES;
        let expected = transport_ram / QUIC_CONNECTION_OVERHEAD_BYTES;
        let max = max_concurrent_connections(4_000_000_000);
        assert_eq!(max, expected as usize);
        // Sanity: should be over 1000 connections on min-spec hardware
        assert!(max > 1000);
    }

    #[test]
    fn max_concurrent_connections_low_ram() {
        // 1.5 GB RAM -> 0 GB transport -> 0 connections
        let max = max_concurrent_connections(1_500_000_000);
        assert_eq!(max, 0);
    }

    #[test]
    fn transport_min_spec_from_hardware() {
        let hw = HardwareCapabilities {
            cpu_cores: 8,
            cpu_clock_mhz: 3000,
            ram_bytes: 16_000_000_000,
            storage_bytes: 500_000_000_000,
            network_bandwidth_bps: 1_000_000_000,
            gpu_available: false,
            gpu_vram_bytes: None,
        };
        let spec = TransportMinSpec::from_hardware(&hw);
        assert_eq!(spec.max_bandwidth_bps, 1_000_000_000);
        assert_eq!(spec.cpu_cores, 8);
        assert_eq!(spec.max_transport_ram_bytes, 14_500_000_000);
    }
}
