// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use super::{
    async_trait, ApplicationDomain, Arc, AssetCategory, AssetCreationSpec, AssetData,
    AssetExtensionHandler, ExtensionAssetRecord, AssetOperation, AssetQuery, AssetRegistration, AssetType,
    AssetUpdate, StateProof, DeploymentResult, ExecutionResult, ExtensionError,
    ExtensionResult, HashMap, NetworkScope, OperationResult, ResourceUsageReport, RwLock,
};

/// Handler for Blockchain assets
pub struct BlockchainHandler {
    /// Blockchain instances registry
    instances: Arc<RwLock<HashMap<AssetRegistration, BlockchainInstance>>>,
}

/// Blockchain instance information
#[derive(Debug, Clone)]
struct BlockchainInstance {
    pub _id: AssetRegistration,
    pub language: String,
    pub version: String,
    pub status: InstanceStatus,
    pub resources: InstanceResources,
}

#[derive(Debug, Clone)]
enum InstanceStatus {
    Created,
    Running,
    _Paused,
    _Stopped,
    _Error(String),
}

#[derive(Debug, Clone)]
struct InstanceResources {
    pub _cpu_cores: f32,
    pub _memory_mb: u64,
    pub storage_mb: u64,
}

impl Default for BlockchainHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockchainHandler {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl AssetExtensionHandler for BlockchainHandler {
    fn asset_type(&self) -> AssetType {
        AssetType::Blockchain
    }

    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetRegistration> {
        let asset_data = AssetData {
            config: spec.name.as_bytes().to_vec(),
            definition: b"catalog_blockchain".to_vec(),
            metadata: b"{}".to_vec(),
        };
        let asset_id = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::Application(ApplicationDomain {
                domain_name: "catalog_blockchain".to_string(),
                domain_hash: *blake3::hash(b"catalog_blockchain").as_bytes(),
            }),
        );

        let language = spec
            .metadata
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("lua")
            .to_string();

        let version = spec
            .metadata
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("latest")
            .to_string();

        let instance = BlockchainInstance {
            _id: asset_id.clone(),
            language: language.clone(),
            version,
            status: InstanceStatus::Created,
            resources: InstanceResources {
                _cpu_cores: 1.0,
                _memory_mb: 512,
                storage_mb: 1024,
            },
        };

        let mut instances = self.instances.write().await;
        instances.insert(asset_id.clone(), instance);

        Ok(asset_id)
    }

    async fn update_asset(
        &self,
        id: &AssetRegistration,
        update: AssetUpdate,
    ) -> ExtensionResult<()> {
        let mut instances = self.instances.write().await;

        let instance = instances
            .get_mut(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Blockchain instance not found: {id}"),
            })?;

        if let Some(metadata) = update.metadata {
            if let Some(version) = metadata.get("version").and_then(|v| v.as_str()) {
                instance.version = version.to_string();
            }
        }

        Ok(())
    }

    async fn delete_asset(&self, id: &AssetRegistration) -> ExtensionResult<()> {
        let mut instances = self.instances.write().await;
        instances
            .remove(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Blockchain instance not found: {id}"),
            })?;

        Ok(())
    }

    async fn query_assets(&self, query: AssetQuery) -> ExtensionResult<Vec<AssetRegistration>> {
        let instances = self.instances.read().await;

        let mut results = Vec::new();
        for (id, instance) in instances.iter() {
            if let Some(ref pattern) = query.name_pattern {
                if !instance.language.contains(pattern) {
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

    async fn get_metadata(&self, id: &AssetRegistration) -> ExtensionResult<ExtensionAssetRecord> {
        let instances = self.instances.read().await;

        let instance = instances
            .get(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Blockchain instance not found: {id}"),
            })?;

        let mut metadata_map = HashMap::new();
        metadata_map.insert("language".to_string(), serde_json::json!(instance.language));
        metadata_map.insert("version".to_string(), serde_json::json!(instance.version));
        metadata_map.insert(
            "status".to_string(),
            serde_json::json!(format!("{:?}", instance.status)),
        );

        Ok(ExtensionAssetRecord {
            id: id.clone(),
            asset_type: AssetType::Blockchain,
            name: format!("{} Blockchain", instance.language),
            description: Some(format!(
                "{} {} Blockchain instance",
                instance.language, instance.version
            )),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            size_bytes: instance.resources.storage_mb * 1024 * 1024,
            metadata: metadata_map,
            privacy_level: hypermesh_lib::PrivacyMode::PRIVATE,
            allocation: None,
            state_proof_status: blockmatrix::extensions::StateProofStatus {
                validated: false,
                last_validated: None,
                proofs: None,
                errors: vec![],
            },
            tags: vec![instance.language.clone(), "blockchain".to_string()],
        })
    }

    async fn validate_asset(
        &self,
        id: &AssetRegistration,
        proof: StateProof,
    ) -> ExtensionResult<bool> {
        let instances = self.instances.read().await;

        if !instances.contains_key(id) {
            return Ok(false);
        }

        // Validate all four Proof of State proofs (PoSpace, PoStake, PoWork, PoTime)
        if !proof.validate() {
            tracing::warn!("Blockchain asset {}: Proof of State validation failed", id);
            return Ok(false);
        }

        if proof.stake_proof.stake_holder_id.is_empty() {
            tracing::warn!("Blockchain asset {}: authorization has no bound identity", id);
            return Ok(false);
        }
        // PoSpace: CANONICAL MODEL — WHERE (location). Require a bound
        // location; capacity is descriptive and never gates admission.
        if proof.space_proof.node_id.is_empty()
            && proof.space_proof.storage_path.is_empty()
        {
            tracing::warn!("blockchain asset {}: PoSpace has no bound location", id);
            return Ok(false);
        }
        if proof.work_proof.work_hash == [0u8; 32] {
            tracing::warn!("Blockchain asset {}: work hash is zero (no work performed)", id);
            return Ok(false);
        }

        Ok(true)
    }

    async fn handle_operation(
        &self,
        id: &AssetRegistration,
        operation: AssetOperation,
    ) -> ExtensionResult<OperationResult> {
        match operation {
            AssetOperation::Execute(exec_spec) => {
                let instances = self.instances.read().await;

                let instance = instances
                    .get(id)
                    .ok_or_else(|| ExtensionError::RuntimeError {
                        message: format!("Blockchain instance not found: {id}"),
                    })?;

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
            }

            AssetOperation::Deploy(_deploy_spec) => {
                let mut instances = self.instances.write().await;

                let instance =
                    instances
                        .get_mut(id)
                        .ok_or_else(|| ExtensionError::RuntimeError {
                            message: format!("Blockchain instance not found: {id}"),
                        })?;

                instance.status = InstanceStatus::Running;

                let result = DeploymentResult {
                    deployment_id: format!("deploy-{}", uuid::Uuid::new_v4()),
                    status: "running".to_string(),
                    endpoints: vec![format!("blockchain://{}/execute", id)],
                    metadata: HashMap::new(),
                };

                Ok(OperationResult::Deployed(result))
            }

            _ => Err(ExtensionError::RuntimeError {
                message: "Operation not supported for Blockchain assets".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blockchain_handler() {
        let handler = BlockchainHandler::new();
        assert_eq!(handler.asset_type(), AssetType::Blockchain);

        let spec = AssetCreationSpec {
            name: "Test Blockchain".to_string(),
            description: Some("Test blockchain instance".to_string()),
            metadata: HashMap::from([
                ("language".to_string(), serde_json::json!("lua")),
                ("version".to_string(), serde_json::json!("1.9.0")),
            ]),
            privacy_level: hypermesh_lib::PrivacyMode::PRIVATE,
            allocation: None,
            state_requirements: blockmatrix::extensions::AssetStateRequirements::default(),
            parent_id: None,
            tags: vec!["test".to_string()],
        };

        let asset_id = handler
            .create_asset(spec)
            .await
            .expect("Blockchain asset creation should succeed with valid spec");
        assert!(!asset_id.to_string().is_empty());
    }
}
