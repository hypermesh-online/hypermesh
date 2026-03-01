// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use super::{
    async_trait, ApplicationDomain, Arc, AssetCategory, AssetCreationSpec, AssetData,
    AssetExtensionHandler, AssetMetadata, AssetOperation, AssetQuery, AssetRegistration, AssetType,
    AssetUpdate, ConsensusProof, ExtensionError, ExtensionResult, HashMap, NetworkScope,
    OperationResult, RwLock,
};

/// Handler for Template assets (asset generation templates)
pub struct TemplateHandler {
    /// Templates registry
    templates: Arc<RwLock<HashMap<AssetRegistration, Template>>>,
}

#[derive(Debug, Clone)]
struct Template {
    pub _id: AssetRegistration,
    pub name: String,
    pub template_type: String,
    pub language: String,
    pub parameters: Vec<String>,
}

impl Default for TemplateHandler {
    fn default() -> Self {
        Self::new()
    }
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
                domain_hash: *blake3::hash(b"catalog_template").as_bytes(),
            }),
        );

        let template = Template {
            _id: asset_id.clone(),
            name: spec.name.clone(),
            template_type: spec
                .metadata
                .get("template_type")
                .and_then(|v| v.as_str())
                .unwrap_or("generic")
                .to_string(),
            language: spec
                .metadata
                .get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("lua")
                .to_string(),
            parameters: vec![],
        };

        let mut templates = self.templates.write().await;
        templates.insert(asset_id.clone(), template);

        Ok(asset_id)
    }

    async fn update_asset(
        &self,
        id: &AssetRegistration,
        update: AssetUpdate,
    ) -> ExtensionResult<()> {
        let mut templates = self.templates.write().await;

        let template = templates
            .get_mut(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Template not found: {id}"),
            })?;

        if let Some(name) = update.name {
            template.name = name;
        }

        Ok(())
    }

    async fn delete_asset(&self, id: &AssetRegistration) -> ExtensionResult<()> {
        let mut templates = self.templates.write().await;
        templates
            .remove(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Template not found: {id}"),
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

        let template = templates
            .get(id)
            .ok_or_else(|| ExtensionError::RuntimeError {
                message: format!("Template not found: {id}"),
            })?;

        let mut metadata_map = HashMap::new();
        metadata_map.insert(
            "template_type".to_string(),
            serde_json::json!(template.template_type),
        );
        metadata_map.insert("language".to_string(), serde_json::json!(template.language));
        metadata_map.insert(
            "parameters".to_string(),
            serde_json::json!(template.parameters),
        );

        Ok(AssetMetadata {
            id: id.clone(),
            // STUB: Template no longer exists, using Library
            asset_type: AssetType::Dns,
            name: template.name.clone(),
            description: Some(format!(
                "{} template for {}",
                template.template_type, template.language
            )),
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
            size_bytes: 1024, // Templates are typically small
            metadata: metadata_map,
            privacy_level: hypermesh_lib::PrivacyMode::PUBLIC,
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

    async fn validate_asset(
        &self,
        id: &AssetRegistration,
        proof: ConsensusProof,
    ) -> ExtensionResult<bool> {
        let templates = self.templates.read().await;
        if !templates.contains_key(id) {
            return Ok(false);
        }

        // Validate all four Proof of State proofs (PoSpace, PoStake, PoWork, PoTime)
        if !proof.validate() {
            tracing::warn!("Template asset {}: Proof of State validation failed", id);
            return Ok(false);
        }

        // Template assets require valid stake and work proofs
        if proof.stake_proof.stake_amount == 0 {
            tracing::warn!("Template asset {}: stake amount is zero", id);
            return Ok(false);
        }
        if proof.work_proof.computational_power == 0 {
            tracing::warn!("Template asset {}: computational power is zero", id);
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
                // Handle template generation operations
                Ok(OperationResult::Custom(value))
            }

            _ => Err(ExtensionError::RuntimeError {
                message: "Operation not supported for Template assets".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_template_handler() {
        let handler = TemplateHandler::new();
        // Template handler uses Container type
        assert_eq!(handler.asset_type(), AssetType::Container);
    }
}
