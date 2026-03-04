// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use super::{
    async_trait, ApplicationDomain, Arc, AssetCategory, AssetCreationSpec, AssetData,
    AssetExtensionHandler, ExtensionAssetRecord, AssetOperation, AssetQuery, AssetRegistration, AssetType,
    AssetUpdate, StateProof, DeploymentResult, ExtensionError, ExtensionResult, HashMap,
    NetworkScope, OperationResult, RwLock,
};

/// Handler for Library assets (packages, frameworks, dependencies)
pub struct LibraryHandler {
    /// Library packages registry
    packages: Arc<RwLock<HashMap<AssetRegistration, LibraryPackage>>>,
}

#[derive(Debug, Clone)]
struct LibraryPackage {
    pub _id: AssetRegistration,
    pub name: String,
    pub version: String,
    pub language: String,
    pub dependencies: Vec<String>,
    pub size_bytes: u64,
}

impl Default for LibraryHandler {
    fn default() -> Self {
        Self::new()
    }
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
        AssetType::Dns
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
                domain_hash: *blake3::hash(b"catalog_library").as_bytes(),
            }),
        );

        let package = LibraryPackage {
            _id: asset_id.clone(),
            name: spec.name.clone(),
            version: spec
                .metadata
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.0.0")
                .to_string(),
            language: spec
                .metadata
                .get("language")
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

    async fn update_asset(
        &self,
        id: &AssetRegistration,
        update: AssetUpdate,
    ) -> ExtensionResult<()> {
        let mut packages = self.packages.write().await;

        let package = packages
            .get_mut(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Library package not found: {id}"),
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
        packages
            .remove(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Library package not found: {id}"),
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

    async fn get_metadata(&self, id: &AssetRegistration) -> ExtensionResult<ExtensionAssetRecord> {
        let packages = self.packages.read().await;

        let package = packages
            .get(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Library package not found: {id}"),
            })?;

        let mut metadata_map = HashMap::new();
        metadata_map.insert(
            "version".to_string(),
            serde_json::json!(package.version.clone()),
        );
        metadata_map.insert("language".to_string(), serde_json::json!(package.language));
        metadata_map.insert(
            "dependencies".to_string(),
            serde_json::json!(package.dependencies.clone()),
        );

        Ok(ExtensionAssetRecord {
            id: id.clone(),
            asset_type: AssetType::Dns,
            name: package.name.clone(),
            description: Some(format!("{} library package", package.language)),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            size_bytes: package.size_bytes,
            metadata: metadata_map,
            privacy_level: hypermesh_lib::PrivacyMode::PUBLIC,
            allocation: None,
            state_proof_status: blockmatrix::extensions::StateProofStatus {
                validated: false,
                last_validated: None,
                proofs: None,
                errors: vec![],
            },
            tags: vec![package.language.clone(), "library".to_string()],
        })
    }

    async fn validate_asset(
        &self,
        id: &AssetRegistration,
        proof: StateProof,
    ) -> ExtensionResult<bool> {
        let packages = self.packages.read().await;
        if !packages.contains_key(id) {
            return Ok(false);
        }

        // Validate all four Proof of State proofs (PoSpace, PoStake, PoWork, PoTime)
        if !proof.validate() {
            tracing::warn!("Library asset {}: Proof of State validation failed", id);
            return Ok(false);
        }

        // Library assets require non-trivial stake (economic commitment)
        if proof.stake_proof.stake_amount == 0 {
            tracing::warn!("Library asset {}: stake amount is zero", id);
            return Ok(false);
        }
        if proof.space_proof.total_storage == 0 {
            tracing::warn!("Library asset {}: space commitment is zero", id);
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
            AssetOperation::Deploy(_) => {
                // Libraries are deployed by installing them
                let packages = self.packages.read().await;

                if !packages.contains_key(id) {
                    return Err(ExtensionError::RuntimeError {
                        message: format!("Library package not found: {id}"),
                    });
                }

                let result = DeploymentResult {
                    deployment_id: format!("lib-{}", uuid::Uuid::new_v4()),
                    status: "installed".to_string(),
                    endpoints: vec![],
                    metadata: HashMap::new(),
                };

                Ok(OperationResult::Deployed(result))
            }

            _ => Err(ExtensionError::RuntimeError {
                message: "Operation not supported for Library assets".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_library_handler() {
        let handler = LibraryHandler::new();
        assert_eq!(handler.asset_type(), AssetType::Dns);

        let spec = AssetCreationSpec {
            name: "TestLib".to_string(),
            description: Some("Test library package".to_string()),
            metadata: HashMap::from([
                ("version".to_string(), serde_json::json!("1.0.0")),
                ("language".to_string(), serde_json::json!("lua")),
            ]),
            privacy_level: hypermesh_lib::PrivacyMode::PUBLIC,
            allocation: None,
            state_requirements: blockmatrix::extensions::StateRequirements::default(),
            parent_id: None,
            tags: vec!["library".to_string()],
        };

        let asset_id = handler
            .create_asset(spec)
            .await
            .expect("Library asset creation should succeed with valid spec");
        // Check that asset_id is valid (not empty)
        assert!(!asset_id.to_string().is_empty());
    }
}
