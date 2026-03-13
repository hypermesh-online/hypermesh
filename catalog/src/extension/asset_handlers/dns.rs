// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use super::{
    async_trait, ApplicationDomain, Arc, AssetCategory, AssetCreationSpec, AssetData,
    AssetExtensionHandler, ExtensionAssetRecord, AssetOperation, AssetQuery, AssetRegistration, AssetType,
    AssetUpdate, StateProof, DeploymentResult, ExtensionError, ExtensionResult, HashMap,
    NetworkScope, OperationResult, RwLock,
};

/// Handler for DNS assets (domain names, namespace registrations)
pub struct DnsHandler {
    /// DNS entries registry
    entries: Arc<RwLock<HashMap<AssetRegistration, DnsEntry>>>,
}

#[derive(Debug, Clone)]
struct DnsEntry {
    pub _id: AssetRegistration,
    pub name: String,
    pub version: String,
    pub language: String,
    pub dependencies: Vec<String>,
    pub size_bytes: u64,
}

impl Default for DnsHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsHandler {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl AssetExtensionHandler for DnsHandler {
    fn asset_type(&self) -> AssetType {
        AssetType::Dns
    }

    async fn create_asset(&self, spec: AssetCreationSpec) -> ExtensionResult<AssetRegistration> {
        let asset_data = AssetData {
            config: spec.name.as_bytes().to_vec(),
            definition: b"catalog_dns".to_vec(),
            metadata: b"{}".to_vec(),
        };
        let asset_id = AssetRegistration::from_asset_data(
            &asset_data,
            NetworkScope::Global,
            AssetCategory::Application(ApplicationDomain {
                domain_name: "catalog_dns".to_string(),
                domain_hash: *blake3::hash(b"catalog_dns").as_bytes(),
            }),
        );

        let entry = DnsEntry {
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

        let mut entries = self.entries.write().await;
        entries.insert(asset_id.clone(), entry);

        Ok(asset_id)
    }

    async fn update_asset(
        &self,
        id: &AssetRegistration,
        update: AssetUpdate,
    ) -> ExtensionResult<()> {
        let mut entries = self.entries.write().await;

        let entry = entries
            .get_mut(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("DNS entry not found: {id}"),
            })?;

        if let Some(name) = update.name {
            entry.name = name;
        }

        if let Some(metadata) = update.metadata {
            if let Some(version) = metadata.get("version").and_then(|v| v.as_str()) {
                entry.version = version.to_string();
            }
        }

        Ok(())
    }

    async fn delete_asset(&self, id: &AssetRegistration) -> ExtensionResult<()> {
        let mut entries = self.entries.write().await;
        entries
            .remove(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("DNS entry not found: {id}"),
            })?;

        Ok(())
    }

    async fn query_assets(&self, query: AssetQuery) -> ExtensionResult<Vec<AssetRegistration>> {
        let entries = self.entries.read().await;

        let mut results = Vec::new();
        for (id, entry) in entries.iter() {
            if let Some(ref pattern) = query.name_pattern {
                if !entry.name.contains(pattern) {
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
        let entries = self.entries.read().await;

        let entry = entries
            .get(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("DNS entry not found: {id}"),
            })?;

        let mut metadata_map = HashMap::new();
        metadata_map.insert(
            "version".to_string(),
            serde_json::json!(entry.version.clone()),
        );
        metadata_map.insert("language".to_string(), serde_json::json!(entry.language));
        metadata_map.insert(
            "dependencies".to_string(),
            serde_json::json!(entry.dependencies.clone()),
        );

        Ok(ExtensionAssetRecord {
            id: id.clone(),
            asset_type: AssetType::Dns,
            name: entry.name.clone(),
            description: Some(format!("{} DNS entry", entry.language)),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            size_bytes: entry.size_bytes,
            metadata: metadata_map,
            privacy_level: hypermesh_lib::PrivacyMode::PUBLIC,
            allocation: None,
            state_proof_status: blockmatrix::extensions::StateProofStatus {
                validated: false,
                last_validated: None,
                proofs: None,
                errors: vec![],
            },
            tags: vec![entry.language.clone(), "dns".to_string()],
        })
    }

    async fn validate_asset(
        &self,
        id: &AssetRegistration,
        proof: StateProof,
    ) -> ExtensionResult<bool> {
        let entries = self.entries.read().await;
        if !entries.contains_key(id) {
            return Ok(false);
        }

        // Validate all four Proof of State proofs (PoSpace, PoStake, PoWork, PoTime)
        if !proof.validate() {
            tracing::warn!("DNS asset {}: Proof of State validation failed", id);
            return Ok(false);
        }

        // DNS assets require non-trivial stake (economic commitment)
        if proof.stake_proof.stake_amount == 0 {
            tracing::warn!("DNS asset {}: stake amount is zero", id);
            return Ok(false);
        }
        if proof.space_proof.total_storage == 0 {
            tracing::warn!("DNS asset {}: space commitment is zero", id);
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
                let entries = self.entries.read().await;

                if !entries.contains_key(id) {
                    return Err(ExtensionError::RuntimeError {
                        message: format!("DNS entry not found: {id}"),
                    });
                }

                let result = DeploymentResult {
                    deployment_id: format!("dns-{}", uuid::Uuid::new_v4()),
                    status: "registered".to_string(),
                    endpoints: vec![],
                    metadata: HashMap::new(),
                };

                Ok(OperationResult::Deployed(result))
            }

            _ => Err(ExtensionError::RuntimeError {
                message: "Operation not supported for DNS assets".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dns_handler() {
        let handler = DnsHandler::new();
        assert_eq!(handler.asset_type(), AssetType::Dns);

        let spec = AssetCreationSpec {
            name: "TestDns".to_string(),
            description: Some("Test DNS entry".to_string()),
            metadata: HashMap::from([
                ("version".to_string(), serde_json::json!("1.0.0")),
                ("language".to_string(), serde_json::json!("lua")),
            ]),
            privacy_level: hypermesh_lib::PrivacyMode::PUBLIC,
            allocation: None,
            state_requirements: blockmatrix::extensions::AssetStateRequirements::default(),
            parent_id: None,
            tags: vec!["dns".to_string()],
        };

        let asset_id = handler
            .create_asset(spec)
            .await
            .expect("DNS asset creation should succeed with valid spec");
        assert!(!asset_id.to_string().is_empty());
    }
}
