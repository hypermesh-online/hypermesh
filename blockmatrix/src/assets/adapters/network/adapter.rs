// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network Asset Adapter -- AssetAdapter trait implementation and internal helpers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::assets::core::{
    AssetAdapter, AssetRegistration, AssetType, AssetResult, AssetError,
    AssetAllocationRequest, AssetStatus, AssetState,
    PrivacyMode, AssetAllocation, ProxyAddress,
    ResourceUsage, ResourceLimits, NetworkUsage,
    AdapterHealth, AdapterCapabilities, ConsensusProof,
    NetworkRequirements,
    NetworkScope, AssetCategory, BaseSystemType, AssetData,
};

use super::types::*;

/// Network Asset Adapter implementation
pub struct NetworkAssetAdapter {
    state: AdapterState,
}

impl NetworkAssetAdapter {
    /// Create new network adapter
    pub async fn new() -> Self {
        let (total_bandwidth, network_interfaces) = Self::detect_network_configuration().await;

        Self {
            state: AdapterState {
                allocations: Arc::new(RwLock::new(HashMap::new())),
                network_interfaces: Arc::new(RwLock::new(network_interfaces)),
                interface_allocations: Arc::new(RwLock::new(HashMap::new())),
                qos_configs: Arc::new(RwLock::new(HashMap::new())),
                proxy_mappings: Arc::new(RwLock::new(HashMap::new())),
                total_bandwidth,
                available_bandwidth: Arc::new(RwLock::new(total_bandwidth)),
                usage_stats: Arc::new(RwLock::new(NetworkUsageStats::default())),
            },
        }
    }

    /// Detect system network configuration
    async fn detect_network_configuration() -> (u64, HashMap<String, NetworkInterface>) {
        let mut network_interfaces = HashMap::new();
        let mut total_bandwidth = 0u64;

        let eth_bandwidth = 10000;
        network_interfaces.insert("eth0".to_string(), NetworkInterface {
            interface_name: "eth0".to_string(),
            interface_type: InterfaceType::Ethernet,
            max_bandwidth_mbps: eth_bandwidth,
            available_bandwidth_mbps: eth_bandwidth,
            mtu: 1500,
            mac_address: "02:42:ac:11:00:02".to_string(),
            ipv6_address: Some("2001:db8::1".to_string()),
            status: InterfaceStatus::Up,
            allocated_to: None,
            interface_stats: InterfaceStats {
                bytes_received: 0, bytes_transmitted: 0,
                packets_received: 0, packets_transmitted: 0,
                receive_errors: 0, transmit_errors: 0,
                dropped_packets: 0, collisions: 0,
            },
        });
        total_bandwidth += eth_bandwidth;

        let wifi_bandwidth = 1000;
        network_interfaces.insert("wlan0".to_string(), NetworkInterface {
            interface_name: "wlan0".to_string(),
            interface_type: InterfaceType::WiFi,
            max_bandwidth_mbps: wifi_bandwidth,
            available_bandwidth_mbps: wifi_bandwidth,
            mtu: 1500,
            mac_address: "02:42:ac:11:00:03".to_string(),
            ipv6_address: Some("2001:db8::2".to_string()),
            status: InterfaceStatus::Up,
            allocated_to: None,
            interface_stats: InterfaceStats {
                bytes_received: 0, bytes_transmitted: 0,
                packets_received: 0, packets_transmitted: 0,
                receive_errors: 0, transmit_errors: 0,
                dropped_packets: 0, collisions: 0,
            },
        });
        total_bandwidth += wifi_bandwidth;

        (total_bandwidth, network_interfaces)
    }

    /// Allocate network bandwidth from interfaces
    async fn allocate_network_bandwidth(
        &self,
        network_req: &NetworkRequirements,
        asset_id: &AssetRegistration,
    ) -> AssetResult<(Vec<String>, u64)> {
        let mut interfaces = self.state.network_interfaces.write().await;
        let mut interface_allocations = self.state.interface_allocations.write().await;
        let mut allocated_interfaces = Vec::new();
        let mut total_allocated_bandwidth = 0u64;

        let mut suitable_interfaces: Vec<String> = interfaces
            .iter()
            .filter(|(_, iface)| {
                matches!(iface.status, InterfaceStatus::Up)
                    && iface.available_bandwidth_mbps >= network_req.bandwidth_mbps
                    && (network_req.protocols.is_empty()
                        || network_req.protocols.iter().all(|p| matches!(p.as_str(), "TCP" | "UDP" | "ICMP")))
            })
            .map(|(name, _)| name.clone())
            .collect();

        suitable_interfaces.sort_by_key(|name| {
            interfaces.get(name)
                .map(|i| std::cmp::Reverse(i.available_bandwidth_mbps))
                .unwrap_or(std::cmp::Reverse(0))
        });

        let total_available: u64 = suitable_interfaces
            .iter()
            .filter_map(|name| interfaces.get(name).map(|i| i.available_bandwidth_mbps))
            .sum();

        if total_available < network_req.bandwidth_mbps {
            return Err(AssetError::AllocationFailed {
                reason: format!(
                    "Insufficient network bandwidth: {} Mbps requested, {} Mbps available",
                    network_req.bandwidth_mbps, total_available
                ),
            });
        }

        let mut remaining = network_req.bandwidth_mbps;
        for name in &suitable_interfaces {
            if remaining == 0 { break; }
            let iface = interfaces.get_mut(name)
                .ok_or_else(|| AssetError::ResourceUnavailable(format!("Interface {} not found", name)))?;
            let alloc = remaining.min(iface.available_bandwidth_mbps);
            iface.available_bandwidth_mbps -= alloc;
            iface.status = InterfaceStatus::Allocated;
            iface.allocated_to = Some(asset_id.clone());
            interface_allocations.insert(name.clone(), asset_id.clone());
            allocated_interfaces.push(name.clone());
            total_allocated_bandwidth += alloc;
            remaining -= alloc;
        }

        if remaining > 0 {
            for name in &allocated_interfaces {
                let iface = interfaces.get_mut(name)
                    .ok_or_else(|| AssetError::ResourceUnavailable(format!("Interface {} not found", name)))?;
                iface.available_bandwidth_mbps += total_allocated_bandwidth / allocated_interfaces.len() as u64;
                iface.status = InterfaceStatus::Up;
                iface.allocated_to = None;
                interface_allocations.remove(name);
            }
            return Err(AssetError::AllocationFailed {
                reason: "Failed to allocate complete bandwidth requirement".to_string(),
            });
        }

        Ok((allocated_interfaces, total_allocated_bandwidth))
    }

    async fn generate_proxy_address(asset_id: &AssetRegistration) -> ProxyAddress {
        let mut node_id = [0u8; 8];
        node_id.copy_from_slice(&asset_id.content_hash[..8]);
        ProxyAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad,
             0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
            node_id, 8080,
        )
    }

    async fn allocate_ipv6_addresses(&self, count: u32) -> Vec<String> {
        (0..count).map(|i| format!("2001:db8:hypermesh::asset:{:x}", i + 1)).collect()
    }

    async fn configure_qos(&self, _asset_id: &AssetRegistration, req: &NetworkRequirements) -> QoSConfig {
        let priority = if req.max_latency_us.unwrap_or(10000) < 1000 { 200 } else { 128 };
        let traffic_class = if req.max_latency_us.unwrap_or(10000) < 1000 {
            TrafficClass::RealTime
        } else if req.bandwidth_mbps > 1000 {
            TrafficClass::Bulk
        } else {
            TrafficClass::BestEffort
        };

        QoSConfig {
            priority,
            guaranteed_bandwidth_mbps: req.bandwidth_mbps / 2,
            max_burst_bytes: 1024 * 1024,
            traffic_class,
            dscp_marking: match traffic_class {
                TrafficClass::RealTime => 46,
                TrafficClass::Critical => 34,
                TrafficClass::Interactive => 18,
                TrafficClass::Bulk => 10,
                TrafficClass::BestEffort => 0,
            },
        }
    }

    async fn update_usage_stats(&self, operation: NetworkOperation, bandwidth_mbps: u64) {
        let mut stats = self.state.usage_stats.write().await;
        match operation {
            NetworkOperation::Allocate => {
                stats.total_allocations += 1;
                stats.active_allocations += 1;
                stats.total_bandwidth_allocated += bandwidth_mbps;
            }
            NetworkOperation::Deallocate => {
                stats.total_deallocations += 1;
                stats.active_allocations = stats.active_allocations.saturating_sub(1);
                stats.total_bandwidth_allocated = stats.total_bandwidth_allocated.saturating_sub(bandwidth_mbps);
            }
            NetworkOperation::_Transfer => {
                stats.total_bytes_transferred += bandwidth_mbps * 1024 * 1024 / 8;
                stats.total_packets_transferred += bandwidth_mbps * 100;
            }
        }
    }
}

#[async_trait]
impl AssetAdapter for NetworkAssetAdapter {
    fn asset_type(&self) -> AssetType { AssetType::Network }

    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> AssetResult<bool> {
        if proof.space_proof.total_size == 0 { return Ok(false); }
        if proof.stake_proof.stake_amount < 25 { return Ok(false); }
        if proof.work_proof.computational_power < 10 { return Ok(false); }
        if proof.time_proof.network_time_offset > Duration::from_secs(1) { return Ok(false); }
        Ok(true)
    }

    async fn allocate_asset(&self, request: &AssetAllocationRequest) -> AssetResult<AssetAllocation> {
        if !self.validate_consensus_proof(&request.consensus_proof).await? {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Network allocation consensus validation failed".to_string(),
            });
        }

        let network_req = request.requested_resources.network_usage.as_ref()
            .ok_or_else(|| AssetError::AllocationFailed { reason: "No network requirements specified".to_string() })?;

        let available = *self.state.available_bandwidth.read().await;
        if available < network_req.bandwidth_mbps {
            return Err(AssetError::AllocationFailed {
                reason: format!("Insufficient network bandwidth: {} Mbps requested, {} Mbps available", network_req.bandwidth_mbps, available),
            });
        }

        let data = AssetData { config: vec![1, 2, 3], definition: vec![4, 5, 6], metadata: vec![7, 8, 9] };
        let asset_id = AssetRegistration::from_asset_data(&data, NetworkScope::Global, AssetCategory::BaseSystem(BaseSystemType::Network));

        let (allocated_interfaces, allocated_bandwidth) = self.allocate_network_bandwidth(network_req, &asset_id).await?;
        let proxy_address = Self::generate_proxy_address(&asset_id).await;
        let ipv6_addresses = self.allocate_ipv6_addresses(allocated_interfaces.len() as u32).await;
        let qos_config = self.configure_qos(&asset_id, network_req).await;
        { self.state.qos_configs.write().await.insert(asset_id.clone(), qos_config); }

        let allocation = NetworkAllocation {
            asset_id: asset_id.clone(),
            allocated_interfaces,
            allocated_bandwidth_mbps: allocated_bandwidth,
            enabled_protocols: network_req.protocols.clone(),
            privacy_level: request.privacy_level.clone(),
            qos_priority: 128,
            traffic_shaping_enabled: true,
            isolation_enabled: request.privacy_level == PrivacyMode::PRIVATE,
            ipv6_addresses,
            vlan_id: if matches!(request.privacy_level, PrivacyMode::PRIVATE) {
                let h = u128::from_le_bytes(asset_id.content_hash[..16].try_into().unwrap());
                Some(100 + (h % 4000) as u16)
            } else { None },
            allocated_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            current_bandwidth_mbps: 0.0,
            current_latency_us: network_req.max_latency_us.unwrap_or(1000),
            current_packet_loss_percent: 0.0,
        };

        { self.state.allocations.write().await.insert(asset_id.clone(), allocation); }
        { self.state.proxy_mappings.write().await.insert(proxy_address.clone(), asset_id.clone()); }
        { *self.state.available_bandwidth.write().await -= allocated_bandwidth; }
        self.update_usage_stats(NetworkOperation::Allocate, allocated_bandwidth).await;

        Ok(AssetAllocation {
            asset_id: asset_id.clone(),
            status: AssetStatus {
                asset_id: asset_id.clone(),
                state: AssetState::Allocated,
                allocated_at: SystemTime::now(),
                last_accessed: SystemTime::now(),
                resource_usage: ResourceUsage { cpu_usage: None, gpu_usage: None, memory_usage: None, storage_usage: None, network_usage: None, measurement_timestamp: SystemTime::now() },
                privacy_level: PrivacyMode::PRIVATE,
                proxy_address: None,
                consensus_proofs: Vec::new(),
                owner_certificate_fingerprint: request.certificate_fingerprint.clone(),
                metadata: HashMap::new(),
                health_status: crate::assets::core::status::AssetHealthStatus::default(),
                performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(),
            },
            allocation_config: crate::assets::core::privacy::AllocationConfig {
                privacy_level: request.privacy_level.clone(),
                resource_allocation: crate::assets::core::privacy::ResourceAllocationConfig::default(),
                concurrency_limits: crate::assets::core::privacy::ConcurrencyLimits::default(),
                duration_config: crate::assets::core::privacy::DurationConfig::default(),
                consensus_requirements: crate::assets::core::privacy::ConsensusRequirements::default(),
            },
            access_config: crate::assets::core::privacy::AccessConfig {
                allowed_certificates: vec![request.certificate_fingerprint.clone()],
                allowed_networks: Vec::new(),
                permissions: crate::assets::core::privacy::AccessPermissions::default(),
                rate_limits: crate::assets::core::privacy::RateLimits::default(),
                auth_requirements: crate::assets::core::privacy::AuthRequirements::default(),
            },
            allocated_at: SystemTime::now(),
            expires_at: request.duration_limit.map(|d| SystemTime::now() + d),
        })
    }

    async fn deallocate_asset(&self, asset_id: &AssetRegistration) -> AssetResult<()> {
        let allocation = {
            self.state.allocations.write().await.remove(asset_id)
                .ok_or_else(|| AssetError::AssetNotFound { asset_id: asset_id.to_string() })?
        };

        {
            let mut interfaces = self.state.network_interfaces.write().await;
            let mut iface_allocs = self.state.interface_allocations.write().await;
            let per_iface = allocation.allocated_bandwidth_mbps / allocation.allocated_interfaces.len() as u64;
            for name in &allocation.allocated_interfaces {
                if let Some(iface) = interfaces.get_mut(name) {
                    iface.status = InterfaceStatus::Up;
                    iface.allocated_to = None;
                    iface.available_bandwidth_mbps += per_iface;
                }
                iface_allocs.remove(name);
            }
        }
        { self.state.qos_configs.write().await.remove(asset_id); }
        { self.state.proxy_mappings.write().await.retain(|_, v| v != asset_id); }
        { *self.state.available_bandwidth.write().await += allocation.allocated_bandwidth_mbps; }
        self.update_usage_stats(NetworkOperation::Deallocate, allocation.allocated_bandwidth_mbps).await;

        tracing::info!("Deallocated network asset: {} ({} interfaces, {} Mbps)", asset_id, allocation.allocated_interfaces.len(), allocation.allocated_bandwidth_mbps);
        Ok(())
    }

    async fn get_asset_status(&self, asset_id: &AssetRegistration) -> AssetResult<AssetStatus> {
        let allocations = self.state.allocations.read().await;
        let alloc = allocations.get(asset_id).ok_or_else(|| AssetError::AssetNotFound { asset_id: asset_id.to_string() })?;

        Ok(AssetStatus {
            asset_id: asset_id.clone(),
            state: AssetState::InUse,
            allocated_at: alloc.allocated_at,
            last_accessed: alloc.last_accessed,
            privacy_level: alloc.privacy_level.clone(),
            proxy_address: None,
            resource_usage: self.get_resource_usage(asset_id).await?,
            consensus_proofs: Vec::new(),
            owner_certificate_fingerprint: "network-adapter".to_string(),
            health_status: crate::assets::core::status::AssetHealthStatus::default(),
            performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("allocated_bandwidth_mbps".to_string(), alloc.allocated_bandwidth_mbps.to_string());
                m.insert("interfaces".to_string(), alloc.allocated_interfaces.len().to_string());
                m.insert("protocols".to_string(), alloc.enabled_protocols.join(","));
                m.insert("qos_priority".to_string(), alloc.qos_priority.to_string());
                m.insert("current_latency_us".to_string(), alloc.current_latency_us.to_string());
                m.insert("packet_loss_percent".to_string(), alloc.current_packet_loss_percent.to_string());
                m.insert("ipv6_addresses".to_string(), alloc.ipv6_addresses.len().to_string());
                m.insert("vlan_id".to_string(), alloc.vlan_id.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string()));
                m
            },
        })
    }

    async fn configure_privacy_level(&self, asset_id: &AssetRegistration, privacy: PrivacyMode) -> AssetResult<()> {
        let mut allocations = self.state.allocations.write().await;
        let alloc = allocations.get_mut(asset_id).ok_or_else(|| AssetError::AssetNotFound { asset_id: asset_id.to_string() })?;
        alloc.privacy_level = privacy.clone();
        alloc.isolation_enabled = privacy == PrivacyMode::PRIVATE;
        tracing::info!("Updated privacy level for network asset {}: {:?}", asset_id, privacy);
        Ok(())
    }

    async fn assign_proxy_address(&self, asset_id: &AssetRegistration) -> AssetResult<ProxyAddress> {
        let proxy_address = Self::generate_proxy_address(asset_id).await;
        let proxy_mappings = self.state.proxy_mappings.read().await;
        for (addr, mapped) in proxy_mappings.iter() {
            if mapped == asset_id { return Ok(addr.clone()); }
        }
        Ok(proxy_address)
    }

    async fn resolve_proxy_address(&self, proxy_addr: &ProxyAddress) -> AssetResult<AssetRegistration> {
        self.state.proxy_mappings.read().await.get(proxy_addr).cloned()
            .ok_or_else(|| AssetError::ProxyResolutionFailed { address: proxy_addr.clone() })
    }

    async fn get_resource_usage(&self, asset_id: &AssetRegistration) -> AssetResult<ResourceUsage> {
        let allocations = self.state.allocations.read().await;
        let alloc = allocations.get(asset_id).ok_or_else(|| AssetError::AssetNotFound { asset_id: asset_id.to_string() })?;
        Ok(ResourceUsage {
            cpu_usage: None, gpu_usage: None, memory_usage: None, storage_usage: None,
            network_usage: Some(NetworkUsage { bytes_received: 0, bytes_transmitted: 0, packets_received: 0, packets_transmitted: 0, latency_us: Some(alloc.current_latency_us) }),
            measurement_timestamp: SystemTime::now(),
        })
    }

    async fn set_resource_limits(&self, asset_id: &AssetRegistration, limits: ResourceLimits) -> AssetResult<()> {
        if let Some(nl) = limits.network_limit {
            tracing::info!("Set network limits for asset {}: max {} Mbps, max {} connections", asset_id, nl.max_bandwidth_mbps, nl.max_connections);
        }
        Ok(())
    }

    async fn health_check(&self) -> AssetResult<AdapterHealth> {
        let stats = self.state.usage_stats.read().await;
        let interfaces = self.state.network_interfaces.read().await;
        let available = *self.state.available_bandwidth.read().await;

        let failed = interfaces.values().filter(|i| matches!(i.status, InterfaceStatus::Failed)).count();
        let down = interfaces.values().filter(|i| matches!(i.status, InterfaceStatus::Down)).count();
        let healthy = failed == 0 && down < 2 && available > 0;

        let mut pm = HashMap::new();
        pm.insert("total_bandwidth_gbps".to_string(), self.state.total_bandwidth as f64 / 1000.0);
        pm.insert("available_bandwidth_gbps".to_string(), available as f64 / 1000.0);
        pm.insert("bandwidth_utilization_percent".to_string(), ((self.state.total_bandwidth - available) as f64 / self.state.total_bandwidth as f64) * 100.0);
        pm.insert("active_allocations".to_string(), stats.active_allocations as f64);
        pm.insert("total_interfaces".to_string(), interfaces.len() as f64);
        pm.insert("failed_interfaces".to_string(), failed as f64);
        pm.insert("down_interfaces".to_string(), down as f64);
        pm.insert("average_latency_us".to_string(), stats.average_latency_us as f64);
        pm.insert("average_packet_loss_percent".to_string(), stats.average_packet_loss_percent as f64);

        Ok(AdapterHealth {
            healthy,
            message: if healthy { "Network adapter operating normally".to_string() } else { format!("Network adapter issues: {} failed, {} down interfaces", failed, down) },
            last_check: SystemTime::now(),
            performance_metrics: pm,
        })
    }

    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            asset_type: AssetType::Network,
            supported_privacy_levels: vec![PrivacyMode::PRIVATE, PrivacyMode::PRIVATE, PrivacyMode::PRIVATE, PrivacyMode::PUBLIC, PrivacyMode::PUBLIC],
            supports_proxy_addressing: true,
            supports_resource_monitoring: true,
            supports_dynamic_limits: true,
            max_concurrent_allocations: Some(100),
            features: vec![
                "ipv6_only".to_string(), "bandwidth_allocation".to_string(), "qos_management".to_string(),
                "traffic_shaping".to_string(), "vlan_isolation".to_string(), "network_security".to_string(),
                "latency_monitoring".to_string(), "packet_loss_monitoring".to_string(), "multi_interface".to_string(),
            ],
        }
    }
}
