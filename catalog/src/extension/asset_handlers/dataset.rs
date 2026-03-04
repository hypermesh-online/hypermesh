// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use super::{
    async_trait, ApplicationDomain, Arc, AssetCategory, AssetCreationSpec, AssetData,
    AssetExtensionHandler, ExtensionAssetRecord, AssetOperation, AssetQuery, AssetRegistration, AssetType,
    AssetUpdate, StateProof, ExtensionError, ExtensionResult, HashMap, NetworkScope,
    OperationResult, RwLock,
};

/// Handler for Dataset assets (ML datasets, scientific data)
pub struct DatasetHandler {
    /// Datasets registry
    datasets: Arc<RwLock<HashMap<AssetRegistration, Dataset>>>,
}

#[derive(Debug, Clone)]
struct Dataset {
    pub _id: AssetRegistration,
    pub name: String,
    pub format: String,
    pub size_bytes: u64,
    pub record_count: u64,
}

impl Default for DatasetHandler {
    fn default() -> Self {
        Self::new()
    }
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
        AssetType::Dns
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
                domain_hash: *blake3::hash(b"catalog_dataset").as_bytes(),
            }),
        );

        let dataset = Dataset {
            _id: asset_id.clone(),
            name: spec.name.clone(),
            format: spec
                .metadata
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("csv")
                .to_string(),
            size_bytes: spec
                .metadata
                .get("size_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            record_count: spec
                .metadata
                .get("record_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
        };

        let mut datasets = self.datasets.write().await;
        datasets.insert(asset_id.clone(), dataset);

        Ok(asset_id)
    }

    async fn update_asset(
        &self,
        id: &AssetRegistration,
        update: AssetUpdate,
    ) -> ExtensionResult<()> {
        let mut datasets = self.datasets.write().await;

        let dataset = datasets
            .get_mut(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Dataset not found: {id}"),
            })?;

        if let Some(name) = update.name {
            dataset.name = name;
        }

        Ok(())
    }

    async fn delete_asset(&self, id: &AssetRegistration) -> ExtensionResult<()> {
        let mut datasets = self.datasets.write().await;
        datasets
            .remove(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Dataset not found: {id}"),
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

    async fn get_metadata(&self, id: &AssetRegistration) -> ExtensionResult<ExtensionAssetRecord> {
        let datasets = self.datasets.read().await;

        let dataset = datasets
            .get(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Dataset not found: {id}"),
            })?;

        let mut metadata_map = HashMap::new();
        metadata_map.insert("format".to_string(), serde_json::json!(dataset.format));
        metadata_map.insert(
            "record_count".to_string(),
            serde_json::json!(dataset.record_count),
        );

        Ok(ExtensionAssetRecord {
            id: id.clone(),
            // STUB: Dataset no longer exists, using Library
            asset_type: AssetType::Dns,
            name: dataset.name.clone(),
            description: Some(format!(
                "{} dataset with {} records",
                dataset.format, dataset.record_count
            )),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            size_bytes: dataset.size_bytes,
            metadata: metadata_map,
            privacy_level: hypermesh_lib::PrivacyMode::PRIVATE,
            allocation: None,
            state_proof_status: blockmatrix::extensions::StateProofStatus {
                validated: false,
                last_validated: None,
                proofs: None,
                errors: vec![],
            },
            tags: vec![dataset.format.clone(), "dataset".to_string()],
        })
    }

    async fn validate_asset(
        &self,
        id: &AssetRegistration,
        proof: StateProof,
    ) -> ExtensionResult<bool> {
        let datasets = self.datasets.read().await;
        if !datasets.contains_key(id) {
            return Ok(false);
        }

        // Validate all four Proof of State proofs (PoSpace, PoStake, PoWork, PoTime)
        if !proof.validate() {
            tracing::warn!("Dataset asset {}: Proof of State validation failed", id);
            return Ok(false);
        }

        // Dataset assets require storage commitment (space proof)
        if proof.stake_proof.stake_amount == 0 {
            tracing::warn!("Dataset asset {}: stake amount is zero", id);
            return Ok(false);
        }
        if proof.space_proof.total_storage == 0 {
            tracing::warn!("Dataset asset {}: space commitment is zero", id);
            return Ok(false);
        }

        Ok(true)
    }

    async fn handle_operation(
        &self,
        _id: &AssetRegistration,
        operation: AssetOperation,
    ) -> ExtensionResult<OperationResult> {
        match operation {
            AssetOperation::Custom(value) => {
                // Handle custom dataset operations
                Ok(OperationResult::Custom(value))
            }

            _ => Err(ExtensionError::RuntimeError {
                message: "Operation not supported for Dataset assets".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dataset_handler() {
        let handler = DatasetHandler::new();
        // STUB: Dataset replaced with Library
        assert_eq!(handler.asset_type(), AssetType::Dns);
    }
}
