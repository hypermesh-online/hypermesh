# Data Structure Migration Guide

## Overview
This document maps current Catalog data structures to HyperMesh Extension Interface structures, showing exact transformations required for plugin integration.

## Core Structure Transformations

### 1. Main Service Structure

#### Current: `Catalog` struct
```rust
// src/lib.rs:49-57
pub struct Catalog {
    consensus_config: Arc<ConsensusContext>,
    asset_registry: Arc<registry::AssetRegistry>,
    template_generator: Arc<template::CatalogTemplateGenerator>,
    asset_validator: Arc<validation::AssetValidator>,
    documentation_generator: Arc<documentation::DocumentationGenerator>,
    version_manager: Arc<versioning::VersionManager>,
    hypermesh_client: Arc<tokio::sync::Mutex<HyperMeshClient>>,
}
```

#### Future: `CatalogExtension` struct
```rust
pub struct CatalogExtension {
    // Extension metadata
    metadata: ExtensionMetadata,

    // Extension-specific components (kept)
    asset_registry: Arc<AssetRegistry>,
    template_generator: Arc<TemplateGenerator>,
    documentation_generator: Arc<DocumentationGenerator>,

    // New: Direct integration points
    asset_handlers: HashMap<AssetType, Box<dyn AssetExtensionHandler>>,

    // State management
    state: Arc<RwLock<ExtensionState>>,
    config: ExtensionConfig,
}
```

### 2. Configuration Structures

#### Current: `CatalogConfig`
```rust
// src/lib.rs:60-76
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogConfig {
    pub consensus: ConsensusContext,           // DELETE - use HyperMesh
    pub registry: registry::RegistryConfig,    // KEEP - extension-specific
    pub template: template::TemplateConfig,    // KEEP - extension-specific
    pub validation: validation::ValidationConfig, // PARTIAL - integrate
    pub documentation: documentation::DocumentationConfig, // KEEP
    pub hypermesh_address: Option<String>,     // DELETE - not needed
    pub trustchain_cert_path: Option<String>,  // DELETE - handled by extension
}
```

#### Future: Extension Configuration
```rust
// Uses standard ExtensionConfig from HyperMesh
pub struct CatalogExtensionSettings {
    pub registry: RegistryConfig,
    pub template: TemplateConfig,
    pub documentation: DocumentationConfig,
    pub cache_dir: PathBuf,
    pub max_package_size: u64,
    pub enable_p2p_distribution: bool,
}

// Wrapped in ExtensionConfig
ExtensionConfig {
    settings: serde_json::to_value(CatalogExtensionSettings { ... }),
    resource_limits: ResourceLimits { ... },
    granted_capabilities: HashSet<ExtensionCapability>,
    privacy_level: PrivacyLevel,
    debug_mode: bool,
}
```

## Consensus Structure Migrations

### Current Consensus Types (DELETE ALL)

These types are duplicates of HyperMesh core types and should be completely removed:

```rust
// src/consensus.rs - ALL TO BE DELETED
pub type AssetId = Uuid;                    // → Use HyperMesh AssetId
pub struct ExecutionResult { ... }          // → Use extension ExecutionResult
pub enum ConsensusProof { ... }            // → Use HyperMesh ConsensusProof
pub struct SpaceProof { ... }              // → Use HyperMesh SpaceProof
pub struct ResourceRequirements { ... }     // → Use extension ResourceRequirements
pub struct ConsensusContext { ... }        // → Use HyperMesh consensus context
pub struct NodeCapability { ... }          // → Internal to HyperMesh
pub trait ConsensusValidator { ... }       // → Use ConsensusAccess capability
```

### Migration Mapping

| Current Type | HyperMesh Type | Location |
|-------------|----------------|----------|
| `catalog::AssetId` | `hypermesh::AssetId` | `assets/core/mod.rs` |
| `catalog::ConsensusProof` | `hypermesh::ConsensusProof` | `assets/core/mod.rs` |
| `catalog::ResourceRequirements` | `extensions::ResourceRequirements` | `extensions/mod.rs:549` |
| `catalog::ExecutionResult` | `extensions::ExecutionResult` | `extensions/mod.rs:714` |

## Asset Structure Transformations

### Current: `AssetPackage`
```rust
// src/assets.rs
pub struct AssetPackage {
    pub id: AssetPackageId,
    pub name: String,
    pub version: String,
    pub spec: AssetSpec,
    pub metadata: AssetMetadata,
    pub content: AssetContent,
    pub security: AssetSecurity,
    pub resources: AssetResources,
    pub execution: Option<AssetExecution>,
    pub dependencies: Vec<AssetDependency>,
}
```

### Future: Extension `AssetPackage`
```rust
// Maps to extensions/mod.rs:1021
pub struct AssetPackage {
    pub id: String,                        // Simplified ID
    pub name: String,                       // Same
    pub version: semver::Version,          // Use semver
    pub description: String,                // From metadata
    pub author: String,                     // From metadata
    pub license: String,                    // From security
    pub asset_types: Vec<AssetType>,       // From spec
    pub size_bytes: u64,                   // From content
    pub install_count: u64,                // New tracking
    pub rating: f32,                        // New feature
    pub dependencies: Vec<PackageDependency>, // Transformed
    pub signature: Option<String>,         // From security
    pub distribution_hash: String,         // For STOQ P2P
    pub metadata: HashMap<String, Value>,  // Flexible metadata
}
```

### Transformation Logic
```rust
impl From<OldAssetPackage> for NewAssetPackage {
    fn from(old: OldAssetPackage) -> Self {
        Self {
            id: old.id.to_string(),
            name: old.name,
            version: Version::parse(&old.version).unwrap(),
            description: old.metadata.description,
            author: old.metadata.author,
            license: old.security.license,
            asset_types: vec![old.spec.asset_type],
            size_bytes: old.content.size,
            install_count: 0,
            rating: 0.0,
            dependencies: old.dependencies.into_iter()
                .map(|d| PackageDependency {
                    package_id: d.package_id,
                    version_req: VersionReq::parse(&d.version).unwrap(),
                    optional: d.optional,
                })
                .collect(),
            signature: old.security.signature,
            distribution_hash: old.content.hash,
            metadata: old.metadata.custom_fields,
        }
    }
}
```

## Registry Structure Migrations

### Current: `AssetRegistry`
```rust
// src/registry.rs:17
pub struct AssetRegistry {
    config: RegistryConfig,
    local_index: Arc<RwLock<AssetIndex>>,
    remote_registries: HashMap<String, Box<dyn RegistryClient>>,
    cache_dir: PathBuf,
}
```

### Future: Internal Extension State
```rust
struct CatalogRegistryState {
    local_index: Arc<RwLock<PackageIndex>>,
    remote_peers: Vec<PeerRegistry>,  // P2P via STOQ
    cache: Arc<PackageCache>,
    distribution: Arc<STOQDistribution>,
}
```

### Current: `AssetIndexEntry`
```rust
// src/registry.rs:119
pub struct AssetIndexEntry {
    pub id: AssetPackageId,
    pub name: String,
    pub version: String,
    pub asset_type: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    pub location: String,
    pub size: u64,
    pub hash: String,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub registry: String,
    pub rating: f64,
    pub download_count: u64,
    pub verified: bool,
}
```

### Future: Simplified Internal Index
```rust
struct PackageIndexEntry {
    pub package: AssetPackage,          // Full package info
    pub local_path: Option<PathBuf>,    // Local cache path
    pub remote_sources: Vec<String>,    // STOQ peer addresses
    pub consensus_proof: ConsensusProof, // Validation proof
    pub indexed_at: SystemTime,         // Index time
}
```

## Validation Structure Migrations

### Current: Validation Components
```rust
// src/validation/validator.rs
pub struct AssetValidator {
    config: ValidationConfig,
    validators: Vec<Box<dyn Validator>>,
    scanners: Vec<Box<dyn Scanner>>,
}
```

### Future: Integrated with Extension Handler
```rust
impl AssetExtensionHandler for CatalogAssetHandler {
    async fn validate_asset(&self,
        id: &AssetId,
        proof: ConsensusProof
    ) -> ExtensionResult<bool> {
        // Validation integrated into handler
        // Uses HyperMesh consensus validation
        self.validate_with_consensus(id, proof).await
    }
}
```

## HyperMesh Integration Structure Changes

### Current: Remote Integration
```rust
// src/hypermesh_integration.rs
pub struct HyperMeshClient {
    network_address: String,
    trustchain_cert_path: Option<String>,
    asset_adapter: HyperMeshAssetAdapter,
}

pub struct CatalogExecutionContext {
    pub execution_id: String,
    pub asset_id: AssetId,
    pub allocated_resources: Vec<HyperMeshResource>,
    pub status: ExecutionStatus,
    // ...
}
```

### Future: Direct Integration (DELETE CLIENT)
```rust
// No client needed - direct access through extension
// Execution handled through AssetOperation

impl AssetExtensionHandler for CatalogAssetHandler {
    async fn handle_operation(&self,
        id: &AssetId,
        operation: AssetOperation
    ) -> ExtensionResult<OperationResult> {
        match operation {
            AssetOperation::Deploy(spec) => {
                // Direct deployment through HyperMesh
                self.deploy_asset(id, spec).await
            },
            AssetOperation::Execute(spec) => {
                // Direct execution through VM subsystem
                self.execute_asset(id, spec).await
            },
            // ...
        }
    }
}
```

## API Request/Response Transformations

### Current: Direct API Calls
```rust
// Current API methods
pub async fn publish_asset(&self, package: AssetPackage) -> Result<AssetId>
pub async fn install_asset(&self, id: &AssetId) -> Result<AssetPackage>
pub async fn search_assets(&self, query: &SearchQuery) -> Result<SearchResults>
```

### Future: Extension Request Handling
```rust
impl HyperMeshExtension for CatalogExtension {
    async fn handle_request(&self,
        request: ExtensionRequest
    ) -> ExtensionResult<ExtensionResponse> {
        match request.method.as_str() {
            "publish" => {
                let spec: AssetPackageSpec = serde_json::from_value(request.params)?;
                let result = self.publish_package(spec, request.consensus_proof).await?;
                Ok(ExtensionResponse {
                    request_id: request.id,
                    success: true,
                    data: Some(serde_json::to_value(result)?),
                    error: None,
                })
            },
            "install" => { /* ... */ },
            "search" => { /* ... */ },
            _ => Err(ExtensionError::RuntimeError {
                message: format!("Unknown method: {}", request.method)
            })
        }
    }
}
```

## State Management Transformations

### Current: Multiple State Components
```rust
// Scattered across modules
consensus_config: Arc<ConsensusContext>
asset_registry: Arc<AssetRegistry>
template_generator: Arc<TemplateGenerator>
// etc.
```

### Future: Unified Extension State
```rust
pub struct CatalogExtensionState {
    // Serializable state for export/import
    pub version: u32,
    pub packages: HashMap<String, AssetPackage>,
    pub installations: HashMap<AssetId, InstallationRecord>,
    pub cache_entries: Vec<CacheEntry>,
    pub peer_registries: Vec<PeerRegistry>,
    pub statistics: ExtensionStatistics,
}

impl CatalogExtension {
    async fn export_state(&self) -> ExtensionResult<ExtensionState> {
        let state_data = self.serialize_state().await?;
        Ok(ExtensionState {
            version: 1,
            metadata: self.metadata.clone(),
            state_data,
            checksum: calculate_checksum(&state_data),
            exported_at: SystemTime::now(),
        })
    }
}
```

## Dependency Structure Changes

### Current: Custom Dependency System
```rust
pub struct AssetDependency {
    pub package_id: String,
    pub version: String,
    pub optional: bool,
    pub resolution: DependencyResolution,
}
```

### Future: Standard Semver Dependencies
```rust
// Use extension standard
pub struct PackageDependency {
    pub package_id: String,
    pub version_req: semver::VersionReq,  // Standard version requirements
    pub optional: bool,
}

// Extension dependencies
pub struct ExtensionDependency {
    pub extension_id: String,
    pub version_requirement: semver::VersionReq,
    pub optional: bool,
}
```

## Error Type Transformations

### Current: Custom Error Types
```rust
// Various error types across modules
pub enum CatalogError {
    ValidationFailed(String),
    PublishError(String),
    InstallError(String),
    // ...
}
```

### Future: Unified Extension Errors
```rust
// Use ExtensionError from HyperMesh
pub enum ExtensionError {
    ExtensionNotFound { id: String },
    DependencyResolutionFailed { extension: String, dependency: String },
    VersionIncompatible { extension: String, required: String, found: String },
    CapabilityNotGranted { capability: String },
    ResourceLimitExceeded { resource: String },
    ConsensusValidationFailed { reason: String },
    RuntimeError { message: String },
    Internal(#[from] anyhow::Error),
}
```

## Migration Utilities

### Data Migration Helper
```rust
pub struct CatalogDataMigrator {
    pub fn migrate_package(old: OldAssetPackage) -> NewAssetPackage { ... }
    pub fn migrate_config(old: CatalogConfig) -> ExtensionConfig { ... }
    pub fn migrate_registry(old: AssetRegistry) -> CatalogRegistryState { ... }
    pub fn migrate_index(old: AssetIndex) -> PackageIndex { ... }
}
```

### Backward Compatibility Shims
```rust
// For gradual migration
pub trait LegacyAdapter {
    fn adapt_old_request(req: OldApiRequest) -> ExtensionRequest;
    fn adapt_new_response(resp: ExtensionResponse) -> OldApiResponse;
}
```

## Summary of Changes

### Structures to Delete (100% removal)
- All consensus-related structures (duplicates)
- HyperMeshClient and related networking
- Custom version types (use semver)
- Blockchain client interfaces

### Structures to Transform (modify)
- AssetPackage → Simplified, standard format
- AssetRegistry → Internal extension state
- Validation structures → Integrated handlers
- Configuration → Extension-specific settings

### Structures to Keep (as-is)
- Template generation structures
- Documentation structures
- Search indexing structures
- Cache management structures

### New Structures to Add
- CatalogExtension main class
- Asset handlers for each type
- Extension state management
- P2P distribution via STOQ

## Migration Checklist

- [ ] Remove all consensus structure duplicates
- [ ] Transform AssetPackage to extension format
- [ ] Convert registry to internal state
- [ ] Implement extension trait methods
- [ ] Create asset handlers
- [ ] Migrate configuration format
- [ ] Update error handling
- [ ] Implement state export/import
- [ ] Add P2P distribution via STOQ
- [ ] Create backward compatibility layer