//! Consensus Proof Validation for VM Operations
//!
//! This module implements the four-proof validation system for VM operations,
//! ensuring every operation meets the Proof of State consensus requirements adapted
//! for HyperMesh's asset-aware execution model.

use std::time::{SystemTime, Duration};
use anyhow::Result;
use async_trait::async_trait;

use crate::consensus::proof::{ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime, AccessLevel};
use super::{ProofValidator, ProofRequirement, VMConsensusContext};

/// Validator for Proof of Space - WHERE operations occur
pub struct SpaceProofValidator {
    /// Minimum space commitment required (bytes)
    min_space_commitment: u64,
    /// Maximum acceptable storage location distance
    max_location_distance: u32,
    /// Required storage utilization percentage
    min_utilization_percentage: f64,
}

impl SpaceProofValidator {
    /// Create new space proof validator
    pub fn new(min_space_commitment: u64) -> Result<Self> {
        Ok(Self {
            min_space_commitment,
            max_location_distance: 1000, // Maximum routing distance
            min_utilization_percentage: 0.1, // 10% minimum utilization
        })
    }
    
    /// Validate storage location accessibility
    async fn validate_storage_location(&self, location: &str) -> Result<bool> {
        // In a real implementation, this would verify:
        // 1. Storage location exists and is accessible
        // 2. Required permissions for access
        // 3. Network reachability
        
        // For now, basic validation
        Ok(!location.is_empty() && location.len() > 3)
    }
    
    /// Validate network position and routing
    async fn validate_network_position(&self, proof: &ProofOfSpace) -> Result<bool> {
        // Validate network position is within acceptable distance
        if proof.network_position.distance_metric > self.max_location_distance {
            return Ok(false);
        }
        
        // Validate IPv6 address format (basic check)
        let addr = &proof.network_position.address;
        if addr != "::1" && !addr.contains(':') && !addr.starts_with("http3://") {
            return Ok(false);
        }
        
        Ok(true)
    }
}

#[async_trait]
impl ProofValidator<ProofOfSpace> for SpaceProofValidator {
    async fn validate(&self, proof: &ProofOfSpace, _context: &VMConsensusContext) -> Result<bool> {
        // 1. Check minimum space commitment
        if proof.committed_space < self.min_space_commitment {
            return Ok(false);
        }
        
        // 2. Validate storage location
        if !self.validate_storage_location(&proof.storage_location).await? {
            return Ok(false);
        }
        
        // 3. Validate network position
        if !self.validate_network_position(proof).await? {
            return Ok(false);
        }
        
        // 4. Validate proof timestamp is recent (within 1 hour)
        let age = SystemTime::now().duration_since(proof.generated_at)
            .unwrap_or(Duration::from_secs(0));
        if age > Duration::from_secs(3600) {
            return Ok(false);
        }
        
        // 5. Validate proof hash integrity
        proof.validate().await
    }
    
    async fn get_requirements(&self, operation_type: &str) -> Result<ProofRequirement> {
        let mut requirements = std::collections::HashMap::new();
        
        requirements.insert("min_space_bytes".to_string(), self.min_space_commitment);
        requirements.insert("max_distance".to_string(), self.max_location_distance as u64);
        
        // Different requirements for different operations
        let additional_constraints = match operation_type {
            "store" => vec![
                "write_access_required".to_string(),
                "persistent_storage".to_string(),
            ],
            "load" => vec![
                "read_access_required".to_string(),
            ],
            _ => vec![],
        };
        
        Ok(ProofRequirement {
            required: true,
            minimum_values: requirements,
            additional_constraints,
        })
    }
}

/// Validator for Proof of Stake - WHO owns/operates the asset
pub struct StakeProofValidator {
    /// Minimum authority level required
    min_authority_level: u64,
    /// Required access levels per operation type
    access_requirements: std::collections::HashMap<String, AccessLevel>,
}

impl StakeProofValidator {
    /// Create new stake proof validator
    pub fn new(min_authority_level: u64) -> Result<Self> {
        let mut access_requirements = std::collections::HashMap::new();
        
        // Set access requirements for different operations
        access_requirements.insert("store".to_string(), AccessLevel::Network);
        access_requirements.insert("load".to_string(), AccessLevel::Private);
        access_requirements.insert("compute".to_string(), AccessLevel::Public);
        access_requirements.insert("sync".to_string(), AccessLevel::Private);
        
        Ok(Self {
            min_authority_level,
            access_requirements,
        })
    }
    
    /// Validate stake holder identity
    async fn validate_stake_holder(&self, stake_holder: &str, stake_holder_id: &str) -> Result<bool> {
        // Basic validation - in real implementation would verify:
        // 1. Stake holder identity against trusted registry
        // 2. Stake holder ID matches known node
        // 3. Delegation relationships are valid
        
        Ok(!stake_holder.is_empty() && 
           !stake_holder_id.is_empty() && 
           stake_holder != stake_holder_id)
    }
    
    /// Validate access permissions for operation
    async fn validate_access_permissions(
        &self, 
        proof: &ProofOfStake, 
        operation_type: &str
    ) -> Result<bool> {
        if let Some(required_level) = self.access_requirements.get(operation_type) {
            let has_required_access = match operation_type {
                "store" => proof.permissions.write_level.level_value() >= required_level.level_value(),
                "load" => proof.permissions.read_level.level_value() >= required_level.level_value(),
                "compute" => proof.permissions.allocation_rights.contains(&"cpu".to_string()) ||
                            proof.permissions.allocation_rights.contains(&"gpu".to_string()),
                "sync" => proof.permissions.read_level.level_value() >= required_level.level_value(),
                _ => proof.permissions.admin_level.level_value() >= required_level.level_value(),
            };
            
            return Ok(has_required_access);
        }
        
        Ok(true) // No specific requirements for unknown operations
    }
}

#[async_trait]
impl ProofValidator<ProofOfStake> for StakeProofValidator {
    async fn validate(&self, proof: &ProofOfStake, context: &VMConsensusContext) -> Result<bool> {
        // 1. Check minimum authority level
        if proof.authority_level < self.min_authority_level {
            return Ok(false);
        }
        
        // 2. Validate stake holder identity
        if !self.validate_stake_holder(&proof.stake_holder, &proof.stake_holder_id).await? {
            return Ok(false);
        }
        
        // 3. Validate access permissions for context operation
        let operation_type = context.current_operation_type().unwrap_or("generic");
        if !self.validate_access_permissions(proof, operation_type).await? {
            return Ok(false);
        }
        
        // 4. Validate proof timestamp is recent (within 1 hour)
        let age = SystemTime::now().duration_since(proof.generated_at)
            .unwrap_or(Duration::from_secs(0));
        if age > Duration::from_secs(3600) {
            return Ok(false);
        }
        
        // 5. Validate proof hash integrity
        proof.validate().await
    }
    
    async fn get_requirements(&self, operation_type: &str) -> Result<ProofRequirement> {
        let mut requirements = std::collections::HashMap::new();
        requirements.insert("min_authority_level".to_string(), self.min_authority_level);
        
        let additional_constraints = match operation_type {
            "store" => vec!["write_permissions_required".to_string()],
            "load" => vec!["read_permissions_required".to_string()],
            "compute" => vec!["allocation_rights_required".to_string()],
            _ => vec!["valid_stake_holder_required".to_string()],
        };
        
        Ok(ProofRequirement {
            required: true,
            minimum_values: requirements,
            additional_constraints,
        })
    }
}

/// Validator for Proof of Work - WHAT/HOW computational work was done
pub struct WorkProofValidator {
    /// Minimum difficulty required
    min_difficulty: u32,
    /// Maximum age for work proofs (seconds)
    max_work_age: u64,
    /// Valid resource types
    valid_resource_types: Vec<String>,
}

impl WorkProofValidator {
    /// Create new work proof validator
    pub fn new(min_difficulty: u32) -> Result<Self> {
        Ok(Self {
            min_difficulty,
            max_work_age: 3600, // 1 hour
            valid_resource_types: vec![
                "cpu".to_string(),
                "gpu".to_string(),
                "memory".to_string(),
                "storage".to_string(),
                "network".to_string(),
            ],
        })
    }
    
    /// Validate computational work meets difficulty target
    async fn validate_difficulty(&self, proof: &ProofOfWork) -> Result<bool> {
        // Check difficulty meets minimum requirement
        if proof.difficulty < self.min_difficulty {
            return Ok(false);
        }
        
        // Validate work hash meets difficulty target
        // Higher difficulty = more leading zeros required
        let leading_zeros = self.count_leading_zeros(&proof.computation_hash);
        let expected_zeros = (proof.difficulty / 8) as usize;
        
        Ok(leading_zeros >= expected_zeros)
    }
    
    /// Count leading zero bits in hash
    fn count_leading_zeros(&self, hash: &[u8; 32]) -> usize {
        let mut zeros = 0;
        for byte in hash {
            if *byte == 0 {
                zeros += 8;
            } else {
                zeros += byte.leading_zeros() as usize;
                break;
            }
        }
        zeros
    }
    
    /// Validate resource type is supported
    async fn validate_resource_type(&self, resource_type: &str) -> Result<bool> {
        Ok(self.valid_resource_types.contains(&resource_type.to_string()))
    }
}

#[async_trait]
impl ProofValidator<ProofOfWork> for WorkProofValidator {
    async fn validate(&self, proof: &ProofOfWork, _context: &VMConsensusContext) -> Result<bool> {
        // 1. Validate difficulty and work hash
        if !self.validate_difficulty(proof).await? {
            return Ok(false);
        }
        
        // 2. Validate resource type
        if !self.validate_resource_type(&proof.resource_type).await? {
            return Ok(false);
        }
        
        // 3. Check work age
        let age = SystemTime::now().duration_since(proof.completed_at)
            .unwrap_or(Duration::from_secs(0));
        if age.as_secs() > self.max_work_age {
            return Ok(false);
        }
        
        // 4. Validate proof integrity
        proof.validate().await
    }
    
    async fn get_requirements(&self, operation_type: &str) -> Result<ProofRequirement> {
        let mut requirements = std::collections::HashMap::new();
        
        // Adjust difficulty based on operation type
        let required_difficulty = match operation_type {
            "compute" => self.min_difficulty * 2, // Higher difficulty for compute operations
            "store" => self.min_difficulty,
            "load" => self.min_difficulty / 2, // Lower difficulty for read operations
            _ => self.min_difficulty,
        };
        
        requirements.insert("min_difficulty".to_string(), required_difficulty as u64);
        requirements.insert("max_age_seconds".to_string(), self.max_work_age);
        
        let additional_constraints = vec![
            "valid_resource_type_required".to_string(),
            "proof_of_work_hash_valid".to_string(),
        ];
        
        Ok(ProofRequirement {
            required: true,
            minimum_values: requirements,
            additional_constraints,
        })
    }
}

/// Validator for Proof of Time - WHEN operations occurred
pub struct TimeProofValidator {
    /// Maximum allowed time drift (microseconds)
    max_time_drift: u64,
    /// Maximum age for time proofs (seconds)
    max_time_proof_age: u64,
    /// Minimum logical timestamp increment
    min_logical_increment: u64,
}

impl TimeProofValidator {
    /// Create new time proof validator
    pub fn new(max_time_drift: u64) -> Result<Self> {
        Ok(Self {
            max_time_drift,
            max_time_proof_age: 300, // 5 minutes
            min_logical_increment: 1,
        })
    }
    
    /// Validate time synchronization and drift
    async fn validate_time_sync(&self, proof: &ProofOfTime) -> Result<bool> {
        let current_time = SystemTime::now();
        
        // Check wall clock drift
        let wall_clock_drift = current_time.duration_since(proof.wall_clock)
            .or_else(|_| proof.wall_clock.duration_since(current_time))?;
        
        if wall_clock_drift.as_micros() as u64 > self.max_time_drift {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Validate logical timestamp ordering
    async fn validate_logical_ordering(&self, proof: &ProofOfTime, context: &VMConsensusContext) -> Result<bool> {
        if let Some(last_timestamp) = context.last_logical_timestamp() {
            // Ensure logical timestamp increases
            if proof.logical_timestamp <= last_timestamp {
                return Ok(false);
            }
            
            // Check minimum increment
            if proof.logical_timestamp - last_timestamp < self.min_logical_increment {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Validate temporal chain integrity
    async fn validate_temporal_chain(&self, proof: &ProofOfTime, context: &VMConsensusContext) -> Result<bool> {
        if let Some(previous_hash) = &proof.previous_hash {
            if let Some(expected_previous) = context.last_temporal_hash() {
                if previous_hash != &expected_previous {
                    return Ok(false);
                }
            }
        }
        
        Ok(true)
    }
}

#[async_trait]
impl ProofValidator<ProofOfTime> for TimeProofValidator {
    async fn validate(&self, proof: &ProofOfTime, context: &VMConsensusContext) -> Result<bool> {
        // 1. Validate time synchronization
        if !self.validate_time_sync(proof).await? {
            return Ok(false);
        }
        
        // 2. Validate logical timestamp ordering
        if !self.validate_logical_ordering(proof, context).await? {
            return Ok(false);
        }
        
        // 3. Validate temporal chain integrity
        if !self.validate_temporal_chain(proof, context).await? {
            return Ok(false);
        }
        
        // 4. Check proof age
        let age = SystemTime::now().duration_since(proof.wall_clock)
            .unwrap_or(Duration::from_secs(0));
        if age.as_secs() > self.max_time_proof_age {
            return Ok(false);
        }
        
        // 5. Validate proof hash integrity
        proof.validate().await
    }
    
    async fn get_requirements(&self, _operation_type: &str) -> Result<ProofRequirement> {
        let mut requirements = std::collections::HashMap::new();
        requirements.insert("max_time_drift_micros".to_string(), self.max_time_drift);
        requirements.insert("max_age_seconds".to_string(), self.max_time_proof_age);
        requirements.insert("min_logical_increment".to_string(), self.min_logical_increment);
        
        let additional_constraints = vec![
            "time_synchronization_required".to_string(),
            "logical_timestamp_ordering_required".to_string(),
            "temporal_chain_integrity_required".to_string(),
        ];
        
        Ok(ProofRequirement {
            required: true,
            minimum_values: requirements,
            additional_constraints,
        })
    }
}

/// Extension trait for AccessLevel to provide numeric comparison
impl AccessLevel {
    /// Get numeric value for access level comparison
    pub fn level_value(&self) -> u8 {
        match self {
            AccessLevel::None => 0,
            AccessLevel::Private => 1,
            AccessLevel::Network => 2,
            AccessLevel::Public => 3,
            AccessLevel::Verified => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::{NetworkPosition, AccessPermissions};
    
    #[tokio::test]
    async fn test_space_proof_validation() {
        let validator = SpaceProofValidator::new(1024).unwrap();
        let context = VMConsensusContext::new();
        
        let proof = ProofOfSpace::new(
            "/test/storage".to_string(),
            NetworkPosition {
                address: "::1".to_string(),
                zone: "test".to_string(),
                distance_metric: 100,
            },
            2048, // Above minimum
        );
        
        let result = validator.validate(&proof, &context).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_work_proof_difficulty() {
        let validator = WorkProofValidator::new(16).unwrap();
        
        // Test difficulty validation
        let proof = ProofOfWork::new(
            b"test-challenge",
            20, // Above minimum
            "cpu".to_string(),
        ).unwrap();
        
        let valid_difficulty = validator.validate_difficulty(&proof).await.unwrap();
        assert!(valid_difficulty);
        
        // Test invalid resource type would fail
        let result = validator.validate_resource_type("invalid").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
    
    #[test]
    fn test_access_level_comparison() {
        assert!(AccessLevel::Public.level_value() > AccessLevel::Private.level_value());
        assert!(AccessLevel::Verified.level_value() > AccessLevel::Public.level_value());
        assert_eq!(AccessLevel::None.level_value(), 0);
    }
}