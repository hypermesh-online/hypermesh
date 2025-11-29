//! Catalog Template Generation System
//!
//! Provides template generation capabilities for creating new asset packages
//! from predefined templates with customizable parameters.

use crate::assets::*;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::Utc;
use handlebars::{Handlebars, Helper, Context as HbContext, RenderContext, Output, HelperResult, Renderable};

/// Template generator for creating asset packages
pub struct CatalogTemplateGenerator {
    /// Handlebars template engine
    handlebars: Handlebars<'static>,
    /// Registered templates
    templates: HashMap<String, TemplateDefinition>,
    /// Template configuration
    config: TemplateConfig,
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
    /// Template type
    pub template_type: TemplateType,
    /// Template parameters
    pub parameters: Vec<TemplateParameter>,
    /// Template files
    pub files: HashMap<String, String>,
    /// Post-generation actions
    pub post_actions: Vec<PostGenerationAction>,
    /// Template metadata
    pub metadata: TemplateMetadata,
}

/// Template types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateType {
    /// Julia program template
    JuliaProgram,
    /// Lua script template
    LuaScript,
    /// WASM module template
    WasmModule,
    /// Container application template
    ContainerApp,
    /// Machine learning model template
    MLModel,
    /// Data processing pipeline template
    DataPipeline,
    /// Security audit template
    SecurityAudit,
    /// Custom template type
    Custom(String),
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
            config,
        };
        
        // Load built-in templates
        generator.load_builtin_templates()?;
        
        Ok(generator)
    }
    
    /// Register built-in template helpers
    fn register_builtin_helpers(handlebars: &mut Handlebars) -> Result<()> {
        // Helper for generating UUIDs
        handlebars.register_helper("uuid", Box::new(uuid_helper));
        
        // Helper for formatting dates
        handlebars.register_helper("date", Box::new(date_helper));
        
        // Helper for converting to uppercase
        handlebars.register_helper("upper", Box::new(upper_helper));
        
        // Helper for converting to lowercase
        handlebars.register_helper("lower", Box::new(lower_helper));
        
        // Helper for replacing strings
        handlebars.register_helper("replace", Box::new(replace_helper));
        
        // Helper for joining arrays
        handlebars.register_helper("join", Box::new(join_helper));
        
        // Helper for default values
        handlebars.register_helper("default", Box::new(default_helper));
        
        // Helper for conditional inclusion
        handlebars.register_helper("if_eq", Box::new(if_eq_helper));
        
        Ok(())
    }
    
    /// Load built-in templates
    fn load_builtin_templates(&mut self) -> Result<()> {
        // Julia program template
        self.register_template(TemplateDefinition {
            name: "julia-program".to_string(),
            description: "Basic Julia program asset template".to_string(),
            version: "1.0.0".to_string(),
            template_type: TemplateType::JuliaProgram,
            parameters: vec![
                TemplateParameter {
                    name: "program_name".to_string(),
                    param_type: "string".to_string(),
                    default: None,
                    description: Some("Name of the Julia program".to_string()),
                    required: true,
                    constraints: Some(ParameterConstraints {
                        min: None,
                        max: None,
                        min_length: Some(1),
                        max_length: Some(64),
                        pattern: Some(r"^[a-zA-Z][a-zA-Z0-9_-]*$".to_string()),
                        allowed_values: None,
                    }),
                },
                TemplateParameter {
                    name: "main_function".to_string(),
                    param_type: "string".to_string(),
                    default: Some(serde_json::Value::String("main".to_string())),
                    description: Some("Name of the main function".to_string()),
                    required: false,
                    constraints: Some(ParameterConstraints {
                        min: None,
                        max: None,
                        min_length: Some(1),
                        max_length: Some(32),
                        pattern: Some(r"^[a-zA-Z][a-zA-Z0-9_]*$".to_string()),
                        allowed_values: None,
                    }),
                },
                TemplateParameter {
                    name: "include_tests".to_string(),
                    param_type: "boolean".to_string(),
                    default: Some(serde_json::Value::Bool(true)),
                    description: Some("Include test files".to_string()),
                    required: false,
                    constraints: None,
                },
            ],
            files: self.get_julia_template_files(),
            post_actions: vec![
                PostGenerationAction::CreateFile {
                    path: "README.md".to_string(),
                    content: include_str!("../templates/julia/README.md.hbs").to_string(),
                },
            ],
            metadata: TemplateMetadata {
                author: "Catalog Team".to_string(),
                created: Utc::now(),
                updated: Utc::now(),
                tags: vec!["julia".to_string(), "program".to_string(), "computation".to_string()],
                compatible_versions: vec!["^1.0.0".to_string()],
                required_tools: vec!["julia".to_string()],
            },
        })?;
        
        // Lua script template
        self.register_template(TemplateDefinition {
            name: "lua-script".to_string(),
            description: "Basic Lua script asset template".to_string(),
            version: "1.0.0".to_string(),
            template_type: TemplateType::LuaScript,
            parameters: vec![
                TemplateParameter {
                    name: "script_name".to_string(),
                    param_type: "string".to_string(),
                    default: None,
                    description: Some("Name of the Lua script".to_string()),
                    required: true,
                    constraints: Some(ParameterConstraints {
                        min: None,
                        max: None,
                        min_length: Some(1),
                        max_length: Some(64),
                        pattern: Some(r"^[a-zA-Z][a-zA-Z0-9_-]*$".to_string()),
                        allowed_values: None,
                    }),
                },
                TemplateParameter {
                    name: "sandbox_level".to_string(),
                    param_type: "string".to_string(),
                    default: Some(serde_json::Value::String("standard".to_string())),
                    description: Some("Security sandbox level".to_string()),
                    required: false,
                    constraints: Some(ParameterConstraints {
                        min: None,
                        max: None,
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        allowed_values: Some(vec![
                            serde_json::Value::String("minimal".to_string()),
                            serde_json::Value::String("standard".to_string()),
                            serde_json::Value::String("strict".to_string()),
                            serde_json::Value::String("paranoid".to_string()),
                        ]),
                    }),
                },
            ],
            files: self.get_lua_template_files(),
            post_actions: vec![],
            metadata: TemplateMetadata {
                author: "Catalog Team".to_string(),
                created: Utc::now(),
                updated: Utc::now(),
                tags: vec!["lua".to_string(), "script".to_string(), "logic".to_string()],
                compatible_versions: vec!["^1.0.0".to_string()],
                required_tools: vec!["lua".to_string()],
            },
        })?;
        
        Ok(())
    }
    
    /// Get Julia template files
    fn get_julia_template_files(&self) -> HashMap<String, String> {
        let mut files = HashMap::new();
        
        files.insert("asset.yaml".to_string(), r#"
apiVersion: "catalog.v1"
kind: "Asset"
metadata:
  name: "{{program_name}}"
  version: "{{asset_version}}"
  tags: ["julia", "program", "computation"]
  description: "{{description}}"
  {{#if author}}author: "{{author}}"{{/if}}
  license: "{{default license "MIT"}}"

spec:
  type: "julia-program"
  content:
    main: "{{program_name}}.jl"
    files:
      {{#if include_tests}}- "test_{{program_name}}.jl"{{/if}}
    binary: []
    templates: []
    
  security:
    consensus_required: {{consensus_required}}
    certificate_pinning: false
    hash_validation: "sha256"
    sandbox_level: "standard"
    allowed_syscalls: []
    network_access:
      enabled: false
      allowed_domains: []
      allowed_ports: []
      require_tls: true
    file_access:
      level: "read_only"
      allowed_paths: []
      denied_paths: []
      allow_temp: true
    permissions: []
    
  resources:
    cpu_limit: "{{default cpu_limit "2000m"}}"
    memory_limit: "{{default memory_limit "2Gi"}}"
    execution_timeout: "{{default execution_timeout "300s"}}"
    gpu_required: {{default gpu_required false}}
    hardware_requirements: []
    
  execution:
    delegation_strategy: "high_performance_cluster"
    minimum_consensus: {{default minimum_consensus 3}}
    retry_policy: "exponential_backoff"
    priority: "normal"
    timeout_config:
      execution: "{{default execution_timeout "300s"}}"
      network: "30s"
      io: "10s"
      compilation: "60s"
    scheduling:
      timing: "immediate"
      allocation_strategy: "best_fit"
      node_affinity: []
      anti_affinity: []
      
  dependencies: []
  environment: {}
"#.to_string());

        files.insert("{{program_name}}.jl".to_string(), r#"
# {{program_name}} - {{description}}
# Generated by Catalog Template Generator on {{date "Y-m-d H:M:S"}}

using LinearAlgebra
using Statistics

"""
    {{main_function}}()

Main function for {{program_name}}.
{{description}}
"""
function {{main_function}}()
    println("Starting {{program_name}}...")
    
    # Your Julia code goes here
    result = perform_computation()
    
    println("{{program_name}} completed successfully!")
    return result
end

"""
    perform_computation()

Core computation logic for {{program_name}}.
"""
function perform_computation()
    # Example computation - replace with your actual logic
    data = randn(1000, 100)
    
    # Perform some mathematical operations
    mean_values = mean(data, dims=1)
    std_values = std(data, dims=1)
    
    # Return results
    return Dict(
        "mean" => mean_values,
        "std" => std_values,
        "size" => size(data)
    )
end

# Execute main function if script is run directly
if abspath(PROGRAM_FILE) == @__FILE__
    result = {{main_function}}()
    println("Result: ", result)
end
"#.to_string());

        if true { // This would be conditional based on include_tests parameter
            files.insert("test_{{program_name}}.jl".to_string(), r#"
# Test suite for {{program_name}}
# Generated by Catalog Template Generator

using Test
include("{{program_name}}.jl")

@testset "{{program_name}} Tests" begin
    @testset "Main Function Tests" begin
        @test {{main_function}}() isa Dict
        
        result = {{main_function}}()
        @test haskey(result, "mean")
        @test haskey(result, "std")
        @test haskey(result, "size")
    end
    
    @testset "Computation Tests" begin
        result = perform_computation()
        @test result isa Dict
        @test result["size"] == (1000, 100)
    end
end

# Run tests
if abspath(PROGRAM_FILE) == @__FILE__
    println("Running tests for {{program_name}}...")
    # Tests run automatically when file is included
end
"#.to_string());
        }
        
        files
    }
    
    /// Get Lua template files
    fn get_lua_template_files(&self) -> HashMap<String, String> {
        let mut files = HashMap::new();
        
        files.insert("asset.yaml".to_string(), r#"
apiVersion: "catalog.v1"
kind: "Asset"
metadata:
  name: "{{script_name}}"
  version: "{{asset_version}}"
  tags: ["lua", "script", "logic"]
  description: "{{description}}"
  {{#if author}}author: "{{author}}"{{/if}}
  license: "{{default license "MIT"}}"

spec:
  type: "lua-script"
  content:
    main: "{{script_name}}.lua"
    files: []
    binary: []
    templates: []
    
  security:
    consensus_required: false
    certificate_pinning: false
    hash_validation: "sha256"
    sandbox_level: "{{sandbox_level}}"
    allowed_syscalls: []
    network_access:
      enabled: false
      allowed_domains: []
      allowed_ports: []
      require_tls: true
    file_access:
      level: "read_only"
      allowed_paths: []
      denied_paths: []
      allow_temp: false
    permissions: []
    
  resources:
    cpu_limit: "{{default cpu_limit "500m"}}"
    memory_limit: "{{default memory_limit "512Mi"}}"
    execution_timeout: "{{default execution_timeout "30s"}}"
    gpu_required: false
    hardware_requirements: []
    
  execution:
    delegation_strategy: "load_balanced"
    minimum_consensus: 1
    retry_policy: "simple"
    priority: "normal"
    timeout_config:
      execution: "{{default execution_timeout "30s"}}"
      network: "10s"
      io: "5s"
    scheduling:
      timing: "immediate"
      allocation_strategy: "first_fit"
      node_affinity: []
      anti_affinity: []
      
  dependencies: []
  environment: {}
"#.to_string());

        files.insert("{{script_name}}.lua".to_string(), r#"
-- {{script_name}} - {{description}}
-- Generated by Catalog Template Generator on {{date "Y-m-d H:M:S"}}

local {{script_name}} = {}

-- Main function
function {{script_name}}.main()
    print("Starting {{script_name}}...")
    
    -- Your Lua code goes here
    local result = {{script_name}}.process_data()
    
    print("{{script_name}} completed successfully!")
    return result
end

-- Core processing logic
function {{script_name}}.process_data()
    -- Example data processing - replace with your actual logic
    local data = {
        values = {1, 2, 3, 4, 5},
        metadata = {
            created = os.date("%Y-%m-%d %H:%M:%S"),
            source = "{{script_name}}"
        }
    }
    
    -- Process the data
    local sum = 0
    for i, value in ipairs(data.values) do
        sum = sum + value
    end
    
    return {
        sum = sum,
        count = #data.values,
        average = sum / #data.values,
        metadata = data.metadata
    }
end

-- Utility function
function {{script_name}}.validate_input(input)
    return type(input) == "table" and input ~= nil
end

-- Execute if run directly
if debug.getinfo(2) == nil then
    local result = {{script_name}}.main()
    print("Result:", result.sum, result.average)
end

return {{script_name}}
"#.to_string());
        
        files
    }
    
    /// Register a new template
    pub fn register_template(&mut self, template: TemplateDefinition) -> Result<()> {
        // Validate template
        self.validate_template(&template)?;
        
        // Register template files with Handlebars
        for (file_name, content) in &template.files {
            let template_name = format!("{}:{}", template.name, file_name);
            self.handlebars.register_template_string(&template_name, content)
                .context("Failed to register template with Handlebars")?;
        }
        
        self.templates.insert(template.name.clone(), template);
        
        Ok(())
    }
    
    /// Validate template definition
    fn validate_template(&self, template: &TemplateDefinition) -> Result<()> {
        // Check required fields
        if template.name.is_empty() {
            return Err(anyhow::anyhow!("Template name cannot be empty"));
        }
        
        if template.files.is_empty() {
            return Err(anyhow::anyhow!("Template must have at least one file"));
        }
        
        // Validate parameters
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
        
        if !["string", "number", "boolean", "array", "object"].contains(&param.param_type.as_str()) {
            return Err(anyhow::anyhow!("Invalid parameter type: {}", param.param_type));
        }
        
        Ok(())
    }
    
    /// Generate asset package from template
    pub async fn generate_from_template(
        &self,
        template_name: &str,
        context: TemplateContext,
    ) -> Result<TemplateGenerationResult> {
        let template = self.templates.get(template_name)
            .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template_name))?;
        
        // Validate context parameters
        self.validate_context(template, &context)?;
        
        // Prepare template context
        let mut template_context = context.parameters.clone();
        template_context.insert("asset_name".to_string(), serde_json::Value::String(context.asset_name.clone()));
        template_context.insert("asset_version".to_string(), serde_json::Value::String(context.asset_version.clone()));
        
        if let Some(author) = &context.author {
            template_context.insert("author".to_string(), serde_json::Value::String(author.clone()));
        }
        
        for (key, value) in &context.metadata {
            template_context.insert(key.clone(), value.clone());
        }
        
        // Generate files
        let mut generated_files = Vec::new();
        let mut file_contents = HashMap::new();
        
        for (file_name, _) in &template.files {
            let template_name_full = format!("{}:{}", template.name, file_name);
            
            // Render file name template
            let rendered_file_name = self.handlebars.render_template(file_name, &template_context)
                .context("Failed to render file name template")?;
            
            // Render file content
            let rendered_content = self.handlebars.render(&template_name_full, &template_context)
                .context("Failed to render template content")?;
            
            // Calculate file hash
            let file_hash = {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(rendered_content.as_bytes());
                hex::encode(hasher.finalize())
            };
            
            generated_files.push(GeneratedFile {
                path: rendered_file_name.clone(),
                size: rendered_content.len() as u64,
                hash: file_hash,
                file_type: if rendered_file_name.ends_with(".yaml") || rendered_file_name.ends_with(".yml") {
                    GeneratedFileType::AssetSpec
                } else if rendered_file_name.ends_with(".jl") || rendered_file_name.ends_with(".lua") {
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
            
            // Create parent directories if needed
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
            warnings: vec![], // TODO: Collect warnings during generation
            generated_at: Utc::now(),
        })
    }
    
    /// Execute post-generation action
    async fn execute_post_action(&self, action: &PostGenerationAction, context: &TemplateContext) -> Result<()> {
        match action {
            PostGenerationAction::CreateFile { path, content } => {
                let rendered_path = self.handlebars.render_template(path, &context.parameters)?;
                let rendered_content = self.handlebars.render_template(content, &context.parameters)?;
                
                let file_path = std::path::Path::new(&context.output_dir).join(rendered_path);
                
                if let Some(parent) = file_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                
                tokio::fs::write(file_path, rendered_content).await?;
            }
            
            PostGenerationAction::ExecuteCommand { command: _, working_dir: _, env: _ } => {
                // CRITICAL SECURITY: Disabled shell command execution
                // Previous implementation used tokio::process::Command::new("sh") with arbitrary commands
                // This created a command injection vulnerability

                tracing::error!(
                    "SECURITY VIOLATION: Template attempted to execute shell command. \
                     All execution must use HyperMesh infrastructure via catalog.hypermesh.online"
                );

                return Err(anyhow::anyhow!(
                    "Template shell execution disabled for security. \
                     Use HyperMesh asset execution instead of post-generation commands."
                ));

                // Previous vulnerable code was fully removed above
                // No shell command execution allowed in HyperMesh architecture
            }
            
            _ => {
                // TODO: Implement other post-generation actions
                tracing::warn!("Post-generation action not implemented: {:?}", action);
            }
        }
        
        Ok(())
    }
    
    /// Validate template context
    fn validate_context(&self, template: &TemplateDefinition, context: &TemplateContext) -> Result<()> {
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
    fn validate_parameter_value(&self, param: &TemplateParameter, value: &serde_json::Value) -> Result<()> {
        if let Some(constraints) = &param.constraints {
            match param.param_type.as_str() {
                "string" => {
                    if let Some(s) = value.as_str() {
                        if let Some(min_len) = constraints.min_length {
                            if s.len() < min_len {
                                return Err(anyhow::anyhow!(
                                    "Parameter '{}' is too short (minimum {} characters)",
                                    param.name, min_len
                                ));
                            }
                        }
                        
                        if let Some(max_len) = constraints.max_length {
                            if s.len() > max_len {
                                return Err(anyhow::anyhow!(
                                    "Parameter '{}' is too long (maximum {} characters)",
                                    param.name, max_len
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
                                    param.name, min
                                ));
                            }
                        }
                        
                        if let Some(max) = constraints.max {
                            if n > max {
                                return Err(anyhow::anyhow!(
                                    "Parameter '{}' is too large (maximum {})",
                                    param.name, max
                                ));
                            }
                        }
                    }
                }
                
                _ => {} // TODO: Validate other parameter types
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

// Handlebars helper functions

fn uuid_helper(_: &Helper, _: &Handlebars, _: &HbContext, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    let uuid = uuid::Uuid::new_v4();
    out.write(&uuid.to_string())?;
    Ok(())
}

fn date_helper(h: &Helper, _: &Handlebars, _: &HbContext, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    let format = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("%Y-%m-%d %H:%M:%S");
    let now = chrono::Utc::now();
    let formatted = now.format(format).to_string();
    out.write(&formatted)?;
    Ok(())
}

fn upper_helper(h: &Helper, _: &Handlebars, _: &HbContext, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(s) = param.value().as_str() {
            out.write(&s.to_uppercase())?;
        }
    }
    Ok(())
}

fn lower_helper(h: &Helper, _: &Handlebars, _: &HbContext, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(s) = param.value().as_str() {
            out.write(&s.to_lowercase())?;
        }
    }
    Ok(())
}

fn replace_helper(h: &Helper, _: &Handlebars, _: &HbContext, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if let (Some(text), Some(from), Some(to)) = (
        h.param(0).and_then(|v| v.value().as_str()),
        h.param(1).and_then(|v| v.value().as_str()),
        h.param(2).and_then(|v| v.value().as_str()),
    ) {
        let result = text.replace(from, to);
        out.write(&result)?;
    }
    Ok(())
}

fn join_helper(h: &Helper, _: &Handlebars, _: &HbContext, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if let (Some(array), Some(separator)) = (
        h.param(0).and_then(|v| v.value().as_array()),
        h.param(1).and_then(|v| v.value().as_str()),
    ) {
        let strings: Vec<String> = array.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        let result = strings.join(separator);
        out.write(&result)?;
    }
    Ok(())
}

fn default_helper(h: &Helper, _: &Handlebars, ctx: &HbContext, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if let Some(param) = h.param(0) {
        let var_name = param.value().as_str().unwrap_or("");
        
        // Try to get the value from context
        if let Some(value) = ctx.data().get(var_name) {
            if !value.is_null() {
                out.write(&value.to_string())?;
                return Ok(());
            }
        }
    }
    
    // Use default value
    if let Some(default) = h.param(1) {
        out.write(&default.value().to_string())?;
    }
    
    Ok(())
}

fn if_eq_helper(h: &Helper, _: &Handlebars, _ctx: &HbContext, _rc: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    let param1 = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let param2 = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");
    
    if param1 == param2 {
        out.write("true")?;
    } else {
        out.write("false")?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_template_generation() {
        let config = TemplateConfig::default();
        let generator = CatalogTemplateGenerator::new(config).unwrap();
        
        let temp_dir = TempDir::new().unwrap();
        
        let mut context_params = HashMap::new();
        context_params.insert("program_name".to_string(), serde_json::Value::String("test_program".to_string()));
        context_params.insert("description".to_string(), serde_json::Value::String("A test program".to_string()));
        context_params.insert("consensus_required".to_string(), serde_json::Value::Bool(false));
        
        let context = TemplateContext {
            parameters: context_params,
            output_dir: temp_dir.path().to_string_lossy().to_string(),
            asset_name: "test_program".to_string(),
            asset_version: "1.0.0".to_string(),
            author: Some("Test Author".to_string()),
            metadata: HashMap::new(),
        };
        
        let result = generator.generate_from_template("julia-program", context).await.unwrap();
        
        assert!(!result.generated_files.is_empty());
        assert_eq!(result.asset_package.spec.metadata.name, "test_program");
        assert_eq!(result.asset_package.spec.metadata.version, "1.0.0");
    }
}