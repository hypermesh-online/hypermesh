//! Catalog - Universal Asset SDK with JuliaVM
//! 
//! A standalone library for secure asset management, scripting, and remote code execution
//! with bidirectional ZeroTrust validation and consensus integration.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod consensus;
pub mod assets;
pub mod template;
pub mod registry;
pub mod validation;
pub mod documentation;
pub mod versioning;
pub mod julia_vm;
pub mod scripting;
pub mod security;

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

// Re-export key types
pub use consensus::{ConsensusProof, ConsensusContext, AssetId, ExecutionResult};
pub use assets::{
    AssetPackage, AssetSpec, AssetMetadata, AssetContent, AssetSecurity,
    AssetResources, AssetExecution, AssetDependency
};
pub use template::{CatalogTemplateGenerator, TemplateConfig, TemplateType};
pub use registry::{AssetRegistry, RegistryConfig, AssetDiscovery};
pub use validation::{AssetValidator, ValidationConfig, ValidationResult};
pub use documentation::DocumentationGenerator;
pub use versioning::{VersionManager, SemanticVersion, DependencyResolver};
pub use julia_vm::{JuliaVMManager, JuliaCompiler, JuliaRuntime};
pub use scripting::{LuaEngine, ScriptingEngine, ScriptResult};
pub use security::{SecuritySandbox, SandboxLevel, ExecutionContext};

/// Catalog version
pub const CATALOG_VERSION: &str = "0.1.0";

/// Main Catalog instance that orchestrates all components
pub struct Catalog {
    consensus_config: Arc<ConsensusContext>,
    asset_registry: Arc<registry::AssetRegistry>,
    template_generator: Arc<template::CatalogTemplateGenerator>,
    asset_validator: Arc<validation::AssetValidator>,
    documentation_generator: Arc<documentation::DocumentationGenerator>,
    version_manager: Arc<versioning::VersionManager>,
}

/// Catalog configuration
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
}

impl Default for CatalogConfig {
    fn default() -> Self {
        Self {
            consensus: ConsensusContext::default(),
            registry: registry::RegistryConfig::default(),
            template: template::TemplateConfig::default(),
            validation: validation::ValidationConfig::default(),
            documentation: documentation::DocumentationConfig::default(),
        }
    }
}

impl Catalog {
    /// Create a new Catalog instance
    pub async fn new(config: CatalogConfig) -> Result<Self> {
        let consensus_config = Arc::new(config.consensus);
        
        // Initialize components
        let asset_registry = Arc::new(registry::AssetRegistry::new(config.registry).await?);
        let template_generator = Arc::new(template::CatalogTemplateGenerator::new(config.template)?);
        let asset_validator = Arc::new(validation::AssetValidator::new(config.validation));
        let documentation_generator = Arc::new(documentation::DocumentationGenerator::new(config.documentation)?);
        let version_manager = Arc::new(versioning::VersionManager::new());
        
        Ok(Self {
            consensus_config,
            asset_registry,
            template_generator,
            asset_validator,
            documentation_generator,
            version_manager,
        })
    }
    
    /// Get consensus configuration
    pub fn consensus_config(&self) -> Arc<ConsensusContext> {
        Arc::clone(&self.consensus_config)
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