// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Container asset adapter implementation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::assets::core::{
    AssetAdapter, AssetId, AssetType, AssetResult, AssetError,
    AssetAllocationRequest, AssetStatus, AssetState,
    PrivacyLevel, AssetAllocation, ProxyAddress,
    ResourceUsage, ResourceLimits,
    AdapterHealth, AdapterCapabilities, ConsensusProof,
    ContainerRequirements, PortMapping, VolumeMount,
    NetworkScope, AssetCategory, BaseSystemType, AssetData,
};
use super::types::*;

/// Container Asset Adapter implementation
#[allow(dead_code)]
pub struct ContainerAssetAdapter {
    allocations: Arc<RwLock<HashMap<AssetId, ContainerAllocation>>>,
    runtime: Arc<ContainerRuntime>,
    proxy_mappings: Arc<RwLock<HashMap<ProxyAddress, AssetId>>>,
    allocated_ports: Arc<RwLock<HashMap<u16, AssetId>>>,
    image_registry: Arc<RwLock<HashMap<String, ImageInfo>>>,
    usage_stats: Arc<RwLock<ContainerUsageStats>>,
}

impl ContainerAssetAdapter {
    /// Create new container adapter
    pub async fn new() -> Self {
        let runtime = Arc::new(Self::detect_container_runtime().await);
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            runtime,
            proxy_mappings: Arc::new(RwLock::new(HashMap::new())),
            allocated_ports: Arc::new(RwLock::new(HashMap::new())),
            image_registry: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(ContainerUsageStats::default())),
        }
    }

    async fn detect_container_runtime() -> ContainerRuntime {
        ContainerRuntime {
            runtime_type: RuntimeType::Docker,
            socket_path: "/var/run/docker.sock".to_string(),
            api_version: "1.41".to_string(),
        }
    }

    async fn generate_container_name(&self, asset_id: &AssetId) -> String {
        format!("hypermesh-{}", &hex::encode(&asset_id.content_hash[..4]))
    }

    async fn create_container(
        &self, container_req: &ContainerRequirements, asset_id: &AssetId,
    ) -> AssetResult<String> {
        let container_id = format!("container_{}", hex::encode(&asset_id.content_hash[..8]));
        tracing::info!(
            "Creating container {} with image {} for asset {}",
            container_id, container_req.image, asset_id
        );
        Ok(container_id)
    }

    async fn allocate_ports(
        &self, port_mappings: &[PortMapping], asset_id: &AssetId,
    ) -> AssetResult<Vec<ContainerPortMapping>> {
        let mut allocated_ports = self.allocated_ports.write().await;
        let mut container_ports = Vec::new();

        for port_mapping in port_mappings {
            let host_port = if let Some(requested_port) = port_mapping.host_port {
                if allocated_ports.contains_key(&requested_port) {
                    return Err(AssetError::AllocationFailed {
                        reason: format!("Port {} already allocated", requested_port),
                    });
                }
                requested_port
            } else {
                let mut port = 30000;
                while allocated_ports.contains_key(&port) && port < 65535 { port += 1; }
                if port >= 65535 {
                    return Err(AssetError::AllocationFailed {
                        reason: "No available ports".to_string(),
                    });
                }
                port
            };

            allocated_ports.insert(host_port, asset_id.clone());
            container_ports.push(ContainerPortMapping {
                container_port: port_mapping.container_port,
                host_port,
                protocol: port_mapping.protocol.clone(),
                bind_address: Some("::".to_string()),
            });
        }
        Ok(container_ports)
    }

    async fn configure_volumes(&self, volume_mounts: &[VolumeMount]) -> Vec<ContainerVolume> {
        volume_mounts.iter().map(|vm| ContainerVolume {
            name: format!("vol-{}", uuid::Uuid::new_v4()),
            host_path: vm.source.clone(),
            container_path: vm.target.clone(),
            read_only: vm.read_only,
            volume_type: VolumeType::HostPath,
            size_limit_bytes: None,
        }).collect()
    }

    async fn generate_proxy_address(asset_id: &AssetId) -> ProxyAddress {
        let mut node_id = [0u8; 8];
        node_id.copy_from_slice(&asset_id.content_hash[..8]);
        ProxyAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad,
             0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
            node_id, 8080,
        )
    }

    async fn get_container_stats(&self, _container_id: &str) -> ContainerRuntimeStats {
        ContainerRuntimeStats {
            cpu_usage_percent: 5.0,
            memory_usage_bytes: 100 * 1024 * 1024,
            network_io: NetworkIoStats {
                rx_bytes: 1024 * 1024, tx_bytes: 512 * 1024,
                rx_packets: 1000, tx_packets: 800,
            },
            block_io: BlockIoStats {
                read_bytes: 10 * 1024 * 1024, write_bytes: 5 * 1024 * 1024,
                read_ops: 100, write_ops: 50,
            },
            process_count: 3, uptime_seconds: 3600,
        }
    }

    async fn update_usage_stats(&self, operation: ContainerOperation) {
        let mut stats = self.usage_stats.write().await;
        match operation {
            ContainerOperation::Create => {
                stats.total_allocations += 1;
                stats.active_containers += 1;
            },
            ContainerOperation::Destroy => {
                stats.total_deallocations += 1;
                stats.active_containers = stats.active_containers.saturating_sub(1);
            },
            ContainerOperation::Restart => { stats.container_restarts += 1; },
        }
    }
}

#[async_trait]
impl AssetAdapter for ContainerAssetAdapter {
    fn asset_type(&self) -> AssetType { AssetType::Container }

    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> AssetResult<bool> {
        if proof.space_proof.total_size == 0 { return Ok(false); }
        if proof.stake_proof.stake_amount < 50 { return Ok(false); }
        if proof.work_proof.computational_power < 30 { return Ok(false); }
        if proof.time_proof.network_time_offset > Duration::from_secs(15) { return Ok(false); }
        Ok(true)
    }

    async fn allocate_asset(&self, request: &AssetAllocationRequest) -> AssetResult<AssetAllocation> {
        if !self.validate_consensus_proof(&request.consensus_proof).await? {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Container allocation consensus validation failed".to_string(),
            });
        }

        let container_req = request.requested_resources.container.as_ref()
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: "No container requirements specified".to_string(),
            })?;

        let data = AssetData {
            config: vec![1, 2, 3], definition: vec![4, 5, 6], metadata: vec![7, 8, 9],
        };
        let asset_id = AssetId::from_asset_data(
            &data, NetworkScope::Global, AssetCategory::BaseSystem(BaseSystemType::Container),
        );

        let container_name = self.generate_container_name(&asset_id).await;
        let container_id = self.create_container(container_req, &asset_id).await?;
        let port_mappings = self.allocate_ports(&container_req.ports, &asset_id).await?;
        let volumes = self.configure_volumes(&container_req.volumes).await;
        let proxy_address = Self::generate_proxy_address(&asset_id).await;

        let cpu_allocation = ContainerCpuAllocation {
            cpu_limit: container_req.cpu_limit,
            cpu_request: container_req.cpu_limit * 0.5,
            cpu_shares: (container_req.cpu_limit * 1024.0) as u32,
            pinned_cores: Vec::new(),
        };

        let memory_allocation = ContainerMemoryAllocation {
            memory_limit_bytes: container_req.memory_limit_bytes,
            memory_request_bytes: container_req.memory_limit_bytes / 2,
            swap_limit_bytes: container_req.memory_limit_bytes,
            oom_kill_disabled: false,
        };

        let network_config = ContainerNetworkConfig {
            network_mode: NetworkMode::Bridge,
            port_mappings,
            ipv6_addresses: vec![{
                let hash_as_u128 = u128::from_le_bytes(asset_id.content_hash[..16].try_into().map_err(|_| AssetError::AllocationFailed {
                    reason: "Content hash too short for IPv6 address generation".to_string(),
                })?);
                format!("2001:db8:hypermesh:container::{:x}", hash_as_u128 & 0xFFFF)
            }],
            network_aliases: vec![container_name.clone()],
            dns_config: DnsConfig {
                nameservers: vec!["2001:4860:4860::8888".to_string()],
                search_domains: vec!["hypermesh.local".to_string()],
                options: vec!["ndots:2".to_string()],
            },
            bandwidth_limits: BandwidthLimits { ingress_mbps: None, egress_mbps: None },
        };

        let security_config = ContainerSecurityConfig {
            user_id: Some(1000), group_id: Some(1000),
            privileged: false, read_only_rootfs: false,
            capabilities: SecurityCapabilities {
                add: Vec::new(), drop: vec!["ALL".to_string()],
            },
            security_labels: HashMap::new(),
            seccomp_profile: Some("default".to_string()),
        };

        let runtime_stats = self.get_container_stats(&container_id).await;

        let allocation = ContainerAllocation {
            asset_id: asset_id.clone(), container_id, image: container_req.image.clone(),
            container_name, cpu_allocation, memory_allocation, volumes, network_config,
            environment: container_req.environment.clone(),
            command: None, working_directory: None,
            container_status: ContainerStatus::Created, security_config,
            privacy_level: request.privacy_level.clone(),
            allocated_at: SystemTime::now(), last_accessed: SystemTime::now(), runtime_stats,
        };

        { self.allocations.write().await.insert(asset_id.clone(), allocation); }
        { self.proxy_mappings.write().await.insert(proxy_address.clone(), asset_id.clone()); }
        self.update_usage_stats(ContainerOperation::Create).await;

        Ok(AssetAllocation {
            asset_id: asset_id.clone(),
            status: AssetStatus {
                asset_id: asset_id.clone(), state: AssetState::Allocated,
                allocated_at: SystemTime::now(), last_accessed: SystemTime::now(),
                resource_usage: ResourceUsage {
                    cpu_usage: None, gpu_usage: None, memory_usage: None,
                    storage_usage: None, network_usage: None,
                    measurement_timestamp: SystemTime::now(),
                },
                privacy_level: PrivacyLevel::PRIVATE, proxy_address: None,
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

    async fn deallocate_asset(&self, asset_id: &AssetId) -> AssetResult<()> {
        let allocation = {
            self.allocations.write().await.remove(asset_id)
                .ok_or_else(|| AssetError::AssetNotFound { asset_id: asset_id.to_string() })?
        };

        tracing::info!("Stopping and removing container {}", allocation.container_id);

        {
            let mut allocated_ports = self.allocated_ports.write().await;
            for port_mapping in &allocation.network_config.port_mappings {
                allocated_ports.remove(&port_mapping.host_port);
            }
        }

        {
            let mut proxy_mappings = self.proxy_mappings.write().await;
            proxy_mappings.retain(|_, mapped_asset_id| mapped_asset_id != asset_id);
        }

        self.update_usage_stats(ContainerOperation::Destroy).await;
        tracing::info!("Deallocated container asset: {} (container: {})", asset_id, allocation.container_id);
        Ok(())
    }

    async fn get_asset_status(&self, asset_id: &AssetId) -> AssetResult<AssetStatus> {
        let allocations = self.allocations.read().await;
        let allocation = allocations.get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound { asset_id: asset_id.to_string() })?;

        Ok(AssetStatus {
            asset_id: asset_id.clone(),
            state: match allocation.container_status {
                ContainerStatus::Running => AssetState::InUse,
                ContainerStatus::Created | ContainerStatus::Stopped => AssetState::Allocated,
                ContainerStatus::Failed(_) => AssetState::Failed,
                _ => AssetState::Available,
            },
            allocated_at: allocation.allocated_at,
            last_accessed: allocation.last_accessed,
            privacy_level: allocation.privacy_level.clone(),
            proxy_address: None,
            resource_usage: self.get_resource_usage(asset_id).await?,
            consensus_proofs: Vec::new(),
            owner_certificate_fingerprint: "container-adapter".to_string(),
            health_status: crate::assets::core::status::AssetHealthStatus::default(),
            performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("container_id".to_string(), allocation.container_id.clone());
                metadata.insert("container_name".to_string(), allocation.container_name.clone());
                metadata.insert("image".to_string(), allocation.image.clone());
                metadata.insert("status".to_string(), format!("{:?}", allocation.container_status));
                metadata.insert("cpu_limit".to_string(), allocation.cpu_allocation.cpu_limit.to_string());
                metadata.insert("memory_limit_bytes".to_string(), allocation.memory_allocation.memory_limit_bytes.to_string());
                metadata.insert("ports".to_string(), allocation.network_config.port_mappings.len().to_string());
                metadata.insert("uptime_seconds".to_string(), allocation.runtime_stats.uptime_seconds.to_string());
                metadata
            },
        })
    }

    async fn configure_privacy_level(&self, asset_id: &AssetId, privacy: PrivacyLevel) -> AssetResult<()> {
        let mut allocations = self.allocations.write().await;
        let allocation = allocations.get_mut(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound { asset_id: asset_id.to_string() })?;

        allocation.privacy_level = privacy.clone();
        if privacy == PrivacyLevel::PRIVATE {
            allocation.network_config.network_mode = NetworkMode::Custom("isolated".to_string());
        }
        tracing::info!("Updated privacy level for container asset {}: {:?}", asset_id, privacy);
        Ok(())
    }

    async fn assign_proxy_address(&self, asset_id: &AssetId) -> AssetResult<ProxyAddress> {
        let proxy_address = Self::generate_proxy_address(asset_id).await;
        let proxy_mappings = self.proxy_mappings.read().await;
        for (proxy_addr, mapped_asset_id) in proxy_mappings.iter() {
            if mapped_asset_id == asset_id { return Ok(proxy_addr.clone()); }
        }
        Ok(proxy_address)
    }

    async fn resolve_proxy_address(&self, proxy_addr: &ProxyAddress) -> AssetResult<AssetId> {
        let proxy_mappings = self.proxy_mappings.read().await;
        proxy_mappings.get(proxy_addr).cloned()
            .ok_or_else(|| AssetError::ProxyResolutionFailed { address: proxy_addr.clone() })
    }

    async fn get_resource_usage(&self, asset_id: &AssetId) -> AssetResult<ResourceUsage> {
        let allocations = self.allocations.read().await;
        let allocation = allocations.get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound { asset_id: asset_id.to_string() })?;

        let runtime_stats = self.get_container_stats(&allocation.container_id).await;

        Ok(ResourceUsage {
            cpu_usage: Some(crate::assets::core::CpuUsage {
                utilization_percent: runtime_stats.cpu_usage_percent,
                frequency_mhz: 2400,
                temperature_celsius: None,
                active_cores: allocation.cpu_allocation.pinned_cores.len() as u32,
            }),
            gpu_usage: None,
            memory_usage: Some(crate::assets::core::MemoryUsage {
                used_bytes: runtime_stats.memory_usage_bytes,
                total_bytes: allocation.memory_allocation.memory_limit_bytes,
                cached_bytes: 0, swap_used_bytes: 0,
            }),
            storage_usage: Some(crate::assets::core::StorageUsage {
                used_bytes: runtime_stats.block_io.read_bytes + runtime_stats.block_io.write_bytes,
                total_bytes: 0, read_iops: 0, write_iops: 0, read_mbps: 0.0, write_mbps: 0.0,
            }),
            network_usage: Some(crate::assets::core::NetworkUsage {
                bytes_received: runtime_stats.network_io.rx_bytes,
                bytes_transmitted: runtime_stats.network_io.tx_bytes,
                packets_received: runtime_stats.network_io.rx_packets,
                packets_transmitted: runtime_stats.network_io.tx_packets,
                latency_us: None,
            }),
            measurement_timestamp: SystemTime::now(),
        })
    }

    async fn set_resource_limits(&self, asset_id: &AssetId, limits: ResourceLimits) -> AssetResult<()> {
        tracing::info!("Set resource limits for container asset {}: {:?}", asset_id, limits);
        Ok(())
    }

    async fn health_check(&self) -> AssetResult<AdapterHealth> {
        let stats = self.usage_stats.read().await;
        let allocations = self.allocations.read().await;

        let failed_containers = allocations.values()
            .filter(|a| matches!(a.container_status, ContainerStatus::Failed(_)))
            .count();

        let healthy = failed_containers == 0 && stats.active_containers < 1000;
        let total_memory_allocated = allocations.values()
            .map(|a| a.memory_allocation.memory_limit_bytes).sum::<u64>();

        let mut performance_metrics = HashMap::new();
        performance_metrics.insert("active_containers".to_string(), stats.active_containers as f64);
        performance_metrics.insert("failed_containers".to_string(), failed_containers as f64);
        performance_metrics.insert("total_memory_allocated_gb".to_string(), (total_memory_allocated / (1024 * 1024 * 1024)) as f64);
        performance_metrics.insert("total_cpu_time_hours".to_string(), stats.total_cpu_time_seconds / 3600.0);
        performance_metrics.insert("container_restarts".to_string(), stats.container_restarts as f64);
        performance_metrics.insert("network_io_gb".to_string(), (stats.total_network_io_bytes / (1024 * 1024 * 1024)) as f64);

        Ok(AdapterHealth {
            healthy,
            message: if healthy {
                "Container adapter operating normally".to_string()
            } else {
                format!("Container adapter issues: {} failed containers", failed_containers)
            },
            last_check: SystemTime::now(),
            performance_metrics,
        })
    }

    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            asset_type: AssetType::Container,
            supported_privacy_levels: vec![
                PrivacyLevel::PRIVATE, PrivacyLevel::PRIVATE,
                PrivacyLevel::PRIVATE, PrivacyLevel::PUBLIC, PrivacyLevel::PUBLIC,
            ],
            supports_proxy_addressing: true,
            supports_resource_monitoring: true,
            supports_dynamic_limits: true,
            max_concurrent_allocations: Some(1000),
            features: vec![
                "container_orchestration".to_string(), "image_management".to_string(),
                "network_isolation".to_string(), "volume_management".to_string(),
                "security_controls".to_string(), "resource_limits".to_string(),
                "port_management".to_string(), "ipv6_networking".to_string(),
                "runtime_stats".to_string(), "lifecycle_management".to_string(),
            ],
        }
    }
}
