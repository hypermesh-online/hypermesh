// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Catalog Template Generation System
//!
//! Provides template generation capabilities for creating new asset packages
//! from predefined templates with customizable parameters.

mod builtin_templates;
mod helpers;

use crate::assets::*;
use anyhow::{Context, Result};
use chrono::Utc;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Template generator for creating asset packages
pub struct CatalogTemplateGenerator {
    /// Handlebars template engine
    handlebars: Handlebars<'static>,
    /// Registered templates
    templates: HashMap<String, TemplateDefinition>,
    /// Template configuration
    _config: TemplateConfig,
}

/// Template configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Default template directory
    pub template_dir: String,
    /// Default author name
    pub default_author: Option<String>,
    /// Default license
    pub default_license: Option<String>,
    /// Template validation settings
    pub validation: TemplateValidationConfig,
    /// Custom helper functions enabled
    pub custom_helpers: bool,
}

/// Template validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateValidationConfig {
    /// Validate generated assets
    pub validate_output: bool,
    /// Check for required parameters
    pub check_required_params: bool,
    /// Validate parameter constraints
    pub validate_constraints: bool,
    /// Check security settings
    pub validate_security: bool,
}

/// Template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDefinition {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Template version
    pub version: String,
    /// Template runtime requirements
    pub runtime: TemplateRuntime,
    /// Template parameters
    pub parameters: Vec<TemplateParameter>,
    /// Template files
    pub files: HashMap<String, String>,
    /// Post-generation actions
    pub post_actions: Vec<PostGenerationAction>,
    /// Template metadata
    pub metadata: TemplateMetadata,
}

/// Template types - REMOVED: Use RuntimeRequirements instead
/// Templates now specify runtime type and version in metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRuntime {
    /// Runtime type (lua, python, wasm, native)
    pub runtime_type: String,
    /// Runtime version
    pub version: String,
}

/// Post-generation action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PostGenerationAction {
    /// Execute a command
    ExecuteCommand {
        /// Command to execute
        command: String,
        /// Working directory
        working_dir: Option<String>,
        /// Environment variables
        env: HashMap<String, String>,
    },
    /// Create additional files
    CreateFile {
        /// File path
        path: String,
        /// File content template
        content: String,
    },
    /// Copy files from source
    CopyFiles {
        /// Source directory
        source: String,
        /// Destination directory
        destination: String,
        /// File patterns to copy
        patterns: Vec<String>,
    },
    /// Download dependencies
    DownloadDependencies {
        /// Package manager to use
        package_manager: String,
        /// Dependencies to download
        dependencies: Vec<String>,
    },
}

/// Template metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Template author
    pub author: String,
    /// Creation date
    pub created: chrono::DateTime<Utc>,
    /// Last updated date
    pub updated: chrono::DateTime<Utc>,
    /// Template tags
    pub tags: Vec<String>,
    /// Compatible asset versions
    pub compatible_versions: Vec<String>,
    /// Required tools/dependencies
    pub required_tools: Vec<String>,
}

/// Template generation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateContext {
    /// Template parameters and values
    pub parameters: HashMap<String, serde_json::Value>,
    /// Output directory
    pub output_dir: String,
    /// Asset name
    pub asset_name: String,
    /// Asset version
    pub asset_version: String,
    /// Author information
    pub author: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Template generation result
#[derive(Debug, Clone)]
pub struct TemplateGenerationResult {
    /// Generated asset package
    pub asset_package: AssetPackage,
    /// Generated files
    pub generated_files: Vec<GeneratedFile>,
    /// Generation warnings
    pub warnings: Vec<String>,
    /// Generation timestamp
    pub generated_at: chrono::DateTime<Utc>,
}

/// Generated file information
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// File path relative to output directory
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// File hash
    pub hash: String,
    /// File type
    pub file_type: GeneratedFileType,
}

/// Generated file types
#[derive(Debug, Clone)]
pub enum GeneratedFileType {
    /// Asset specification YAML
    AssetSpec,
    /// Source code file
    SourceCode,
    /// Configuration file
    Configuration,
    /// Documentation file
    Documentation,
    /// Test file
    Test,
    /// Binary file
    Binary,
    /// Other file type
    Other(String),
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            template_dir: "templates".to_string(),
            default_author: None,
            default_license: Some("MIT".to_string()),
            validation: TemplateValidationConfig {
                validate_output: true,
                check_required_params: true,
                validate_constraints: true,
                validate_security: true,
            },
            custom_helpers: true,
        }
    }
}

impl CatalogTemplateGenerator {
    /// Create a new template generator
    pub fn new(config: TemplateConfig) -> Result<Self> {
        let mut handlebars = Handlebars::new();

        // Register built-in helpers
        Self::register_builtin_helpers(&mut handlebars)?;

        let mut generator = Self {
            handlebars,
            templates: HashMap::new(),
            _config: config,
        };

        // Load built-in templates
        generator.load_builtin_templates()?;

        Ok(generator)
    }

    /// Register built-in template helpers
    fn register_builtin_helpers(handlebars: &mut Handlebars) -> Result<()> {
        handlebars.register_helper("uuid", Box::new(helpers::uuid_helper));
        handlebars.register_helper("date", Box::new(helpers::date_helper));
        handlebars.register_helper("upper", Box::new(helpers::upper_helper));
        handlebars.register_helper("lower", Box::new(helpers::lower_helper));
        handlebars.register_helper("replace", Box::new(helpers::replace_helper));
        handlebars.register_helper("join", Box::new(helpers::join_helper));
        handlebars.register_helper("default", Box::new(helpers::default_helper));
        handlebars.register_helper("if_eq", Box::new(helpers::if_eq_helper));

        Ok(())
    }

    /// Register a new template
    pub fn register_template(&mut self, template: TemplateDefinition) -> Result<()> {
        // Validate template
        self.validate_template(&template)?;

        // Register template files with Handlebars
        for (file_name, content) in &template.files {
            let template_name = format!("{}:{}", template.name, file_name);
            self.handlebars
                .register_template_string(&template_name, content)
                .context("Failed to register template with Handlebars")?;
        }

        self.templates.insert(template.name.clone(), template);

        Ok(())
    }

    /// Validate template definition
    fn validate_template(&self, template: &TemplateDefinition) -> Result<()> {
        if template.name.is_empty() {
            return Err(anyhow::anyhow!("Template name cannot be empty"));
        }

        if template.files.is_empty() {
            return Err(anyhow::anyhow!("Template must have at least one file"));
        }

        for param in &template.parameters {
            self.validate_template_parameter(param)?;
        }

        Ok(())
    }

    /// Validate template parameter
    fn validate_template_parameter(&self, param: &TemplateParameter) -> Result<()> {
        if param.name.is_empty() {
            return Err(anyhow::anyhow!("Parameter name cannot be empty"));
        }

        if !["string", "number", "boolean", "array", "object"].contains(&param.param_type.as_str())
        {
            return Err(anyhow::anyhow!(
                "Invalid parameter type: {}",
                param.param_type
            ));
        }

        Ok(())
    }

    /// Generate asset package from template
    pub async fn generate_from_template(
        &self,
        template_name: &str,
        context: TemplateContext,
    ) -> Result<TemplateGenerationResult> {
        let template = self
            .templates
            .get(template_name)
            .ok_or_else(|| anyhow::anyhow!("Template '{template_name}' not found"))?;

        // Validate context parameters
        self.validate_context(template, &context)?;

        // Prepare template context
        let mut template_context = context.parameters.clone();
        template_context.insert(
            "asset_name".to_string(),
            serde_json::Value::String(context.asset_name.clone()),
        );
        template_context.insert(
            "asset_version".to_string(),
            serde_json::Value::String(context.asset_version.clone()),
        );

        if let Some(author) = &context.author {
            template_context.insert(
                "author".to_string(),
                serde_json::Value::String(author.clone()),
            );
        }

        for (key, value) in &context.metadata {
            template_context.insert(key.clone(), value.clone());
        }

        // Generate files
        let mut generated_files = Vec::new();
        let mut file_contents = HashMap::new();

        for file_name in template.files.keys() {
            let template_name_full = format!("{}:{}", template.name, file_name);

            let rendered_file_name = self
                .handlebars
                .render_template(file_name, &template_context)
                .context("Failed to render file name template")?;

            let rendered_content = self
                .handlebars
                .render(&template_name_full, &template_context)
                .context("Failed to render template content")?;

            let file_hash = blake3::hash(rendered_content.as_bytes())
                .to_hex()
                .to_string();

            generated_files.push(GeneratedFile {
                path: rendered_file_name.clone(),
                size: rendered_content.len() as u64,
                hash: file_hash,
                file_type: if rendered_file_name.ends_with(".yaml")
                    || rendered_file_name.ends_with(".yml")
                {
                    GeneratedFileType::AssetSpec
                } else if rendered_file_name.ends_with(".jl")
                    || rendered_file_name.ends_with(".lua")
                {
                    GeneratedFileType::SourceCode
                } else if rendered_file_name.starts_with("test_") {
                    GeneratedFileType::Test
                } else if rendered_file_name.ends_with(".md") {
                    GeneratedFileType::Documentation
                } else {
                    GeneratedFileType::Other("unknown".to_string())
                },
            });

            file_contents.insert(rendered_file_name, rendered_content);
        }

        // Write files to output directory
        tokio::fs::create_dir_all(&context.output_dir).await?;

        for (file_name, content) in &file_contents {
            let file_path = std::path::Path::new(&context.output_dir).join(file_name);

            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            tokio::fs::write(&file_path, content).await?;
        }

        // Execute post-generation actions
        for action in &template.post_actions {
            self.execute_post_action(action, &context).await?;
        }

        // Load the generated asset package
        let asset_spec_path = std::path::Path::new(&context.output_dir).join("asset.yaml");
        let asset_package = AssetPackage::from_yaml(&asset_spec_path).await?;

        Ok(TemplateGenerationResult {
            asset_package,
            generated_files,
            warnings: vec![],
            generated_at: Utc::now(),
        })
    }

    /// Execute post-generation action
    async fn execute_post_action(
        &self,
        action: &PostGenerationAction,
        context: &TemplateContext,
    ) -> Result<()> {
        match action {
            PostGenerationAction::CreateFile { path, content } => {
                let rendered_path = self.handlebars.render_template(path, &context.parameters)?;
                let rendered_content = self
                    .handlebars
                    .render_template(content, &context.parameters)?;

                let file_path = std::path::Path::new(&context.output_dir).join(rendered_path);

                if let Some(parent) = file_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                tokio::fs::write(file_path, rendered_content).await?;
            }

            PostGenerationAction::ExecuteCommand {
                command: _,
                working_dir: _,
                env: _,
            } => {
                tracing::error!(
                    "SECURITY VIOLATION: Template attempted to execute shell command. \
                     All execution must use HyperMesh infrastructure via catalog.hypermesh.online"
                );

                return Err(anyhow::anyhow!(
                    "Template shell execution disabled for security. \
                     Use HyperMesh asset execution instead of post-generation commands."
                ));
            }

            _ => {
                tracing::warn!("Post-generation action not implemented: {:?}", action);
            }
        }

        Ok(())
    }

    /// Validate template context
    fn validate_context(
        &self,
        template: &TemplateDefinition,
        context: &TemplateContext,
    ) -> Result<()> {
        for param in &template.parameters {
            if param.required && !context.parameters.contains_key(&param.name) {
                return Err(anyhow::anyhow!(
                    "Required parameter '{}' not provided",
                    param.name
                ));
            }

            if let Some(value) = context.parameters.get(&param.name) {
                self.validate_parameter_value(param, value)?;
            }
        }

        Ok(())
    }

    /// Validate parameter value against constraints
    fn validate_parameter_value(
        &self,
        param: &TemplateParameter,
        value: &serde_json::Value,
    ) -> Result<()> {
        if let Some(constraints) = &param.constraints {
            match param.param_type.as_str() {
                "string" => {
                    if let Some(s) = value.as_str() {
                        if let Some(min_len) = constraints.min_length {
                            if s.len() < min_len {
                                return Err(anyhow::anyhow!(
                                    "Parameter '{}' is too short (minimum {} characters)",
                                    param.name,
                                    min_len
                                ));
                            }
                        }

                        if let Some(max_len) = constraints.max_length {
                            if s.len() > max_len {
                                return Err(anyhow::anyhow!(
                                    "Parameter '{}' is too long (maximum {} characters)",
                                    param.name,
                                    max_len
                                ));
                            }
                        }

                        if let Some(pattern) = &constraints.pattern {
                            let regex = regex::Regex::new(pattern)?;
                            if !regex.is_match(s) {
                                return Err(anyhow::anyhow!(
                                    "Parameter '{}' does not match required pattern",
                                    param.name
                                ));
                            }
                        }

                        if let Some(allowed_values) = &constraints.allowed_values {
                            if !allowed_values.contains(value) {
                                return Err(anyhow::anyhow!(
                                    "Parameter '{}' has invalid value",
                                    param.name
                                ));
                            }
                        }
                    }
                }

                "number" => {
                    if let Some(n) = value.as_f64() {
                        if let Some(min) = constraints.min {
                            if n < min {
                                return Err(anyhow::anyhow!(
                                    "Parameter '{}' is too small (minimum {})",
                                    param.name,
                                    min
                                ));
                            }
                        }

                        if let Some(max) = constraints.max {
                            if n > max {
                                return Err(anyhow::anyhow!(
                                    "Parameter '{}' is too large (maximum {})",
                                    param.name,
                                    max
                                ));
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        Ok(())
    }

    /// List available templates
    pub fn list_templates(&self) -> Vec<&TemplateDefinition> {
        self.templates.values().collect()
    }

    /// Get template by name
    pub fn get_template(&self, name: &str) -> Option<&TemplateDefinition> {
        self.templates.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_template_generation() {
        let config = TemplateConfig::default();
        let generator = CatalogTemplateGenerator::new(config).expect("test: creation");

        let temp_dir = TempDir::new().expect("test: temp dir creation");

        let mut context_params = HashMap::new();
        context_params.insert(
            "program_name".to_string(),
            serde_json::Value::String("test_program".to_string()),
        );
        context_params.insert(
            "script_name".to_string(),
            serde_json::Value::String("test_script".to_string()),
        );
        context_params.insert(
            "description".to_string(),
            serde_json::Value::String("A test program".to_string()),
        );
        context_params.insert(
            "state_proof_required".to_string(),
            serde_json::Value::Bool(false),
        );
        context_params.insert(
            "sandbox_level".to_string(),
            serde_json::Value::String("standard".to_string()),
        );

        let context = TemplateContext {
            parameters: context_params,
            output_dir: temp_dir.path().to_string_lossy().to_string(),
            asset_name: "test_program".to_string(),
            asset_version: "1.0.0".to_string(),
            author: Some("Test Author".to_string()),
            metadata: HashMap::new(),
        };

        let result = generator
            .generate_from_template("lua-script", context)
            .await
            .expect("test: expected success");

        assert!(!result.generated_files.is_empty());
        assert_eq!(result.asset_package.spec.metadata.name, "test_program");
        assert_eq!(result.asset_package.spec.metadata.version, "1.0.0");
    }
}
