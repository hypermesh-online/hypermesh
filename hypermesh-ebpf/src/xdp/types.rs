// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! XDP types, configuration structs, and enums.
//!
//! Contains all public types for XDP management: `KernelPosConfig`,
//! `PacketDecision`, `FilterAction`, `XdpAttachMode`, `OffloadPolicy`,
//! `XdpStats`, `XdpFilterConfig`, and internal `AttachedProgram`.

use hypermesh_lib::MatrixPosition;

// -----------------------------------------------------------------------
// Kernel-side PoS configuration
// -----------------------------------------------------------------------

/// Configuration for kernel-side PoS structural validation.
///
/// Synced to the `pos_config_map` BPF array map (index 0).
///
/// These checks are non-cryptographic -- they reject obviously invalid
/// packets at wire speed (wrong algorithm byte, insufficient PoW
/// difficulty, stale cache entries).  Full asymmetric crypto
/// verification (FALCON-1024, Ed25519, ECDSA) MUST remain in
/// userspace because the BPF instruction set has no such helpers.
///
/// Serialization layout (32 bytes, little-endian, natural alignment — matches
/// C `struct pos_config`; u64 fields are 8-byte aligned):
///   `[0..4]`   min_difficulty        (u32 LE)
///   `[4..12]`  max_timestamp_skew_ns (u64 LE)
///   `[12..20]` validation_ttl_ns     (u64 LE)
///   `[20..24]` enabled               (u32 LE, 1 or 0)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelPosConfig {
    /// Minimum leading zero bits for PoW difficulty (0 = disabled)
    pub min_difficulty: u32,
    /// Maximum clock skew tolerance in nanoseconds (0 = disabled).
    /// Stored in the BPF map for future use; current kernel code
    /// uses `validation_ttl_ns` for staleness enforcement.
    pub max_timestamp_skew_ns: u64,
    /// How long a cached PoS validation is considered valid (ns).
    /// 0 means cached entries never expire (infinite TTL).
    pub validation_ttl_ns: u64,
    /// Whether kernel-side PoS structural checks are enabled.
    /// When false, the XDP program falls back to cache-only lookup.
    pub enabled: bool,
}

impl Default for KernelPosConfig {
    fn default() -> Self {
        Self {
            min_difficulty: 8, // Match userspace default (first byte must be 0x00)
            max_timestamp_skew_ns: 5 * 60 * 1_000_000_000, // 5 minutes
            validation_ttl_ns: 60 * 60 * 1_000_000_000, // 1 hour
            enabled: true,
        }
    }
}

impl KernelPosConfig {
    /// Serialize to 32 bytes matching the C `struct pos_config` NATURAL
    /// (non-packed) layout — the u64 fields are 8-byte aligned, so there are
    /// 4 bytes of padding after `min_difficulty` and 4 trailing pad bytes.
    ///
    /// Layout (all little-endian; matches `struct pos_config` in hypermesh_xdp.c):
    ///   `[0..4]`   min_difficulty        u32
    ///   `[4..8]`   (padding)
    ///   `[8..16]`  max_timestamp_skew_ns u64
    ///   `[16..24]` validation_ttl_ns     u64
    ///   `[24..28]` enabled               u32
    ///   `[28..32]` (padding)
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];
        buf[0..4].copy_from_slice(&self.min_difficulty.to_le_bytes());
        buf[8..16].copy_from_slice(&self.max_timestamp_skew_ns.to_le_bytes());
        buf[16..24].copy_from_slice(&self.validation_ttl_ns.to_le_bytes());
        buf[24..28].copy_from_slice(&(self.enabled as u32).to_le_bytes());
        buf
    }

    /// Deserialize from 32 bytes (C `struct pos_config` natural layout).
    ///
    /// Returns `None` if the slice is too short.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 32 {
            return None;
        }
        Some(Self {
            min_difficulty: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            max_timestamp_skew_ns: u64::from_le_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ]),
            validation_ttl_ns: u64::from_le_bytes([
                bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22],
                bytes[23],
            ]),
            enabled: u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]) != 0,
        })
    }
}

// -----------------------------------------------------------------------
// Packet decision types (the three execution paths)
// -----------------------------------------------------------------------

/// Decision for an incoming packet. Represents the three HyperMesh execution paths:
/// 1. Pass (local execution)
/// 2. Redirect (zero-copy AF_XDP to STOQ)
/// 3. Forward (delegate to another matrix node)
/// 4. Drop (invalid)
#[derive(Debug, Clone, PartialEq)]
pub enum PacketDecision {
    /// XDP_PASS - deliver to local userspace for processing
    Pass,
    /// XDP_REDIRECT - zero-copy transfer to AF_XDP socket for STOQ
    Redirect { socket_index: u32 },
    /// XDP_TX / forward - delegate to another matrix node
    Forward { next_hop: MatrixPosition },
    /// XDP_DROP - packet is invalid, discard
    Drop { reason: String },
}

/// Legacy filter action (kept for backward compatibility with existing tests)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// Pass packet to userspace
    Pass,
    /// Drop packet at kernel level
    Drop,
    /// Redirect to AF_XDP socket for zero-copy
    Redirect,
}

// -----------------------------------------------------------------------
// XDP attach mode and stats
// -----------------------------------------------------------------------

/// XDP attach mode
#[derive(Debug, Clone, Copy)]
pub enum XdpAttachMode {
    /// Native mode (fastest, requires driver support)
    Native,
    /// Generic/SKB mode (slower, works everywhere)
    Generic,
    /// Offloaded to NIC hardware (if supported)
    Offload,
}

/// Policy for handling XDP hardware offload
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffloadPolicy {
    /// Never attempt hardware offload (default)
    Disabled,
    /// Try hardware offload, fall back to native XDP if unavailable
    Opportunistic,
    /// Require hardware offload, fail if NIC doesn't support it
    Required,
}

impl Default for OffloadPolicy {
    fn default() -> Self {
        Self::Disabled
    }
}

/// XDP program statistics aggregated from kernel maps.
///
/// Byte-length telemetry is intentionally omitted: the kernel cannot see the
/// QUIC-encrypted payload, so a `bytes_processed` counter would always be zero
/// and misleading. Only packet-decision counters are surfaced.
#[derive(Debug, Default, Clone)]
pub struct XdpStats {
    pub packets_passed: u64,
    pub packets_dropped: u64,
    pub packets_redirected: u64,
}

/// XDP filter configuration
#[derive(Debug, Clone)]
pub struct XdpFilterConfig {
    /// Allow only QUIC packets (UDP port 9292)
    pub filter_quic_only: bool,
    /// Drop non-IPv6 packets
    pub drop_ipv4: bool,
    /// Maximum packet size to process
    pub max_packet_size: usize,
    /// Enable connection tracking in kernel map
    pub enable_connection_tracking: bool,
}

impl Default for XdpFilterConfig {
    fn default() -> Self {
        Self {
            filter_quic_only: true,
            drop_ipv4: true,
            max_packet_size: 65535,
            enable_connection_tracking: true,
        }
    }
}

// -----------------------------------------------------------------------
// Attached program tracking
// -----------------------------------------------------------------------

pub(crate) struct AttachedProgram {
    pub _interface: String,
    pub _attach_mode: XdpAttachMode,
}

/// XDP action to take on packets (matches kernel XDP_* constants)
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum XdpAction {
    /// Drop the packet
    Drop = 1,
    /// Pass packet to normal network stack
    Pass = 2,
    /// Redirect packet to AF_XDP socket
    Redirect = 3,
}
