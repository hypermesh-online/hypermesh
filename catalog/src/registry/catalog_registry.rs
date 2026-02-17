// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog Registry Service
//!
//! Provides indexing and discovery for asset type definitions.
//! The registry itself is stored as a BlockMatrix Asset.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use blockmatrix::assets::{AssetId, ConsensusProof, PrivacyLevel};
use blockmatrix::assets::core::{AssetData, NetworkScope, AssetCategory, BaseSystemType};

use super::asset_type::AssetTypeDefinition;

/// Catalog Registry - provides indexing/discovery for asset types
///
/// The registry itself is stored as a BlockMatrix Asset
#[derive(Clone)]
pub struct CatalogRegistry {
    /// Registry ID (this registry is an Asset)
    registry_id: AssetId,

    /// Index of type names → asset IDs
    index: Arc<RwLock<HashMap<String, AssetId>>>,

    /// Privacy level configuration
    privacy: PrivacyLevel,

    /// Trust policy
    trust_policy: TrustPolicy,

    /// Registry configuration
    config: RegistryConfig,
}

/// Trust policy for registry operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPolicy {
    /// Require consensus proof for registration
    pub require_consensus_proof: bool,

    /// Minimum stake amount for registration
    pub minimum_stake: u64,

    /// Allowed publishers (empty = allow all)
    pub allowed_publishers: Vec<String>,

    /// Require certificate validation
    pub require_certificate: bool,
}

impl Default for TrustPolicy {
    fn default() -> Self {
        Self {
            require_consensus_proof: true,
            minimum_stake: 1000,
            allowed_publishers: Vec::new(),
            require_certificate: true,
        }
    }
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry name
    pub name: String,

    /// Registry description
    pub description: Option<String>,

    /// Maximum entries
    pub max_entries: usize,

    /// Enable versioning
    pub enable_versioning: bool,

    /// Enable dependency resolution
    pub enable_dependency_resolution: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: None,
            max_entries: 100_000,
            enable_versioning: true,
            enable_dependency_resolution: true,
        }
    }
}

impl CatalogRegistry {
    /// Create a new registry
    pub fn new(privacy: PrivacyLevel, trust_policy: TrustPolicy, config: RegistryConfig) -> Self {
        // Create registry AssetId from registry configuration
        let asset_data = AssetData {
            config: format!("registry_{:?}", privacy).as_bytes().to_vec(),
            definition: b"catalog_registry".to_vec(),
            metadata: b"{}".to_vec(),
        };
        let registry_id = AssetId::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Storage),
        );

        Self {
            registry_id,
            index: Arc::new(RwLock::new(HashMap::new())),
            privacy,
            trust_policy,
            config,
        }
    }

    /// Register a new asset type definition
    pub async fn register_type(&self, type_def: AssetTypeDefinition) -> Result<AssetId> {
        // Validate consensus proof if required
        if self.trust_policy.require_consensus_proof {
            self.validate_consensus_proof(&type_def.consensus_proof)?;
        }

        // Check if type already exists
        let mut index = self.index.write().await;
        if index.contains_key(&type_def.type_name) {
            return Err(anyhow::anyhow!(
                "Type '{}' already registered",
                type_def.type_name
            ));
        }

        // Check registry capacity
        if index.len() >= self.config.max_entries {
            return Err(anyhow::anyhow!("Registry capacity exceeded"));
        }

        // Store in index
        let asset_id = type_def.asset_id.clone();
        index.insert(type_def.type_name.clone(), asset_id.clone());

        // STUB: Phase 4b - Store type definition as BlockMatrix Asset
        // For now, just maintain in-memory index
        // Future: Use AssetManager to store type_def as Asset

        tracing::info!("Registered asset type: {}", type_def.type_name);
        Ok(asset_id)
    }

    /// Find asset type by name
    pub async fn find_type(&self, name: &str) -> Result<AssetId> {
        let index = self.index.read().await;
        index
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Type '{}' not found", name))
    }

    /// List all registered types
    pub async fn list_types(&self) -> Result<Vec<String>> {
        let index = self.index.read().await;
        let types: Vec<String> = index.keys().cloned().collect();
        Ok(types)
    }

    /// Search types by query
    pub async fn search_types(&self, query: &SearchQuery) -> Result<SearchResults> {
        let index = self.index.read().await;

        let mut matching_types = Vec::new();

        for (type_name, asset_id) in index.iter() {
            // Simple name-based search for now
            if query.query.is_empty()
                || type_name.to_lowercase().contains(&query.query.to_lowercase())
            {
                matching_types.push(SearchResult {
                    type_name: type_name.clone(),
                    asset_id: asset_id.clone(),
                    score: 1.0, // STUB: Phase 4b - Implement scoring
                });
            }
        }

        // Apply limit
        matching_types.truncate(query.limit);

        Ok(SearchResults {
            results: matching_types,
            total_count: index.len(),
            query: query.clone(),
        })
    }

    /// Resolve dependencies for a type
    pub async fn resolve_dependencies(&self, _type_name: &str) -> Result<Vec<AssetId>> {
        if !self.config.enable_dependency_resolution {
            return Ok(Vec::new());
        }

        // STUB: Phase 4b - Implement dependency resolution
        // For now, return empty list
        Ok(Vec::new())
    }

    /// Get registry statistics
    pub async fn get_statistics(&self) -> RegistryStatistics {
        let index = self.index.read().await;
        RegistryStatistics {
            total_types: index.len(),
            registry_id: self.registry_id.to_string(),
            privacy_level: format!("{:?}", self.privacy),
        }
    }

    /// Validate consensus proof
    fn validate_consensus_proof(&self, proof: &ConsensusProof) -> Result<()> {
        // Basic validation
        if !proof.validate() {
            return Err(anyhow::anyhow!("Consensus proof validation failed"));
        }

        // Check stake requirement
        if proof.stake_proof.stake_amount < self.trust_policy.minimum_stake {
            return Err(anyhow::anyhow!(
                "Insufficient stake: {} < required {}",
                proof.stake_proof.stake_amount,
                self.trust_policy.minimum_stake
            ));
        }

        Ok(())
    }

    /// Get registry ID
    pub fn registry_id(&self) -> &AssetId {
        &self.registry_id
    }

    /// Get privacy level
    pub fn privacy_level(&self) -> &PrivacyLevel {
        &self.privacy
    }
}

/// Search query (compatible with old registry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search terms
    pub query: String,

    /// Asset type filter
    pub asset_type: Option<String>,

    /// Tag filters
    pub tags: Vec<String>,

    /// Author filter
    pub author: Option<String>,

    /// Version constraints
    pub version: Option<String>,

    /// Date range filter
    pub date_range: Option<DateRange>,

    /// Sort criteria
    pub sort_by: SortCriteria,

    /// Maximum results to return
    pub limit: usize,

    /// Offset for pagination
    pub offset: usize,
}

/// Sort criteria for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortCriteria {
    Relevance,
    Name,
    Rating,
    Downloads,
    Updated,
    Published,
}

impl Default for SortCriteria {
    fn default() -> Self {
        SortCriteria::Relevance
    }
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<chrono::DateTime<chrono::Utc>>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            asset_type: None,
            tags: Vec::new(),
            author: None,
            version: None,
            date_range: None,
            sort_by: SortCriteria::default(),
            limit: 20,
            offset: 0,
        }
    }
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Type name
    pub type_name: String,

    /// Asset ID
    pub asset_id: AssetId,

    /// Relevance score (0.0 - 1.0)
    pub score: f64,
}

/// Search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// Matching results
    pub results: Vec<SearchResult>,

    /// Total count (before limit)
    pub total_count: usize,

    /// Original query
    pub query: SearchQuery,
}

/// Registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatistics {
    /// Total registered types
    pub total_types: usize,

    /// Registry asset ID
    pub registry_id: String,

    /// Privacy level
    pub privacy_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::asset_type::AssetTypeDefinition;
    use blockmatrix::consensus::proof_of_state_integration::{
        SpaceProof, StakeProof, WorkProof, TimeProof,
        WorkloadType, WorkState,
    };
    use std::time::Duration;
    use serde_json::json;

    fn create_test_consensus_proof() -> ConsensusProof {
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

    #[tokio::test]
    async fn test_register_and_find_type() {
        let registry = CatalogRegistry::new(
            PrivacyLevel::FullPublic,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

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

        let asset_id = registry.register_type(type_def).await.unwrap();
        let found_id = registry.find_type("Vehicle").await.unwrap();

        assert_eq!(asset_id, found_id);
    }

    #[tokio::test]
    async fn test_search_types() {
        let registry = CatalogRegistry::new(
            PrivacyLevel::FullPublic,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        // Register multiple types
        for name in &["Vehicle", "VehicleInsurance", "Driver"] {
            let schema = json!({ "type": "object" });
            let consensus_proof = create_test_consensus_proof();
            let type_def = AssetTypeDefinition::new(
                name.to_string(),
                schema,
                consensus_proof,
            );
            registry.register_type(type_def).await.unwrap();
        }

        // Search for "Vehicle"
        let query = SearchQuery {
            query: "Vehicle".to_string(),
            ..Default::default()
        };

        let results = registry.search_types(&query).await.unwrap();
        assert_eq!(results.results.len(), 2); // Vehicle and VehicleInsurance
    }

    #[tokio::test]
    async fn test_registry_statistics() {
        let registry = CatalogRegistry::new(
            PrivacyLevel::FullPublic,
            TrustPolicy::default(),
            RegistryConfig::default(),
        );

        let stats = registry.get_statistics().await;
        assert_eq!(stats.total_types, 0);
    }
}
