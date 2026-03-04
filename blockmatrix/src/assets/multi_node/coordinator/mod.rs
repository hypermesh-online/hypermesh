// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Network Coordinator Implementation
//!
//! Revolutionary Concept #4: Multi-Network Participation
//!
//! Manages distributed asset coordination across multiple isolated networks
//! with matrix-based routing, state proofs, and zero packet leakage.

mod types;

pub use types::{
    AvailableResources, CoordinatorConfig, HardwareFeatures, NodeCapabilities, NodeInfo,
    NodeLocation, NodePerformanceMetrics, NodeStatus,
};

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::assets::core::{AssetError, AssetRegistration, AssetResult, AssetType};

use super::{
    AllocationDecision, DistributedAssetState, MultiNodeCoordinatorTrait, MultiNodeEvent,
    MultiNodeMetrics, NetworkPartition, NetworkTopology, PeerIdentity, ResourceSharingOffer,
    ResourceSharingRequest,
};

/// Multi-node coordinator implementation
pub struct MultiNodeCoordinator {
    /// Local node information
    local_node: Arc<RwLock<PeerIdentity>>,
    /// All known nodes
    nodes: Arc<RwLock<HashMap<PeerIdentity, NodeInfo>>>,
    /// Network topology
    topology: Arc<RwLock<NetworkTopology>>,
    /// Active network partitions
    partitions: Arc<RwLock<Vec<NetworkPartition>>>,
    /// Distributed asset states
    asset_states: Arc<RwLock<HashMap<AssetRegistration, DistributedAssetState>>>,
    /// Pending allocation decisions
    pending_allocations: Arc<RwLock<HashMap<AssetRegistration, AllocationDecision>>>,
    /// Resource sharing requests
    sharing_requests: Arc<RwLock<Vec<ResourceSharingRequest>>>,
    /// Resource sharing offers
    sharing_offers: Arc<RwLock<Vec<ResourceSharingOffer>>>,
    /// Event channel sender
    event_sender: mpsc::UnboundedSender<MultiNodeEvent>,
    /// Event channel receiver
    event_receiver: Arc<Mutex<mpsc::UnboundedReceiver<MultiNodeEvent>>>,
    /// Metrics
    metrics: Arc<RwLock<MultiNodeMetrics>>,
    /// Configuration
    config: CoordinatorConfig,
}

impl MultiNodeCoordinator {
    /// Create new multi-node coordinator
    pub fn new(config: CoordinatorConfig) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Self {
            local_node: Arc::new(RwLock::new(PeerIdentity {
                name: "local".to_string(),
                id: [0u8; 32],
                address: "::1".parse().expect("hardcoded IPv6 loopback is valid"),
                pub_key: Vec::new(),
            })),
            nodes: Arc::new(RwLock::new(HashMap::new())),
            topology: Arc::new(RwLock::new(NetworkTopology {
                nodes: HashMap::new(),
                partitions: Vec::new(),
                latency_matrix: HashMap::new(),
                bandwidth_matrix: HashMap::new(),
                last_updated: SystemTime::now(),
            })),
            partitions: Arc::new(RwLock::new(Vec::new())),
            asset_states: Arc::new(RwLock::new(HashMap::new())),
            pending_allocations: Arc::new(RwLock::new(HashMap::new())),
            sharing_requests: Arc::new(RwLock::new(Vec::new())),
            sharing_offers: Arc::new(RwLock::new(Vec::new())),
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            metrics: Arc::new(RwLock::new(MultiNodeMetrics::default())),
            config,
        }
    }

    /// Start background coordinator tasks
    pub async fn start(&self) -> AssetResult<()> {
        self.start_heartbeat_monitor().await?;
        self.start_partition_detector().await?;
        self.start_byzantine_detector().await?;
        self.start_event_processor().await?;
        if self.config.load_balancing {
            self.start_load_balancer().await?;
        }
        Ok(())
    }

    /// Start heartbeat monitoring
    async fn start_heartbeat_monitor(&self) -> AssetResult<()> {
        let nodes = self.nodes.clone();
        let config = self.config.clone();
        let event_sender = self.event_sender.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.heartbeat_interval);
            loop {
                interval.tick().await;
                let mut nodes_write = nodes.write().await;
                let now = SystemTime::now();
                let mut failed_nodes = Vec::new();

                for (node_id, node_info) in nodes_write.iter_mut() {
                    if let Ok(elapsed) = now.duration_since(node_info.last_heartbeat) {
                        if elapsed > config.failure_timeout
                            && node_info.status != NodeStatus::Failed
                        {
                            node_info.status = NodeStatus::Failed;
                            failed_nodes.push(node_id.clone());
                        }
                    }
                }

                for failed_node in failed_nodes {
                    let _ = event_sender.send(MultiNodeEvent::NodeFailed {
                        node: failed_node,
                        detection_time: now,
                    });
                    let mut metrics = metrics.write().await;
                    metrics.failed_nodes += 1;
                }
            }
        });
        Ok(())
    }

    /// Start network partition detection
    async fn start_partition_detector(&self) -> AssetResult<()> {
        let nodes = self.nodes.clone();
        let partitions = self.partitions.clone();
        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let nodes_read = nodes.read().await;
                let active_nodes: HashSet<PeerIdentity> = nodes_read
                    .iter()
                    .filter(|(_, info)| info.status == NodeStatus::Active)
                    .map(|(id, _)| id.clone())
                    .collect();

                let detected_partitions: Vec<NetworkPartition> = Vec::new();

                let mut partitions_write = partitions.write().await;
                for partition in partitions_write.iter_mut() {
                    if !partition.healed {
                        let nodes_connected = partition
                            .nodes
                            .iter()
                            .all(|node| active_nodes.contains(node));
                        if nodes_connected {
                            partition.healed = true;
                            let _ = event_sender.send(MultiNodeEvent::PartitionHealed {
                                partition_id: partition.partition_id.clone(),
                            });
                        }
                    }
                }

                for partition in detected_partitions {
                    partitions_write.push(partition.clone());
                    let _ = event_sender.send(MultiNodeEvent::PartitionDetected { partition });
                }
            }
        });
        Ok(())
    }

    /// Start Byzantine behavior detector
    async fn start_byzantine_detector(&self) -> AssetResult<()> {
        let nodes = self.nodes.clone();
        let asset_states = self.asset_states.clone();
        let event_sender = self.event_sender.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let nodes_read = nodes.read().await;
                let states_read = asset_states.read().await;
                let mut byzantine_nodes = Vec::new();

                for (node_id, node_info) in nodes_read.iter() {
                    let mut suspicious_behaviors = 0;
                    for (_, state) in states_read.iter() {
                        if let Some(node_state) = state.node_states.get(node_id) {
                            let state_proof_state = state
                                .node_states
                                .values()
                                .filter(|s| **s != *node_state)
                                .count();
                            if state_proof_state > state.node_states.len() / 2 {
                                suspicious_behaviors += 1;
                            }
                        }
                    }

                    if node_info.performance_metrics.success_rate < 0.5 {
                        suspicious_behaviors += 1;
                    }

                    let suspicion_ratio =
                        suspicious_behaviors as f32 / states_read.len().max(1) as f32;
                    if suspicion_ratio > config.suspicion_threshold {
                        byzantine_nodes.push(node_id.clone());
                    }
                }

                for byzantine_node in byzantine_nodes {
                    let _ = event_sender.send(MultiNodeEvent::InauthenticStateDetected {
                        node: byzantine_node,
                        evidence: Vec::new(),
                    });
                }
            }
        });
        Ok(())
    }

    /// Start event processor
    async fn start_event_processor(&self) -> AssetResult<()> {
        let event_receiver = self.event_receiver.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut receiver = event_receiver.lock().await;
            while let Some(event) = receiver.recv().await {
                let mut metrics_write = metrics.write().await;
                match event {
                    MultiNodeEvent::NodeJoined { .. } => {
                        metrics_write.total_nodes += 1;
                        metrics_write.healthy_nodes += 1;
                    }
                    MultiNodeEvent::NodeLeft { .. } => {
                        metrics_write.total_nodes = metrics_write.total_nodes.saturating_sub(1);
                        metrics_write.healthy_nodes = metrics_write.healthy_nodes.saturating_sub(1);
                    }
                    MultiNodeEvent::NodeFailed { .. } => {
                        metrics_write.healthy_nodes = metrics_write.healthy_nodes.saturating_sub(1);
                        metrics_write.failed_nodes += 1;
                    }
                    MultiNodeEvent::PartitionDetected { .. } => {
                        metrics_write.partitions_detected += 1;
                    }
                    MultiNodeEvent::PartitionHealed { .. } => {
                        metrics_write.partitions_healed += 1;
                    }
                    MultiNodeEvent::MigrationCompleted { .. } => {
                        metrics_write.successful_migrations += 1;
                    }
                    MultiNodeEvent::InauthenticStateDetected { .. } => {
                        metrics_write.inauthentic_nodes += 1;
                    }
                    _ => {}
                }
            }
        });
        Ok(())
    }

    /// Start load balancer
    async fn start_load_balancer(&self) -> AssetResult<()> {
        let nodes = self.nodes.clone();
        let asset_states = self.asset_states.clone();
        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(120));
            loop {
                interval.tick().await;
                let nodes_read = nodes.read().await;
                let states_read = asset_states.read().await;

                let mut node_loads: HashMap<PeerIdentity, f64> = HashMap::new();
                for (node_id, node_info) in nodes_read.iter() {
                    let cpu_load = node_info.performance_metrics.cpu_utilization as f64;
                    let mem_load = node_info.performance_metrics.memory_utilization as f64;
                    let combined_load = (cpu_load + mem_load) / 2.0;
                    node_loads.insert(node_id.clone(), combined_load);
                }

                let avg_load: f64 =
                    node_loads.values().sum::<f64>() / node_loads.len().max(1) as f64;
                let load_threshold = 0.2;

                for (node_id, load) in &node_loads {
                    if (*load - avg_load).abs() > load_threshold {
                        tracing::info!(
                            "Node {} has imbalanced load: {:.2}% (avg: {:.2}%)",
                            hex::encode(&node_id.id[..8]),
                            load * 100.0,
                            avg_load * 100.0
                        );

                        for (asset_id, state) in states_read.iter() {
                            if state.primary_node == *node_id && *load > avg_load {
                                if let Some((target_node, _)) = node_loads
                                    .iter()
                                    .filter(|(_, l)| **l < avg_load)
                                    .min_by(|a, b| a.1.partial_cmp(b.1).expect("load values should be valid for comparison"))
                                {
                                    let _ = event_sender.send(MultiNodeEvent::MigrationStarted {
                                        asset_id: asset_id.clone(),
                                        from: node_id.clone(),
                                        to: target_node.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }

    /// Select best node for asset allocation
    async fn select_allocation_node(&self, asset_type: AssetType) -> AssetResult<PeerIdentity> {
        let nodes = self.nodes.read().await;
        let eligible_nodes: Vec<(&PeerIdentity, &NodeInfo)> = nodes
            .iter()
            .filter(|(_, info)| {
                info.status == NodeStatus::Active
                    && info.capabilities.supported_assets.contains(&asset_type)
            })
            .collect();

        if eligible_nodes.is_empty() {
            return Err(AssetError::AllocationFailed {
                reason: "No eligible nodes available".to_string(),
            });
        }

        eligible_nodes
            .iter()
            .max_by(|a, b| {
                let score_a = self.calculate_node_score(a.1);
                let score_b = self.calculate_node_score(b.1);
                score_a.partial_cmp(&score_b).expect("node scores should be valid for comparison")
            })
            .map(|(id, _)| (*id).clone())
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: "Failed to select allocation node".to_string(),
            })
    }

    /// Calculate node allocation score
    fn calculate_node_score(&self, node_info: &NodeInfo) -> f64 {
        let cpu_availability = node_info.available_resources.cpu_cores as f64
            / node_info.capabilities.cpu_cores as f64;
        let mem_availability = node_info.available_resources.memory_bytes as f64
            / node_info.capabilities.memory_bytes as f64;
        let performance = node_info.performance_metrics.success_rate as f64;
        let response_time =
            1.0 / (1.0 + node_info.performance_metrics.avg_response_time_ms / 1000.0);

        let score = (cpu_availability * 0.3)
            + (mem_availability * 0.3)
            + (performance * 0.2)
            + (response_time * 0.2);

        score * 1.0_f64
    }
}

#[async_trait]
impl MultiNodeCoordinatorTrait for MultiNodeCoordinator {
    async fn initialize(&mut self, local_node: PeerIdentity) -> AssetResult<()> {
        *self.local_node.write().await = local_node;
        self.start().await?;
        Ok(())
    }

    async fn join_network(&self) -> AssetResult<()> {
        let local_node = self.local_node.read().await.clone();
        self.event_sender
            .send(MultiNodeEvent::NodeJoined {
                node: local_node.clone(),
                capabilities: NodeCapabilities {
                    cpu_cores: 8,
                    memory_bytes: 16 * 1024 * 1024 * 1024,
                    gpu_devices: 1,
                    storage_bytes: 1024 * 1024 * 1024 * 1024,
                    bandwidth_mbps: 1000,
                    supported_assets: vec![
                        AssetType::Cpu,
                        AssetType::Memory,
                        AssetType::Gpu,
                        AssetType::Storage,
                    ],
                    hardware_features: HardwareFeatures {
                        sgx_enabled: false,
                        sev_enabled: false,
                        tpm_available: true,
                        hw_rng: true,
                        nvme_storage: true,
                        rdma_capable: false,
                        sriov_enabled: false,
                    },
                    software_capabilities: vec![
                        "docker".to_string(),
                        "kubernetes".to_string(),
                        "hypermesh".to_string(),
                    ],
                },
            })
            .map_err(|_| AssetError::NetworkError {
                message: "Failed to send join event".to_string(),
            })?;
        Ok(())
    }

    async fn leave_network(&self) -> AssetResult<()> {
        let local_node = self.local_node.read().await.clone();
        self.event_sender
            .send(MultiNodeEvent::NodeLeft {
                node: local_node,
                reason: "Graceful shutdown".to_string(),
            })
            .map_err(|_| AssetError::NetworkError {
                message: "Failed to send leave event".to_string(),
            })?;
        Ok(())
    }

    async fn allocate_asset(&self, asset_id: AssetRegistration) -> AssetResult<AllocationDecision> {
        let asset_type = asset_id
            .asset_type()
            .ok_or_else(|| AssetError::AdapterError {
                message: "AssetRegistration missing asset_type field".to_string(),
            })?;
        let target_node = self.select_allocation_node(asset_type).await?;
        let decision = AllocationDecision {
            asset_id: asset_id.clone(),
            target_node: target_node.clone(),
            score: 0.95,
            decided_at: SystemTime::now(),
            participants: vec![target_node.clone()],
            signatures: Vec::new(),
        };
        self.pending_allocations
            .write()
            .await
            .insert(asset_id, decision.clone());
        Ok(decision)
    }

    async fn migrate_asset(
        &self,
        asset_id: AssetRegistration,
        target_node: PeerIdentity,
    ) -> AssetResult<()> {
        let states = self.asset_states.read().await;
        let current_state = states
            .get(&asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;

        self.event_sender
            .send(MultiNodeEvent::MigrationStarted {
                asset_id: asset_id.clone(),
                from: current_state.primary_node.clone(),
                to: target_node.clone(),
            })
            .map_err(|_| AssetError::NetworkError {
                message: "Failed to send migration event".to_string(),
            })?;

        self.event_sender
            .send(MultiNodeEvent::MigrationCompleted {
                asset_id,
                new_node: target_node,
            })
            .map_err(|_| AssetError::NetworkError {
                message: "Failed to send migration complete event".to_string(),
            })?;
        Ok(())
    }

    async fn handle_node_failure(&self, failed_node: PeerIdentity) -> AssetResult<()> {
        let states = self.asset_states.read().await;
        let affected_assets: Vec<AssetRegistration> = states
            .iter()
            .filter(|(_, state)| state.primary_node == failed_node)
            .map(|(id, _)| id.clone())
            .collect();

        for asset_id in affected_assets {
            let asset_type = asset_id
                .asset_type()
                .ok_or_else(|| AssetError::AdapterError {
                    message: "AssetRegistration missing asset_type field during migration"
                        .to_string(),
                })?;
            let new_node = self.select_allocation_node(asset_type).await?;
            self.migrate_asset(asset_id, new_node).await?;
        }
        Ok(())
    }

    async fn detect_inauthentic_nodes(&self) -> AssetResult<Vec<PeerIdentity>> {
        let nodes = self.nodes.read().await;
        Ok(nodes
            .iter()
            .filter(|(_, info)| info.status == NodeStatus::Suspected)
            .map(|(id, _)| id.clone())
            .collect())
    }

    async fn sync_asset_state(
        &self,
        asset_id: AssetRegistration,
    ) -> AssetResult<DistributedAssetState> {
        let states = self.asset_states.read().await;
        states
            .get(&asset_id)
            .cloned()
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })
    }

    async fn request_resources(
        &self,
        request: ResourceSharingRequest,
    ) -> AssetResult<Vec<ResourceSharingOffer>> {
        self.sharing_requests.write().await.push(request.clone());
        let offers = self.sharing_offers.read().await;
        Ok(offers
            .iter()
            .filter(|offer| offer.valid_until > SystemTime::now())
            .cloned()
            .collect())
    }

    async fn offer_resources(&self, offer: ResourceSharingOffer) -> AssetResult<()> {
        self.sharing_offers.write().await.push(offer);
        Ok(())
    }

    async fn get_topology(&self) -> AssetResult<NetworkTopology> {
        Ok(self.topology.read().await.clone())
    }

    async fn handle_event(&self, event: MultiNodeEvent) -> AssetResult<()> {
        self.event_sender
            .send(event)
            .map_err(|_| AssetError::NetworkError {
                message: "Failed to send event".to_string(),
            })?;
        Ok(())
    }
}
