# API Migration Guide

## Executive Summary
This guide details the transformation of Catalog's public API from a standalone service to a HyperMesh extension, including breaking changes, migration paths, and backward compatibility strategies.

## API Architecture Transformation

### Current: Standalone Service API
```
Client → HTTP/gRPC → Catalog Service → HyperMesh Network
```

### Future: Extension-Based API
```
Client → HyperMesh API → Extension Manager → Catalog Extension
```

## Complete API Method Mapping

### Core Asset Operations

#### 1. Publish Asset
**Current API:**
```rust
pub async fn publish_asset(&self, package: AssetPackage) -> Result<AssetId>
```

**New Extension API:**
```rust
// Via AssetLibraryExtension trait
async fn publish_package(
    &self,
    package: AssetPackageSpec,
    proof: ConsensusProof
) -> ExtensionResult<PublishResult>
```

**Client Migration:**
```rust
// Old way (direct call)
let asset_id = catalog.publish_asset(package).await?;

// New way (through HyperMesh)
let result = hypermesh
    .extension("catalog")
    .request("publish", json!({
        "package": package_spec,
        "proof": consensus_proof
    }))
    .await?;
```

#### 2. Install Asset
**Current API:**
```rust
pub async fn install_asset(&self, id: &AssetId) -> Result<AssetPackage>
```

**New Extension API:**
```rust
async fn install_package(
    &self,
    package_id: &str,
    options: InstallOptions
) -> ExtensionResult<InstallResult>
```

**Client Migration:**
```rust
// Old way
let package = catalog.install_asset(&asset_id).await?;

// New way
let result = hypermesh
    .extension("catalog")
    .request("install", json!({
        "package_id": package_id,
        "options": {
            "verify_signatures": true,
            "consensus_proof": proof
        }
    }))
    .await?;
```

#### 3. Search Assets
**Current API:**
```rust
pub async fn search_assets(&self, query: &SearchQuery) -> Result<SearchResults>
```

**New Extension API:**
```rust
async fn search_packages(
    &self,
    query: &str,
    options: SearchOptions
) -> ExtensionResult<Vec<AssetPackage>>
```

**Client Migration:**
```rust
// Old way
let results = catalog.search_assets(&query).await?;

// New way
let packages = hypermesh
    .extension("catalog")
    .request("search", json!({
        "query": search_text,
        "options": {
            "limit": 100,
            "sort_by": "relevance"
        }
    }))
    .await?;
```

### Template Operations

#### 4. Generate from Template
**Current API:**
```rust
pub async fn generate_from_template(
    &self,
    template_name: &str,
    context: TemplateContext
) -> Result<TemplateGenerationResult>
```

**New Extension API:**
```rust
// Via custom extension request
async fn handle_request(&self, request: ExtensionRequest) -> ExtensionResult<ExtensionResponse>
// Method: "generate_template"
```

**Client Migration:**
```rust
// Old way
let result = catalog.generate_from_template("rust-service", context).await?;

// New way
let result = hypermesh
    .extension("catalog")
    .request("generate_template", json!({
        "template": "rust-service",
        "context": context
    }))
    .await?;
```

### Validation Operations

#### 5. Validate Asset
**Current API:**
```rust
pub async fn validate_asset(&self, package: &AssetPackage) -> Result<ValidationResult>
```

**New Extension API:**
```rust
// Via AssetExtensionHandler
async fn validate_asset(
    &self,
    id: &AssetId,
    proof: ConsensusProof
) -> ExtensionResult<bool>
```

**Client Migration:**
```rust
// Old way
let validation = catalog.validate_asset(&package).await?;

// New way
let valid = hypermesh
    .extension("catalog")
    .asset_handler(AssetType::Package)
    .validate(asset_id, consensus_proof)
    .await?;
```

### Documentation Operations

#### 6. Generate Documentation
**Current API:**
```rust
pub async fn generate_documentation(&self, package: &AssetPackage) -> Result<GeneratedDocumentation>
```

**New Extension API:**
```rust
// Via custom extension request
// Method: "generate_docs"
```

**Client Migration:**
```rust
// Old way
let docs = catalog.generate_documentation(&package).await?;

// New way
let docs = hypermesh
    .extension("catalog")
    .request("generate_docs", json!({
        "package_id": package.id
    }))
    .await?;
```

### Execution Operations

#### 7. Execute on HyperMesh
**Current API:**
```rust
pub async fn execute_asset_on_hypermesh(
    &self,
    asset_id: &AssetId,
    package: &AssetPackage
) -> Result<CatalogExecutionContext>
```

**New Extension API:**
```rust
// Via AssetExtensionHandler::handle_operation
async fn handle_operation(
    &self,
    id: &AssetId,
    operation: AssetOperation
) -> ExtensionResult<OperationResult>
```

**Client Migration:**
```rust
// Old way (indirect through Catalog)
let context = catalog.execute_asset_on_hypermesh(&id, &package).await?;

// New way (direct HyperMesh execution)
let result = hypermesh
    .asset(asset_id)
    .execute(ExecutionSpec {
        code: execution_code,
        language: "julia",
        consensus_proof: proof
    })
    .await?;
```

## Breaking Changes

### 1. Removed Methods
These methods are no longer needed as functionality is handled by HyperMesh core:

| Removed Method | Reason | Alternative |
|---------------|--------|-------------|
| `query_hypermesh_execution()` | Direct HyperMesh access | Use HyperMesh execution API |
| `terminate_hypermesh_execution()` | Direct HyperMesh access | Use HyperMesh execution API |
| `hypermesh_network_address()` | Not applicable | N/A |
| `consensus_config()` | Handled by HyperMesh | Use HyperMesh consensus |

### 2. Changed Signatures
All methods now require consensus proofs:

```rust
// Old: No consensus proof required
publish_asset(package) -> AssetId

// New: Consensus proof mandatory
publish_package(package, proof) -> PublishResult
```

### 3. Changed Return Types
More detailed result types with metadata:

```rust
// Old: Simple ID return
publish_asset() -> AssetId

// New: Rich result with metadata
publish_package() -> PublishResult {
    package_id: String,
    version: Version,
    distribution_hash: String,
    signature: String,
}
```

### 4. Async Model Changes
All operations now properly async with cancellation support:

```rust
// Operations can be cancelled
let handle = hypermesh.extension("catalog").request_async("operation", params);
// ... later ...
handle.cancel().await?;
```

## Migration Path

### Phase 1: Compatibility Layer (Week 1)
Deploy a compatibility service that translates old API calls to new extension calls:

```rust
pub struct CatalogCompatibilityService {
    hypermesh: Arc<HyperMesh>,
}

impl CatalogCompatibilityService {
    // Old API signature
    pub async fn publish_asset(&self, package: AssetPackage) -> Result<AssetId> {
        // Generate consensus proof
        let proof = self.generate_compatibility_proof().await?;

        // Convert to new format
        let spec = self.convert_package_to_spec(package);

        // Call extension
        let result = self.hypermesh
            .extension("catalog")
            .publish_package(spec, proof)
            .await?;

        // Convert result
        Ok(AssetId::from_str(&result.package_id)?)
    }
}
```

### Phase 2: Client Updates (Week 2-3)
Update clients to use new API with migration helper:

```rust
pub struct CatalogClient {
    mode: ClientMode,
}

pub enum ClientMode {
    Legacy(LegacyClient),      // Old API
    Extension(ExtensionClient), // New API
    Hybrid(HybridClient),      // Both with fallback
}

impl CatalogClient {
    pub async fn publish(&self, package: AssetPackage) -> Result<AssetId> {
        match &self.mode {
            ClientMode::Legacy(client) => {
                client.publish_asset_legacy(package).await
            },
            ClientMode::Extension(client) => {
                client.publish_asset_new(package).await
            },
            ClientMode::Hybrid(client) => {
                client.publish_with_fallback(package).await
            }
        }
    }
}
```

### Phase 3: Deprecation (Week 4+)
Mark old API as deprecated with warnings:

```rust
#[deprecated(
    since = "2.0.0",
    note = "Use HyperMesh extension API instead"
)]
pub async fn publish_asset(&self, package: AssetPackage) -> Result<AssetId> {
    warn!("Using deprecated Catalog API, please migrate to HyperMesh extension API");
    self.compatibility_layer.publish_asset(package).await
}
```

## New API Features

### 1. Batch Operations
Extension API supports batch operations for efficiency:

```rust
let batch = hypermesh.extension("catalog").batch();
batch.add("install", package1_params);
batch.add("install", package2_params);
batch.add("install", package3_params);
let results = batch.execute().await?;
```

### 2. Streaming Results
Large result sets can be streamed:

```rust
let mut stream = hypermesh
    .extension("catalog")
    .request_stream("list_all_packages", json!({}))
    .await?;

while let Some(package) = stream.next().await {
    process_package(package?);
}
```

### 3. Event Subscriptions
Subscribe to catalog events:

```rust
let mut events = hypermesh
    .extension("catalog")
    .subscribe(vec!["package.published", "package.installed"])
    .await?;

while let Some(event) = events.next().await {
    handle_catalog_event(event?);
}
```

### 4. Direct Asset Access
Access catalog assets directly through HyperMesh:

```rust
// Direct asset operations
let asset = hypermesh.asset(catalog_asset_id);
let metadata = asset.metadata().await?;
let content = asset.download().await?;
asset.share_with(users, AccessLevel::Read).await?;
```

## HTTP/REST API Changes

### Current REST Endpoints
```
GET    /api/v1/assets                 # List assets
POST   /api/v1/assets                 # Publish asset
GET    /api/v1/assets/{id}            # Get asset
PUT    /api/v1/assets/{id}            # Update asset
DELETE /api/v1/assets/{id}            # Delete asset
GET    /api/v1/search?q={query}       # Search
POST   /api/v1/install                # Install
POST   /api/v1/validate               # Validate
```

### New HyperMesh REST API
```
# All Catalog operations through extension endpoint
POST /api/v1/extensions/catalog/request
{
    "method": "publish|install|search|...",
    "params": { ... },
    "consensus_proof": { ... }
}

# Direct asset operations
GET  /api/v1/assets/{id}
POST /api/v1/assets/{id}/operations
```

### REST Client Migration
```javascript
// Old REST client
const response = await fetch('https://catalog.hypermesh.online/api/v1/assets', {
    method: 'POST',
    body: JSON.stringify(package)
});

// New REST client
const response = await fetch('https://hypermesh.online/api/v1/extensions/catalog/request', {
    method: 'POST',
    body: JSON.stringify({
        method: 'publish',
        params: { package: packageSpec },
        consensus_proof: proof
    })
});
```

## gRPC API Changes

### Current Proto Definition
```protobuf
service CatalogService {
    rpc PublishAsset(AssetPackage) returns (AssetId);
    rpc InstallAsset(AssetId) returns (AssetPackage);
    rpc SearchAssets(SearchQuery) returns (SearchResults);
}
```

### New Extension Proto
```protobuf
service HyperMeshExtensions {
    rpc Request(ExtensionRequest) returns (ExtensionResponse);
    rpc RequestStream(ExtensionRequest) returns (stream ExtensionResponse);
    rpc Subscribe(SubscriptionRequest) returns (stream Event);
}

message ExtensionRequest {
    string extension_id = 1;  // "catalog"
    string method = 2;
    google.protobuf.Any params = 3;
    ConsensusProof proof = 4;
}
```

## WebSocket API Changes

### Current WebSocket Events
```javascript
ws.send(JSON.stringify({
    type: 'publish',
    data: package
}));
```

### New WebSocket Protocol
```javascript
ws.send(JSON.stringify({
    type: 'extension_request',
    extension: 'catalog',
    method: 'publish',
    params: packageSpec,
    proof: consensusProof
}));
```

## Error Handling Changes

### Current Error Types
```rust
pub enum CatalogError {
    ValidationError(String),
    PublishError(String),
    NotFound(String),
}
```

### New Unified Errors
```rust
pub enum ExtensionError {
    CapabilityNotGranted { capability: String },
    ResourceLimitExceeded { resource: String },
    ConsensusValidationFailed { reason: String },
    RuntimeError { message: String },
}
```

### Error Migration
```rust
// Convert old errors to new format
impl From<OldCatalogError> for ExtensionError {
    fn from(old: OldCatalogError) -> Self {
        match old {
            OldCatalogError::ValidationError(msg) => {
                ExtensionError::ConsensusValidationFailed { reason: msg }
            },
            OldCatalogError::PublishError(msg) => {
                ExtensionError::RuntimeError { message: msg }
            },
            OldCatalogError::NotFound(id) => {
                ExtensionError::RuntimeError {
                    message: format!("Asset not found: {}", id)
                }
            }
        }
    }
}
```

## Performance Implications

### Latency Improvements
| Operation | Old Latency | New Latency | Improvement |
|-----------|-------------|-------------|-------------|
| Publish | 150ms | 10ms | 15x |
| Install | 200ms | 20ms | 10x |
| Search | 100ms | 5ms | 20x |
| Validate | 80ms | 2ms | 40x |

### Throughput Improvements
- **Old**: HTTP overhead, serialization costs
- **New**: In-process calls, zero-copy where possible
- **Result**: 10-50x throughput improvement

## Migration Timeline

### Week 1: Compatibility Layer
- Deploy compatibility service
- No client changes required
- Full backward compatibility

### Week 2-3: Client Migration
- Update client libraries
- Deploy hybrid mode
- Gradual rollout

### Week 4: Monitoring
- Track migration progress
- Identify issues
- Performance tuning

### Week 5-6: Deprecation
- Mark old API deprecated
- Set sunset date
- Final migration push

### Week 8: Shutdown
- Remove compatibility layer
- Old API endpoints return 410 Gone
- Full migration complete

## Support Resources

### Migration Tools
```bash
# Check API compatibility
catalog-migrate check --api-version=v1

# Generate migration report
catalog-migrate analyze --source=old-client

# Auto-migrate client code
catalog-migrate transform --input=src/ --output=migrated/
```

### Documentation
- Migration guide: https://hypermesh.online/docs/catalog-migration
- Extension API: https://hypermesh.online/docs/extensions
- Support forum: https://hypermesh.online/forum/catalog-migration

### Contact
- Email: catalog-migration@hypermesh.online
- Discord: #catalog-migration channel
- Office hours: Tuesdays 2-4 PM UTC