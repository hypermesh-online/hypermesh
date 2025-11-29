//! Execution Context - Container for consensus-aware execution state
//!
//! This module defines the execution context that carries all necessary
//! information for consensus-native VM execution, including proofs,
//! asset allocations, privacy settings, and blockchain context.

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::consensus::proof::ConsensusProof;
use crate::catalog::vm::{AssetAllocation, PrivacyConfig, AssetId};

/// Execution context containing all necessary state for consensus execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Execution identifier
    pub execution_id: String,
    /// Consensus proof for this execution
    pub consensus_proof: ConsensusProof,
    /// Programming language being executed
    pub language: String,
    /// Asset allocations available for this execution
    pub asset_allocations: HashMap<String, AssetAllocation>,
    /// Privacy settings and resource sharing configuration
    pub privacy_settings: PrivacyConfig,
    /// Blockchain integration context
    pub blockchain_context: BlockchainExecutionContext,
    /// P2P networking context for distributed execution
    pub p2p_context: P2PExecutionContext,
    /// Execution permissions
    pub permissions: ExecutionPermissions,
    /// Resource limits for this execution
    pub resource_limits: ResourceLimits,
    /// Execution priority and scheduling hints
    pub scheduling_info: SchedulingInfo,
}

/// Blockchain execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainExecutionContext {
    /// Current blockchain state hash
    pub state_hash: Option<[u8; 32]>,
    /// Block number at execution start
    pub block_number: Option<u64>,
    /// Gas limit for blockchain operations
    pub gas_limit: u64,
    /// Gas price for operations
    pub gas_price: u64,
    /// Available blockchain storage quota
    pub storage_quota: u64,
    /// Smart contract addresses available
    pub contract_addresses: HashMap<String, String>,
}

/// P2P execution context for distributed operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PExecutionContext {
    /// Current peer connections
    pub connected_peers: Vec<PeerInfo>,
    /// Available peer resources
    pub peer_resources: HashMap<String, PeerResourceInfo>,
    /// Network topology information
    pub network_topology: NetworkTopology,
    /// Trust scores for connected peers
    pub trust_scores: HashMap<String, f64>,
    /// Routing preferences
    pub routing_preferences: RoutingPreferences,
}

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer identifier
    pub peer_id: String,
    /// Peer network address
    pub address: String,
    /// Connection quality metrics
    pub connection_quality: ConnectionQuality,
    /// Capabilities offered by peer
    pub capabilities: Vec<String>,
    /// Last seen timestamp
    pub last_seen: std::time::SystemTime,
}

/// Peer resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerResourceInfo {
    /// Available CPU cores
    pub cpu_cores: f64,
    /// Available memory (bytes)
    pub memory_bytes: u64,
    /// Available storage (bytes)
    pub storage_bytes: u64,
    /// Network bandwidth (bytes/sec)
    pub bandwidth_bytes_per_sec: u64,
    /// Specializations (GPU, quantum, etc.)
    pub specializations: Vec<String>,
}

/// Connection quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionQuality {
    /// Latency in microseconds
    pub latency_micros: u64,
    /// Bandwidth in bytes/sec
    pub bandwidth_bytes_per_sec: u64,
    /// Packet loss percentage
    pub packet_loss_percentage: f64,
    /// Connection stability score (0-100)
    pub stability_score: u8,
}

/// Network topology information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    /// Network diameter (max hops between any two nodes)
    pub network_diameter: u32,
    /// Local cluster information
    pub local_cluster: Vec<String>,
    /// Regional nodes
    pub regional_nodes: HashMap<String, Vec<String>>,
    /// Global backbone nodes
    pub backbone_nodes: Vec<String>,
}

/// Routing preferences for P2P operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPreferences {
    /// Prefer low-latency routes
    pub prefer_low_latency: bool,
    /// Prefer high-bandwidth routes
    pub prefer_high_bandwidth: bool,
    /// Maximum acceptable latency (microseconds)
    pub max_latency_micros: u64,
    /// Minimum required bandwidth (bytes/sec)
    pub min_bandwidth_bytes_per_sec: u64,
    /// Geographic routing preferences
    pub geographic_preferences: Vec<String>,
}

/// Execution permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPermissions {
    /// Can read from blockchain
    pub blockchain_read: bool,
    /// Can write to blockchain
    pub blockchain_write: bool,
    /// Can access external networks
    pub network_access: bool,
    /// Can perform file system operations
    pub filesystem_access: bool,
    /// Can execute system commands
    pub system_command_access: bool,
    /// Can access cryptographic operations
    pub crypto_access: bool,
    /// Asset types that can be accessed
    pub allowed_asset_types: Vec<String>,
    /// Maximum asset allocation percentages
    pub max_asset_allocation: HashMap<String, f64>,
}

/// Resource limits for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU time (microseconds)
    pub max_cpu_time_micros: u64,
    /// Maximum memory usage (bytes)
    pub max_memory_bytes: u64,
    /// Maximum storage usage (bytes)
    pub max_storage_bytes: u64,
    /// Maximum network bandwidth (bytes/sec)
    pub max_network_bandwidth_bytes_per_sec: u64,
    /// Maximum execution duration (microseconds)
    pub max_execution_duration_micros: u64,
    /// Maximum number of operations
    pub max_operations: u64,
}

/// Scheduling information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingInfo {
    /// Execution priority (0-100, higher = more priority)
    pub priority: u8,
    /// Deadline for execution completion
    pub deadline: Option<std::time::SystemTime>,
    /// Can be preempted by higher priority tasks
    pub preemptible: bool,
    /// Affinity for specific resource types
    pub resource_affinity: HashMap<String, f64>,
    /// Scheduling policy preferences
    pub scheduling_policy: SchedulingPolicy,
}

/// Scheduling policy options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    /// First-come, first-served
    FIFO,
    /// Shortest job first
    SJF,
    /// Round-robin with time slicing
    RoundRobin { time_slice_micros: u64 },
    /// Priority-based scheduling
    Priority,
    /// Deadline-aware scheduling
    EarliestDeadlineFirst,
    /// Consensus-aware scheduling (prioritizes proof validation)
    ConsensusAware,
}

impl ExecutionContext {
    /// Create new execution context
    pub fn new(
        consensus_proof: ConsensusProof,
        language: String,
        asset_allocations: HashMap<String, AssetAllocation>,
        privacy_settings: PrivacyConfig,
    ) -> Self {
        Self {
            execution_id: Uuid::new_v4().to_string(),
            consensus_proof,
            language,
            asset_allocations,
            privacy_settings,
            blockchain_context: BlockchainExecutionContext::default(),
            p2p_context: P2PExecutionContext::default(),
            permissions: ExecutionPermissions::default(),
            resource_limits: ResourceLimits::default(),
            scheduling_info: SchedulingInfo::default(),
        }
    }
    
    /// Get asset ID from consensus proof
    pub fn asset_id(&self) -> AssetId {
        // In practice, this would be derived from the consensus proof or context
        Uuid::new_v4()
    }
    
    /// Get consensus proof
    pub fn consensus_proof(&self) -> &ConsensusProof {
        &self.consensus_proof
    }
    
    /// Get asset allocations
    pub fn asset_allocations(&self) -> &HashMap<String, AssetAllocation> {
        &self.asset_allocations
    }
    
    /// Get privacy settings
    pub fn privacy_settings(&self) -> &PrivacyConfig {
        &self.privacy_settings
    }
    
    /// Check if execution has permission for specific operation
    pub fn has_permission(&self, operation: &str) -> bool {
        match operation {
            "blockchain_read" => self.permissions.blockchain_read,
            "blockchain_write" => self.permissions.blockchain_write,
            "network_access" => self.permissions.network_access,
            "filesystem_access" => self.permissions.filesystem_access,
            "system_command" => self.permissions.system_command_access,
            "crypto" => self.permissions.crypto_access,
            _ => false,
        }
    }
    
    /// Check if resource limit allows operation
    pub fn check_resource_limit(&self, resource_type: &str, requested_amount: u64) -> bool {
        match resource_type {
            "cpu_time" => requested_amount <= self.resource_limits.max_cpu_time_micros,
            "memory" => requested_amount <= self.resource_limits.max_memory_bytes,
            "storage" => requested_amount <= self.resource_limits.max_storage_bytes,
            "network" => requested_amount <= self.resource_limits.max_network_bandwidth_bytes_per_sec,
            "operations" => requested_amount <= self.resource_limits.max_operations,
            _ => false,
        }
    }
    
    /// Get available peers for distributed execution
    pub fn get_available_peers(&self) -> Vec<&PeerInfo> {
        self.p2p_context.connected_peers.iter().collect()
    }
    
    /// Get peer trust score
    pub fn get_peer_trust_score(&self, peer_id: &str) -> Option<f64> {
        self.p2p_context.trust_scores.get(peer_id).copied()
    }
    
    /// Check if execution should use blockchain storage
    pub fn use_blockchain_storage(&self) -> bool {
        self.permissions.blockchain_write && 
        self.blockchain_context.storage_quota > 0
    }
    
    /// Get execution priority
    pub fn priority(&self) -> u8 {
        self.scheduling_info.priority
    }
    
    /// Check if execution is time-sensitive
    pub fn is_time_sensitive(&self) -> bool {
        self.scheduling_info.deadline.is_some() ||
        matches!(self.scheduling_info.scheduling_policy, 
                SchedulingPolicy::EarliestDeadlineFirst)
    }
    
    /// Clone context for distributed execution
    pub fn clone_for_peer(&self, peer_id: &str) -> Self {
        let mut cloned = self.clone();
        cloned.execution_id = format!("{}-peer-{}", self.execution_id, peer_id);
        // Adjust resource limits for peer execution
        cloned.resource_limits.max_memory_bytes /= 2; // Conservative allocation
        cloned.resource_limits.max_cpu_time_micros /= 2;
        cloned
    }
}

impl Default for BlockchainExecutionContext {
    fn default() -> Self {
        Self {
            state_hash: None,
            block_number: None,
            gas_limit: 1_000_000,
            gas_price: 20_000_000_000, // 20 Gwei
            storage_quota: 1024 * 1024, // 1MB
            contract_addresses: HashMap::new(),
        }
    }
}

impl Default for P2PExecutionContext {
    fn default() -> Self {
        Self {
            connected_peers: Vec::new(),
            peer_resources: HashMap::new(),
            network_topology: NetworkTopology::default(),
            trust_scores: HashMap::new(),
            routing_preferences: RoutingPreferences::default(),
        }
    }
}

impl Default for NetworkTopology {
    fn default() -> Self {
        Self {
            network_diameter: 6, // Small world assumption
            local_cluster: Vec::new(),
            regional_nodes: HashMap::new(),
            backbone_nodes: Vec::new(),
        }
    }
}

impl Default for RoutingPreferences {
    fn default() -> Self {
        Self {
            prefer_low_latency: true,
            prefer_high_bandwidth: false,
            max_latency_micros: 100_000, // 100ms
            min_bandwidth_bytes_per_sec: 1024 * 1024, // 1MB/s
            geographic_preferences: Vec::new(),
        }
    }
}

impl Default for ExecutionPermissions {
    fn default() -> Self {
        Self {
            blockchain_read: true,
            blockchain_write: false, // Conservative default
            network_access: false, // Conservative default
            filesystem_access: false, // Conservative default
            system_command_access: false, // Conservative default
            crypto_access: true,
            allowed_asset_types: vec!["cpu".to_string(), "memory".to_string()],
            max_asset_allocation: {
                let mut map = HashMap::new();
                map.insert("cpu".to_string(), 50.0);
                map.insert("memory".to_string(), 50.0);
                map
            },
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_time_micros: 30_000_000, // 30 seconds
            max_memory_bytes: 1024 * 1024 * 100, // 100MB
            max_storage_bytes: 1024 * 1024 * 1024, // 1GB
            max_network_bandwidth_bytes_per_sec: 1024 * 1024 * 10, // 10MB/s
            max_execution_duration_micros: 300_000_000, // 5 minutes
            max_operations: 1_000_000,
        }
    }
}

impl Default for SchedulingInfo {
    fn default() -> Self {
        Self {
            priority: 50, // Medium priority
            deadline: None,
            preemptible: true,
            resource_affinity: HashMap::new(),
            scheduling_policy: SchedulingPolicy::ConsensusAware,
        }
    }
}

/// Builder for creating execution contexts with specific configurations
pub struct ExecutionContextBuilder {
    consensus_proof: Option<ConsensusProof>,
    language: String,
    asset_allocations: HashMap<String, AssetAllocation>,
    privacy_settings: Option<PrivacyConfig>,
    blockchain_context: Option<BlockchainExecutionContext>,
    p2p_context: Option<P2PExecutionContext>,
    permissions: Option<ExecutionPermissions>,
    resource_limits: Option<ResourceLimits>,
    scheduling_info: Option<SchedulingInfo>,
}

impl ExecutionContextBuilder {
    /// Create new builder
    pub fn new(language: String) -> Self {
        Self {
            consensus_proof: None,
            language,
            asset_allocations: HashMap::new(),
            privacy_settings: None,
            blockchain_context: None,
            p2p_context: None,
            permissions: None,
            resource_limits: None,
            scheduling_info: None,
        }
    }
    
    /// Set consensus proof
    pub fn with_consensus_proof(mut self, proof: ConsensusProof) -> Self {
        self.consensus_proof = Some(proof);
        self
    }
    
    /// Add asset allocation
    pub fn with_asset_allocation(mut self, asset_type: String, allocation: AssetAllocation) -> Self {
        self.asset_allocations.insert(asset_type, allocation);
        self
    }
    
    /// Set privacy settings
    pub fn with_privacy_settings(mut self, settings: PrivacyConfig) -> Self {
        self.privacy_settings = Some(settings);
        self
    }
    
    /// Set execution priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        if self.scheduling_info.is_none() {
            self.scheduling_info = Some(SchedulingInfo::default());
        }
        self.scheduling_info.as_mut().unwrap().priority = priority;
        self
    }
    
    /// Set resource limit
    pub fn with_memory_limit(mut self, limit_bytes: u64) -> Self {
        if self.resource_limits.is_none() {
            self.resource_limits = Some(ResourceLimits::default());
        }
        self.resource_limits.as_mut().unwrap().max_memory_bytes = limit_bytes;
        self
    }
    
    /// Build the execution context
    pub fn build(self) -> Result<ExecutionContext, String> {
        let consensus_proof = self.consensus_proof
            .ok_or("Consensus proof is required")?;
        
        let privacy_settings = self.privacy_settings
            .unwrap_or_else(PrivacyConfig::default);
        
        let mut context = ExecutionContext::new(
            consensus_proof,
            self.language,
            self.asset_allocations,
            privacy_settings,
        );
        
        if let Some(blockchain_ctx) = self.blockchain_context {
            context.blockchain_context = blockchain_ctx;
        }
        
        if let Some(p2p_ctx) = self.p2p_context {
            context.p2p_context = p2p_ctx;
        }
        
        if let Some(permissions) = self.permissions {
            context.permissions = permissions;
        }
        
        if let Some(limits) = self.resource_limits {
            context.resource_limits = limits;
        }
        
        if let Some(scheduling) = self.scheduling_info {
            context.scheduling_info = scheduling;
        }
        
        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::proof::{ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime, ConsensusProof};
    
    #[test]
    fn test_execution_context_creation() {
        let consensus_proof = ConsensusProof::new(
            ProofOfSpace::default(),
            ProofOfStake::default(),
            ProofOfWork::default(),
            ProofOfTime::default(),
        );
        
        let context = ExecutionContext::new(
            consensus_proof,
            "julia".to_string(),
            HashMap::new(),
            PrivacyConfig::default(),
        );
        
        assert_eq!(context.language, "julia");
        assert!(!context.execution_id.is_empty());
    }
    
    #[test]
    fn test_execution_context_builder() {
        let consensus_proof = ConsensusProof::new(
            ProofOfSpace::default(),
            ProofOfStake::default(),
            ProofOfWork::default(),
            ProofOfTime::default(),
        );
        
        let context = ExecutionContextBuilder::new("python".to_string())
            .with_consensus_proof(consensus_proof)
            .with_priority(75)
            .with_memory_limit(1024 * 1024 * 50) // 50MB
            .build();
        
        assert!(context.is_ok());
        let ctx = context.unwrap();
        assert_eq!(ctx.language, "python");
        assert_eq!(ctx.scheduling_info.priority, 75);
        assert_eq!(ctx.resource_limits.max_memory_bytes, 1024 * 1024 * 50);
    }
    
    #[test]
    fn test_permission_checking() {
        let mut permissions = ExecutionPermissions::default();
        permissions.network_access = true;
        
        let context = ExecutionContext {
            permissions,
            ..ExecutionContext::new(
                ConsensusProof::new(
                    ProofOfSpace::default(),
                    ProofOfStake::default(),
                    ProofOfWork::default(),
                    ProofOfTime::default(),
                ),
                "test".to_string(),
                HashMap::new(),
                PrivacyConfig::default(),
            )
        };
        
        assert!(context.has_permission("network_access"));
        assert!(!context.has_permission("system_command"));
    }
}