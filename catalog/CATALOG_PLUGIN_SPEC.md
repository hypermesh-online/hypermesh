# Catalog Plugin Implementation Specification

## Overview

This document provides the complete implementation specification for the `CatalogExtension` struct that integrates Catalog as a lightweight HyperMesh plugin. The design leverages HyperMesh's extension system while maintaining high performance and minimal resource usage.

## Plugin Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                        CatalogExtension                         │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │
│  │   Extension  │  │    Asset     │  │   Library    │        │
│  │   Interface  │  │   Handlers   │  │   Manager    │        │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘        │
│         │                  │                  │                 │
│  ┌──────┴──────────────────┴──────────────────┴──────┐        │
│  │           State Management & Configuration          │        │
│  └──────────────────────┬──────────────────────────┘          │
│                         │                                       │
│  ┌──────────────────────┴──────────────────────────┐          │
│  │              HyperMesh Integration               │          │
│  │                                                  │          │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐     │          │
│  │  │  Asset   │  │ Consensus│  │ Transport│     │          │
│  │  │  Manager │  │  (NKrypt)│  │  (STOQ)  │     │          │
│  │  └──────────┘  └──────────┘  └──────────┘     │          │
│  └──────────────────────────────────────────────┘            │
└─────────────────────────────────────────────────────────────────┘
```

## Core Implementation

### 1. CatalogExtension Structure

```rust
use async_trait::async_trait;
use hypermesh::extensions::{
    HyperMeshExtension, ExtensionMetadata, ExtensionConfig,
    ExtensionResult, ExtensionError, ExtensionRequest,
    ExtensionResponse, ExtensionStatus, ExtensionState,
    ValidationReport, AssetExtensionHandler, AssetLibraryExtension,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Catalog extension for HyperMesh
pub struct CatalogExtension {
    /// Extension metadata
    metadata: ExtensionMetadata,

    /// Extension configuration
    config: Arc<RwLock<CatalogConfig>>,

    /// Extension state
    state: Arc<RwLock<ExtensionStateManager>>,

    /// Asset library core
    library: Arc<AssetLibrary>,

    /// Distribution layer
    distribution: Arc<DistributionLayer>,

    /// Asset handlers registry
    handlers: Arc<RwLock<HandlerRegistry>>,

    /// HyperMesh integration
    hypermesh: Arc<HyperMeshIntegration>,

    /// Performance monitor
    monitor: Arc<PerformanceMonitor>,

    /// Error handler
    error_handler: Arc<ErrorHandler>,
}

impl CatalogExtension {
    /// Create new Catalog extension
    pub fn new() -> Self {
        let metadata = ExtensionMetadata {
            id: "io.hypermesh.catalog".to_string(),
            name: "Catalog Asset Library".to_string(),
            version: semver::Version::parse("2.0.0").unwrap(),
            description: "Decentralized asset library and package management for HyperMesh".to_string(),
            author: "HyperMesh Team".to_string(),
            license: "Apache-2.0".to_string(),
            homepage: Some("https://hypermesh.online/catalog".to_string()),
            category: ExtensionCategory::AssetLibrary,
            hypermesh_version: semver::Version::parse("1.0.0").unwrap(),
            dependencies: vec![
                ExtensionDependency {
                    extension_id: "io.hypermesh.stoq".to_string(),
                    version_requirement: semver::VersionReq::parse(">=1.0.0").unwrap(),
                    optional: false,
                },
                ExtensionDependency {
                    extension_id: "io.hypermesh.trustchain".to_string(),
                    version_requirement: semver::VersionReq::parse(">=1.0.0").unwrap(),
                    optional: false,
                },
            ],
            required_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::NetworkAccess,
                ExtensionCapability::ConsensusAccess,
                ExtensionCapability::TransportAccess,
                ExtensionCapability::FileSystemAccess,
            ]),
            provided_assets: vec![
                AssetType::Package,
                AssetType::Library,
                AssetType::Template,
                AssetType::VirtualMachine,
                AssetType::Container,
            ],
            certificate_fingerprint: Some("SHA256:catalog_cert_fingerprint".to_string()),
            config_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "cache_size": { "type": "integer", "minimum": 0 },
                    "max_package_size": { "type": "integer", "minimum": 1024 },
                    "enable_mirroring": { "type": "boolean" },
                    "replication_factor": { "type": "integer", "minimum": 1, "maximum": 10 }
                }
            })),
        };

        Self {
            metadata,
            config: Arc::new(RwLock::new(CatalogConfig::default())),
            state: Arc::new(RwLock::new(ExtensionStateManager::new())),
            library: Arc::new(AssetLibrary::new()),
            distribution: Arc::new(DistributionLayer::new()),
            handlers: Arc::new(RwLock::new(HandlerRegistry::new())),
            hypermesh: Arc::new(HyperMeshIntegration::new()),
            monitor: Arc::new(PerformanceMonitor::new()),
            error_handler: Arc::new(ErrorHandler::new()),
        }
    }
}
```

### 2. HyperMeshExtension Implementation

```rust
#[async_trait]
impl HyperMeshExtension for CatalogExtension {
    fn metadata(&self) -> ExtensionMetadata {
        self.metadata.clone()
    }

    async fn initialize(&mut self, config: ExtensionConfig) -> ExtensionResult<()> {
        // Update state to initializing
        self.state.write().await.transition(ExtensionState::Initializing)?;

        // Parse and validate configuration
        let catalog_config = self.parse_config(&config)?;
        *self.config.write().await = catalog_config;

        // Initialize components
        self.initialize_components(&config).await?;

        // Register asset handlers
        self.register_handlers().await?;

        // Start background services
        self.start_services().await?;

        // Transition to running state
        self.state.write().await.transition(ExtensionState::Running)?;

        Ok(())
    }

    async fn register_assets(&self) -> ExtensionResult<HashMap<AssetType, Box<dyn AssetExtensionHandler>>> {
        let mut handlers = HashMap::new();

        // Package handler
        handlers.insert(
            AssetType::Package,
            Box::new(PackageAssetHandler::new(self.library.clone())) as Box<dyn AssetExtensionHandler>
        );

        // Library handler
        handlers.insert(
            AssetType::Library,
            Box::new(LibraryAssetHandler::new(self.library.clone())) as Box<dyn AssetExtensionHandler>
        );

        // Template handler
        handlers.insert(
            AssetType::Template,
            Box::new(TemplateAssetHandler::new(self.library.clone())) as Box<dyn AssetExtensionHandler>
        );

        // VM handler
        handlers.insert(
            AssetType::VirtualMachine,
            Box::new(VMAssetHandler::new(self.library.clone())) as Box<dyn AssetExtensionHandler>
        );

        // Container handler
        handlers.insert(
            AssetType::Container,
            Box::new(ContainerAssetHandler::new(self.library.clone())) as Box<dyn AssetExtensionHandler>
        );

        Ok(handlers)
    }

    async fn extend_manager(&self, asset_manager: Arc<AssetManager>) -> ExtensionResult<()> {
        // Store reference to asset manager
        self.hypermesh.set_asset_manager(asset_manager.clone()).await;

        // Register catalog-specific operations
        asset_manager.register_operation(
            "catalog.search",
            Arc::new(SearchOperation::new(self.library.clone()))
        ).await?;

        asset_manager.register_operation(
            "catalog.install",
            Arc::new(InstallOperation::new(self.library.clone()))
        ).await?;

        asset_manager.register_operation(
            "catalog.publish",
            Arc::new(PublishOperation::new(self.library.clone()))
        ).await?;

        asset_manager.register_operation(
            "catalog.update",
            Arc::new(UpdateOperation::new(self.library.clone()))
        ).await?;

        Ok(())
    }

    async fn handle_request(&self, request: ExtensionRequest) -> ExtensionResult<ExtensionResponse> {
        // Record request for monitoring
        self.monitor.record_request(&request).await;

        let response = match request.method.as_str() {
            "search" => self.handle_search(request).await,
            "install" => self.handle_install(request).await,
            "publish" => self.handle_publish(request).await,
            "update" => self.handle_update(request).await,
            "list" => self.handle_list(request).await,
            "info" => self.handle_info(request).await,
            "verify" => self.handle_verify(request).await,
            "mirror" => self.handle_mirror(request).await,
            _ => Err(ExtensionError::RuntimeError {
                message: format!("Unknown method: {}", request.method),
            }),
        };

        match response {
            Ok(resp) => Ok(resp),
            Err(e) => {
                self.error_handler.handle_error(&e).await;
                Ok(ExtensionResponse {
                    request_id: request.id,
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    async fn status(&self) -> ExtensionStatus {
        let state = self.state.read().await;
        let stats = self.monitor.get_statistics().await;

        ExtensionStatus {
            state: state.current_state(),
            health: state.health_status(),
            resource_usage: stats.resource_usage,
            active_operations: state.active_operations(),
            total_requests: stats.total_requests,
            error_count: stats.error_count,
            uptime: state.uptime(),
        }
    }

    async fn validate(&self) -> ExtensionResult<ValidationReport> {
        let mut report = ValidationReport {
            valid: true,
            certificate_valid: None,
            dependencies_satisfied: true,
            resource_compliance: true,
            security_compliance: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Validate certificate
        if let Some(fingerprint) = &self.metadata.certificate_fingerprint {
            report.certificate_valid = Some(
                self.hypermesh.verify_certificate(fingerprint).await?
            );
        }

        // Validate dependencies
        for dep in &self.metadata.dependencies {
            if !self.hypermesh.check_dependency(&dep).await? {
                report.dependencies_satisfied = false;
                report.errors.push(ValidationError {
                    code: "DEP_MISSING".to_string(),
                    message: format!("Missing dependency: {}", dep.extension_id),
                    context: None,
                });
            }
        }

        // Validate resource usage
        let usage = self.monitor.get_resource_usage().await;
        let limits = self.config.read().await.resource_limits;
        if !usage.within_limits(&limits) {
            report.resource_compliance = false;
            report.warnings.push(ValidationWarning {
                code: "RESOURCE_HIGH".to_string(),
                message: "Resource usage approaching limits".to_string(),
                context: Some(serde_json::to_value(usage).unwrap()),
            });
        }

        // Validate security
        let security_check = self.validate_security().await?;
        report.security_compliance = security_check.passed;
        if !security_check.passed {
            report.errors.extend(security_check.errors);
        }

        report.valid = report.dependencies_satisfied &&
                      report.resource_compliance &&
                      report.security_compliance;

        Ok(report)
    }

    async fn export_state(&self) -> ExtensionResult<ExtensionState> {
        let state = self.state.read().await;
        let library_state = self.library.export_state().await?;
        let config = self.config.read().await;

        let state_data = StateData {
            version: 1,
            config: config.clone(),
            library: library_state,
            statistics: self.monitor.export_stats().await,
            timestamp: SystemTime::now(),
        };

        let serialized = bincode::serialize(&state_data)
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to serialize state: {}", e),
            })?;

        Ok(ExtensionState {
            version: 1,
            metadata: self.metadata.clone(),
            state_data: serialized,
            checksum: calculate_checksum(&serialized),
            exported_at: SystemTime::now(),
        })
    }

    async fn import_state(&mut self, state: ExtensionState) -> ExtensionResult<()> {
        // Verify state version compatibility
        if state.version != 1 {
            return Err(ExtensionError::RuntimeError {
                message: format!("Incompatible state version: {}", state.version),
            });
        }

        // Verify checksum
        if calculate_checksum(&state.state_data) != state.checksum {
            return Err(ExtensionError::RuntimeError {
                message: "State checksum verification failed".to_string(),
            });
        }

        // Deserialize state
        let state_data: StateData = bincode::deserialize(&state.state_data)
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to deserialize state: {}", e),
            })?;

        // Import configuration
        *self.config.write().await = state_data.config;

        // Import library state
        self.library.import_state(state_data.library).await?;

        // Import statistics
        self.monitor.import_stats(state_data.statistics).await;

        Ok(())
    }

    async fn shutdown(&mut self) -> ExtensionResult<()> {
        // Transition to shutting down state
        self.state.write().await.transition(ExtensionState::ShuttingDown)?;

        // Stop background services
        self.stop_services().await?;

        // Flush pending operations
        self.flush_operations().await?;

        // Save state
        let state = self.export_state().await?;
        self.save_state(state).await?;

        // Clean up resources
        self.cleanup().await?;

        // Transition to stopped state
        self.state.write().await.transition(ExtensionState::Stopped)?;

        Ok(())
    }
}
```

### 3. Asset Handlers Implementation

```rust
/// Package asset handler
pub struct PackageAssetHandler {
    library: Arc<AssetLibrary>,
}

#[async_trait]
impl AssetExtensionHandler for PackageAssetHandler {
    fn asset_type(&self) -> AssetType {
        AssetType::Package
    }

    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetId> {
        // Parse package specification
        let package_spec: PackageSpec = serde_json::from_value(spec.metadata.get("spec")
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: "Missing package specification".to_string(),
            })?
            .clone())?;

        // Build package
        let package = PackageBuilder::new(spec.name)
            .description(spec.description)
            .metadata(spec.metadata)
            .privacy_level(spec.privacy_level)
            .consensus(spec.consensus_requirements)
            .build()
            .await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to build package: {}", e),
            })?;

        // Store in library
        let package_id = self.library.store_package(package).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to store package: {}", e),
            })?;

        // Convert to AssetId
        Ok(AssetId::from(package_id))
    }

    async fn update_asset(&self, id: &AssetId, update: AssetUpdate) -> ExtensionResult<()> {
        let package_id = PackageId::from(id);

        // Get existing package
        let mut package = self.library.get_package(&package_id).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Package not found: {}", e),
            })?;

        // Apply updates
        if let Some(name) = update.name {
            package.metadata.name = name;
        }
        if let Some(description) = update.description {
            package.metadata.description = Some(description);
        }
        if let Some(metadata) = update.metadata {
            package.metadata.custom.extend(metadata);
        }
        if let Some(privacy) = update.privacy_level {
            package.access.privacy_level = privacy;
        }

        // Save updated package
        self.library.update_package(package).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to update package: {}", e),
            })?;

        Ok(())
    }

    async fn delete_asset(&self, id: &AssetId) -> ExtensionResult<()> {
        let package_id = PackageId::from(id);

        self.library.delete_package(&package_id).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Failed to delete package: {}", e),
            })?;

        Ok(())
    }

    async fn query_assets(&self, query: AssetQuery) -> ExtensionResult<Vec<AssetId>> {
        let packages = self.library.query_packages(LibraryQuery {
            name_pattern: query.name_pattern,
            tags: query.tags,
            privacy_level: query.privacy_level,
            limit: query.limit,
            offset: query.offset,
        }).await
        .map_err(|e| ExtensionError::RuntimeError {
            message: format!("Query failed: {}", e),
        })?;

        Ok(packages.into_iter()
            .map(|p| AssetId::from(p.id))
            .collect())
    }

    async fn get_metadata(&self, id: &AssetId) -> ExtensionResult<AssetMetadata> {
        let package_id = PackageId::from(id);
        let package = self.library.get_package(&package_id).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Package not found: {}", e),
            })?;

        Ok(AssetMetadata {
            id: id.clone(),
            asset_type: AssetType::Package,
            name: package.metadata.name,
            description: package.metadata.description,
            created_at: package.created_at,
            updated_at: package.updated_at,
            size_bytes: package.size,
            metadata: package.metadata.custom,
            privacy_level: package.access.privacy_level,
            allocation: None,
            consensus_status: package.consensus_status,
            tags: package.metadata.keywords,
        })
    }

    async fn validate_asset(&self, id: &AssetId, proof: ConsensusProof) -> ExtensionResult<bool> {
        let package_id = PackageId::from(id);

        // Validate package with consensus proof
        let validation_result = self.library.validate_package(&package_id, &proof).await
            .map_err(|e| ExtensionError::RuntimeError {
                message: format!("Validation failed: {}", e),
            })?;

        Ok(validation_result.is_valid)
    }

    async fn handle_operation(&self, id: &AssetId, operation: AssetOperation) -> ExtensionResult<OperationResult> {
        match operation {
            AssetOperation::Deploy(spec) => {
                let result = self.deploy_package(id, spec).await?;
                Ok(OperationResult::Deployed(result))
            }
            AssetOperation::Execute(spec) => {
                let result = self.execute_package(id, spec).await?;
                Ok(OperationResult::Executed(result))
            }
            AssetOperation::Transfer(spec) => {
                let result = self.transfer_package(id, spec).await?;
                Ok(OperationResult::Transferred(result))
            }
            AssetOperation::Share(spec) => {
                let result = self.share_package(id, spec).await?;
                Ok(OperationResult::Shared(result))
            }
            AssetOperation::Validate(proof) => {
                let valid = self.validate_asset(id, proof).await?;
                Ok(OperationResult::Validated(valid))
            }
            AssetOperation::Custom(value) => {
                let result = self.handle_custom_operation(id, value).await?;
                Ok(OperationResult::Custom(result))
            }
        }
    }
}
```

### 4. State Management

```rust
/// Extension state manager
pub struct ExtensionStateManager {
    /// Current state
    current_state: ExtensionState,

    /// State history
    history: Vec<StateTransition>,

    /// Active operations
    active_operations: HashMap<OperationId, Operation>,

    /// State persistence
    persistence: StatePersistence,

    /// State observers
    observers: Vec<Box<dyn StateObserver>>,
}

impl ExtensionStateManager {
    /// Transition to new state
    pub fn transition(&mut self, new_state: ExtensionState) -> Result<()> {
        // Validate transition
        if !self.is_valid_transition(&self.current_state, &new_state) {
            return Err(Error::InvalidStateTransition {
                from: self.current_state.clone(),
                to: new_state,
            });
        }

        // Record transition
        self.history.push(StateTransition {
            from: self.current_state.clone(),
            to: new_state.clone(),
            timestamp: SystemTime::now(),
        });

        // Update state
        let old_state = std::mem::replace(&mut self.current_state, new_state);

        // Notify observers
        for observer in &self.observers {
            observer.on_state_change(&old_state, &self.current_state);
        }

        // Persist state
        self.persistence.save_state(&self.current_state)?;

        Ok(())
    }

    /// Check if transition is valid
    fn is_valid_transition(&self, from: &ExtensionState, to: &ExtensionState) -> bool {
        match (from, to) {
            (ExtensionState::Initializing, ExtensionState::Running) => true,
            (ExtensionState::Running, ExtensionState::Paused) => true,
            (ExtensionState::Running, ExtensionState::ShuttingDown) => true,
            (ExtensionState::Paused, ExtensionState::Running) => true,
            (ExtensionState::Paused, ExtensionState::ShuttingDown) => true,
            (ExtensionState::ShuttingDown, ExtensionState::Stopped) => true,
            (_, ExtensionState::Error(_)) => true, // Can error from any state
            _ => false,
        }
    }

    /// Track operation
    pub fn track_operation(&mut self, operation: Operation) -> OperationId {
        let id = OperationId::new();
        self.active_operations.insert(id.clone(), operation);
        id
    }

    /// Complete operation
    pub fn complete_operation(&mut self, id: &OperationId) -> Option<Operation> {
        self.active_operations.remove(id)
    }

    /// Get active operations count
    pub fn active_operations(&self) -> usize {
        self.active_operations.len()
    }

    /// Get current state
    pub fn current_state(&self) -> ExtensionState {
        self.current_state.clone()
    }

    /// Get health status
    pub fn health_status(&self) -> ExtensionHealth {
        match &self.current_state {
            ExtensionState::Running => {
                if self.active_operations.len() > 1000 {
                    ExtensionHealth::Degraded("High operation count".to_string())
                } else {
                    ExtensionHealth::Healthy
                }
            }
            ExtensionState::Error(msg) => ExtensionHealth::Unhealthy(msg.clone()),
            _ => ExtensionHealth::Healthy,
        }
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.history.first()
            .map(|t| SystemTime::now().duration_since(t.timestamp).unwrap_or_default())
            .unwrap_or_default()
    }
}
```

### 5. Configuration Management

```rust
/// Catalog configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogConfig {
    /// Cache configuration
    pub cache: CacheConfig,

    /// Distribution configuration
    pub distribution: DistributionConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Performance configuration
    pub performance: PerformanceConfig,

    /// Resource limits
    pub resource_limits: ResourceLimits,

    /// Feature flags
    pub features: FeatureFlags,
}

impl Default for CatalogConfig {
    fn default() -> Self {
        Self {
            cache: CacheConfig {
                max_size: 1024 * 1024 * 1024, // 1GB
                ttl: Duration::from_secs(3600),
                eviction_policy: EvictionPolicy::LRU,
            },
            distribution: DistributionConfig {
                replication_factor: 3,
                max_peers: 100,
                enable_mirroring: true,
                mirror_sync_interval: Duration::from_secs(3600),
            },
            security: SecurityConfig {
                verify_signatures: true,
                require_consensus: true,
                min_trust_score: 0.7,
                enable_sandboxing: true,
            },
            performance: PerformanceConfig {
                max_concurrent_operations: 1000,
                operation_timeout: Duration::from_secs(300),
                batch_size: 100,
                prefetch_count: 10,
            },
            resource_limits: ResourceLimits::default(),
            features: FeatureFlags {
                enable_p2p: true,
                enable_dht: true,
                enable_smart_caching: true,
                enable_predictive_prefetch: false,
                enable_experimental: false,
            },
        }
    }
}

/// Configuration validator
pub struct ConfigValidator {
    schema: JsonSchema,
}

impl ConfigValidator {
    /// Validate configuration
    pub fn validate(&self, config: &CatalogConfig) -> Result<ValidationResult> {
        let config_json = serde_json::to_value(config)?;

        let validation_result = self.schema.validate(&config_json);

        if let Err(errors) = validation_result {
            return Ok(ValidationResult {
                valid: false,
                errors: errors.into_iter().map(|e| e.to_string()).collect(),
                warnings: Vec::new(),
            });
        }

        // Additional semantic validation
        let mut warnings = Vec::new();

        if config.distribution.replication_factor < 2 {
            warnings.push("Low replication factor may impact availability".to_string());
        }

        if config.cache.max_size < 100 * 1024 * 1024 {
            warnings.push("Small cache size may impact performance".to_string());
        }

        Ok(ValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings,
        })
    }
}
```

### 6. Error Handling and Recovery

```rust
/// Error handler with recovery strategies
pub struct ErrorHandler {
    /// Error log
    error_log: Arc<RwLock<Vec<ErrorEntry>>>,

    /// Recovery strategies
    recovery_strategies: HashMap<ErrorType, Box<dyn RecoveryStrategy>>,

    /// Circuit breaker
    circuit_breaker: CircuitBreaker,

    /// Alert system
    alerter: Alerter,
}

impl ErrorHandler {
    /// Handle error with recovery
    pub async fn handle_error(&self, error: &ExtensionError) -> RecoveryResult {
        // Log error
        self.log_error(error).await;

        // Check circuit breaker
        if self.circuit_breaker.is_open() {
            return RecoveryResult::CircuitOpen;
        }

        // Determine error type
        let error_type = self.classify_error(error);

        // Apply recovery strategy
        if let Some(strategy) = self.recovery_strategies.get(&error_type) {
            match strategy.recover(error).await {
                Ok(()) => {
                    self.circuit_breaker.record_success();
                    RecoveryResult::Recovered
                }
                Err(e) => {
                    self.circuit_breaker.record_failure();

                    // Send alert for critical errors
                    if error_type.is_critical() {
                        self.alerter.send_alert(Alert {
                            level: AlertLevel::Critical,
                            message: format!("Recovery failed: {}", e),
                            error: Some(error.clone()),
                        }).await;
                    }

                    RecoveryResult::Failed(e)
                }
            }
        } else {
            RecoveryResult::NoStrategy
        }
    }

    /// Classify error type
    fn classify_error(&self, error: &ExtensionError) -> ErrorType {
        match error {
            ExtensionError::Network(_) => ErrorType::Network,
            ExtensionError::Storage(_) => ErrorType::Storage,
            ExtensionError::ConsensusValidationFailed { .. } => ErrorType::Consensus,
            ExtensionError::ResourceLimitExceeded { .. } => ErrorType::Resource,
            _ => ErrorType::Unknown,
        }
    }

    /// Log error
    async fn log_error(&self, error: &ExtensionError) {
        let entry = ErrorEntry {
            timestamp: SystemTime::now(),
            error: error.clone(),
            context: self.capture_context().await,
        };

        self.error_log.write().await.push(entry);

        // Rotate log if needed
        if self.error_log.read().await.len() > 10000 {
            self.rotate_log().await;
        }
    }
}

/// Recovery strategies
#[async_trait]
pub trait RecoveryStrategy: Send + Sync {
    async fn recover(&self, error: &ExtensionError) -> Result<()>;
}

/// Network error recovery
pub struct NetworkRecovery {
    retry_policy: RetryPolicy,
    fallback_endpoints: Vec<Endpoint>,
}

#[async_trait]
impl RecoveryStrategy for NetworkRecovery {
    async fn recover(&self, error: &ExtensionError) -> Result<()> {
        // Retry with exponential backoff
        for attempt in 0..self.retry_policy.max_attempts {
            tokio::time::sleep(self.retry_policy.delay(attempt)).await;

            // Try primary endpoint
            if self.test_connectivity().await {
                return Ok(());
            }

            // Try fallback endpoints
            for endpoint in &self.fallback_endpoints {
                if self.test_endpoint(endpoint).await {
                    self.switch_to_endpoint(endpoint).await?;
                    return Ok(());
                }
            }
        }

        Err(Error::RecoveryFailed)
    }
}
```

### 7. Performance Monitoring

```rust
/// Performance monitor
pub struct PerformanceMonitor {
    /// Metrics collector
    metrics: Arc<MetricsCollector>,

    /// Performance analyzer
    analyzer: PerformanceAnalyzer,

    /// Optimization engine
    optimizer: OptimizationEngine,

    /// Alert thresholds
    thresholds: PerformanceThresholds,
}

impl PerformanceMonitor {
    /// Record operation
    pub async fn record_operation(&self, operation: &str, duration: Duration, success: bool) {
        self.metrics.record(Metric::Operation {
            name: operation.to_string(),
            duration,
            success,
            timestamp: SystemTime::now(),
        }).await;

        // Analyze performance
        if let Some(issue) = self.analyzer.detect_issue(operation, duration).await {
            // Apply optimization
            self.optimizer.optimize(issue).await;
        }
    }

    /// Get performance statistics
    pub async fn get_statistics(&self) -> PerformanceStats {
        let metrics = self.metrics.get_recent(Duration::from_secs(3600)).await;

        PerformanceStats {
            avg_latency: self.calculate_avg_latency(&metrics),
            p95_latency: self.calculate_p95_latency(&metrics),
            p99_latency: self.calculate_p99_latency(&metrics),
            throughput: self.calculate_throughput(&metrics),
            error_rate: self.calculate_error_rate(&metrics),
            resource_usage: self.get_resource_usage().await,
            total_requests: metrics.len() as u64,
            error_count: metrics.iter().filter(|m| !m.is_success()).count() as u64,
        }
    }

    /// Detect performance anomalies
    pub async fn detect_anomalies(&self) -> Vec<PerformanceAnomaly> {
        self.analyzer.detect_anomalies(
            &self.metrics.get_recent(Duration::from_secs(300)).await
        ).await
    }

    /// Apply auto-tuning
    pub async fn auto_tune(&self) -> TuningResult {
        let current_stats = self.get_statistics().await;
        let recommendations = self.optimizer.recommend(&current_stats).await;

        let mut applied = Vec::new();
        for recommendation in recommendations {
            if self.is_safe_to_apply(&recommendation) {
                self.apply_tuning(recommendation.clone()).await;
                applied.push(recommendation);
            }
        }

        TuningResult {
            applied_changes: applied,
            new_stats: self.get_statistics().await,
        }
    }
}
```

## Testing and Validation

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extension_initialization() {
        let mut extension = CatalogExtension::new();
        let config = ExtensionConfig {
            settings: serde_json::json!({
                "cache_size": 1000000,
                "enable_mirroring": true,
            }),
            resource_limits: ResourceLimits::default(),
            granted_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::NetworkAccess,
            ]),
            privacy_level: PrivacyLevel::Public,
            debug_mode: false,
        };

        let result = extension.initialize(config).await;
        assert!(result.is_ok());

        let status = extension.status().await;
        assert!(matches!(status.state, ExtensionState::Running));
    }

    #[tokio::test]
    async fn test_package_creation() {
        let extension = create_test_extension().await;
        let handler = PackageAssetHandler::new(extension.library.clone());

        let spec = AssetCreationSpec {
            name: "test-package".to_string(),
            description: Some("Test package".to_string()),
            metadata: HashMap::from([
                ("version".to_string(), json!("1.0.0")),
            ]),
            privacy_level: PrivacyLevel::Public,
            allocation: None,
            consensus_requirements: ConsensusRequirements::default(),
            parent_id: None,
            tags: vec!["test".to_string()],
        };

        let asset_id = handler.create_asset(spec).await.unwrap();
        assert!(!asset_id.is_empty());
    }

    #[tokio::test]
    async fn test_state_management() {
        let mut state_manager = ExtensionStateManager::new();

        // Test valid transition
        assert!(state_manager.transition(ExtensionState::Running).is_ok());

        // Test invalid transition
        assert!(state_manager.transition(ExtensionState::Stopped).is_err());

        // Test operation tracking
        let op = Operation::new("test_op");
        let op_id = state_manager.track_operation(op);
        assert_eq!(state_manager.active_operations(), 1);

        state_manager.complete_operation(&op_id);
        assert_eq!(state_manager.active_operations(), 0);
    }

    #[tokio::test]
    async fn test_error_recovery() {
        let error_handler = ErrorHandler::new();
        let error = ExtensionError::Network(NetworkError::Timeout);

        let result = error_handler.handle_error(&error).await;
        assert!(matches!(result, RecoveryResult::Recovered | RecoveryResult::Failed(_)));
    }
}
```

### 2. Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use hypermesh::test_utils::*;

    #[tokio::test]
    async fn test_hypermesh_integration() {
        // Create test HyperMesh instance
        let hypermesh = create_test_hypermesh().await;

        // Load Catalog extension
        let extension = Box::new(CatalogExtension::new());
        hypermesh.extension_manager().load_extension(extension).await.unwrap();

        // Verify extension is loaded
        let extensions = hypermesh.extension_manager().list_extensions().await;
        assert!(extensions.iter().any(|e| e.id == "io.hypermesh.catalog"));

        // Test asset operations
        let asset_id = hypermesh.create_asset(AssetCreationSpec {
            name: "test-asset".to_string(),
            asset_type: AssetType::Package,
            // ... other fields
        }).await.unwrap();

        let metadata = hypermesh.get_asset_metadata(&asset_id).await.unwrap();
        assert_eq!(metadata.name, "test-asset");
    }

    #[tokio::test]
    async fn test_distribution_integration() {
        let extension = create_test_extension().await;

        // Create test package
        let package = create_test_package().await;

        // Distribute package
        let result = extension.distribution.distribute(&package).await.unwrap();
        assert!(result.successful);

        // Verify package can be retrieved
        let retrieved = extension.distribution.retrieve(&package.id()).await.unwrap();
        assert_eq!(retrieved.id(), package.id());
    }
}
```

## Performance Benchmarks

```rust
#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn bench_package_creation(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let extension = rt.block_on(create_test_extension());

        c.bench_function("package_creation", |b| {
            b.iter(|| {
                rt.block_on(async {
                    let package = create_test_package().await;
                    extension.library.store_package(black_box(package)).await
                })
            });
        });
    }

    fn bench_package_search(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let extension = rt.block_on(create_populated_extension(1000));

        c.bench_function("package_search", |b| {
            b.iter(|| {
                rt.block_on(async {
                    extension.library.search(black_box("test"), SearchOptions::default()).await
                })
            });
        });
    }

    criterion_group!(benches, bench_package_creation, bench_package_search);
    criterion_main!(benches);
}
```

## Migration Timeline

### Phase 1: Core Plugin (Days 1-3)
- Implement `CatalogExtension` structure
- Create `HyperMeshExtension` trait implementation
- Build state management system

### Phase 2: Asset Handlers (Days 4-6)
- Implement package asset handler
- Create library asset handler
- Build template and VM handlers

### Phase 3: Integration (Days 7-9)
- Integrate with HyperMesh AssetManager
- Connect to STOQ transport
- Implement consensus validation

### Phase 4: Testing (Days 10-12)
- Write comprehensive unit tests
- Create integration test suite
- Perform load testing

### Phase 5: Optimization (Days 13-15)
- Profile and optimize hot paths
- Tune cache and resource usage
- Implement auto-tuning

## Success Metrics

1. **Startup Time**: <100ms extension initialization
2. **Memory Usage**: <50MB base footprint
3. **Operation Latency**: <5ms for metadata operations
4. **Throughput**: >10,000 operations/second
5. **Error Rate**: <0.01% operation failures
6. **Recovery Time**: <1s for transient errors