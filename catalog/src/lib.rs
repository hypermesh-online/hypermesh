//! Catalog - HyperMesh Asset Package Manager
//!
//! Pure asset package manager for the HyperMesh ecosystem.
//! Runs on HyperMesh infrastructure via catalog.hypermesh.online trustchain network.
//!
//! ARCHITECTURE:
//! - Asset package management and distribution
//! - HyperMesh native resource utilization
//! - TrustChain certificate-based security
//! - No local execution - delegates to HyperMesh nodes
//!
//! NETWORK ADDRESS: catalog.hypermesh.online (via TrustChain DNS)

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod assets;
pub mod template;
pub mod registry;
pub mod validation;
pub mod documentation;
pub mod versioning;
pub mod scripting;
pub mod hypermesh_integration;
pub mod library;
pub mod hypermesh_bridge;
pub mod extension;
pub mod plugin;
pub mod distribution;
pub mod security;
pub mod sharing;

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

// Re-export key types from HyperMesh
pub use blockmatrix::consensus::proof_of_state_integration::{ConsensusProof, SpaceProof, StakeProof, WorkProof, TimeProof};
pub use blockmatrix::assets::core::{AssetId, AssetType};

// Define ExecutionResult locally (Catalog-specific)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Result data or error message
    pub message: String,
    /// Optional output data
    pub output: Option<serde_json::Value>,
}

// Define ConsensusContext locally (Catalog-specific configuration)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsensusContext {
    /// Network difficulty for PoW
    pub network_difficulty: u64,
    /// Minimum space commitment for PoS
    pub min_space_commitment: u64,
    /// Minimum stake amount
    pub min_stake_amount: u64,
    /// Time synchronization tolerance (ms)
    pub time_sync_tolerance_ms: u64,
}
pub use assets::{
    AssetPackage, AssetSpec, AssetMetadata, AssetContent, AssetSecurity,
    AssetResources, AssetExecution, AssetDependency
};
pub use template::{CatalogTemplateGenerator, TemplateConfig, TemplateType};
pub use registry::{AssetRegistry, RegistryConfig, AssetDiscovery};
pub use validation::{AssetValidator, ValidationConfig, ValidationResult};
pub use documentation::DocumentationGenerator;
pub use versioning::{VersionManager, SemanticVersion, DependencyResolver};
pub use scripting::{ScriptingEngine, ScriptResult};
pub use hypermesh_integration::{HyperMeshClient, HyperMeshAssetAdapter};
pub use hypermesh_bridge::{HyperMeshAssetRegistry, BridgeConfig};

/// Catalog version
pub const CATALOG_VERSION: &str = "0.1.0";

/// Main Catalog instance - HyperMesh Asset Package Manager
pub struct Catalog {
    consensus_context: Arc<ConsensusContext>,
    asset_registry: Arc<registry::AssetRegistry>,
    template_generator: Arc<template::CatalogTemplateGenerator>,
    asset_validator: Arc<validation::AssetValidator>,
    documentation_generator: Arc<documentation::DocumentationGenerator>,
    version_manager: Arc<versioning::VersionManager>,
    hypermesh_client: Arc<tokio::sync::Mutex<hypermesh_integration::HyperMeshClient>>,
}

/// Catalog configuration for HyperMesh integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogConfig {
    /// Consensus configuration
    pub consensus: ConsensusContext,
    /// Registry configuration
    pub registry: registry::RegistryConfig,
    /// Template configuration
    pub template: template::TemplateConfig,
    /// Validation configuration
    pub validation: validation::ValidationConfig,
    /// Documentation configuration
    pub documentation: documentation::DocumentationConfig,
    /// HyperMesh network address
    pub hypermesh_address: Option<String>,
    /// TrustChain certificate path
    pub trustchain_cert_path: Option<String>,
}

impl Default for CatalogConfig {
    fn default() -> Self {
        Self {
            consensus: ConsensusContext::default(),
            registry: registry::RegistryConfig::default(),
            template: template::TemplateConfig::default(),
            validation: validation::ValidationConfig::default(),
            documentation: documentation::DocumentationConfig::default(),
            hypermesh_address: Some("catalog.hypermesh.online".to_string()),
            trustchain_cert_path: None,
        }
    }
}

impl Catalog {
    /// Create a new Catalog instance with HyperMesh integration
    pub async fn new(config: CatalogConfig) -> Result<Self> {
        let consensus_context = Arc::new(config.consensus);

        // Initialize components
        let asset_registry = Arc::new(registry::AssetRegistry::new(config.registry).await?);
        let template_generator = Arc::new(template::CatalogTemplateGenerator::new(config.template)?);
        let asset_validator = Arc::new(validation::AssetValidator::new(config.validation));
        let documentation_generator = Arc::new(documentation::DocumentationGenerator::new(config.documentation)?);
        let version_manager = Arc::new(versioning::VersionManager::new());

        // Initialize HyperMesh client
        let hypermesh_address = config.hypermesh_address
            .unwrap_or_else(|| "catalog.hypermesh.online".to_string());
        let mut hypermesh_client = hypermesh_integration::HyperMeshClient::new(hypermesh_address);

        if let Some(cert_path) = config.trustchain_cert_path {
            hypermesh_client.set_trustchain_certificate(cert_path);
        }

        // Connect to HyperMesh network
        hypermesh_client.connect().await?;

        Ok(Self {
            consensus_context,
            asset_registry,
            template_generator,
            asset_validator,
            documentation_generator,
            version_manager,
            hypermesh_client: Arc::new(tokio::sync::Mutex::new(hypermesh_client)),
        })
    }
    
    /// Get consensus configuration
    pub fn consensus_context(&self) -> Arc<ConsensusContext> {
        Arc::clone(&self.consensus_context)
    }
    
    /// Get asset registry
    pub fn asset_registry(&self) -> Arc<registry::AssetRegistry> {
        Arc::clone(&self.asset_registry)
    }
    
    /// Get template generator
    pub fn template_generator(&self) -> Arc<template::CatalogTemplateGenerator> {
        Arc::clone(&self.template_generator)
    }
    
    /// Get asset validator
    pub fn asset_validator(&self) -> Arc<validation::AssetValidator> {
        Arc::clone(&self.asset_validator)
    }
    
    /// Get documentation generator
    pub fn documentation_generator(&self) -> Arc<documentation::DocumentationGenerator> {
        Arc::clone(&self.documentation_generator)
    }
    
    /// Get version manager
    pub fn version_manager(&self) -> Arc<versioning::VersionManager> {
        Arc::clone(&self.version_manager)
    }
    
    /// Publish an asset package
    pub async fn publish_asset(&self, package: AssetPackage) -> Result<AssetId> {
        // Validate the asset package
        let validation_result = self.asset_validator.validate(&package).await?;
        
        if !validation_result.is_valid {
            return Err(anyhow::anyhow!(
                "Asset validation failed: {:?}", 
                validation_result.summary.categories_failed
            ));
        }
        
        // Publish to registry
        let package_id = self.asset_registry.publish(package).await?;
        
        Ok(package_id)
    }
    
    /// Install an asset package
    pub async fn install_asset(&self, id: &AssetId) -> Result<AssetPackage> {
        self.asset_registry.install(id).await
    }
    
    /// Search for assets
    pub async fn search_assets(&self, query: &registry::SearchQuery) -> Result<registry::SearchResults> {
        self.asset_registry.search(query).await
    }
    
    /// Generate asset from template
    pub async fn generate_from_template(
        &self,
        template_name: &str,
        context: template::TemplateContext,
    ) -> Result<template::TemplateGenerationResult> {
        self.template_generator.generate_from_template(template_name, context).await
    }
    
    /// Validate an asset package
    pub async fn validate_asset(&self, package: &AssetPackage) -> Result<validation::ValidationResult> {
        self.asset_validator.validate(package).await
    }
    
    /// Generate documentation for an asset
    pub async fn generate_documentation(&self, package: &AssetPackage) -> Result<documentation::GeneratedDocumentation> {
        self.documentation_generator.generate(package).await
    }

    /// Execute asset on HyperMesh infrastructure
    pub async fn execute_asset_on_hypermesh(
        &self,
        asset_id: &AssetId,
        package: &AssetPackage,
    ) -> Result<hypermesh_integration::CatalogExecutionContext> {
        let hypermesh_client = self.hypermesh_client.lock().await;

        // Map asset requirements to HyperMesh resources
        let asset_adapter = hypermesh_integration::HyperMeshAssetAdapter::new();
        let resource_requirements = asset_adapter.map_asset_to_resources(package);

        // Execute on HyperMesh
        hypermesh_client.execute_asset(asset_id, resource_requirements).await
    }

    /// Query execution status on HyperMesh
    pub async fn query_hypermesh_execution(
        &self,
        execution_id: &str,
    ) -> Result<hypermesh_integration::CatalogExecutionContext> {
        let hypermesh_client = self.hypermesh_client.lock().await;
        hypermesh_client.query_execution(execution_id).await
    }

    /// Terminate execution on HyperMesh
    pub async fn terminate_hypermesh_execution(&self, execution_id: &str) -> Result<()> {
        let hypermesh_client = self.hypermesh_client.lock().await;
        hypermesh_client.terminate_execution(execution_id).await
    }

    /// Get HyperMesh network address
    pub async fn hypermesh_network_address(&self) -> String {
        let hypermesh_client = self.hypermesh_client.lock().await;
        hypermesh_client.network_address().to_string()
    }
}

/// Builder for creating Catalog instances
pub struct CatalogBuilder {
    config: CatalogConfig,
}

impl CatalogBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: CatalogConfig::default(),
        }
    }
    
    /// Set consensus configuration
    pub fn with_consensus(mut self, config: ConsensusContext) -> Self {
        self.config.consensus = config;
        self
    }

    /// Set HyperMesh network address
    pub fn with_hypermesh_address<S: Into<String>>(mut self, address: S) -> Self {
        self.config.hypermesh_address = Some(address.into());
        self
    }

    /// Set TrustChain certificate path
    pub fn with_trustchain_certificate<P: Into<String>>(mut self, cert_path: P) -> Self {
        self.config.trustchain_cert_path = Some(cert_path.into());
        self
    }
    
    /// Build the Catalog instance
    pub async fn build(self) -> Result<Catalog> {
        Catalog::new(self.config).await
    }
}

impl Default for CatalogBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_catalog_creation() {
        let catalog = CatalogBuilder::new().build().await;
        assert!(catalog.is_ok());
    }
    
    #[test]
    fn test_catalog_version() {
        assert_eq!(CATALOG_VERSION, "0.1.0");
    }
}