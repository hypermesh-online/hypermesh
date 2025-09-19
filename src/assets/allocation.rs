//! Asset Allocation System for HyperMesh
//! 
//! Manages allocation of assets with consensus validation and resource tracking.
//! Integrates with the four-proof consensus system for secure allocation.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::config::AssetConfig;
use crate::assets::consensus::{FourProofConsensus, ConsensusProof};

/// Asset allocator for HyperMesh system
pub struct AssetAllocator {
    /// Configuration
    config: AssetConfig,
    
    /// Consensus system for allocation validation
    consensus: Arc<FourProofConsensus>,
    
    /// Active allocations
    allocations: Arc<RwLock<HashMap<AllocationId, Arc<AssetAllocation>>>>,
    
    /// Resource pools by asset type
    resource_pools: Arc<RwLock<HashMap<String, ResourcePool>>>,
    
    /// Allocation statistics
    stats: Arc<RwLock<AllocationStats>>,
}

/// Asset allocation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationRequest {
    /// Asset to allocate
    pub asset_id: String,
    
    /// Amount to allocate (in asset-specific units)
    pub amount: f64,
    
    /// Duration of allocation
    pub duration: Option<Duration>,
    
    /// Allocation priority
    pub priority: AllocationPriority,
    
    /// Requester information
    pub requester_id: String,
    pub requester_type: RequesterType,
    
    /// Allocation constraints
    pub constraints: AllocationConstraints,
    
    /// Performance requirements
    pub performance_requirements: Option<PerformanceRequirements>,
}

/// Asset allocation response
#[derive(Debug)]
pub struct AssetAllocation {
    /// Allocation ID
    pub id: AllocationId,
    
    /// Original request
    pub request: AllocationRequest,
    
    /// Allocation details
    pub allocated_amount: f64,
    pub allocation_efficiency: f64,
    
    /// Timing information
    pub allocated_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub last_used: Arc<RwLock<Option<SystemTime>>>,
    
    /// Allocation status (mutable)
    pub status: Arc<RwLock<AllocationStatus>>,
    
    /// Consensus proof for allocation
    pub consensus_proof: Option<ConsensusProof>,
    
    /// Resource tracking
    pub resource_usage: ResourceUsage,
    
    /// Performance metrics (mutable)
    pub performance_metrics: Arc<RwLock<Option<AllocationPerformanceMetrics>>>,
}

/// Allocation priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum AllocationPriority {
    Low,
    Normal,
    High,
    Critical,
    Emergency,
}

/// Requester types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequesterType {
    User,
    Service,
    Application,
    System,
    Vm,
    Container,
}

/// Allocation constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationConstraints {
    /// Maximum allocation amount
    pub max_amount: Option<f64>,
    
    /// Minimum allocation amount
    pub min_amount: Option<f64>,
    
    /// Exclusive allocation required
    pub exclusive: bool,
    
    /// Co-location preferences
    pub preferred_locations: Vec<String>,
    pub excluded_locations: Vec<String>,
    
    /// Resource isolation requirements
    pub isolation_level: IsolationLevel,
    
    /// Compatibility requirements
    pub compatible_with: Vec<String>,
    pub incompatible_with: Vec<String>,
}

/// Resource isolation levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    None,
    Soft,
    Hard,
    Secure,
}

/// Performance requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirements {
    /// Minimum throughput (units per second)
    pub min_throughput: Option<f64>,
    
    /// Maximum latency (milliseconds)
    pub max_latency_ms: Option<f64>,
    
    /// Availability requirement (percentage)
    pub availability: Option<f64>,
    
    /// Quality of Service level
    pub qos_level: QosLevel,
}

/// Quality of Service levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QosLevel {
    BestEffort,
    Guaranteed,
    Premium,
}

/// Allocation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AllocationStatus {
    Pending,
    Active,
    Suspended,
    Expired,
    Released,
    Failed,
}

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Current usage metrics
    pub current_usage: f64,
    pub peak_usage: f64,
    pub average_usage: f64,
    
    /// Usage over time
    pub usage_history: Vec<UsageDataPoint>,
    
    /// Resource efficiency
    pub efficiency_score: f64,
    pub utilization_rate: f64,
}

/// Usage data point for historical tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDataPoint {
    pub timestamp: SystemTime,
    pub usage_amount: f64,
    pub efficiency: f64,
}

/// Performance metrics for allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationPerformanceMetrics {
    /// Throughput metrics
    pub actual_throughput: f64,
    pub throughput_variance: f64,
    
    /// Latency metrics
    pub actual_latency_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    
    /// Availability metrics
    pub uptime_percentage: f64,
    pub downtime_incidents: u32,
    
    /// Performance score
    pub performance_score: f64,
}

/// Resource pool for asset type
#[derive(Debug, Clone)]
pub struct ResourcePool {
    /// Pool identification
    pub pool_id: String,
    pub asset_type: String,
    
    /// Pool capacity
    pub total_capacity: f64,
    pub available_capacity: f64,
    pub allocated_capacity: f64,
    pub reserved_capacity: f64,
    
    /// Pool performance
    pub pool_efficiency: f64,
    pub allocation_success_rate: f64,
    
    /// Active allocations in this pool
    pub active_allocations: Vec<AllocationId>,
    
    /// Pool statistics
    pub allocation_count: u64,
    pub peak_utilization: f64,
    pub avg_allocation_duration: Duration,
}

/// Allocation statistics
#[derive(Debug, Clone, Default)]
pub struct AllocationStats {
    pub total_allocations: u64,
    pub successful_allocations: u64,
    pub failed_allocations: u64,
    pub active_allocations: u64,
    pub avg_allocation_time_ms: f64,
    pub avg_allocation_duration: Duration,
    pub allocation_success_rate: f64,
    pub resource_utilization: f64,
    pub consensus_validations: u64,
}

/// Allocation ID type
pub type AllocationId = String;

impl Default for AllocationConstraints {
    fn default() -> Self {
        Self {
            max_amount: None,
            min_amount: None,
            exclusive: false,
            preferred_locations: Vec::new(),
            excluded_locations: Vec::new(),
            isolation_level: IsolationLevel::Soft,
            compatible_with: Vec::new(),
            incompatible_with: Vec::new(),
        }
    }
}

impl AssetAllocator {
    /// Create new asset allocator
    pub async fn new(
        config: &AssetConfig,
        consensus: Arc<FourProofConsensus>
    ) -> Result<Self> {
        info!("🏗️  Initializing Asset Allocator");
        info!("   Features: Consensus validation, Resource pooling, Performance tracking");
        
        Ok(Self {
            config: config.clone(),
            consensus,
            allocations: Arc::new(RwLock::new(HashMap::new())),
            resource_pools: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(AllocationStats::default())),
        })
    }
    
    /// Start asset allocator
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Asset Allocator");
        
        // Initialize resource pools
        self.initialize_resource_pools().await?;
        
        // Start allocation monitoring
        self.start_allocation_monitoring().await?;
        
        info!("✅ Asset Allocator started");
        info!("   Resource pools: {}", self.resource_pools.read().await.len());
        
        Ok(())
    }
    
    /// Initialize resource pools for different asset types
    async fn initialize_resource_pools(&self) -> Result<()> {
        let mut pools = self.resource_pools.write().await;
        
        // Create pools for common asset types
        let asset_types = vec!["cpu", "gpu", "memory", "storage", "network"];
        
        for asset_type in asset_types {
            let pool = ResourcePool {
                pool_id: format!("{}-pool", asset_type),
                asset_type: asset_type.to_string(),
                total_capacity: self.config.default_resource_capacity,
                available_capacity: self.config.default_resource_capacity,
                allocated_capacity: 0.0,
                reserved_capacity: 0.0,
                pool_efficiency: 1.0,
                allocation_success_rate: 1.0,
                active_allocations: Vec::new(),
                allocation_count: 0,
                peak_utilization: 0.0,
                avg_allocation_duration: Duration::from_secs(3600), // 1 hour default
            };
            
            pools.insert(asset_type.to_string(), pool);
        }
        
        Ok(())
    }
    
    /// Start allocation monitoring tasks
    async fn start_allocation_monitoring(&self) -> Result<()> {
        // Allocation expiration monitoring
        let allocations = self.allocations.clone();
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                let mut allocations_guard = allocations.write().await;
                let mut expired_allocations = Vec::new();
                
                for (allocation_id, allocation) in allocations_guard.iter() {
                    // Check for expired allocations
                    if let Some(expires_at) = allocation.expires_at {
                        if SystemTime::now() > expires_at {
                            // Mark as expired
                            if let Ok(mut status) = allocation.status.try_write() {
                                *status = AllocationStatus::Expired;
                            }
                            expired_allocations.push(allocation_id.clone());
                        }
                    }
                }
                
                // Clean up expired allocations
                for allocation_id in expired_allocations {
                    allocations_guard.remove(&allocation_id);
                    
                    let mut stats_guard = stats.write().await;
                    stats_guard.active_allocations = stats_guard.active_allocations.saturating_sub(1);
                }
            }
        });
        
        Ok(())
    }
    
    /// Allocate asset with consensus validation
    pub async fn allocate(&self, request: &AllocationRequest) -> Result<Arc<AssetAllocation>> {
        let start_time = Instant::now();
        
        info!("📋 Processing allocation request: {} units of {}", request.amount, request.asset_id);
        
        // Validate allocation request
        self.validate_allocation_request(request).await?;
        
        // Check resource availability
        let pool_available = self.check_resource_availability(&request.asset_id, request.amount).await?;
        if !pool_available {
            return Err(anyhow!("Insufficient resources available for allocation"));
        }
        
        // Generate allocation ID
        let allocation_id = format!("alloc-{}", Uuid::new_v4());
        
        // Validate allocation through consensus if mandatory
        let consensus_proof = if self.config.require_consensus_for_allocation {
            debug!("🔐 Validating allocation through consensus");
            let proofs = self.consensus.validate_asset_operation(
                &request.asset_id,
                "allocate",
                request
            ).await?;
            
            Some(proofs.into_iter().next().unwrap())
        } else {
            None
        };
        
        // Perform the allocation
        let allocation = self.perform_allocation(&allocation_id, request, consensus_proof.clone()).await?;
        
        // Update resource pools
        self.update_resource_pools(&request.asset_id, request.amount, true).await?;
        
        let allocation_time = start_time.elapsed();
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_allocations += 1;
            stats.successful_allocations += 1;
            stats.active_allocations += 1;
            
            if consensus_proof.is_some() {
                stats.consensus_validations += 1;
            }
            
            // Update average allocation time
            let total_time = stats.avg_allocation_time_ms * (stats.total_allocations - 1) as f64;
            stats.avg_allocation_time_ms = (total_time + allocation_time.as_millis() as f64) / stats.total_allocations as f64;
            
            // Update success rate
            stats.allocation_success_rate = stats.successful_allocations as f64 / stats.total_allocations as f64;
        }
        
        // Store allocation
        let allocation_arc = Arc::new(allocation);
        self.allocations.write().await.insert(allocation_id.clone(), allocation_arc.clone());
        
        info!("✅ Asset allocation completed: {} in {:?}", allocation_id, allocation_time);
        
        Ok(allocation_arc)
    }
    
    /// Validate allocation request
    async fn validate_allocation_request(&self, request: &AllocationRequest) -> Result<()> {
        // Basic validation
        if request.amount <= 0.0 {
            return Err(anyhow!("Allocation amount must be positive"));
        }
        
        if request.asset_id.is_empty() {
            return Err(anyhow!("Asset ID cannot be empty"));
        }
        
        if request.requester_id.is_empty() {
            return Err(anyhow!("Requester ID cannot be empty"));
        }
        
        // Constraint validation
        if let Some(max_amount) = request.constraints.max_amount {
            if request.amount > max_amount {
                return Err(anyhow!("Allocation amount exceeds maximum constraint"));
            }
        }
        
        if let Some(min_amount) = request.constraints.min_amount {
            if request.amount < min_amount {
                return Err(anyhow!("Allocation amount below minimum constraint"));
            }
        }
        
        Ok(())
    }
    
    /// Check resource availability in pools
    async fn check_resource_availability(&self, asset_id: &str, amount: f64) -> Result<bool> {
        let pools = self.resource_pools.read().await;
        
        // Find appropriate pool (simplified - assumes asset_id contains type)
        for (_, pool) in pools.iter() {
            if asset_id.contains(&pool.asset_type) {
                return Ok(pool.available_capacity >= amount);
            }
        }
        
        // Default to checking if any pool has capacity
        Ok(pools.values().any(|pool| pool.available_capacity >= amount))
    }
    
    /// Perform the actual allocation
    async fn perform_allocation(
        &self,
        allocation_id: &str,
        request: &AllocationRequest,
        consensus_proof: Option<ConsensusProof>
    ) -> Result<AssetAllocation> {
        let now = SystemTime::now();
        
        // Calculate expiration time
        let expires_at = request.duration.map(|duration| {
            now + duration
        });
        
        // Initialize resource usage tracking
        let resource_usage = ResourceUsage {
            current_usage: 0.0,
            peak_usage: 0.0,
            average_usage: 0.0,
            usage_history: Vec::new(),
            efficiency_score: 1.0,
            utilization_rate: 0.0,
        };
        
        // Calculate allocation efficiency (simplified)
        let allocation_efficiency = if request.amount > 0.0 {
            (request.amount / request.amount).min(1.0)
        } else {
            0.0
        };
        
        let allocation = AssetAllocation {
            id: allocation_id.to_string(),
            request: request.clone(),
            allocated_amount: request.amount,
            allocation_efficiency,
            allocated_at: now,
            expires_at,
            last_used: Arc::new(RwLock::new(None)),
            status: Arc::new(RwLock::new(AllocationStatus::Active)),
            consensus_proof,
            resource_usage,
            performance_metrics: Arc::new(RwLock::new(None)),
        };
        
        Ok(allocation)
    }
    
    /// Update resource pools after allocation/deallocation
    async fn update_resource_pools(&self, asset_id: &str, amount: f64, allocate: bool) -> Result<()> {
        let mut pools = self.resource_pools.write().await;
        
        for (_, pool) in pools.iter_mut() {
            if asset_id.contains(&pool.asset_type) {
                if allocate {
                    pool.available_capacity -= amount;
                    pool.allocated_capacity += amount;
                    pool.allocation_count += 1;
                } else {
                    pool.available_capacity += amount;
                    pool.allocated_capacity -= amount;
                }
                
                // Update peak utilization
                let utilization = pool.allocated_capacity / pool.total_capacity;
                if utilization > pool.peak_utilization {
                    pool.peak_utilization = utilization;
                }
                
                break;
            }
        }
        
        Ok(())
    }
    
    /// Release allocation
    pub async fn release(&self, allocation_id: &str) -> Result<()> {
        info!("🔓 Releasing allocation: {}", allocation_id);
        
        let mut allocations = self.allocations.write().await;
        
        if let Some(allocation) = allocations.get(allocation_id) {
            // Update allocation status
            *allocation.status.write().await = AllocationStatus::Released;
            
            // Update resource pools
            self.update_resource_pools(
                &allocation.request.asset_id,
                allocation.allocated_amount,
                false
            ).await?;
            
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.active_allocations = stats.active_allocations.saturating_sub(1);
            
            info!("✅ Allocation released: {}", allocation_id);
        } else {
            return Err(anyhow!("Allocation not found: {}", allocation_id));
        }
        
        Ok(())
    }
    
    /// Get allocation by ID
    pub async fn get_allocation(&self, allocation_id: &str) -> Option<Arc<AssetAllocation>> {
        self.allocations.read().await.get(allocation_id).cloned()
    }
    
    /// List active allocations
    pub async fn list_active_allocations(&self) -> Vec<Arc<AssetAllocation>> {
        let allocations = self.allocations.read().await;
        let mut active_allocations = Vec::new();
        
        for allocation in allocations.values() {
            let status = allocation.status.read().await;
            if *status == AllocationStatus::Active {
                active_allocations.push(allocation.clone());
            }
        }
        
        active_allocations
    }
    
    /// Get allocation statistics
    pub async fn get_statistics(&self) -> AllocationStats {
        let stats = self.stats.read().await;
        let active_count = self.allocations.read().await.len() as u64;
        
        AllocationStats {
            active_allocations: active_count,
            ..stats.clone()
        }
    }
    
    /// Get resource pool information
    pub async fn get_resource_pools(&self) -> HashMap<String, ResourcePool> {
        self.resource_pools.read().await.clone()
    }
    
    /// Update allocation performance metrics
    pub async fn update_allocation_performance(
        &self,
        allocation_id: &str,
        metrics: AllocationPerformanceMetrics
    ) -> Result<()> {
        let mut allocations = self.allocations.write().await;
        
        if let Some(allocation) = allocations.get(allocation_id) {
            *allocation.performance_metrics.write().await = Some(metrics);
            *allocation.last_used.write().await = Some(SystemTime::now());
            
            debug!("📊 Updated performance metrics for allocation: {}", allocation_id);
        } else {
            return Err(anyhow!("Allocation not found: {}", allocation_id));
        }
        
        Ok(())
    }
    
    /// Find allocations by criteria
    pub async fn find_allocations(&self, _criteria: AllocationSearchCriteria) -> Vec<Arc<AssetAllocation>> {
        // Simplified implementation - would need async criteria matching for Arc<RwLock<T>> fields
        let allocations = self.allocations.read().await;
        allocations.values().cloned().collect()
    }
    
    /// Shutdown asset allocator
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down Asset Allocator");
        
        // Release all active allocations
        let allocation_ids: Vec<String> = self.allocations.read().await.keys().cloned().collect();
        
        for allocation_id in allocation_ids {
            if let Err(e) = self.release(&allocation_id).await {
                warn!("Error releasing allocation {} during shutdown: {}", allocation_id, e);
            }
        }
        
        // Clear allocations and pools
        self.allocations.write().await.clear();
        self.resource_pools.write().await.clear();
        
        info!("✅ Asset Allocator shutdown complete");
        Ok(())
    }
}

/// Search criteria for finding allocations
#[derive(Debug, Clone)]
pub struct AllocationSearchCriteria {
    pub requester_id: Option<String>,
    pub asset_id: Option<String>,
    pub status: Option<AllocationStatus>,
    pub priority: Option<AllocationPriority>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
}

impl AllocationSearchCriteria {
    pub fn matches(&self, allocation: &AssetAllocation) -> bool {
        if let Some(ref requester_id) = self.requester_id {
            if allocation.request.requester_id != *requester_id {
                return false;
            }
        }
        
        if let Some(ref asset_id) = self.asset_id {
            if allocation.request.asset_id != *asset_id {
                return false;
            }
        }
        
        if let Some(ref _status) = self.status {
            // TODO: Would need async access to allocation.status Arc<RwLock<AllocationStatus>>
            // For now, skip status comparison
        }
        
        if let Some(ref priority) = self.priority {
            if allocation.request.priority != *priority {
                return false;
            }
        }
        
        if let Some(min_amount) = self.min_amount {
            if allocation.allocated_amount < min_amount {
                return false;
            }
        }
        
        if let Some(max_amount) = self.max_amount {
            if allocation.allocated_amount > max_amount {
                return false;
            }
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AssetConfig, ConsensusConfig};
    use crate::assets::consensus::FourProofConsensus;
    use crate::transport::StoqTransportLayer;
    
    #[tokio::test]
    async fn test_allocation_request_validation() {
        // Test allocation request validation
    }
    
    #[tokio::test]
    async fn test_resource_pool_management() {
        // Test resource pool operations
    }
    
    #[tokio::test] 
    async fn test_allocation_lifecycle() {
        // Test complete allocation lifecycle
    }
}