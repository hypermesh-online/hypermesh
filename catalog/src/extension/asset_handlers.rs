// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Asset Handlers for Catalog Extension
//!
//! This module implements specific asset handlers for each asset type
//! that Catalog manages within the HyperMesh ecosystem.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use blockmatrix::extensions::{
    AssetExtensionHandler, ExtensionResult, ExtensionError,
    AssetCreationSpec, AssetUpdate, AssetQuery, AssetMetadata,
    AssetOperation, OperationResult, DeploymentResult, ExecutionResult,
    ResourceUsageReport,
};

use blockmatrix::assets::core::{AssetRegistration, AssetType, AssetData, NetworkScope, AssetCategory, ApplicationDomain};
use blockmatrix::consensus::proof_of_state_integration::ConsensusProof;
use sha2::Digest;

/// Handler for Virtual Machine assets (Lua, WASM, etc.)
pub struct VirtualMachineHandler {
    /// VM instances registry
    instances: Arc<RwLock<HashMap<AssetRegistration, VMInstance>>>,
}

/// VM instance information
#[allow(dead_code)] // VM tracking fields populated during lifecycle
#[derive(Debug, Clone)]
struct VMInstance {
    pub id: AssetRegistration,
    pub language: String,
    pub version: String,
    pub status: VMStatus,
    pub resources: VMResources,
}

#[allow(dead_code)] // VM lifecycle states
#[derive(Debug, Clone)]
enum VMStatus {
    Created,
    Running,
    Paused,
    Stopped,
    Error(String),
}

#[allow(dead_code)] // VM resource tracking
#[derive(Debug, Clone)]
struct VMResources {
    pub cpu_cores: f32,
    pub memory_mb: u64,
    pub storage_mb: u64,
}

impl VirtualMachineHandler {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl AssetExtensionHandler for VirtualMachineHandler {
    fn asset_type(&self) -> AssetType {
        AssetType::VirtualMachine
    }

    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetRegistration> {
        // Create AssetRegistration from asset specification
        let asset_data = AssetData {
            config: spec.name.as_bytes().to_vec(),
            definition: b"catalog_vm".to_vec(),
            metadata: b"{}".to_vec(),
        };
        let asset_id = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::Application(ApplicationDomain {
                domain_name: "catalog_vm".to_string(),
                domain_hash: {
                    let mut hasher = sha2::Sha256::new();
                    hasher.update(b"catalog_vm");
                    hasher.finalize().into()
                },
            }),
        );

        // Extract VM configuration from metadata
        let language = spec.metadata.get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("lua")
            .to_string();

        let version = spec.metadata.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("latest")
            .to_string();

        // Create VM instance
        let instance = VMInstance {
            id: asset_id.clone(),
            language: language.clone(),
            version,
            status: VMStatus::Created,
            resources: VMResources {
                cpu_cores: 1.0,
                memory_mb: 512,
                storage_mb: 1024,
            },
        };

        // Store instance
        let mut instances = self.instances.write().await;
        instances.insert(asset_id.clone(), instance);

        Ok(asset_id)
    }

    async fn update_asset(&self, id: &AssetRegistration, update: AssetUpdate) -> ExtensionResult<()> {
        let mut instances = self.instances.write().await;

        let instance = instances.get_mut(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("VM instance not found: {}", id)
            })?;

        // Apply updates
        if let Some(metadata) = update.metadata {
            if let Some(version) = metadata.get("version").and_then(|v| v.as_str()) {
                instance.version = version.to_string();
            }
        }

        Ok(())
    }

    async fn delete_asset(&self, id: &AssetRegistration) -> ExtensionResult<()> {
        let mut instances = self.instances.write().await;
        instances.remove(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("VM instance not found: {}", id)
            })?;

        Ok(())
    }

    async fn query_assets(&self, query: AssetQuery) -> ExtensionResult<Vec<AssetRegistration>> {
        let instances = self.instances.read().await;

        let mut results = Vec::new();
        for (id, instance) in instances.iter() {
            // Apply query filters
            if let Some(ref pattern) = query.name_pattern {
                if !instance.language.contains(pattern) {
                    continue;
                }
            }

            results.push(id.clone());

            // Apply limit
            if let Some(limit) = query.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    async fn get_metadata(&self, id: &AssetRegistration) -> ExtensionResult<AssetMetadata> {
        let instances = self.instances.read().await;

        let instance = instances.get(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("VM instance not found: {}", id)
            })?;

        let mut metadata_map = HashMap::new();
        metadata_map.insert("language".to_string(), serde_json::json!(instance.language));
        metadata_map.insert("version".to_string(), serde_json::json!(instance.version));
        metadata_map.insert("status".to_string(), serde_json::json!(format!("{:?}", instance.status)));

        Ok(AssetMetadata {
            id: id.clone(),
            asset_type: AssetType::VirtualMachine,
            name: format!("{} VM", instance.language),
            description: Some(format!("{} {} Virtual Machine", instance.language, instance.version)),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            size_bytes: instance.resources.storage_mb * 1024 * 1024,
            metadata: metadata_map,
            privacy_level: blockmatrix::assets::core::PrivacyLevel::PRIVATE,
            allocation: None,
            consensus_status: blockmatrix::extensions::ConsensusStatus {
                validated: false,
                last_validated: None,
                proofs: None,
                errors: vec![],
            },
            tags: vec![instance.language.clone(), "vm".to_string()],
        })
    }

    async fn validate_asset(&self, id: &AssetRegistration, _proof: ConsensusProof) -> ExtensionResult<bool> {
        let instances = self.instances.read().await;

        // Check if instance exists
        if !instances.contains_key(id) {
            return Ok(false);
        }

        // In a real implementation, validate consensus proofs
        // For now, return true if instance exists
        Ok(true)
    }

    async fn handle_operation(&self, id: &AssetRegistration, operation: AssetOperation) -> ExtensionResult<OperationResult> {
        match operation {
            AssetOperation::Execute(exec_spec) => {
                // Execute code in VM
                let instances = self.instances.read().await;

                let instance = instances.get(id)
                    .ok_or_else(|| ExtensionError::RuntimeError {
                        message: format!("VM instance not found: {}", id)
                    })?;

                // Simulate execution
                let result = ExecutionResult {
                    execution_id: format!("exec-{}", uuid::Uuid::new_v4()),
                    output: serde_json::json!({
                        "success": true,
                        "language": instance.language,
                        "code": exec_spec.code,
                        "result": "Execution simulated"
                    }),
                    execution_time: std::time::Duration::from_millis(100),
                    resource_usage: ResourceUsageReport {
                        cpu_usage: 0.5,
                        memory_usage: 100 * 1024 * 1024,
                        network_bytes: 0,
                        storage_bytes: 0,
                    },
                };

                Ok(OperationResult::Executed(result))
            },

            AssetOperation::Deploy(_deploy_spec) => {
                // Deploy VM to environment
                let mut instances = self.instances.write().await;

                let instance = instances.get_mut(id)
                    .ok_or_else(|| ExtensionError::RuntimeError {
                        message: format!("VM instance not found: {}", id)
                    })?;

                instance.status = VMStatus::Running;

                let result = DeploymentResult {
                    deployment_id: format!("deploy-{}", uuid::Uuid::new_v4()),
                    status: "running".to_string(),
                    endpoints: vec![format!("vm://{}/execute", id)],
                    metadata: HashMap::new(),
                };

                Ok(OperationResult::Deployed(result))
            },

            _ => Err(ExtensionError::RuntimeError {
                message: "Operation not supported for VM assets".to_string()
            })
        }
    }
}

/// Handler for Library assets (packages, frameworks, dependencies)
pub struct LibraryHandler {
    /// Library packages registry
    packages: Arc<RwLock<HashMap<AssetRegistration, LibraryPackage>>>,
}

#[allow(dead_code)] // Package tracking fields
#[derive(Debug, Clone)]
struct LibraryPackage {
    pub id: AssetRegistration,
    pub name: String,
    pub version: String,
    pub language: String,
    pub dependencies: Vec<String>,
    pub size_bytes: u64,
}

impl LibraryHandler {
    pub fn new() -> Self {
        Self {
            packages: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl AssetExtensionHandler for LibraryHandler {
    fn asset_type(&self) -> AssetType {
        AssetType::Library
    }

    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetRegistration> {
        // Create AssetRegistration from library asset specification
        let asset_data = AssetData {
            config: spec.name.as_bytes().to_vec(),
            definition: b"catalog_library".to_vec(),
            metadata: b"{}".to_vec(),
        };
        let asset_id = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::Application(ApplicationDomain {
                domain_name: "catalog_library".to_string(),
                domain_hash: {
                    let mut hasher = sha2::Sha256::new();
                    hasher.update(b"catalog_library");
                    hasher.finalize().into()
                },
            }),
        );

        let package = LibraryPackage {
            id: asset_id.clone(),
            name: spec.name.clone(),
            version: spec.metadata.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string(),
            language: spec.metadata.get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("lua")
                .to_string(),
            dependencies: vec![],
            size_bytes: 1024 * 1024, // 1MB default
        };

        let mut packages = self.packages.write().await;
        packages.insert(asset_id.clone(), package);

        Ok(asset_id)
    }

    async fn update_asset(&self, id: &AssetRegistration, update: AssetUpdate) -> ExtensionResult<()> {
        let mut packages = self.packages.write().await;

        let package = packages.get_mut(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Library package not found: {}", id)
            })?;

        if let Some(name) = update.name {
            package.name = name;
        }

        if let Some(metadata) = update.metadata {
            if let Some(version) = metadata.get("version").and_then(|v| v.as_str()) {
                package.version = version.to_string();
            }
        }

        Ok(())
    }

    async fn delete_asset(&self, id: &AssetRegistration) -> ExtensionResult<()> {
        let mut packages = self.packages.write().await;
        packages.remove(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Library package not found: {}", id)
            })?;

        Ok(())
    }

    async fn query_assets(&self, query: AssetQuery) -> ExtensionResult<Vec<AssetRegistration>> {
        let packages = self.packages.read().await;

        let mut results = Vec::new();
        for (id, package) in packages.iter() {
            if let Some(ref pattern) = query.name_pattern {
                if !package.name.contains(pattern) {
                    continue;
                }
            }

            results.push(id.clone());

            if let Some(limit) = query.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    async fn get_metadata(&self, id: &AssetRegistration) -> ExtensionResult<AssetMetadata> {
        let packages = self.packages.read().await;

        let package = packages.get(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Library package not found: {}", id)
            })?;

        let mut metadata_map = HashMap::new();
        metadata_map.insert("version".to_string(), serde_json::json!(package.version.clone()));
        metadata_map.insert("language".to_string(), serde_json::json!(package.language));
        metadata_map.insert("dependencies".to_string(), serde_json::json!(package.dependencies.clone()));

        Ok(AssetMetadata {
            id: id.clone(),
            asset_type: AssetType::Library,
            name: package.name.clone(),
            description: Some(format!("{} library package", package.language)),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            size_bytes: package.size_bytes,
            metadata: metadata_map,
            privacy_level: blockmatrix::assets::core::PrivacyLevel::PUBLIC,
            allocation: None,
            consensus_status: blockmatrix::extensions::ConsensusStatus {
                validated: false,
                last_validated: None,
                proofs: None,
                errors: vec![],
            },
            tags: vec![package.language.clone(), "library".to_string()],
        })
    }

    async fn validate_asset(&self, id: &AssetRegistration, _proof: ConsensusProof) -> ExtensionResult<bool> {
        let packages = self.packages.read().await;
        Ok(packages.contains_key(id))
    }

    async fn handle_operation(&self, id: &AssetRegistration, operation: AssetOperation) -> ExtensionResult<OperationResult> {
        match operation {
            AssetOperation::Deploy(_) => {
                // Libraries are deployed by installing them
                let packages = self.packages.read().await;

                if !packages.contains_key(id) {
                    return Err(ExtensionError::RuntimeError {
                        message: format!("Library package not found: {}", id)
                    });
                }

                let result = DeploymentResult {
                    deployment_id: format!("lib-{}", uuid::Uuid::new_v4()),
                    status: "installed".to_string(),
                    endpoints: vec![],
                    metadata: HashMap::new(),
                };

                Ok(OperationResult::Deployed(result))
            },

            _ => Err(ExtensionError::RuntimeError {
                message: "Operation not supported for Library assets".to_string()
            })
        }
    }
}

/// Handler for Dataset assets (ML datasets, scientific data)
pub struct DatasetHandler {
    /// Datasets registry
    datasets: Arc<RwLock<HashMap<AssetRegistration, Dataset>>>,
}

#[allow(dead_code)] // Dataset tracking fields
#[derive(Debug, Clone)]
struct Dataset {
    pub id: AssetRegistration,
    pub name: String,
    pub format: String,
    pub size_bytes: u64,
    pub record_count: u64,
}

impl DatasetHandler {
    pub fn new() -> Self {
        Self {
            datasets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl AssetExtensionHandler for DatasetHandler {
    fn asset_type(&self) -> AssetType {
        AssetType::Library
    }

    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetRegistration> {
        // Create AssetRegistration from dataset asset specification
        let asset_data = AssetData {
            config: spec.name.as_bytes().to_vec(),
            definition: b"catalog_dataset".to_vec(),
            metadata: b"{}".to_vec(),
        };
        let asset_id = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::Application(ApplicationDomain {
                domain_name: "catalog_dataset".to_string(),
                domain_hash: {
                    let mut hasher = sha2::Sha256::new();
                    hasher.update(b"catalog_dataset");
                    hasher.finalize().into()
                },
            }),
        );

        let dataset = Dataset {
            id: asset_id.clone(),
            name: spec.name.clone(),
            format: spec.metadata.get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("csv")
                .to_string(),
            size_bytes: spec.metadata.get("size_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            record_count: spec.metadata.get("record_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
        };

        let mut datasets = self.datasets.write().await;
        datasets.insert(asset_id.clone(), dataset);

        Ok(asset_id)
    }

    async fn update_asset(&self, id: &AssetRegistration, update: AssetUpdate) -> ExtensionResult<()> {
        let mut datasets = self.datasets.write().await;

        let dataset = datasets.get_mut(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Dataset not found: {}", id)
            })?;

        if let Some(name) = update.name {
            dataset.name = name;
        }

        Ok(())
    }

    async fn delete_asset(&self, id: &AssetRegistration) -> ExtensionResult<()> {
        let mut datasets = self.datasets.write().await;
        datasets.remove(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Dataset not found: {}", id)
            })?;

        Ok(())
    }

    async fn query_assets(&self, query: AssetQuery) -> ExtensionResult<Vec<AssetRegistration>> {
        let datasets = self.datasets.read().await;

        let mut results = Vec::new();
        for (id, dataset) in datasets.iter() {
            if let Some(ref pattern) = query.name_pattern {
                if !dataset.name.contains(pattern) {
                    continue;
                }
            }

            results.push(id.clone());

            if let Some(limit) = query.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    async fn get_metadata(&self, id: &AssetRegistration) -> ExtensionResult<AssetMetadata> {
        let datasets = self.datasets.read().await;

        let dataset = datasets.get(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Dataset not found: {}", id)
            })?;

        let mut metadata_map = HashMap::new();
        metadata_map.insert("format".to_string(), serde_json::json!(dataset.format));
        metadata_map.insert("record_count".to_string(), serde_json::json!(dataset.record_count));

        Ok(AssetMetadata {
            id: id.clone(),
            // STUB: Dataset no longer exists, using Library
            asset_type: AssetType::Library,
            name: dataset.name.clone(),
            description: Some(format!("{} dataset with {} records", dataset.format, dataset.record_count)),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            size_bytes: dataset.size_bytes,
            metadata: metadata_map,
            privacy_level: blockmatrix::assets::core::PrivacyLevel::PRIVATE,
            allocation: None,
            consensus_status: blockmatrix::extensions::ConsensusStatus {
                validated: false,
                last_validated: None,
                proofs: None,
                errors: vec![],
            },
            tags: vec![dataset.format.clone(), "dataset".to_string()],
        })
    }

    async fn validate_asset(&self, id: &AssetRegistration, _proof: ConsensusProof) -> ExtensionResult<bool> {
        let datasets = self.datasets.read().await;
        Ok(datasets.contains_key(id))
    }

    async fn handle_operation(&self, _id: &AssetRegistration, operation: AssetOperation) -> ExtensionResult<OperationResult> {
        match operation {
            AssetOperation::Custom(value) => {
                // Handle custom dataset operations
                Ok(OperationResult::Custom(value))
            },

            _ => Err(ExtensionError::RuntimeError {
                message: "Operation not supported for Dataset assets".to_string()
            })
        }
    }
}

/// Handler for Template assets (asset generation templates)
pub struct TemplateHandler {
    /// Templates registry
    templates: Arc<RwLock<HashMap<AssetRegistration, Template>>>,
}

#[allow(dead_code)] // Template tracking fields
#[derive(Debug, Clone)]
struct Template {
    pub id: AssetRegistration,
    pub name: String,
    pub template_type: String,
    pub language: String,
    pub parameters: Vec<String>,
}

impl TemplateHandler {
    pub fn new() -> Self {
        Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl AssetExtensionHandler for TemplateHandler {
    fn asset_type(&self) -> AssetType {
        AssetType::Container
    }

    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetRegistration> {
        // Create AssetRegistration from template asset specification
        let asset_data = AssetData {
            config: spec.name.as_bytes().to_vec(),
            definition: b"catalog_template".to_vec(),
            metadata: b"{}".to_vec(),
        };
        let asset_id = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::Application(ApplicationDomain {
                domain_name: "catalog_template".to_string(),
                domain_hash: {
                    let mut hasher = sha2::Sha256::new();
                    hasher.update(b"catalog_template");
                    hasher.finalize().into()
                },
            }),
        );

        let template = Template {
            id: asset_id.clone(),
            name: spec.name.clone(),
            template_type: spec.metadata.get("template_type")
                .and_then(|v| v.as_str())
                .unwrap_or("generic")
                .to_string(),
            language: spec.metadata.get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("lua")
                .to_string(),
            parameters: vec![],
        };

        let mut templates = self.templates.write().await;
        templates.insert(asset_id.clone(), template);

        Ok(asset_id)
    }

    async fn update_asset(&self, id: &AssetRegistration, update: AssetUpdate) -> ExtensionResult<()> {
        let mut templates = self.templates.write().await;

        let template = templates.get_mut(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Template not found: {}", id)
            })?;

        if let Some(name) = update.name {
            template.name = name;
        }

        Ok(())
    }

    async fn delete_asset(&self, id: &AssetRegistration) -> ExtensionResult<()> {
        let mut templates = self.templates.write().await;
        templates.remove(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Template not found: {}", id)
            })?;

        Ok(())
    }

    async fn query_assets(&self, query: AssetQuery) -> ExtensionResult<Vec<AssetRegistration>> {
        let templates = self.templates.read().await;

        let mut results = Vec::new();
        for (id, template) in templates.iter() {
            if let Some(ref pattern) = query.name_pattern {
                if !template.name.contains(pattern) {
                    continue;
                }
            }

            results.push(id.clone());

            if let Some(limit) = query.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    async fn get_metadata(&self, id: &AssetRegistration) -> ExtensionResult<AssetMetadata> {
        let templates = self.templates.read().await;

        let template = templates.get(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Template not found: {}", id)
            })?;

        let mut metadata_map = HashMap::new();
        metadata_map.insert("template_type".to_string(), serde_json::json!(template.template_type));
        metadata_map.insert("language".to_string(), serde_json::json!(template.language));
        metadata_map.insert("parameters".to_string(), serde_json::json!(template.parameters));

        Ok(AssetMetadata {
            id: id.clone(),
            // STUB: Template no longer exists, using Library
            asset_type: AssetType::Library,
            name: template.name.clone(),
            description: Some(format!("{} template for {}", template.template_type, template.language)),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            size_bytes: 1024, // Templates are typically small
            metadata: metadata_map,
            privacy_level: blockmatrix::assets::core::PrivacyLevel::PUBLIC,
            allocation: None,
            consensus_status: blockmatrix::extensions::ConsensusStatus {
                validated: false,
                last_validated: None,
                proofs: None,
                errors: vec![],
            },
            tags: vec![template.template_type.clone(), "template".to_string()],
        })
    }

    async fn validate_asset(&self, id: &AssetRegistration, _proof: ConsensusProof) -> ExtensionResult<bool> {
        let templates = self.templates.read().await;
        Ok(templates.contains_key(id))
    }

    async fn handle_operation(&self, _id: &AssetRegistration, operation: AssetOperation) -> ExtensionResult<OperationResult> {
        match operation {
            AssetOperation::Custom(value) => {
                // Handle template generation operations
                Ok(OperationResult::Custom(value))
            },

            _ => Err(ExtensionError::RuntimeError {
                message: "Operation not supported for Template assets".to_string()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vm_handler() {
        let handler = VirtualMachineHandler::new();
        assert_eq!(handler.asset_type(), AssetType::VirtualMachine);

        let spec = AssetCreationSpec {
            name: "Test VM".to_string(),
            description: Some("Test virtual machine".to_string()),
            metadata: HashMap::from([
                ("language".to_string(), serde_json::json!("lua")),
                ("version".to_string(), serde_json::json!("1.9.0")),
            ]),
            privacy_level: blockmatrix::assets::core::PrivacyLevel::PRIVATE,
            allocation: None,
            consensus_requirements: blockmatrix::extensions::ConsensusRequirements::default(),
            parent_id: None,
            tags: vec!["test".to_string()],
        };

        let asset_id = handler.create_asset(spec).await
            .expect("VM asset creation should succeed with valid spec");
        // Check that asset_id is valid (not empty)
        assert!(!asset_id.to_string().is_empty());
    }

    #[tokio::test]
    async fn test_library_handler() {
        let handler = LibraryHandler::new();
        assert_eq!(handler.asset_type(), AssetType::Library);

        let spec = AssetCreationSpec {
            name: "TestLib".to_string(),
            description: Some("Test library package".to_string()),
            metadata: HashMap::from([
                ("version".to_string(), serde_json::json!("1.0.0")),
                ("language".to_string(), serde_json::json!("lua")),
            ]),
            privacy_level: blockmatrix::assets::core::PrivacyLevel::PUBLIC,
            allocation: None,
            consensus_requirements: blockmatrix::extensions::ConsensusRequirements::default(),
            parent_id: None,
            tags: vec!["library".to_string()],
        };

        let asset_id = handler.create_asset(spec).await
            .expect("Library asset creation should succeed with valid spec");
        // Check that asset_id is valid (not empty)
        assert!(!asset_id.to_string().is_empty());
    }

    #[tokio::test]
    async fn test_dataset_handler() {
        let handler = DatasetHandler::new();
        // STUB: Dataset replaced with Library
        assert_eq!(handler.asset_type(), AssetType::Library);
    }

    #[tokio::test]
    async fn test_template_handler() {
        let handler = TemplateHandler::new();
        // Template handler uses Container type
        assert_eq!(handler.asset_type(), AssetType::Container);
    }
}