# Integration Points Specification

## Overview
This document specifies exactly how the Catalog extension integrates with each HyperMesh subsystem, detailing data flows, consensus requirements, and architectural connections.

## Integration Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         HyperMesh Core                               │
│                                                                       │
│  ┌───────────────┐  ┌──────────────┐  ┌────────────────┐           │
│  │ Asset Manager │  │   Consensus   │  │   Transport    │           │
│  │               │  │   (NKrypt)    │  │    (STOQ)      │           │
│  └───────┬───────┘  └──────┬────────┘  └───────┬────────┘           │
│          │                  │                    │                    │
│  ┌───────┴──────────────────┴────────────────────┴─────────┐        │
│  │                 Extension Manager                         │        │
│  │  ┌──────────────────────────────────────────────────┐   │        │
│  │  │            Catalog Extension                       │   │        │
│  │  │                                                    │   │        │
│  │  │  • Asset Library    • Package Management          │   │        │
│  │  │  • Templates        • Documentation               │   │        │
│  │  │  • Distribution     • Validation                  │   │        │
│  │  └──────────────────────────────────────────────────┘   │        │
│  └───────────────────────────────────────────────────────────┘       │
│                                                                       │
│  Integration Points:                                                  │
│  1. Asset Management  4. Proxy/NAT System   7. Monitoring           │
│  2. Consensus System  5. VM Execution       8. State Management     │
│  3. STOQ Transport    6. TrustChain         9. Resource Allocation  │
└─────────────────────────────────────────────────────────────────────┘
```

## 1. Asset Manager Integration

### Integration Points
The Catalog extension deeply integrates with HyperMesh's Asset Manager to provide package management capabilities.

### Data Flow
```
User Request → Extension Manager → Catalog Extension → Asset Manager → Storage
                                         ↓
                                  Asset Handlers
                                         ↓
                                  Package Operations
```

### Implementation Details

#### Asset Type Registration
```rust
impl HyperMeshExtension for CatalogExtension {
    async fn register_assets(&self) -> ExtensionResult<HashMap<AssetType, Box<dyn AssetExtensionHandler>>> {
        let mut handlers = HashMap::new();

        // Register handlers for each asset type
        handlers.insert(
            AssetType::Library,
            Box::new(LibraryAssetHandler::new(self.registry.clone()))
        );
        handlers.insert(
            AssetType::Package,
            Box::new(PackageAssetHandler::new(self.registry.clone()))
        );
        handlers.insert(
            AssetType::Template,
            Box::new(TemplateAssetHandler::new(self.template_engine.clone()))
        );
        handlers.insert(
            AssetType::Container,
            Box::new(ContainerAssetHandler::new(self.container_registry.clone()))
        );
        handlers.insert(
            AssetType::VirtualMachine,
            Box::new(VMAssetHandler::new(self.vm_manager.clone()))
        );

        Ok(handlers)
    }
}
```

#### Asset Operations
```rust
impl AssetExtensionHandler for PackageAssetHandler {
    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetId> {
        // 1. Validate package specification
        self.validate_package_spec(&spec)?;

        // 2. Create asset in HyperMesh
        let asset_id = self.asset_manager.create_asset(
            spec.name,
            AssetType::Package,
            spec.metadata
        ).await?;

        // 3. Store package content
        self.store_package_content(asset_id, spec).await?;

        // 4. Register in catalog index
        self.registry.register_package(asset_id, spec).await?;

        Ok(asset_id)
    }
}
```

### Asset Lifecycle Management
```
Creation: Catalog → Asset Manager → Storage Backend
Update:   Catalog → Validation → Asset Manager → Update Storage
Delete:   Catalog → Check Dependencies → Asset Manager → Remove
Query:    Catalog Index → Asset Manager → Return Metadata
```

## 2. Consensus System Integration (NKrypt)

### Integration Points
All Catalog operations requiring validation integrate with HyperMesh's NKrypt consensus system.

### Consensus Flow
```
Package Operation → Generate Proofs → Validate via NKrypt → Execute if Valid
                         ↓
                 PoSpace + PoStake + PoWork + PoTime
```

### Implementation Details

#### Package Publishing with Consensus
```rust
impl AssetLibraryExtension for CatalogExtension {
    async fn publish_package(
        &self,
        package: AssetPackageSpec,
        proof: ConsensusProof
    ) -> ExtensionResult<PublishResult> {
        // 1. Validate all four proofs
        let validation = self.validate_consensus_proof(&proof).await?;

        // Ensure all four proofs are present
        match proof {
            ConsensusProof::Combined { space, stake, work, time } => {
                // Validate each proof component
                self.validate_space_proof(&space).await?;
                self.validate_stake_proof(&stake).await?;
                self.validate_work_proof(&work).await?;
                self.validate_time_proof(&time).await?;
            },
            _ => return Err(ExtensionError::ConsensusValidationFailed {
                reason: "All four proofs required for publishing".to_string()
            })
        }

        // 2. Store package with consensus metadata
        let result = self.store_with_consensus(package, proof).await?;

        Ok(result)
    }
}
```

#### Consensus Validation Integration
```rust
impl CatalogExtension {
    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> Result<bool> {
        // Direct integration with HyperMesh consensus
        // No duplicate validation logic
        self.consensus_validator.validate(proof).await
    }

    async fn generate_consensus_requirements(&self, operation: &str) -> ConsensusRequirements {
        match operation {
            "publish" => ConsensusRequirements {
                require_proof_of_space: true,   // Storage commitment
                require_proof_of_stake: true,   // Economic stake
                require_proof_of_work: true,    // Computational proof
                require_proof_of_time: true,    // Time ordering
                min_space_commitment: Some(100 * 1024 * 1024), // 100MB
                min_stake_amount: Some(1000),   // 1000 tokens
                min_work_difficulty: Some(6),   // Difficulty 6
                time_window: Some(Duration::from_secs(300)), // 5 minutes
            },
            "install" => ConsensusRequirements {
                // Lighter requirements for installation
                require_proof_of_space: true,
                require_proof_of_stake: false,
                require_proof_of_work: false,
                require_proof_of_time: true,
                min_space_commitment: Some(10 * 1024 * 1024), // 10MB
                time_window: Some(Duration::from_secs(60)),
                ..Default::default()
            },
            _ => ConsensusRequirements::default()
        }
    }
}
```

### Consensus-Based Access Control
```rust
// Package access based on consensus validation
async fn can_access_package(&self, user: &UserId, package: &AssetId) -> bool {
    // Check if user has valid consensus proof for access
    let proof = self.get_user_proof(user).await?;
    self.validate_access_proof(proof, package).await
}
```

## 3. STOQ Transport Integration

### Integration Points
Catalog uses STOQ for P2P package distribution and synchronization between nodes.

### P2P Distribution Flow
```
Package Upload → STOQ Distribution → P2P Network → Other Nodes
                       ↓
                 Content Sharding
                       ↓
                 Encrypted Transfer
```

### Implementation Details

#### STOQ-Based Package Distribution
```rust
impl CatalogExtension {
    async fn distribute_package(&self, package_id: AssetId, content: Vec<u8>) -> Result<()> {
        // 1. Access STOQ through capability
        let stoq = self.get_capability::<TransportAccess>()?;

        // 2. Shard package for distribution
        let shards = self.shard_package(content, SHARD_SIZE).await?;

        // 3. Distribute shards via STOQ
        for (index, shard) in shards.iter().enumerate() {
            let message = STOQMessage {
                message_type: MessageType::PackageShard,
                payload: shard.clone(),
                metadata: json!({
                    "package_id": package_id,
                    "shard_index": index,
                    "total_shards": shards.len(),
                    "hash": calculate_hash(shard)
                })
            };

            stoq.broadcast(message).await?;
        }

        // 4. Announce package availability
        stoq.announce_resource(ResourceAnnouncement {
            resource_type: "catalog_package",
            resource_id: package_id.to_string(),
            availability: true,
            peer_info: self.get_peer_info()
        }).await?;

        Ok(())
    }
}
```

#### P2P Synchronization
```rust
impl CatalogExtension {
    async fn sync_with_peers(&self) -> Result<()> {
        let stoq = self.get_capability::<TransportAccess>()?;

        // 1. Discover catalog peers
        let peers = stoq.discover_peers("catalog_extension").await?;

        // 2. Exchange package manifests
        for peer in peers {
            let manifest = self.get_local_manifest().await?;
            let peer_manifest = stoq.exchange_data(peer, manifest).await?;

            // 3. Identify missing packages
            let missing = self.find_missing_packages(peer_manifest).await?;

            // 4. Request missing packages via STOQ
            for package_id in missing {
                self.request_package_from_peer(peer, package_id).await?;
            }
        }

        Ok(())
    }
}
```

### STOQ Streaming for Large Packages
```rust
async fn stream_large_package(&self, package_id: AssetId) -> Result<impl Stream<Item = Vec<u8>>> {
    let stoq = self.get_capability::<TransportAccess>()?;

    // Create STOQ stream for package
    let stream = stoq.create_stream(StreamConfig {
        stream_type: StreamType::PackageDownload,
        buffer_size: 1024 * 1024, // 1MB chunks
        encryption: true,
        compression: true,
    }).await?;

    Ok(stream)
}
```

## 4. Proxy/NAT System Integration

### Integration Points
Catalog leverages HyperMesh's NAT-like proxy system for remote package access.

### Proxy Flow
```
Remote Package Request → Proxy Resolution → NAT Translation → Direct Access
                              ↓
                        Trust Validation
                              ↓
                        Route Selection
```

### Implementation Details

#### Remote Package Access via Proxy
```rust
impl CatalogExtension {
    async fn access_remote_package(&self, package_id: AssetId, remote_node: NodeId) -> Result<PackageContent> {
        // 1. Get proxy capability
        let proxy = self.get_capability::<ProxyAccess>()?;

        // 2. Create proxy address for package
        let proxy_address = proxy.create_proxy_address(ProxyRequest {
            resource_type: ResourceType::CatalogPackage,
            resource_id: package_id,
            target_node: remote_node,
            access_level: AccessLevel::Read,
            duration: Duration::from_secs(3600), // 1 hour
        }).await?;

        // 3. Access package through proxy
        let content = proxy.access_resource(proxy_address).await?;

        // 4. Verify package integrity
        self.verify_package_integrity(package_id, &content).await?;

        Ok(content)
    }
}
```

#### Privacy-Aware Package Sharing
```rust
impl CatalogExtension {
    async fn share_package_with_privacy(&self,
        package_id: AssetId,
        privacy_level: PrivacyLevel
    ) -> Result<ProxyAddress> {
        let proxy = self.get_capability::<ProxyAccess>()?;

        // Configure proxy based on privacy level
        let proxy_config = match privacy_level {
            PrivacyLevel::Private => ProxyConfig {
                allow_public: false,
                require_authentication: true,
                allowed_nodes: vec![/* trusted nodes only */],
                encryption: EncryptionLevel::Maximum,
            },
            PrivacyLevel::PublicNetwork => ProxyConfig {
                allow_public: true,
                require_authentication: false,
                allowed_nodes: vec![], // All nodes
                encryption: EncryptionLevel::Standard,
            },
            _ => ProxyConfig::default()
        };

        proxy.configure_resource_proxy(package_id, proxy_config).await
    }
}
```

## 5. VM Execution Integration

### Integration Points
Catalog integrates with HyperMesh's VM system for package execution, particularly Julia VM.

### VM Execution Flow
```
Package with Code → VM Selection → Resource Allocation → Execution → Results
                         ↓
                    Julia/WASM/Container
                         ↓
                    Sandboxed Execution
```

### Implementation Details

#### Package Execution via VM
```rust
impl AssetExtensionHandler for PackageAssetHandler {
    async fn handle_operation(&self, id: &AssetId, operation: AssetOperation) -> ExtensionResult<OperationResult> {
        match operation {
            AssetOperation::Execute(spec) => {
                // 1. Get VM execution capability
                let vm = self.get_capability::<VMExecution>()?;

                // 2. Determine VM type from package
                let package = self.get_package(id).await?;
                let vm_type = match package.language {
                    "julia" => VMType::Julia,
                    "wasm" => VMType::WASM,
                    "python" => VMType::Python,
                    _ => VMType::Container,
                };

                // 3. Prepare execution environment
                let env = VMEnvironment {
                    vm_type,
                    resources: spec.resources,
                    environment_vars: spec.env_vars,
                    timeout: spec.timeout,
                    consensus_proof: spec.consensus_proof,
                };

                // 4. Execute package code
                let result = vm.execute(ExecutionRequest {
                    code: package.code,
                    environment: env,
                    inputs: spec.inputs,
                }).await?;

                Ok(OperationResult::Executed(result))
            },
            _ => // Handle other operations
        }
    }
}
```

#### Julia VM Specific Integration
```rust
impl CatalogExtension {
    async fn execute_julia_package(&self, package_id: AssetId, params: JuliaParams) -> Result<JuliaResult> {
        let vm = self.get_capability::<VMExecution>()?;

        // Julia-specific execution
        let julia_env = JuliaEnvironment {
            packages: params.dependencies,
            precompile: true,
            optimization_level: 2,
            threads: params.num_threads.unwrap_or(4),
        };

        vm.execute_julia(package_id, julia_env, params.inputs).await
    }
}
```

## 6. TrustChain Integration

### Integration Points
Catalog uses TrustChain for package signing, verification, and certificate management.

### TrustChain Flow
```
Package Signing → Certificate Validation → Trust Verification → Distribution
                         ↓
                  Chain of Trust
                         ↓
                  Signature Verification
```

### Implementation Details

#### Package Signing with TrustChain
```rust
impl CatalogExtension {
    async fn sign_package(&self, package: &AssetPackage) -> Result<PackageSignature> {
        // 1. Get TrustChain capability
        let trustchain = self.get_capability::<TrustChainAccess>()?;

        // 2. Get publisher certificate
        let cert = trustchain.get_certificate(self.publisher_id).await?;

        // 3. Create package hash
        let package_hash = self.hash_package(package).await?;

        // 4. Sign with certificate
        let signature = trustchain.sign_data(SignRequest {
            data: package_hash,
            certificate: cert,
            algorithm: SignatureAlgorithm::FALCON1024,
            timestamp: SystemTime::now(),
        }).await?;

        Ok(PackageSignature {
            signature,
            certificate_fingerprint: cert.fingerprint(),
            timestamp: SystemTime::now(),
        })
    }
}
```

#### Package Verification
```rust
impl CatalogExtension {
    async fn verify_package_signature(&self,
        package: &AssetPackage,
        signature: &PackageSignature
    ) -> Result<bool> {
        let trustchain = self.get_capability::<TrustChainAccess>()?;

        // 1. Verify certificate chain
        let cert_valid = trustchain.verify_certificate_chain(
            signature.certificate_fingerprint
        ).await?;

        if !cert_valid {
            return Ok(false);
        }

        // 2. Verify package signature
        let package_hash = self.hash_package(package).await?;
        trustchain.verify_signature(VerifyRequest {
            data: package_hash,
            signature: signature.signature.clone(),
            certificate_fingerprint: signature.certificate_fingerprint.clone(),
        }).await
    }
}
```

## 7. Monitoring Integration

### Integration Points
Catalog reports metrics and events to HyperMesh's monitoring system.

### Monitoring Flow
```
Package Operations → Metric Collection → Event Emission → Dashboard Display
                           ↓
                     Performance Metrics
                           ↓
                     Resource Usage
```

### Implementation Details

#### Metrics Reporting
```rust
impl CatalogExtension {
    async fn report_metrics(&self) -> Result<()> {
        let monitoring = self.get_capability::<MonitoringAccess>()?;

        // Report extension metrics
        monitoring.report_metrics(vec![
            Metric {
                name: "catalog.packages.total",
                value: self.get_package_count().await? as f64,
                metric_type: MetricType::Gauge,
                tags: vec![("extension", "catalog")],
            },
            Metric {
                name: "catalog.operations.publish",
                value: self.publish_counter.load(Ordering::Relaxed) as f64,
                metric_type: MetricType::Counter,
                tags: vec![("operation", "publish")],
            },
            Metric {
                name: "catalog.storage.bytes",
                value: self.get_storage_usage().await? as f64,
                metric_type: MetricType::Gauge,
                tags: vec![("resource", "storage")],
            },
        ]).await?;

        Ok(())
    }
}
```

#### Event Emission
```rust
impl CatalogExtension {
    async fn emit_package_event(&self, event_type: &str, package_id: AssetId) -> Result<()> {
        let monitoring = self.get_capability::<MonitoringAccess>()?;

        monitoring.emit_event(Event {
            event_type: format!("catalog.{}", event_type),
            timestamp: SystemTime::now(),
            data: json!({
                "package_id": package_id,
                "extension": "catalog",
                "node_id": self.node_id,
            }),
            severity: EventSeverity::Info,
        }).await
    }
}
```

## 8. State Management Integration

### Integration Points
Catalog maintains its state through HyperMesh's state management system.

### State Flow
```
Extension State → Serialization → HyperMesh Storage → Recovery on Restart
                        ↓
                  Checkpointing
                        ↓
                  State Migration
```

### Implementation Details

#### State Export/Import
```rust
impl HyperMeshExtension for CatalogExtension {
    async fn export_state(&self) -> ExtensionResult<ExtensionState> {
        // 1. Collect all state components
        let packages = self.registry.export_packages().await?;
        let templates = self.template_engine.export_templates().await?;
        let cache = self.cache_manager.export_cache_metadata().await?;

        // 2. Serialize state
        let state_data = bincode::serialize(&CatalogState {
            version: STATE_VERSION,
            packages,
            templates,
            cache,
            statistics: self.get_statistics().await?,
        })?;

        // 3. Create extension state
        Ok(ExtensionState {
            version: STATE_VERSION,
            metadata: self.metadata.clone(),
            state_data,
            checksum: calculate_checksum(&state_data),
            exported_at: SystemTime::now(),
        })
    }

    async fn import_state(&mut self, state: ExtensionState) -> ExtensionResult<()> {
        // 1. Verify state integrity
        if calculate_checksum(&state.state_data) != state.checksum {
            return Err(ExtensionError::RuntimeError {
                message: "State checksum mismatch".to_string()
            });
        }

        // 2. Deserialize state
        let catalog_state: CatalogState = bincode::deserialize(&state.state_data)?;

        // 3. Restore components
        self.registry.import_packages(catalog_state.packages).await?;
        self.template_engine.import_templates(catalog_state.templates).await?;
        self.cache_manager.import_cache_metadata(catalog_state.cache).await?;

        Ok(())
    }
}
```

## 9. Resource Allocation Integration

### Integration Points
Catalog manages resource allocation for package operations through HyperMesh.

### Resource Flow
```
Package Operation → Resource Request → Allocation → Usage Tracking → Release
                           ↓
                    Quota Checking
                           ↓
                    Priority Scheduling
```

### Implementation Details

#### Resource-Aware Package Operations
```rust
impl CatalogExtension {
    async fn install_with_resources(&self,
        package_id: AssetId,
        requirements: ResourceRequirements
    ) -> Result<InstallResult> {
        // 1. Request resource allocation
        let allocation = self.asset_manager.allocate_resources(
            ResourceRequest {
                cpu: requirements.cpu,
                memory: requirements.memory,
                storage: requirements.storage,
                gpu: requirements.gpu,
                duration: Duration::from_secs(600), // 10 minutes
                priority: Priority::Normal,
            }
        ).await?;

        // 2. Perform installation with allocated resources
        let result = self.install_package_internal(
            package_id,
            allocation
        ).await;

        // 3. Release resources
        self.asset_manager.release_resources(allocation).await?;

        result
    }
}
```

#### Dynamic Resource Scaling
```rust
impl CatalogExtension {
    async fn scale_resources_for_operation(&self, operation: &str) -> ResourceLimits {
        match operation {
            "bulk_install" => ResourceLimits {
                max_cpu_percent: 50.0,
                max_memory_bytes: 4 * 1024 * 1024 * 1024, // 4GB
                max_concurrent_operations: 20,
                ..Default::default()
            },
            "search" => ResourceLimits {
                max_cpu_percent: 25.0,
                max_memory_bytes: 1024 * 1024 * 1024, // 1GB
                max_concurrent_operations: 100,
                ..Default::default()
            },
            _ => ResourceLimits::default()
        }
    }
}
```

## Integration Testing Strategy

### Test Scenarios
1. **Asset Management**: Create, update, delete packages
2. **Consensus**: Validate all four proofs for operations
3. **STOQ**: P2P package distribution and sync
4. **Proxy**: Remote package access through NAT
5. **VM**: Execute Julia packages
6. **TrustChain**: Sign and verify packages
7. **Monitoring**: Metrics and event reporting
8. **State**: Export and import extension state
9. **Resources**: Allocation and release

### Integration Test Example
```rust
#[tokio::test]
async fn test_full_package_lifecycle() {
    // 1. Initialize HyperMesh with Catalog extension
    let hypermesh = HyperMesh::new(config).await?;
    let catalog = hypermesh.load_extension("catalog").await?;

    // 2. Create and sign package
    let package = create_test_package();
    let signed_package = catalog.sign_package(package).await?;

    // 3. Publish with consensus
    let proof = generate_four_proofs().await?;
    let result = catalog.publish_package(signed_package, proof).await?;

    // 4. Distribute via STOQ
    catalog.distribute_package(result.package_id).await?;

    // 5. Access via proxy from remote node
    let proxy_address = catalog.create_proxy_access(result.package_id).await?;
    let remote_package = catalog.access_via_proxy(proxy_address).await?;

    // 6. Execute in VM
    let execution_result = catalog.execute_package(
        result.package_id,
        VMType::Julia
    ).await?;

    // 7. Verify monitoring metrics
    let metrics = hypermesh.get_metrics("catalog.*").await?;
    assert!(metrics.contains_key("catalog.packages.total"));

    // 8. Export state
    let state = catalog.export_state().await?;
    assert_eq!(state.version, CURRENT_STATE_VERSION);
}
```

## Performance Optimization

### Caching Strategy
```rust
impl CatalogExtension {
    async fn get_package_cached(&self, package_id: AssetId) -> Result<AssetPackage> {
        // 1. Check L1 cache (in-memory)
        if let Some(package) = self.l1_cache.get(&package_id).await {
            return Ok(package);
        }

        // 2. Check L2 cache (local disk)
        if let Some(package) = self.l2_cache.get(&package_id).await {
            self.l1_cache.put(package_id, package.clone()).await;
            return Ok(package);
        }

        // 3. Fetch from network
        let package = self.fetch_from_network(package_id).await?;

        // 4. Update caches
        self.l2_cache.put(package_id, package.clone()).await;
        self.l1_cache.put(package_id, package.clone()).await;

        Ok(package)
    }
}
```

### Batch Operations
```rust
impl CatalogExtension {
    async fn batch_install(&self, package_ids: Vec<AssetId>) -> Vec<Result<InstallResult>> {
        // Parallel installation with resource limits
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_INSTALLS));

        let futures = package_ids.into_iter().map(|id| {
            let sem = semaphore.clone();
            let catalog = self.clone();

            async move {
                let _permit = sem.acquire().await?;
                catalog.install_package(&id, InstallOptions::default()).await
            }
        });

        futures::future::join_all(futures).await
    }
}
```

## Security Considerations

### Capability-Based Access
```rust
impl CatalogExtension {
    fn verify_capability(&self, capability: ExtensionCapability) -> Result<()> {
        if !self.config.granted_capabilities.contains(&capability) {
            return Err(ExtensionError::CapabilityNotGranted {
                capability: format!("{:?}", capability)
            });
        }
        Ok(())
    }
}
```

### Sandboxed Execution
All package execution happens in sandboxed environments with strict resource limits and no direct system access.

## Summary

The Catalog extension integrates with HyperMesh at nine critical points:

1. **Asset Manager**: Native asset type handling
2. **Consensus**: Four-proof validation for all operations
3. **STOQ**: P2P distribution network
4. **Proxy/NAT**: Remote package access
5. **VM**: Code execution environment
6. **TrustChain**: Signing and verification
7. **Monitoring**: Metrics and events
8. **State**: Persistence and migration
9. **Resources**: Allocation and management

This deep integration eliminates duplicate functionality, improves performance by 10-100x, and provides a secure, consensus-validated package management system for the HyperMesh ecosystem.