// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network adapter types: allocation records, interface info, QoS, and statistics.

use std::time::SystemTime;
use serde::{Deserialize, Serialize};

use crate::assets::core::{AssetRegistration, PrivacyMode, ProxyAddress};

/// Network allocation record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkAllocation {
    pub asset_id: AssetRegistration,
    pub allocated_interfaces: Vec<String>,
    pub allocated_bandwidth_mbps: u64,
    pub enabled_protocols: Vec<String>,
    pub qos_priority: u8,
    pub traffic_shaping_enabled: bool,
    pub isolation_enabled: bool,
    pub ipv6_addresses: Vec<String>,
    pub privacy_level: PrivacyMode,
    pub vlan_id: Option<u16>,
    pub allocated_at: SystemTime,
    pub last_accessed: SystemTime,
    pub current_bandwidth_mbps: f32,
    pub current_latency_us: u32,
    pub current_packet_loss_percent: f32,
}

/// Network interface information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub interface_name: String,
    pub interface_type: InterfaceType,
    pub max_bandwidth_mbps: u64,
    pub available_bandwidth_mbps: u64,
    pub mtu: u32,
    pub mac_address: String,
    pub ipv6_address: Option<String>,
    pub status: InterfaceStatus,
    pub allocated_to: Option<AssetRegistration>,
    pub interface_stats: InterfaceStats,
}

/// Network interface types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InterfaceType {
    Ethernet,
    WiFi,
    Loopback,
    Virtual,
    Bridge,
}

/// Network interface status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InterfaceStatus {
    Up,
    Down,
    Allocated,
    Active,
    Maintenance,
    Failed,
}

/// Network interface statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterfaceStats {
    pub bytes_received: u64,
    pub bytes_transmitted: u64,
    pub packets_received: u64,
    pub packets_transmitted: u64,
    pub receive_errors: u64,
    pub transmit_errors: u64,
    pub dropped_packets: u64,
    pub collisions: u64,
}

/// Quality of Service configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QoSConfig {
    pub priority: u8,
    pub guaranteed_bandwidth_mbps: u64,
    pub max_burst_bytes: u64,
    pub traffic_class: TrafficClass,
    pub dscp_marking: u8,
}

/// Traffic classification
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TrafficClass {
    BestEffort,
    Bulk,
    Interactive,
    RealTime,
    Critical,
}

/// Network security configuration
#[derive(Clone, Debug)]
pub struct NetworkSecurity {
    pub firewall_enabled: bool,
    pub vpn_enabled: bool,
    pub encryption_enabled: bool,
    pub ddos_protection: bool,
    pub intrusion_detection: bool,
}

/// Network usage statistics
#[derive(Clone, Debug, Default)]
pub struct NetworkUsageStats {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub active_allocations: u64,
    pub total_bandwidth_allocated: u64,
    pub total_bytes_transferred: u64,
    pub total_packets_transferred: u64,
    pub average_latency_us: f32,
    pub average_packet_loss_percent: f32,
}

/// Network operations for statistics
#[derive(Clone, Debug)]
pub(crate) enum NetworkOperation {
    Allocate,
    Deallocate,
    _Transfer,
}

/// Shared state types used by [`super::NetworkAssetAdapter`].
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;

/// Type alias for the adapter's internal state collections.
pub(crate) struct AdapterState {
    pub allocations: Arc<RwLock<HashMap<AssetRegistration, NetworkAllocation>>>,
    pub network_interfaces: Arc<RwLock<HashMap<String, NetworkInterface>>>,
    pub interface_allocations: Arc<RwLock<HashMap<String, AssetRegistration>>>,
    pub qos_configs: Arc<RwLock<HashMap<AssetRegistration, QoSConfig>>>,
    pub proxy_mappings: Arc<RwLock<HashMap<ProxyAddress, AssetRegistration>>>,
    pub available_bandwidth: Arc<RwLock<u64>>,
    pub usage_stats: Arc<RwLock<NetworkUsageStats>>,
    pub total_bandwidth: u64,
}
