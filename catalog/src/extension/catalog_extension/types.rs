// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CatalogExtension type definition, constructor, and internal helpers

use semver::Version;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use blockmatrix::extensions::{
    AssetExtensionHandler, ExtensionCapability, ExtensionCategory, ExtensionHealth,
    ExtensionMetadata, ExtensionStateData, ResourceUsageReport,
};

use blockmatrix::assets::core::AssetType;

use crate::{
    hypermesh_bridge::HyperMeshAssetRegistry,
    library::{AssetLibrary, LibraryConfig},
    sharing::SharingManager,
    Catalog,
};

use super::super::asset_handlers::{
    DatasetHandler, LibraryHandler, TemplateHandler, VirtualMachineHandler,
};
use super::super::config::CatalogExtensionConfig;

/// CatalogExtension - HyperMesh plugin for asset library management
pub struct CatalogExtension {
    pub(crate) metadata: ExtensionMetadata,
    pub(crate) catalog: Option<Arc<Catalog>>,
    pub(crate) library_manager: Arc<RwLock<AssetLibrary>>,
    pub(crate) _asset_registry: Option<Arc<HyperMeshAssetRegistry>>,
    pub(crate) sharing_manager: Option<Arc<SharingManager>>,
    pub(crate) _asset_handlers: HashMap<AssetType, Box<dyn AssetExtensionHandler>>,
    pub(crate) config: CatalogExtensionConfig,
    pub(crate) state: Arc<RwLock<ExtensionStateData>>,
    pub(crate) health: Arc<RwLock<ExtensionHealth>>,
    pub(crate) resource_usage: Arc<RwLock<ResourceUsageReport>>,
    pub(crate) active_operations: Arc<RwLock<usize>>,
    pub(crate) total_requests: Arc<RwLock<u64>>,
    pub(crate) error_count: Arc<RwLock<u64>>,
    pub(crate) start_time: std::time::Instant,
}

impl CatalogExtension {
    /// Create a new CatalogExtension instance
    pub fn new(config: CatalogExtensionConfig) -> Self {
        let metadata = ExtensionMetadata {
            id: "catalog".to_string(),
            name: "HyperMesh Catalog".to_string(),
            version: Version::parse("0.1.0")
                .expect("Hardcoded extension version must be valid semver"),
            description: "Decentralized asset library and package manager for HyperMesh"
                .to_string(),
            author: "HyperMesh Team".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://catalog.hypermesh.online".to_string()),
            category: ExtensionCategory::AssetLibrary,
            hypermesh_version: Version::parse("1.0.0")
                .expect("Hardcoded HyperMesh version must be valid semver"),
            dependencies: vec![],
            required_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::NetworkAccess,
                ExtensionCapability::ConsensusAccess,
                ExtensionCapability::TransportAccess,
                ExtensionCapability::TrustChainAccess,
                ExtensionCapability::VMExecution,
                ExtensionCapability::FileSystemAccess,
            ]),
            provided_assets: vec![
                AssetType::Blockchain,
                AssetType::Dns,
                AssetType::Dns,
                AssetType::Container,
            ],
            certificate_fingerprint: config.certificate_fingerprint.clone(),
            config_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "library_path": { "type": "string" },
                    "cache_size": { "type": "integer" },
                    "max_package_size": { "type": "integer" },
                    "enable_p2p": { "type": "boolean" },
                    "consensus_validation": { "type": "boolean" }
                }
            })),
        };

        let library_config = LibraryConfig {
            enable_cache: true,
            l1_cache_size: 100,
            l2_cache_size: config.cache_size as usize,
            l3_cache_path: Some(config.library_path.to_string_lossy().to_string()),
            enable_zero_copy: true,
            max_concurrent_ops: 100,
            enable_metrics: true,
        };

        let library_manager = Arc::new(RwLock::new(AssetLibrary::with_config(library_config)));

        let asset_registry = None;

        let mut asset_handlers = HashMap::new();
        asset_handlers.insert(
            AssetType::Blockchain,
            Box::new(VirtualMachineHandler::new()) as Box<dyn AssetExtensionHandler>,
        );
        asset_handlers.insert(
            AssetType::Dns,
            Box::new(LibraryHandler::new()) as Box<dyn AssetExtensionHandler>,
        );
        asset_handlers.insert(
            AssetType::Dns,
            Box::new(DatasetHandler::new()) as Box<dyn AssetExtensionHandler>,
        );
        asset_handlers.insert(
            AssetType::Container,
            Box::new(TemplateHandler::new()) as Box<dyn AssetExtensionHandler>,
        );

        Self {
            metadata: metadata.clone(),
            catalog: None,
            library_manager,
            _asset_registry: asset_registry,
            sharing_manager: None,
            _asset_handlers: asset_handlers,
            config,
            state: Arc::new(RwLock::new(ExtensionStateData {
                version: 1,
                metadata,
                state_data: vec![],
                checksum: String::new(),
                exported_at: std::time::SystemTime::now(),
            })),
            health: Arc::new(RwLock::new(ExtensionHealth::Healthy)),
            resource_usage: Arc::new(RwLock::new(ResourceUsageReport {
                cpu_usage: 0.0,
                memory_usage: 0,
                network_bytes: 0,
                storage_bytes: 0,
            })),
            active_operations: Arc::new(RwLock::new(0)),
            total_requests: Arc::new(RwLock::new(0)),
            error_count: Arc::new(RwLock::new(0)),
            start_time: std::time::Instant::now(),
        }
    }

    /// Internal helper to increment request counter
    pub(crate) async fn increment_requests(&self) {
        let mut count = self.total_requests.write().await;
        *count += 1;
    }

    /// Internal helper to track errors
    pub(crate) async fn track_error(&self, _error: &str) {
        let mut count = self.error_count.write().await;
        *count += 1;

        if *count > 100 {
            let mut health = self.health.write().await;
            *health = ExtensionHealth::Degraded(format!("High error rate: {} errors", *count));
        }
    }

    /// Internal helper to track active operations
    pub(crate) async fn start_operation(&self) {
        let mut ops = self.active_operations.write().await;
        *ops += 1;
    }

    /// Internal helper to complete operations
    pub(crate) async fn complete_operation(&self) {
        let mut ops = self.active_operations.write().await;
        if *ops > 0 {
            *ops -= 1;
        }
    }

    /// Update resource usage metrics
    pub(crate) async fn update_resource_usage(&self, delta: ResourceUsageReport) {
        let mut usage = self.resource_usage.write().await;
        usage.cpu_usage += delta.cpu_usage;
        usage.memory_usage += delta.memory_usage;
        usage.network_bytes += delta.network_bytes;
        usage.storage_bytes += delta.storage_bytes;
    }
}
