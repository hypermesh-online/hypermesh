// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Type definitions for container asset adapter.

use crate::assets::core::{AssetRegistration, PrivacyMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Container allocation record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerAllocation {
    pub asset_id: AssetRegistration,
    pub container_id: String,
    pub image: String,
    pub container_name: String,
    pub cpu_allocation: ContainerCpuAllocation,
    pub memory_allocation: ContainerMemoryAllocation,
    pub volumes: Vec<ContainerVolume>,
    pub network_config: ContainerNetworkConfig,
    pub environment: HashMap<String, String>,
    pub command: Option<Vec<String>>,
    pub working_directory: Option<String>,
    pub container_status: ContainerStatus,
    pub privacy_level: PrivacyMode,
    pub security_config: ContainerSecurityConfig,
    pub allocated_at: SystemTime,
    pub last_accessed: SystemTime,
    pub runtime_stats: ContainerRuntimeStats,
}

/// Container CPU allocation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerCpuAllocation {
    pub cpu_limit: f32,
    pub cpu_request: f32,
    pub cpu_shares: u32,
    pub pinned_cores: Vec<u32>,
}

/// Container memory allocation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerMemoryAllocation {
    pub memory_limit_bytes: u64,
    pub memory_request_bytes: u64,
    pub swap_limit_bytes: u64,
    pub oom_kill_disabled: bool,
}

/// Container volume configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerVolume {
    pub name: String,
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
    pub volume_type: VolumeType,
    pub size_limit_bytes: Option<u64>,
}

/// Volume types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VolumeType {
    HostPath,
    TmpFs,
    Volume,
    ConfigMap,
    Secret,
}

/// Container network configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerNetworkConfig {
    pub network_mode: NetworkMode,
    pub port_mappings: Vec<ContainerPortMapping>,
    pub ipv6_addresses: Vec<String>,
    pub network_aliases: Vec<String>,
    pub dns_config: DnsConfig,
    pub bandwidth_limits: BandwidthLimits,
}

/// Network modes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkMode {
    Bridge,
    Host,
    None,
    Custom(String),
}

/// Container port mapping
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerPortMapping {
    pub container_port: u16,
    pub host_port: u16,
    pub protocol: String,
    pub bind_address: Option<String>,
}

/// DNS configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsConfig {
    pub nameservers: Vec<String>,
    pub search_domains: Vec<String>,
    pub options: Vec<String>,
}

/// Bandwidth limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BandwidthLimits {
    pub ingress_mbps: Option<u64>,
    pub egress_mbps: Option<u64>,
}

/// Container security configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerSecurityConfig {
    pub user_id: Option<u32>,
    pub group_id: Option<u32>,
    pub privileged: bool,
    pub read_only_rootfs: bool,
    pub capabilities: SecurityCapabilities,
    pub security_labels: HashMap<String, String>,
    pub seccomp_profile: Option<String>,
}

/// Security capabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityCapabilities {
    pub add: Vec<String>,
    pub drop: Vec<String>,
}

/// Container status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContainerStatus {
    Created,
    Running,
    Paused,
    Stopped,
    Exited(i32),
    Failed(String),
}

/// Container runtime statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerRuntimeStats {
    pub cpu_usage_percent: f32,
    pub memory_usage_bytes: u64,
    pub network_io: NetworkIoStats,
    pub block_io: BlockIoStats,
    pub process_count: u32,
    pub uptime_seconds: u64,
}

/// Network I/O statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkIoStats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

/// Block I/O statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockIoStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
}

/// Container runtime interface
#[derive(Clone, Debug)]
pub struct ContainerRuntime {
    pub runtime_type: RuntimeType,
    pub socket_path: String,
    pub api_version: String,
}

/// Container runtime types
#[derive(Clone, Debug)]
pub enum RuntimeType {
    Docker,
    Containerd,
    CriO,
    Podman,
}

/// Container image information
#[derive(Clone, Debug)]
pub struct ImageInfo {
    pub image_id: String,
    pub image_name: String,
    pub size_bytes: u64,
    pub created_at: SystemTime,
    pub architecture: String,
    pub os: String,
    pub security_status: SecurityScanStatus,
}

/// Security scan status for images
#[derive(Clone, Debug)]
pub enum SecurityScanStatus {
    NotScanned,
    Scanning,
    Passed,
    Vulnerabilities(u32),
    Failed(String),
}

/// Container usage statistics
#[derive(Clone, Debug, Default)]
pub struct ContainerUsageStats {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub active_containers: u64,
    pub total_cpu_time_seconds: f64,
    pub total_memory_allocated: u64,
    pub total_network_io_bytes: u64,
    pub total_block_io_bytes: u64,
    pub container_restarts: u64,
}

/// Container operations for statistics
#[derive(Clone, Debug)]
pub(crate) enum ContainerOperation {
    Create,
    Destroy,
    _Restart,
}
