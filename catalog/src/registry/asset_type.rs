// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset Type Definition System
//!
//! Defines asset types (schemas, validation rules) that are themselves BlockMatrix Assets.
//! This module provides the registry layer over BlockMatrix's core asset system.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// Import BlockMatrix asset types directly
use blockmatrix::assets::{AssetRegistration, ConsensusProof};

/// Asset Type Definition - defines schema and validation for a type of asset
///
/// This itself IS a BlockMatrix Asset (type: AssetType::Library)
/// Examples: "Vehicle", "CarInsurance", "HealthRecord", "LoanAgreement"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetTypeDefinition {
    /// Unique asset ID (this type definition is an asset)
    pub asset_id: AssetRegistration,

    /// Type name (e.g., "Vehicle", "CarInsurance")
    pub type_name: String,

    /// JSON schema defining the structure
    pub schema: JsonValue,

    /// Validation rules for this type
    pub validation_rules: Vec<ValidationRule>,

    /// Execution templates (references to contract/script assets)
    pub execution_templates: Vec<AssetRegistration>,

    /// Dependencies on other type definitions
    pub dependencies: Vec<String>,

    /// Consensus proof (all four: PoSp/PoSt/PoWk/PoTm)
    pub consensus_proof: ConsensusProof,

    /// Metadata
    pub metadata: TypeMetadata,

    /// Canonical asset kind (UserDefined with type_name + schema hash).
    pub asset_kind: hypermesh_lib::AssetKind,
}

/// Type metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMetadata {
    /// Type version (semantic versioning)
    pub version: String,

    /// Author/creator
    pub author: Option<String>,

    /// Description
    pub description: Option<String>,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// License information
    pub license: Option<String>,
}

/// Validation rule for asset type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,

    /// Rule type (schema, regex, custom)
    pub rule_type: ValidationRuleType,

    /// Rule definition
    pub definition: JsonValue,

    /// Error message if validation fails
    pub error_message: String,
}

/// Validation rule types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRuleType {
    /// JSON Schema validation
    Schema,
    /// Regular expression validation
    Regex,
    /// Custom validation (references to validator asset)
    Custom,
    /// Range validation
    Range,
    /// Enum validation
    Enum,
}

impl AssetTypeDefinition {
    /// Create a new asset type definition
    pub fn new(
        type_name: String,
        schema: JsonValue,
        consensus_proof: ConsensusProof,
    ) -> Self {
        // Create AssetRegistration from type definition data
        let asset_data = blockmatrix::assets::core::AssetData {
            config: type_name.as_bytes().to_vec(),
            definition: b"catalog_asset_type".to_vec(),
            metadata: b"{}".to_vec(),
        };
        let asset_id = AssetRegistration::from_asset_data(
            &asset_data,
            blockmatrix::assets::core::NetworkScope::Global,
            blockmatrix::assets::core::AssetCategory::BaseSystem(
                blockmatrix::assets::core::BaseSystemType::Storage,
            ),
        );

        // Compute UserAssetKind hash from type_name + schema
        use sha2::{Sha256, Digest};
        let mut kind_hasher = Sha256::new();
        kind_hasher.update(type_name.as_bytes());
        kind_hasher.update(serde_json::to_vec(&schema).unwrap_or_default());
        let hash_bytes: [u8; 32] = kind_hasher.finalize().into();

        let asset_kind = hypermesh_lib::AssetKind::UserDefined(hypermesh_lib::UserAssetKind {
            type_name: type_name.clone(),
            type_hash: hypermesh_lib::ContentHash::from_bytes(hash_bytes),
        });

        Self {
            asset_id,
            type_name,
            schema,
            validation_rules: Vec::new(),
            execution_templates: Vec::new(),
            dependencies: Vec::new(),
            consensus_proof,
            metadata: TypeMetadata {
                version: "1.0.0".to_string(),
                author: None,
                description: None,
                tags: Vec::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                license: None,
            },
            asset_kind,
        }
    }

    /// Add validation rule
    pub fn add_validation_rule(&mut self, rule: ValidationRule) {
        self.validation_rules.push(rule);
        self.metadata.updated_at = chrono::Utc::now();
    }

    /// Add dependency
    pub fn add_dependency(&mut self, dependency: String) {
        self.dependencies.push(dependency);
        self.metadata.updated_at = chrono::Utc::now();
    }

    /// Add execution template
    pub fn add_execution_template(&mut self, template_id: AssetRegistration) {
        self.execution_templates.push(template_id);
        self.metadata.updated_at = chrono::Utc::now();
    }

    /// Validate instance data against this type definition
    pub fn validate_instance(&self, instance_data: &JsonValue) -> Result<ValidationResult> {
        let mut errors = Vec::new();

        // Validate against JSON schema
        if let Err(e) = self.validate_schema(instance_data) {
            errors.push(format!("Schema validation failed: {}", e));
        }

        // Apply validation rules
        for rule in &self.validation_rules {
            if let Err(e) = self.apply_validation_rule(rule, instance_data) {
                errors.push(format!("Rule '{}' failed: {}", rule.name, e));
            }
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
        })
    }

    /// Validate against JSON schema
    fn validate_schema(&self, instance_data: &JsonValue) -> Result<()> {
        // STUB: Phase 4b - JSON schema validation implementation
        // For now, basic type checking
        if instance_data.is_object() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Expected object, got {:?}", instance_data))
        }
    }

    /// Apply validation rule
    fn apply_validation_rule(&self, rule: &ValidationRule, data: &JsonValue) -> Result<()> {
        match rule.rule_type {
            ValidationRuleType::Schema => self.validate_schema(data),
            ValidationRuleType::Regex => {
                // STUB: Phase 4b - Regex validation
                Ok(())
            }
            ValidationRuleType::Custom => {
                // STUB: Phase 4b - Custom validation
                Ok(())
            }
            ValidationRuleType::Range => {
                // STUB: Phase 4b - Range validation
                Ok(())
            }
            ValidationRuleType::Enum => {
                // STUB: Phase 4b - Enum validation
                Ok(())
            }
        }
    }

    /// Convert to storage format (JSON)
    pub fn to_storage_format(&self) -> Result<Vec<u8>> {
        let json = serde_json::to_vec_pretty(self)?;
        Ok(json)
    }

    /// Load from storage format (JSON)
    pub fn from_storage_format(data: &[u8]) -> Result<Self> {
        let type_def = serde_json::from_slice(data)?;
        Ok(type_def)
    }
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,

    /// Validation errors (empty if valid)
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_consensus_proof() -> ConsensusProof {
        use blockmatrix::consensus::proof_of_state_integration::{
            SpaceProof, StakeProof, WorkProof, TimeProof,
            WorkloadType, WorkState,
        };
        use std::time::Duration;

        let stake_proof = StakeProof::new(
            "test-holder".to_string(),
            "test-id".to_string(),
            1000
        );

        let space_proof = SpaceProof::new(
            "test-node".to_string(),
            "/test".to_string(),
            1024
        );

        let work_proof = WorkProof::new(
            "test-owner".to_string(),
            "test-workload".to_string(),
            12345,
            100,
            WorkloadType::Compute,
            WorkState::Completed,
        );

        let time_proof = TimeProof::new(Duration::from_secs(10));

        ConsensusProof::new(stake_proof, time_proof, space_proof, work_proof)
    }

    #[test]
    fn test_create_asset_type_definition() {
        let schema = json!({
            "type": "object",
            "properties": {
                "vin": { "type": "string" },
                "make": { "type": "string" },
                "model": { "type": "string" },
                "year": { "type": "integer" }
            },
            "required": ["vin", "make", "model", "year"]
        });

        let consensus_proof = create_test_consensus_proof();
        let type_def = AssetTypeDefinition::new(
            "Vehicle".to_string(),
            schema,
            consensus_proof,
        );

        assert_eq!(type_def.type_name, "Vehicle");
        // asset_type is a method on AssetRegistration, not a field
        // We can't compare AssetType here without importing blockmatrix types
        // Just verify the asset_id exists
        assert!(!type_def.asset_id.to_string().is_empty());
    }

    #[test]
    fn test_validate_instance() {
        let schema = json!({
            "type": "object",
            "properties": {
                "vin": { "type": "string" }
            }
        });

        let consensus_proof = create_test_consensus_proof();
        let type_def = AssetTypeDefinition::new(
            "Vehicle".to_string(),
            schema,
            consensus_proof,
        );

        let instance = json!({
            "vin": "1HGBH41JXMN109186",
            "make": "Honda",
            "model": "Accord"
        });

        let result = type_def.validate_instance(&instance).unwrap();
        assert!(result.valid);
    }
}
