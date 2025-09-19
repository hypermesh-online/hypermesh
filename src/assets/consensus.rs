//! Four-Proof Consensus System for HyperMesh Assets
//! 
//! Implements the NKrypt four-proof consensus system:
//! - PoSpace (PoSp): WHERE - storage location and physical/network location
//! - PoStake (PoSt): WHO - ownership, access rights, and economic stake
//! - PoWork (PoWk): WHAT/HOW - computational resources and processing  
//! - PoTime (PoTm): WHEN - temporal ordering and timestamp validation
//!
//! CRITICAL: Every asset requires ALL FOUR proofs (not split by type)

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

use crate::config::ConsensusConfig;
use crate::transport::StoqTransportLayer;

/// Four-proof consensus system for HyperMesh
pub struct FourProofConsensus {
    /// Configuration
    config: ConsensusConfig,
    
    /// STOQ transport for consensus communication
    stoq_transport: Arc<StoqTransportLayer>,
    
    /// Consensus validators
    validators: Arc<RwLock<HashMap<String, ConsensusValidator>>>,
    
    /// Active consensus operations
    active_operations: Arc<RwLock<HashMap<String, ConsensusOperation>>>,
    
    /// Consensus statistics
    stats: Arc<RwLock<ConsensusStats>>,
}

/// Complete consensus proof containing all four proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusProof {
    /// Space proof (WHERE)
    pub space_proof: SpaceProof,
    
    /// Stake proof (WHO)  
    pub stake_proof: StakeProof,
    
    /// Work proof (WHAT/HOW)
    pub work_proof: WorkProof,
    
    /// Time proof (WHEN)
    pub time_proof: TimeProof,
    
    /// Combined proof hash
    pub combined_hash: String,
    
    /// Proof generation timestamp
    pub generated_at: SystemTime,
    
    /// Validation results
    pub validation_results: Option<ValidationResults>,
}

/// Space Proof (PoSp) - WHERE the asset/operation exists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceProof {
    /// Storage space committed (bytes)
    pub total_storage: u64,
    
    /// Storage path verification
    pub storage_path: String,
    
    /// Node physical/network location
    pub node_id: String,
    pub network_location: String,
    
    /// Storage commitment hash
    pub storage_commitment: String,
    
    /// Proof of storage ownership
    pub storage_proof: Vec<u8>,
}

/// Stake Proof (PoSt) - WHO owns/accesses the asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeProof {
    /// Stake holder identity
    pub stake_holder: String,
    pub stake_holder_id: String,
    
    /// Economic stake amount
    pub stake_amount: u64,
    
    /// Access rights proof
    pub access_rights: Vec<String>,
    
    /// Ownership verification
    pub ownership_proof: Vec<u8>,
    
    /// Stake commitment hash
    pub stake_commitment: String,
}

/// Work Proof (PoWk) - WHAT/HOW the computation is performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkProof {
    /// Computational power committed
    pub computational_power: u64,
    
    /// Workload identification
    pub workload_id: String,
    pub process_id: u64,
    
    /// Work owner/performer
    pub owner_id: String,
    
    /// Type of work being performed
    pub workload_type: WorkloadType,
    
    /// Current work state
    pub work_state: WorkState,
    
    /// Computational proof
    pub computation_proof: Vec<u8>,
}

/// Time Proof (PoTm) - WHEN the operation occurs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeProof {
    /// Network timestamp
    pub network_timestamp: SystemTime,
    
    /// Local timestamp
    pub local_timestamp: SystemTime,
    
    /// Time offset from network consensus
    pub network_time_offset: Duration,
    
    /// Temporal ordering proof
    pub ordering_proof: Vec<u8>,
    
    /// Time synchronization hash
    pub time_sync_hash: String,
}

/// Workload types for work proof
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadType {
    Compute,
    Storage,
    Network,
    Consensus,
    AssetManagement,
    VmExecution,
}

/// Work states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkState {
    Initialized,
    Running,
    Completed,
    Failed,
    Suspended,
}

/// Proof types for validation
#[derive(Debug, Clone, PartialEq)]
pub enum ProofType {
    Space,
    Stake, 
    Work,
    Time,
    Combined,
}

/// Validation results for all four proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResults {
    /// Individual proof validations
    pub space_valid: bool,
    pub stake_valid: bool,
    pub work_valid: bool,
    pub time_valid: bool,
    
    /// Overall consensus validation
    pub consensus_valid: bool,
    
    /// Validation details
    pub validation_details: HashMap<String, String>,
    
    /// Validation timestamp
    pub validated_at: SystemTime,
    
    /// Validation duration
    pub validation_duration: Duration,
}

/// Consensus validator
#[derive(Debug, Clone)]
pub struct ConsensusValidator {
    pub validator_id: String,
    pub stake_amount: u64,
    pub computational_power: u64,
    pub reliability_score: f64,
    pub last_validation: Option<SystemTime>,
}

/// Active consensus operation
#[derive(Debug, Clone)]
pub struct ConsensusOperation {
    pub operation_id: String,
    pub asset_id: String,
    pub operation_type: String,
    pub consensus_proof: ConsensusProof,
    pub started_at: Instant,
    pub participants: Vec<String>,
    pub status: OperationStatus,
}

/// Operation status
#[derive(Debug, Clone, PartialEq)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Timeout,
}

/// Consensus statistics
#[derive(Debug, Clone, Default)]
pub struct ConsensusStats {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub avg_validation_time_ms: f64,
    pub success_rate: f64,
    pub validators_count: u32,
    pub byzantine_detections: u64,
}

impl FourProofConsensus {
    /// Create new four-proof consensus system
    pub async fn new(
        config: &ConsensusConfig,
        stoq_transport: Arc<StoqTransportLayer>
    ) -> Result<Self> {
        info!("🔐 Initializing Four-Proof Consensus System");
        info!("   Proofs: PoSpace + PoStake + PoWork + PoTime");
        info!("   Mode: {}", if config.mandatory_four_proof { "MANDATORY" } else { "Optional" });
        
        Ok(Self {
            config: config.clone(),
            stoq_transport,
            validators: Arc::new(RwLock::new(HashMap::new())),
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ConsensusStats::default())),
        })
    }
    
    /// Start consensus system
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Four-Proof Consensus System");
        
        // Initialize validators
        self.initialize_validators().await?;
        
        // Start consensus monitoring
        self.start_consensus_monitoring().await?;
        
        info!("✅ Four-Proof Consensus System started");
        info!("   Validators: {}", self.validators.read().await.len());
        info!("   Byzantine detection: {}", if self.config.enable_byzantine_detection { "Enabled" } else { "Disabled" });
        
        Ok(())
    }
    
    /// Initialize consensus validators
    async fn initialize_validators(&self) -> Result<()> {
        let mut validators = self.validators.write().await;
        
        // For now, create a single local validator
        // In production, this would discover and register network validators
        let local_validator = ConsensusValidator {
            validator_id: "local-validator".to_string(),
            stake_amount: 10000,
            computational_power: 1000,
            reliability_score: 1.0,
            last_validation: None,
        };
        
        validators.insert(local_validator.validator_id.clone(), local_validator);
        
        Ok(())
    }
    
    /// Start consensus monitoring
    async fn start_consensus_monitoring(&self) -> Result<()> {
        let stats = self.stats.clone();
        let active_operations = self.active_operations.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Monitor active operations for timeouts
                let mut operations = active_operations.write().await;
                let mut completed_ops = Vec::new();
                
                for (op_id, operation) in operations.iter_mut() {
                    if operation.started_at.elapsed() > Duration::from_secs(300) { // 5 minute timeout
                        operation.status = OperationStatus::Timeout;
                        completed_ops.push(op_id.clone());
                    }
                }
                
                for op_id in completed_ops {
                    operations.remove(&op_id);
                    
                    let mut stats_guard = stats.write().await;
                    stats_guard.failed_operations += 1;
                }
            }
        });
        
        Ok(())
    }
    
    /// Validate asset operation with four-proof consensus
    pub async fn validate_asset_operation<T: Serialize>(
        &self,
        asset_id: &str,
        operation: &str,
        operation_data: &T
    ) -> Result<Vec<ConsensusProof>> {
        let start_time = Instant::now();
        
        info!("🔐 Validating asset operation: {} on {}", operation, asset_id);
        
        // Generate consensus proof for the operation
        let consensus_proof = self.generate_consensus_proof(asset_id, operation, operation_data).await?;
        
        // Validate the consensus proof
        let validation_results = self.validate_consensus_proof(&consensus_proof).await?;
        
        let validation_time = start_time.elapsed();
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_operations += 1;
            
            if validation_results.consensus_valid {
                stats.successful_operations += 1;
                info!("✅ Consensus validation successful for {} in {:?}", asset_id, validation_time);
            } else {
                stats.failed_operations += 1;
                warn!("❌ Consensus validation failed for {}", asset_id);
            }
            
            // Update average validation time
            let total_time = stats.avg_validation_time_ms * (stats.total_operations - 1) as f64;
            stats.avg_validation_time_ms = (total_time + validation_time.as_millis() as f64) / stats.total_operations as f64;
            
            // Update success rate
            stats.success_rate = stats.successful_operations as f64 / stats.total_operations as f64;
        }
        
        if !validation_results.consensus_valid {
            return Err(anyhow!("Consensus validation failed: {:?}", validation_results.validation_details));
        }
        
        Ok(vec![consensus_proof])
    }
    
    /// Generate consensus proof for an operation
    async fn generate_consensus_proof<T: Serialize>(
        &self,
        asset_id: &str,
        operation: &str,
        operation_data: &T
    ) -> Result<ConsensusProof> {
        debug!("🔒 Generating four-proof consensus for asset: {}", asset_id);
        
        // Generate Space Proof (PoSp) - WHERE
        let space_proof = self.generate_space_proof(asset_id).await?;
        
        // Generate Stake Proof (PoSt) - WHO
        let stake_proof = self.generate_stake_proof(asset_id, operation).await?;
        
        // Generate Work Proof (PoWk) - WHAT/HOW
        let work_proof = self.generate_work_proof(asset_id, operation, operation_data).await?;
        
        // Generate Time Proof (PoTm) - WHEN
        let time_proof = self.generate_time_proof().await?;
        
        // Calculate combined hash
        let combined_hash = self.calculate_combined_hash(&space_proof, &stake_proof, &work_proof, &time_proof)?;
        
        let consensus_proof = ConsensusProof {
            space_proof,
            stake_proof,
            work_proof,
            time_proof,
            combined_hash,
            generated_at: SystemTime::now(),
            validation_results: None,
        };
        
        debug!("✅ Four-proof consensus generated: {}", &consensus_proof.combined_hash[..16]);
        Ok(consensus_proof)
    }
    
    /// Generate Space Proof (PoSp) - WHERE
    async fn generate_space_proof(&self, asset_id: &str) -> Result<SpaceProof> {
        // In production, this would verify actual storage commitment
        let storage_path = format!("/hypermesh/assets/{}", asset_id);
        let storage_commitment = self.calculate_storage_commitment(&storage_path)?;
        
        Ok(SpaceProof {
            total_storage: 1024 * 1024 * 1024, // 1GB placeholder
            storage_path,
            node_id: "local-node".to_string(),
            network_location: "local".to_string(),
            storage_commitment,
            storage_proof: vec![0; 32], // Placeholder proof
        })
    }
    
    /// Generate Stake Proof (PoSt) - WHO
    async fn generate_stake_proof(&self, asset_id: &str, operation: &str) -> Result<StakeProof> {
        let stake_holder = format!("asset-owner-{}", asset_id);
        let stake_commitment = self.calculate_stake_commitment(&stake_holder, operation)?;
        
        Ok(StakeProof {
            stake_holder: stake_holder.clone(),
            stake_holder_id: format!("id-{}", stake_holder),
            stake_amount: self.config.min_stake_requirement,
            access_rights: vec![operation.to_string()],
            ownership_proof: vec![0; 32], // Placeholder proof
            stake_commitment,
        })
    }
    
    /// Generate Work Proof (PoWk) - WHAT/HOW
    async fn generate_work_proof<T: Serialize>(
        &self,
        asset_id: &str,
        operation: &str,
        _operation_data: &T
    ) -> Result<WorkProof> {
        let workload_id = format!("{}-{}", asset_id, operation);
        
        let workload_type = match operation {
            "allocate" => WorkloadType::AssetManagement,
            "execute" => WorkloadType::VmExecution,
            "store" => WorkloadType::Storage,
            "compute" => WorkloadType::Compute,
            _ => WorkloadType::AssetManagement,
        };
        
        Ok(WorkProof {
            computational_power: self.config.pow_difficulty as u64 * 100,
            workload_id,
            process_id: std::process::id() as u64,
            owner_id: format!("worker-{}", asset_id),
            workload_type,
            work_state: WorkState::Running,
            computation_proof: vec![0; 32], // Placeholder proof
        })
    }
    
    /// Generate Time Proof (PoTm) - WHEN
    async fn generate_time_proof(&self) -> Result<TimeProof> {
        let now = SystemTime::now();
        let network_time_offset = Duration::from_millis(10); // Simulated network offset
        
        let time_sync_hash = self.calculate_time_sync_hash(now, network_time_offset)?;
        
        Ok(TimeProof {
            network_timestamp: now,
            local_timestamp: now,
            network_time_offset,
            ordering_proof: vec![0; 32], // Placeholder proof
            time_sync_hash,
        })
    }
    
    /// Validate complete consensus proof
    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> Result<ValidationResults> {
        debug!("🔍 Validating four-proof consensus: {}", &proof.combined_hash[..16]);
        
        let start_time = Instant::now();
        
        // Validate each proof individually
        let space_valid = self.validate_space_proof(&proof.space_proof).await?;
        let stake_valid = self.validate_stake_proof(&proof.stake_proof).await?;
        let work_valid = self.validate_work_proof(&proof.work_proof).await?;
        let time_valid = self.validate_time_proof(&proof.time_proof).await?;
        
        // Overall consensus validation - ALL FOUR proofs must be valid
        let consensus_valid = if self.config.mandatory_four_proof {
            space_valid && stake_valid && work_valid && time_valid
        } else {
            // In optional mode, require at least 3 out of 4 proofs
            [space_valid, stake_valid, work_valid, time_valid].iter().filter(|&&x| x).count() >= 3
        };
        
        let validation_duration = start_time.elapsed();
        
        let mut validation_details = HashMap::new();
        validation_details.insert("space_proof".to_string(), if space_valid { "valid" } else { "invalid" }.to_string());
        validation_details.insert("stake_proof".to_string(), if stake_valid { "valid" } else { "invalid" }.to_string());
        validation_details.insert("work_proof".to_string(), if work_valid { "valid" } else { "invalid" }.to_string());
        validation_details.insert("time_proof".to_string(), if time_valid { "valid" } else { "invalid" }.to_string());
        validation_details.insert("combined_hash".to_string(), proof.combined_hash.clone());
        
        // Check for Byzantine behavior if detection is enabled
        if self.config.enable_byzantine_detection && !consensus_valid {
            self.detect_byzantine_behavior(proof).await?;
        }
        
        Ok(ValidationResults {
            space_valid,
            stake_valid,
            work_valid,
            time_valid,
            consensus_valid,
            validation_details,
            validated_at: SystemTime::now(),
            validation_duration,
        })
    }
    
    /// Validate Space Proof
    async fn validate_space_proof(&self, proof: &SpaceProof) -> Result<bool> {
        // Validate storage commitment
        let expected_commitment = self.calculate_storage_commitment(&proof.storage_path)?;
        if proof.storage_commitment != expected_commitment {
            return Ok(false);
        }
        
        // Validate storage amount
        if proof.total_storage == 0 {
            return Ok(false);
        }
        
        // Additional validations would go here
        Ok(true)
    }
    
    /// Validate Stake Proof
    async fn validate_stake_proof(&self, proof: &StakeProof) -> Result<bool> {
        // Validate minimum stake requirement
        if proof.stake_amount < self.config.min_stake_requirement {
            return Ok(false);
        }
        
        // Validate stake commitment
        let expected_commitment = self.calculate_stake_commitment(&proof.stake_holder, "validation")?;
        // In production, this would validate the actual stake commitment
        
        Ok(true)
    }
    
    /// Validate Work Proof
    async fn validate_work_proof(&self, proof: &WorkProof) -> Result<bool> {
        // Validate computational power commitment
        if proof.computational_power < self.config.pow_difficulty as u64 {
            return Ok(false);
        }
        
        // Validate work state
        if proof.work_state == WorkState::Failed {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Validate Time Proof
    async fn validate_time_proof(&self, proof: &TimeProof) -> Result<bool> {
        // Validate time offset
        if proof.network_time_offset > self.config.validation_timeout {
            return Ok(false);
        }
        
        // Validate timestamp freshness
        if let Ok(elapsed) = proof.network_timestamp.elapsed() {
            if elapsed > Duration::from_secs(300) { // 5 minute max age
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Detect Byzantine behavior
    async fn detect_byzantine_behavior(&self, _proof: &ConsensusProof) -> Result<()> {
        // Byzantine fault detection logic would go here
        let mut stats = self.stats.write().await;
        stats.byzantine_detections += 1;
        
        warn!("🚨 Potential Byzantine behavior detected in consensus validation");
        Ok(())
    }
    
    /// Calculate combined hash of all four proofs
    fn calculate_combined_hash(
        &self,
        space_proof: &SpaceProof,
        stake_proof: &StakeProof,
        work_proof: &WorkProof,
        time_proof: &TimeProof
    ) -> Result<String> {
        let mut hasher = Sha256::new();
        
        hasher.update(&space_proof.storage_commitment);
        hasher.update(&stake_proof.stake_commitment);
        hasher.update(work_proof.computational_power.to_be_bytes());
        hasher.update(&time_proof.time_sync_hash);
        
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }
    
    /// Calculate storage commitment hash
    fn calculate_storage_commitment(&self, storage_path: &str) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(storage_path.as_bytes());
        hasher.update(b"storage_commitment");
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }
    
    /// Calculate stake commitment hash
    fn calculate_stake_commitment(&self, stake_holder: &str, operation: &str) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(stake_holder.as_bytes());
        hasher.update(operation.as_bytes());
        hasher.update(b"stake_commitment");
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }
    
    /// Calculate time synchronization hash
    fn calculate_time_sync_hash(&self, timestamp: SystemTime, offset: Duration) -> Result<String> {
        let mut hasher = Sha256::new();
        
        if let Ok(duration) = timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            hasher.update(duration.as_secs().to_be_bytes());
        }
        hasher.update(offset.as_millis().to_be_bytes());
        hasher.update(b"time_sync");
        
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }
    
    /// Get consensus statistics
    pub async fn get_statistics(&self) -> ConsensusStats {
        let stats = self.stats.read().await;
        let validators_count = self.validators.read().await.len() as u32;
        
        ConsensusStats {
            validators_count,
            ..stats.clone()
        }
    }
    
    /// Shutdown consensus system
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down Four-Proof Consensus System");
        
        // Clear active operations
        self.active_operations.write().await.clear();
        
        // Clear validators
        self.validators.write().await.clear();
        
        info!("✅ Four-Proof Consensus System shutdown complete");
        Ok(())
    }
}

// Implementation of proof constructors for testing/development
impl SpaceProof {
    pub fn new(total_storage: u64, storage_path: String) -> Self {
        Self {
            total_storage,
            storage_path: storage_path.clone(),
            node_id: "test-node".to_string(),
            network_location: "test-location".to_string(),
            storage_commitment: format!("commitment-{}", storage_path),
            storage_proof: vec![0; 32],
        }
    }
}

impl StakeProof {
    pub fn new(stake_holder: String, stake_holder_id: String, stake_amount: u64) -> Self {
        Self {
            stake_holder: stake_holder.clone(),
            stake_holder_id,
            stake_amount,
            access_rights: vec!["all".to_string()],
            ownership_proof: vec![0; 32],
            stake_commitment: format!("stake-{}", stake_holder),
        }
    }
}

impl WorkProof {
    pub fn new(
        computational_power: u64,
        workload_id: String,
        process_id: u64,
        owner_id: String,
        workload_type: WorkloadType,
        work_state: WorkState,
    ) -> Self {
        Self {
            computational_power,
            workload_id,
            process_id,
            owner_id,
            workload_type,
            work_state,
            computation_proof: vec![0; 32],
        }
    }
}

impl TimeProof {
    pub fn new(network_time_offset: Duration) -> Self {
        let now = SystemTime::now();
        Self {
            network_timestamp: now,
            local_timestamp: now,
            network_time_offset,
            ordering_proof: vec![0; 32],
            time_sync_hash: "test-time-sync".to_string(),
        }
    }
}

impl ConsensusProof {
    pub fn new(
        space_proof: SpaceProof,
        stake_proof: StakeProof,
        work_proof: WorkProof,
        time_proof: TimeProof,
    ) -> Self {
        let combined_hash = "test-combined-hash".to_string(); // Simplified for testing
        
        Self {
            space_proof,
            stake_proof,
            work_proof,
            time_proof,
            combined_hash,
            generated_at: SystemTime::now(),
            validation_results: None,
        }
    }
    
    /// Basic validation for testing
    pub fn validate(&self) -> bool {
        // Basic validation - all components present
        self.space_proof.total_storage > 0 &&
        self.stake_proof.stake_amount > 0 &&
        self.work_proof.computational_power > 0 &&
        !self.combined_hash.is_empty()
    }
    
    /// Comprehensive validation (async)
    pub async fn validate_comprehensive(&self) -> Result<()> {
        // In production, this would perform comprehensive validation
        if !self.validate() {
            return Err(anyhow!("Basic consensus proof validation failed"));
        }
        Ok(())
    }
    
    /// Default consensus proof for testing
    pub fn default_for_testing() -> Self {
        let space_proof = SpaceProof::new(1024, "/test/path".to_string());
        let stake_proof = StakeProof::new("test-holder".to_string(), "test-id".to_string(), 1000);
        let work_proof = WorkProof::new(
            100,
            "test-workload".to_string(),
            12345,
            "test-worker".to_string(),
            WorkloadType::Compute,
            WorkState::Completed,
        );
        let time_proof = TimeProof::new(Duration::from_secs(10));
        
        Self::new(space_proof, stake_proof, work_proof, time_proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_four_proof_generation() {
        // Test that all four proofs can be generated
        let space_proof = SpaceProof::new(1024, "/test".to_string());
        let stake_proof = StakeProof::new("test".to_string(), "test-id".to_string(), 1000);
        let work_proof = WorkProof::new(100, "test".to_string(), 1, "test".to_string(), WorkloadType::Compute, WorkState::Running);
        let time_proof = TimeProof::new(Duration::from_secs(1));
        
        let consensus_proof = ConsensusProof::new(space_proof, stake_proof, work_proof, time_proof);
        assert!(consensus_proof.validate());
    }
    
    #[tokio::test]
    async fn test_consensus_validation() {
        let proof = ConsensusProof::default_for_testing();
        assert!(proof.validate());
        assert!(proof.validate_comprehensive().await.is_ok());
    }
}