//! Extension system trait definitions

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use semver::Version;

use crate::assets::core::{AssetManager, AssetId, AssetType, ConsensusProof};
use super::types::*;
use super::asset_types::*;
use super::package_types::*;

/// Core trait that all HyperMesh extensions must implement
#[async_trait]
pub trait HyperMeshExtension: Send + Sync {
    /// Get extension metadata
    fn metadata(&self) -> ExtensionMetadata;

    /// Initialize the extension with configuration
    async fn initialize(&mut self, config: ExtensionConfig) -> ExtensionResult<()>;

    /// Register assets provided by this extension
    async fn register_assets(&self) -> ExtensionResult<HashMap<AssetType, Box<dyn AssetExtensionHandler>>>;

    /// Extend the asset manager with custom functionality
    async fn extend_manager(&self, asset_manager: Arc<AssetManager>) -> ExtensionResult<()>;

    /// Handle extension-specific API calls
    async fn handle_request(&self, request: ExtensionRequest) -> ExtensionResult<ExtensionResponse>;

    /// Get current extension status
    async fn status(&self) -> ExtensionStatus;

    /// Validate extension integrity and configuration
    async fn validate(&self) -> ExtensionResult<ValidationReport>;

    /// Export extension state for migration or backup
    async fn export_state(&self) -> ExtensionResult<ExtensionStateData>;

    /// Import previously exported state
    async fn import_state(&mut self, state: ExtensionStateData) -> ExtensionResult<()>;

    /// Shutdown the extension gracefully
    async fn shutdown(&mut self) -> ExtensionResult<()>;
}

/// Handler for extension-provided assets
#[async_trait]
pub trait AssetExtensionHandler: Send + Sync {
    /// Get asset type this handler manages
    fn asset_type(&self) -> AssetType;

    /// Create a new asset instance
    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetId>;

    /// Update an existing asset
    async fn update_asset(&self, id: &AssetId, update: AssetUpdate) -> ExtensionResult<()>;

    /// Delete an asset
    async fn delete_asset(&self, id: &AssetId) -> ExtensionResult<()>;

    /// Query assets based on criteria
    async fn query_assets(&self, query: AssetQuery) -> ExtensionResult<Vec<AssetId>>;

    /// Get asset metadata
    async fn get_metadata(&self, id: &AssetId) -> ExtensionResult<AssetMetadata>;

    /// Validate asset with consensus proofs
    async fn validate_asset(&self, id: &AssetId, proof: ConsensusProof) -> ExtensionResult<bool>;

    /// Handle asset-specific operations
    async fn handle_operation(&self, id: &AssetId, operation: AssetOperation) -> ExtensionResult<OperationResult>;
}

/// Asset library extension trait for Catalog-like functionality
#[async_trait]
pub trait AssetLibraryExtension: HyperMeshExtension {
    /// List available asset packages
    async fn list_packages(&self, filter: PackageFilter) -> ExtensionResult<Vec<AssetPackage>>;

    /// Get package details
    async fn get_package(&self, package_id: &str) -> ExtensionResult<AssetPackage>;

    /// Install an asset package
    async fn install_package(&self, package_id: &str, options: InstallOptions) -> ExtensionResult<InstallResult>;

    /// Uninstall an asset package
    async fn uninstall_package(&self, package_id: &str) -> ExtensionResult<()>;

    /// Update an installed package
    async fn update_package(&self, package_id: &str, version: Option<Version>) -> ExtensionResult<UpdateResult>;

    /// Search for packages
    async fn search_packages(&self, query: &str, options: SearchOptions) -> ExtensionResult<Vec<AssetPackage>>;

    /// Publish a new package to the library
    async fn publish_package(&self, package: AssetPackageSpec, proof: ConsensusProof) -> ExtensionResult<PublishResult>;

    /// Verify package integrity
    async fn verify_package(&self, package_id: &str) -> ExtensionResult<VerificationResult>;
}
