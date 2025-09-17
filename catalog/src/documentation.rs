//! Documentation Generation System
//!
//! Automatically generates comprehensive documentation for asset packages
//! including API documentation, usage examples, and deployment guides.

use crate::assets::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use handlebars::Handlebars;

/// Documentation generator for asset packages
pub struct DocumentationGenerator {
    /// Handlebars template engine
    handlebars: Handlebars<'static>,
    /// Documentation templates
    templates: HashMap<String, String>,
    /// Generation configuration
    config: DocumentationConfig,
}

/// Documentation generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationConfig {
    /// Output format preferences
    pub formats: Vec<DocumentationFormat>,
    /// Include API documentation
    pub include_api_docs: bool,
    /// Include usage examples
    pub include_examples: bool,
    /// Include deployment guide
    pub include_deployment: bool,
    /// Include performance benchmarks
    pub include_benchmarks: bool,
    /// Custom sections to include
    pub custom_sections: Vec<CustomSection>,
    /// Documentation theme
    pub theme: DocumentationTheme,
}

/// Documentation output formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DocumentationFormat {
    /// Markdown format
    Markdown,
    /// HTML format
    Html,
    /// PDF format
    Pdf,
    /// JSON format (structured data)
    Json,
    /// Plain text format
    Text,
}

/// Custom documentation section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSection {
    /// Section name
    pub name: String,
    /// Section content template
    pub template: String,
    /// Section order (lower = earlier)
    pub order: u32,
    /// Whether section is required
    pub required: bool,
}

/// Documentation theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationTheme {
    /// Theme name
    pub name: String,
    /// Primary color
    pub primary_color: String,
    /// Secondary color
    pub secondary_color: String,
    /// Font family
    pub font_family: String,
    /// Logo URL
    pub logo_url: Option<String>,
    /// Custom CSS
    pub custom_css: Option<String>,
}

/// Generated documentation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedDocumentation {
    /// Documentation in different formats
    pub formats: HashMap<DocumentationFormat, String>,
    /// Generated files
    pub files: Vec<DocumentationFile>,
    /// Documentation metadata
    pub metadata: DocumentationMetadata,
    /// Generation timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Documentation file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationFile {
    /// File name
    pub name: String,
    /// File content
    pub content: String,
    /// File format
    pub format: DocumentationFormat,
    /// File size in bytes
    pub size: u64,
}

/// Documentation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationMetadata {
    /// Asset name
    pub asset_name: String,
    /// Asset version
    pub asset_version: String,
    /// Documentation version
    pub doc_version: String,
    /// Number of sections
    pub section_count: u32,
    /// Total word count
    pub word_count: u32,
    /// Estimated reading time (minutes)
    pub reading_time: u32,
}

impl Default for DocumentationConfig {
    fn default() -> Self {
        Self {
            formats: vec![DocumentationFormat::Markdown, DocumentationFormat::Html],
            include_api_docs: true,
            include_examples: true,
            include_deployment: true,
            include_benchmarks: false,
            custom_sections: vec![],
            theme: DocumentationTheme {
                name: "default".to_string(),
                primary_color: "#2563eb".to_string(),
                secondary_color: "#64748b".to_string(),
                font_family: "Inter, system-ui, sans-serif".to_string(),
                logo_url: None,
                custom_css: None,
            },
        }
    }
}

impl DocumentationGenerator {
    /// Create a new documentation generator
    pub fn new(config: DocumentationConfig) -> Result<Self> {
        let mut handlebars = Handlebars::new();
        
        let mut generator = Self {
            handlebars,
            templates: HashMap::new(),
            config,
        };
        
        // Load built-in templates
        generator.load_builtin_templates()?;
        
        Ok(generator)
    }
    
    /// Load built-in documentation templates
    fn load_builtin_templates(&mut self) -> Result<()> {
        // Main README template
        let readme_template = r#"# {{asset.name}}

{{#if asset.description}}{{asset.description}}{{else}}{{asset.name}} - A Catalog asset package{{/if}}

## Overview

- **Version**: {{asset.version}}
- **Type**: {{asset.type}}
{{#if asset.author}}- **Author**: {{asset.author}}{{/if}}
{{#if asset.license}}- **License**: {{asset.license}}{{/if}}

## Installation

```bash
catalog install {{asset.name}}
```

## Configuration

### Resource Requirements

- **CPU**: {{resources.cpu_limit}}
- **Memory**: {{resources.memory_limit}}
- **Execution Timeout**: {{resources.execution_timeout}}

### Security Settings

- **Sandbox Level**: {{security.sandbox_level}}
- **Consensus Required**: {{security.consensus_required}}

---

*Generated by Catalog Documentation Generator*"#;
        
        self.templates.insert("readme".to_string(), readme_template.to_string());
        
        // Simple HTML template
        let html_template = r#"<!DOCTYPE html>
<html>
<head><title>{{asset.name}}</title></head>
<body>
<h1>{{asset.name}}</h1>
<p>{{asset.description}}</p>
<p>Version: {{asset.version}}</p>
</body>
</html>"#;
        
        self.templates.insert("html".to_string(), html_template.to_string());
        
        // Register templates with Handlebars
        for (name, template) in &self.templates {
            self.handlebars.register_template_string(name, template)?;
        }
        
        Ok(())
    }
    
    /// Generate comprehensive documentation for an asset package
    pub async fn generate(&self, package: &AssetPackage) -> Result<GeneratedDocumentation> {
        let mut formats = HashMap::new();
        let mut files = Vec::new();
        
        // Prepare template context
        let context = self.prepare_context(package).await?;
        
        // Generate documentation in requested formats
        for format in &self.config.formats {
            match format {
                DocumentationFormat::Markdown => {
                    let markdown = self.generate_markdown(&context).await?;
                    formats.insert(DocumentationFormat::Markdown, markdown.clone());
                    files.push(DocumentationFile {
                        name: "README.md".to_string(),
                        content: markdown.clone(),
                        format: DocumentationFormat::Markdown,
                        size: markdown.len() as u64,
                    });
                }
                
                DocumentationFormat::Html => {
                    let html = self.generate_html(&context).await?;
                    formats.insert(DocumentationFormat::Html, html.clone());
                    files.push(DocumentationFile {
                        name: "index.html".to_string(),
                        content: html.clone(),
                        format: DocumentationFormat::Html,
                        size: html.len() as u64,
                    });
                }
                
                DocumentationFormat::Json => {
                    let json = self.generate_json(&context).await?;
                    formats.insert(DocumentationFormat::Json, json.clone());
                    files.push(DocumentationFile {
                        name: "documentation.json".to_string(),
                        content: json.clone(),
                        format: DocumentationFormat::Json,
                        size: json.len() as u64,
                    });
                }
                
                _ => {
                    // TODO: Implement other formats
                    tracing::warn!("Documentation format {:?} not yet implemented", format);
                }
            }
        }
        
        // Generate metadata
        let metadata = self.generate_metadata(package, &files);
        
        Ok(GeneratedDocumentation {
            formats,
            files,
            metadata,
            generated_at: chrono::Utc::now(),
        })
    }
    
    /// Prepare template context from asset package
    async fn prepare_context(&self, package: &AssetPackage) -> Result<serde_json::Value> {
        let mut context = serde_json::json!({
            "asset": {
                "name": package.spec.metadata.name,
                "version": package.spec.metadata.version,
                "description": package.spec.metadata.description,
                "author": package.spec.metadata.author,
                "license": package.spec.metadata.license,
                "repository": package.spec.metadata.repository,
                "homepage": package.spec.metadata.homepage,
                "tags": package.spec.metadata.tags,
                "keywords": package.spec.metadata.keywords,
                "type": package.spec.spec.asset_type,
                "created": package.spec.metadata.created,
                "updated": package.spec.metadata.updated,
            },
            "security": {
                "consensus_required": package.spec.spec.security.consensus_required,
                "sandbox_level": package.spec.spec.security.sandbox_level,
                "hash_validation": package.spec.spec.security.hash_validation,
                "network_access": package.spec.spec.security.network_access,
                "file_access": package.spec.spec.security.file_access,
            },
            "resources": {
                "cpu_limit": package.spec.spec.resources.cpu_limit,
                "memory_limit": package.spec.spec.resources.memory_limit,
                "execution_timeout": package.spec.spec.resources.execution_timeout,
                "gpu_required": package.spec.spec.resources.gpu_required,
            },
            "execution": {
                "delegation_strategy": package.spec.spec.execution.delegation_strategy,
                "minimum_consensus": package.spec.spec.execution.minimum_consensus,
                "retry_policy": package.spec.spec.execution.retry_policy,
                "priority": package.spec.spec.execution.priority,
            },
            "dependencies": package.spec.spec.dependencies,
            "environment": package.spec.spec.environment,
            "validation": {
                "is_valid": package.validation.is_valid,
                "security_score": package.validation.security_results.security_score,
                "errors": package.validation.errors,
                "warnings": package.validation.warnings,
            },
            "content": {
                "main_file": package.spec.spec.content.main,
                "additional_files": package.spec.spec.content.files,
                "binary_files": package.spec.spec.content.binary.iter().map(|b| &b.name).collect::<Vec<_>>(),
                "has_inline_content": package.spec.spec.content.inline.is_some(),
            },
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "generator": "Catalog Documentation Generator",
            "version": env!("CARGO_PKG_VERSION"),
        });
        
        // Add API documentation if available
        if self.config.include_api_docs {
            context["api"] = self.extract_api_documentation(package).await?;
        }
        
        // Add usage examples if available
        if self.config.include_examples {
            context["examples"] = self.extract_usage_examples(package).await?;
        }
        
        // Add deployment information
        if self.config.include_deployment {
            context["deployment"] = self.generate_deployment_info(package).await?;
        }
        
        // Add theme configuration
        context["theme"] = serde_json::to_value(&self.config.theme)?;
        
        Ok(context)
    }
    
    /// Extract API documentation from package content
    async fn extract_api_documentation(&self, package: &AssetPackage) -> Result<serde_json::Value> {
        let mut api_docs = serde_json::json!({
            "functions": [],
            "classes": [],
            "modules": [],
            "constants": [],
        });
        
        match package.spec.spec.asset_type.as_str() {
            "julia-program" => {
                api_docs = self.extract_julia_api(&package.content.main_content).await?;
            }
            "lua-script" => {
                api_docs = self.extract_lua_api(&package.content.main_content).await?;
            }
            _ => {
                // Generic API extraction
                api_docs["notes"] = serde_json::Value::String(
                    "API documentation extraction not implemented for this asset type".to_string()
                );
            }
        }
        
        Ok(api_docs)
    }
    
    /// Extract Julia API documentation
    async fn extract_julia_api(&self, content: &str) -> Result<serde_json::Value> {
        let mut functions = Vec::new();
        let mut constants = Vec::new();
        
        // Simple regex-based extraction (could be improved with proper parsing)
        let function_regex = regex::Regex::new(r#"(?m)^function\s+(\w+)\s*\(([^)]*)\)"#)?;
        let const_regex = regex::Regex::new(r#"(?m)^const\s+(\w+)\s*=\s*(.+)$"#)?;
        let comment_regex = regex::Regex::new(r#"(?m)^#\s*(.+)$"#)?;
        
        // Extract functions
        for cap in function_regex.captures_iter(content) {
            let name = cap.get(1).unwrap().as_str();
            let params = cap.get(2).unwrap().as_str();
            
            // Look for documentation comment above function
            let function_start = cap.get(0).unwrap().start();
            let lines_before = content[..function_start].lines().rev().take(10);
            let mut doc_comments = Vec::new();
            
            for line in lines_before {
                let trimmed = line.trim();
                if trimmed.starts_with('#') && !trimmed.starts_with("##") {
                    doc_comments.push(trimmed.trim_start_matches('#').trim());
                } else if !trimmed.is_empty() {
                    break;
                }
            }
            doc_comments.reverse();
            
            functions.push(serde_json::json!({
                "name": name,
                "parameters": params,
                "description": doc_comments.join(" "),
                "type": "function"
            }));
        }
        
        // Extract constants
        for cap in const_regex.captures_iter(content) {
            let name = cap.get(1).unwrap().as_str();
            let value = cap.get(2).unwrap().as_str();
            
            constants.push(serde_json::json!({
                "name": name,
                "value": value,
                "type": "constant"
            }));
        }
        
        Ok(serde_json::json!({
            "functions": functions,
            "constants": constants,
            "modules": [],
            "classes": []
        }))
    }
    
    /// Extract Lua API documentation
    async fn extract_lua_api(&self, content: &str) -> Result<serde_json::Value> {
        let mut functions = Vec::new();
        let mut tables = Vec::new();
        
        // Simple regex-based extraction for Lua
        let function_regex = regex::Regex::new(r#"(?m)^(?:local\s+)?function\s+([^(]+)\s*\(([^)]*)\)"#)?;
        let table_regex = regex::Regex::new(r#"(?m)^(?:local\s+)?(\w+)\s*=\s*\{"#)?;
        
        // Extract functions
        for cap in function_regex.captures_iter(content) {
            let name = cap.get(1).unwrap().as_str();
            let params = cap.get(2).unwrap().as_str();
            
            // Look for documentation comment above function
            let function_start = cap.get(0).unwrap().start();
            let lines_before = content[..function_start].lines().rev().take(10);
            let mut doc_comments = Vec::new();
            
            for line in lines_before {
                let trimmed = line.trim();
                if trimmed.starts_with("--") && !trimmed.starts_with("---") {
                    doc_comments.push(trimmed.trim_start_matches("--").trim());
                } else if !trimmed.is_empty() {
                    break;
                }
            }
            doc_comments.reverse();
            
            functions.push(serde_json::json!({
                "name": name,
                "parameters": params,
                "description": doc_comments.join(" "),
                "type": "function"
            }));
        }
        
        // Extract tables
        for cap in table_regex.captures_iter(content) {
            let name = cap.get(1).unwrap().as_str();
            
            tables.push(serde_json::json!({
                "name": name,
                "type": "table"
            }));
        }
        
        Ok(serde_json::json!({
            "functions": functions,
            "tables": tables,
            "modules": [],
            "constants": []
        }))
    }
    
    /// Extract usage examples from package
    async fn extract_usage_examples(&self, package: &AssetPackage) -> Result<serde_json::Value> {
        let mut examples = Vec::new();
        
        // Look for example comments in main content
        let example_patterns = [
            r#"(?m)^#\s*Example:?\s*\n((?:^#.*\n?)*)"#,  // Julia examples
            r#"(?m)^--\s*Example:?\s*\n((?:^--.*\n?)*)"#, // Lua examples
        ];
        
        for pattern in &example_patterns {
            let regex = regex::Regex::new(pattern)?;
            for cap in regex.captures_iter(&package.content.main_content) {
                let example_text = cap.get(1).unwrap().as_str()
                    .lines()
                    .map(|line| line.trim_start_matches('#').trim_start_matches("--").trim())
                    .collect::<Vec<_>>()
                    .join("\n");
                
                if !example_text.trim().is_empty() {
                    examples.push(serde_json::json!({
                        "title": "Basic Usage",
                        "code": example_text,
                        "language": package.spec.spec.asset_type.split('-').next().unwrap_or("text")
                    }));
                }
            }
        }
        
        // Add basic execution example
        examples.push(serde_json::json!({
            "title": "Running the Asset",
            "code": format!(
                "# Install the asset\ncatalog install {}\n\n# Execute the asset\ncatalog run {} --config config.yaml",
                package.spec.metadata.name,
                package.spec.metadata.name
            ),
            "language": "bash"
        }));
        
        Ok(serde_json::json!(examples))
    }
    
    /// Generate deployment information
    async fn generate_deployment_info(&self, package: &AssetPackage) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "requirements": {
                "cpu": package.spec.spec.resources.cpu_limit,
                "memory": package.spec.spec.resources.memory_limit,
                "timeout": package.spec.spec.resources.execution_timeout,
                "gpu": package.spec.spec.resources.gpu_required,
                "consensus": package.spec.spec.execution.minimum_consensus,
            },
            "security": {
                "sandbox_level": package.spec.spec.security.sandbox_level,
                "network_access": package.spec.spec.security.network_access.enabled,
                "file_access": package.spec.spec.security.file_access.level,
            },
            "steps": [
                "1. Verify asset package integrity",
                "2. Check resource requirements",
                "3. Validate security permissions",
                "4. Deploy to execution environment",
                "5. Configure monitoring and logging",
                "6. Test execution and validate results"
            ],
            "monitoring": {
                "metrics": ["cpu_usage", "memory_usage", "execution_time", "error_rate"],
                "alerts": ["resource_exhaustion", "execution_timeout", "security_violation"]
            }
        }))
    }
    
    /// Generate Markdown documentation
    async fn generate_markdown(&self, context: &serde_json::Value) -> Result<String> {
        self.handlebars.render("readme", context)
            .map_err(|e| anyhow::anyhow!("Failed to render markdown template: {}", e))
    }
    
    /// Generate HTML documentation
    async fn generate_html(&self, context: &serde_json::Value) -> Result<String> {
        self.handlebars.render("html", context)
            .map_err(|e| anyhow::anyhow!("Failed to render HTML template: {}", e))
    }
    
    /// Generate JSON documentation
    async fn generate_json(&self, context: &serde_json::Value) -> Result<String> {
        serde_json::to_string_pretty(context)
            .map_err(|e| anyhow::anyhow!("Failed to serialize JSON documentation: {}", e))
    }
    
    /// Generate documentation metadata
    fn generate_metadata(&self, package: &AssetPackage, files: &[DocumentationFile]) -> DocumentationMetadata {
        let total_content: String = files.iter().map(|f| f.content.as_str()).collect();
        let word_count = total_content.split_whitespace().count() as u32;
        let reading_time = (word_count / 200).max(1); // Assume 200 words per minute
        
        DocumentationMetadata {
            asset_name: package.spec.metadata.name.clone(),
            asset_version: package.spec.metadata.version.clone(),
            doc_version: "1.0.0".to_string(),
            section_count: files.len() as u32,
            word_count,
            reading_time,
        }
    }
    
    /// Save documentation to files
    pub async fn save_to_directory<P: AsRef<Path>>(&self, docs: &GeneratedDocumentation, output_dir: P) -> Result<()> {
        let output_path = output_dir.as_ref();
        tokio::fs::create_dir_all(output_path).await?;
        
        for file in &docs.files {
            let file_path = output_path.join(&file.name);
            tokio::fs::write(file_path, &file.content).await?;
        }
        
        // Save metadata
        let metadata_json = serde_json::to_string_pretty(&docs.metadata)?;
        let metadata_path = output_path.join("metadata.json");
        tokio::fs::write(metadata_path, metadata_json).await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_documentation_generation() {
        let config = DocumentationConfig::default();
        let generator = DocumentationGenerator::new(config).unwrap();
        
        // Create a simple test package
        let package = create_test_package();
        
        let docs = generator.generate(&package).await.unwrap();
        
        assert!(!docs.formats.is_empty());
        assert!(!docs.files.is_empty());
        assert_eq!(docs.metadata.asset_name, "test-asset");
    }
    
    #[tokio::test]
    async fn test_save_documentation() {
        let config = DocumentationConfig::default();
        let generator = DocumentationGenerator::new(config).unwrap();
        let temp_dir = TempDir::new().unwrap();
        
        let package = create_test_package();
        let docs = generator.generate(&package).await.unwrap();
        
        generator.save_to_directory(&docs, temp_dir.path()).await.unwrap();
        
        // Verify files were created
        assert!(temp_dir.path().join("README.md").exists());
        assert!(temp_dir.path().join("metadata.json").exists());
    }
    
    fn create_test_package() -> AssetPackage {
        // Create a minimal test package for testing
        AssetPackage {
            spec: AssetSpec {
                api_version: "catalog.v1".to_string(),
                kind: "Asset".to_string(),
                metadata: AssetMetadata {
                    name: "test-asset".to_string(),
                    version: "1.0.0".to_string(),
                    description: Some("A test asset for documentation generation".to_string()),
                    tags: vec!["test".to_string()],
                    keywords: vec!["test".to_string(), "example".to_string()],
                    author: Some("Test Author".to_string()),
                    license: Some("MIT".to_string()),
                    homepage: None,
                    repository: None,
                    created: None,
                    updated: None,
                },
                spec: AssetSpecification {
                    asset_type: "julia-program".to_string(),
                    content: AssetContent {
                        main: "main.jl".to_string(),
                        files: vec![],
                        inline: None,
                        binary: vec![],
                        templates: vec![],
                    },
                    security: AssetSecurity {
                        consensus_required: false,
                        certificate_pinning: false,
                        hash_validation: "sha256".to_string(),
                        sandbox_level: "standard".to_string(),
                        allowed_syscalls: vec![],
                        network_access: NetworkAccess {
                            enabled: false,
                            allowed_domains: vec![],
                            allowed_ports: vec![],
                            require_tls: true,
                        },
                        file_access: FileAccess {
                            level: "read_only".to_string(),
                            allowed_paths: vec![],
                            denied_paths: vec![],
                            allow_temp: false,
                        },
                        permissions: vec![],
                    },
                    resources: AssetResources {
                        cpu_limit: "1000m".to_string(),
                        memory_limit: "1Gi".to_string(),
                        execution_timeout: "30s".to_string(),
                        storage_required: None,
                        network_bandwidth: None,
                        gpu_required: false,
                        hardware_requirements: vec![],
                    },
                    execution: AssetExecution {
                        delegation_strategy: "nearest_node".to_string(),
                        minimum_consensus: 1,
                        retry_policy: "none".to_string(),
                        max_concurrent: None,
                        priority: "normal".to_string(),
                        timeout_config: TimeoutConfig {
                            execution: "30s".to_string(),
                            network: "10s".to_string(),
                            io: "5s".to_string(),
                            compilation: None,
                        },
                        scheduling: SchedulingConfig {
                            timing: "immediate".to_string(),
                            allocation_strategy: "best_fit".to_string(),
                            node_affinity: vec![],
                            anti_affinity: vec![],
                        },
                    },
                    dependencies: vec![],
                    environment: std::collections::HashMap::new(),
                    config_schema: None,
                },
            },
            content: AssetContentResolved {
                main_content: "# Test function\nfunction main()\n    println(\"Hello, world!\")\nend".to_string(),
                file_contents: std::collections::HashMap::new(),
                binary_contents: std::collections::HashMap::new(),
                template_content: std::collections::HashMap::new(),
                resolved_dependencies: vec![],
            },
            validation: AssetValidationStatus {
                is_valid: true,
                validated_at: chrono::Utc::now(),
                errors: vec![],
                warnings: vec![],
                security_results: SecurityScanResults {
                    security_score: 85,
                    vulnerabilities: vec![],
                    recommendations: vec![],
                    scanned_at: chrono::Utc::now(),
                },
                dependency_results: DependencyValidationResults {
                    dependencies_valid: true,
                    total_dependencies: 0,
                    valid_dependencies: 0,
                    invalid_dependencies: vec![],
                    conflicts: vec![],
                    validated_at: chrono::Utc::now(),
                },
            },
            package_hash: "test-hash".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}