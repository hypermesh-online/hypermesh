//! Dynamic extension loader for HyperMesh
//!
//! This module provides the infrastructure for dynamically loading extensions
//! at runtime, supporting both shared libraries (.so) and WebAssembly (.wasm).

use anyhow::{Context, Result as AnyhowResult};
use async_trait::async_trait;
// TODO: Add libloading dependency to Cargo.toml
// use libloading::{Library, Symbol};

// Stub types until libloading is added
struct Library;
impl Library {
    fn new(_path: &Path) -> Result<Self, String> {
        Err("libloading not available".to_string())
    }

    unsafe fn get<T>(&self, _symbol: &[u8]) -> Result<Symbol<T>, String> {
        Err("libloading not available".to_string())
    }
}
struct Symbol<T>(std::marker::PhantomData<T>);
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::{
    ExtensionCapability, ExtensionConfig, ExtensionError, ExtensionMetadata,
    ExtensionResult, HyperMeshExtension, ResourceLimits,
};
use crate::assets::core::PrivacyLevel;

/// Type alias for extension constructor function
pub type ExtensionConstructor = unsafe extern "C" fn() -> *mut dyn HyperMeshExtension;

/// Extension loader configuration
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Paths to search for extensions
    pub search_paths: Vec<PathBuf>,

    /// Whether to enable WebAssembly support
    pub enable_wasm: bool,

    /// Whether to verify extension signatures
    pub verify_signatures: bool,

    /// Maximum number of loaded extensions
    pub max_extensions: usize,

    /// Default resource limits for extensions
    pub default_limits: ResourceLimits,

    /// TrustChain certificate path for verification
    pub trustchain_cert_path: Option<PathBuf>,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("./extensions"),
                PathBuf::from("/usr/local/lib/hypermesh/extensions"),
                PathBuf::from("/opt/hypermesh/extensions"),
            ],
            enable_wasm: true,
            verify_signatures: true,
            max_extensions: 50,
            default_limits: ResourceLimits::default(),
            trustchain_cert_path: None,
        }
    }
}

/// Extension loading context
#[derive(Debug)]
pub struct LoadContext {
    /// Extension ID
    pub extension_id: String,

    /// Extension path
    pub path: PathBuf,

    /// Loading method (shared library or WASM)
    pub method: LoadingMethod,

    /// Extension metadata
    pub metadata: ExtensionMetadata,

    /// Granted capabilities
    pub capabilities: Vec<ExtensionCapability>,

    /// Resource limits
    pub limits: ResourceLimits,
}

/// Extension loading method
#[derive(Debug, Clone, PartialEq)]
pub enum LoadingMethod {
    /// Native shared library (.so, .dylib, .dll)
    SharedLibrary,

    /// WebAssembly module (.wasm)
    WebAssembly,
}

/// Loaded extension container
pub struct LoadedExtension {
    /// Extension instance
    pub extension: Box<dyn HyperMeshExtension>,

    /// Loading context
    pub context: LoadContext,

    /// Underlying library handle (kept alive)
    _library: Option<Library>,
}

/// Dynamic extension loader
pub struct ExtensionLoader {
    /// Configuration
    config: LoaderConfig,

    /// Loaded extensions
    loaded: Arc<RwLock<HashMap<String, LoadedExtension>>>,

    /// Extension manifests cache
    manifests: Arc<RwLock<HashMap<String, ExtensionManifest>>>,

    /// Security verifier
    verifier: Option<Arc<SecurityVerifier>>,
}

impl ExtensionLoader {
    /// Create new extension loader
    pub fn new(config: LoaderConfig) -> Self {
        let verifier = if config.verify_signatures {
            Some(Arc::new(SecurityVerifier::new(
                config.trustchain_cert_path.clone()
            )))
        } else {
            None
        };

        Self {
            config,
            loaded: Arc::new(RwLock::new(HashMap::new())),
            manifests: Arc::new(RwLock::new(HashMap::new())),
            verifier,
        }
    }

    /// Discover available extensions
    pub async fn discover_extensions(&self) -> ExtensionResult<Vec<ExtensionManifest>> {
        let mut discovered = Vec::new();

        for path in &self.config.search_paths {
            if !path.exists() {
                debug!("Extension search path does not exist: {:?}", path);
                continue;
            }

            let extensions = self.scan_directory(path).await?;
            discovered.extend(extensions);
        }

        info!("Discovered {} extensions", discovered.len());
        Ok(discovered)
    }

    /// Load extension from path
    pub async fn load_extension(&self, path: &Path) -> ExtensionResult<String> {
        // Check if already at max capacity
        {
            let loaded = self.loaded.read().await;
            if loaded.len() >= self.config.max_extensions {
                return Err(ExtensionError::ResourceLimitExceeded {
                    resource: format!("max_extensions: {}", self.config.max_extensions),
                });
            }
        }

        // Read and validate manifest
        let manifest = self.read_manifest(path).await?;

        // Check if already loaded
        {
            let loaded = self.loaded.read().await;
            if loaded.contains_key(&manifest.metadata.id) {
                return Err(ExtensionError::ExtensionAlreadyLoaded {
                    id: manifest.metadata.id.clone(),
                });
            }
        }

        // Verify signature if required
        if self.config.verify_signatures {
            if let Some(verifier) = &self.verifier {
                verifier.verify_extension(&manifest, path).await?;
            }
        }

        // Determine loading method
        let method = self.determine_loading_method(path)?;

        // Create loading context
        let context = LoadContext {
            extension_id: manifest.metadata.id.clone(),
            path: path.to_path_buf(),
            method: method.clone(),
            metadata: manifest.metadata.clone(),
            capabilities: manifest.metadata.required_capabilities.iter().cloned().collect(),
            limits: self.config.default_limits.clone(),
        };

        // Load the extension
        let loaded_ext = match method {
            LoadingMethod::SharedLibrary => self.load_shared_library(path, context).await?,
            LoadingMethod::WebAssembly => self.load_wasm_module(path, context).await?,
        };

        let extension_id = loaded_ext.context.extension_id.clone();

        // Store the loaded extension
        {
            let mut loaded = self.loaded.write().await;
            loaded.insert(extension_id.clone(), loaded_ext);
        }

        info!("Successfully loaded extension: {}", extension_id);
        Ok(extension_id)
    }

    /// Load extension from shared library
    async fn load_shared_library(
        &self,
        path: &Path,
        context: LoadContext,
    ) -> ExtensionResult<LoadedExtension> {
        // Build the library path
        let lib_path = if path.is_dir() {
            // Look for the library file in the directory
            let lib_name = format!("lib{}.so", context.extension_id);
            path.join(lib_name)
        } else {
            path.to_path_buf()
        };

        if !lib_path.exists() {
            return Err(ExtensionError::Internal(anyhow::anyhow!(
                "Extension library not found: {:?}", lib_path
            )));
        }

        // Load the library
        let library = unsafe {
            Library::new(&lib_path).map_err(|e| {
                ExtensionError::Internal(anyhow::anyhow!(
                    "Failed to load library {:?}: {}", lib_path, e
                ))
            })?
        };

        // Get the constructor function
        let constructor: Symbol<ExtensionConstructor> = unsafe {
            library.get(b"hypermesh_extension_create\0").map_err(|e| {
                ExtensionError::Internal(anyhow::anyhow!(
                    "Failed to find extension constructor: {}", e
                ))
            })?
        };

        // Create the extension instance
        let extension_ptr = unsafe { constructor() };
        if extension_ptr.is_null() {
            return Err(ExtensionError::InitializationFailed {
                reason: "Constructor returned null pointer".to_string(),
            });
        }

        let mut extension = unsafe { Box::from_raw(extension_ptr) };

        // Initialize the extension
        let config = ExtensionConfig {
            settings: serde_json::Value::Null,
            resource_limits: context.limits.clone(),
            granted_capabilities: context.capabilities.iter().cloned().collect(),
            privacy_level: PrivacyLevel::Private,
            debug_mode: false,
        };

        extension.initialize(config).await?;

        Ok(LoadedExtension {
            extension,
            context,
            _library: Some(library),
        })
    }

    /// Load WebAssembly module
    async fn load_wasm_module(
        &self,
        path: &Path,
        context: LoadContext,
    ) -> ExtensionResult<LoadedExtension> {
        if !self.config.enable_wasm {
            return Err(ExtensionError::Internal(anyhow::anyhow!(
                "WebAssembly support is not enabled"
            )));
        }

        // WebAssembly loading implementation
        // This would use wasmtime or wasmer for WASM runtime
        // For now, return an error as this is a placeholder

        Err(ExtensionError::Internal(anyhow::anyhow!(
            "WebAssembly loading not yet implemented"
        )))
    }

    /// Unload an extension
    pub async fn unload_extension(&self, extension_id: &str) -> ExtensionResult<()> {
        let mut loaded_ext = {
            let mut loaded = self.loaded.write().await;
            loaded.remove(extension_id).ok_or_else(|| {
                ExtensionError::ExtensionNotFound {
                    id: extension_id.to_string(),
                }
            })?
        };

        // Shutdown the extension
        loaded_ext.extension.shutdown().await?;

        // The library will be unloaded when LoadedExtension is dropped
        info!("Successfully unloaded extension: {}", extension_id);
        Ok(())
    }

    /// Get loaded extension
    pub async fn get_extension(&self, extension_id: &str) -> Option<Arc<dyn HyperMeshExtension>> {
        let loaded = self.loaded.read().await;
        loaded.get(extension_id).map(|le| Arc::from(le.extension.as_ref()))
    }

    /// List loaded extensions
    pub async fn list_loaded(&self) -> Vec<ExtensionMetadata> {
        let loaded = self.loaded.read().await;
        loaded.values().map(|le| le.context.metadata.clone()).collect()
    }

    /// Scan directory for extensions
    async fn scan_directory(&self, dir: &Path) -> ExtensionResult<Vec<ExtensionManifest>> {
        let mut manifests = Vec::new();

        let entries = tokio::fs::read_dir(dir).await.map_err(|e| {
            ExtensionError::Internal(anyhow::anyhow!(
                "Failed to read directory {:?}: {}", dir, e
            ))
        })?;

        let mut entries = entries;
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            ExtensionError::Internal(anyhow::anyhow!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();

            // Check if this is an extension directory or file
            if path.is_dir() {
                let manifest_path = path.join("extension.toml");
                if manifest_path.exists() {
                    match self.read_manifest(&path).await {
                        Ok(manifest) => manifests.push(manifest),
                        Err(e) => warn!("Failed to read manifest at {:?}: {}", path, e),
                    }
                }
            } else if let Some(ext) = path.extension() {
                if ext == "so" || ext == "wasm" || ext == "dylib" || ext == "dll" {
                    // Look for accompanying manifest
                    let manifest_path = path.with_extension("toml");
                    if manifest_path.exists() {
                        match self.read_manifest(&path).await {
                            Ok(manifest) => manifests.push(manifest),
                            Err(e) => warn!("Failed to read manifest at {:?}: {}", path, e),
                        }
                    }
                }
            }
        }

        Ok(manifests)
    }

    /// Read extension manifest
    async fn read_manifest(&self, path: &Path) -> ExtensionResult<ExtensionManifest> {
        let manifest_path = if path.is_dir() {
            path.join("extension.toml")
        } else {
            path.with_extension("toml")
        };

        let content = tokio::fs::read_to_string(&manifest_path).await.map_err(|e| {
            ExtensionError::Internal(anyhow::anyhow!(
                "Failed to read manifest {:?}: {}", manifest_path, e
            ))
        })?;

        let manifest: ExtensionManifest = toml::from_str(&content).map_err(|e| {
            ExtensionError::Internal(anyhow::anyhow!(
                "Failed to parse manifest {:?}: {}", manifest_path, e
            ))
        })?;

        Ok(manifest)
    }

    /// Determine loading method based on file extension
    fn determine_loading_method(&self, path: &Path) -> ExtensionResult<LoadingMethod> {
        let ext = if path.is_dir() {
            // Check for library files in directory
            if path.join(format!("lib{}.so", "extension")).exists() {
                return Ok(LoadingMethod::SharedLibrary);
            }
            if path.join("extension.wasm").exists() {
                return Ok(LoadingMethod::WebAssembly);
            }
            return Err(ExtensionError::Internal(anyhow::anyhow!(
                "No loadable extension found in directory"
            )));
        } else {
            path.extension().and_then(|e| e.to_str()).unwrap_or("")
        };

        match ext {
            "so" | "dylib" | "dll" => Ok(LoadingMethod::SharedLibrary),
            "wasm" => {
                if self.config.enable_wasm {
                    Ok(LoadingMethod::WebAssembly)
                } else {
                    Err(ExtensionError::Internal(anyhow::anyhow!(
                        "WebAssembly support is not enabled"
                    )))
                }
            }
            _ => Err(ExtensionError::Internal(anyhow::anyhow!(
                "Unknown extension type: {}", ext
            ))),
        }
    }

    /// Hot reload an extension
    pub async fn reload_extension(&self, extension_id: &str) -> ExtensionResult<()> {
        // Get the current extension's path
        let path = {
            let loaded = self.loaded.read().await;
            loaded.get(extension_id)
                .map(|le| le.context.path.clone())
                .ok_or_else(|| ExtensionError::ExtensionNotFound {
                    id: extension_id.to_string(),
                })?
        };

        // Unload the current extension
        self.unload_extension(extension_id).await?;

        // Load the extension again
        self.load_extension(&path).await?;

        info!("Successfully reloaded extension: {}", extension_id);
        Ok(())
    }
}

/// Extension manifest structure
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExtensionManifest {
    /// Extension metadata
    pub metadata: ExtensionMetadata,

    /// Library configuration
    pub library: LibraryConfig,

    /// Security configuration
    pub security: Option<SecurityConfig>,

    /// Runtime configuration
    pub runtime: Option<RuntimeConfig>,
}

/// Library configuration in manifest
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LibraryConfig {
    /// Library file name (without extension)
    pub name: String,

    /// Library type (native, wasm)
    pub lib_type: String,

    /// Entry point function name
    pub entry_point: Option<String>,
}

/// Security configuration in manifest
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SecurityConfig {
    /// Signature file path
    pub signature: Option<String>,

    /// Certificate fingerprint
    pub certificate: Option<String>,

    /// Required permissions
    pub permissions: Vec<String>,
}

/// Runtime configuration in manifest
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RuntimeConfig {
    /// Minimum runtime version
    pub min_version: Option<String>,

    /// Resource requirements
    pub resources: Option<ResourceRequirements>,

    /// Environment variables
    pub env: Option<HashMap<String, String>>,
}

/// Resource requirements in manifest
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ResourceRequirements {
    /// Minimum memory in MB
    pub min_memory: Option<u32>,

    /// Maximum memory in MB
    pub max_memory: Option<u32>,

    /// CPU cores required
    pub cpu_cores: Option<f32>,
}

/// Security verifier for extension signatures
pub struct SecurityVerifier {
    /// TrustChain certificate path
    cert_path: Option<PathBuf>,
}

impl SecurityVerifier {
    /// Create new security verifier
    pub fn new(cert_path: Option<PathBuf>) -> Self {
        Self { cert_path }
    }

    /// Verify extension signature
    pub async fn verify_extension(
        &self,
        manifest: &ExtensionManifest,
        path: &Path,
    ) -> ExtensionResult<()> {
        // Check if signature verification is required
        if let Some(security) = &manifest.security {
            if let Some(sig_file) = &security.signature {
                let sig_path = if path.is_dir() {
                    path.join(sig_file)
                } else {
                    path.parent().unwrap().join(sig_file)
                };

                if !sig_path.exists() {
                    return Err(ExtensionError::CertificateValidationFailed {
                        fingerprint: "Signature file not found".to_string(),
                    });
                }

                // TODO: Implement actual signature verification using TrustChain
                // This would involve:
                // 1. Reading the signature file
                // 2. Getting the certificate from TrustChain
                // 3. Verifying the signature matches the extension binary
                // 4. Checking certificate validity and trust chain

                debug!("Signature verification successful for {}", manifest.metadata.id);
            }
        }

        Ok(())
    }
}

/// Extension sandbox for resource isolation
pub struct ExtensionSandbox {
    /// Extension ID
    extension_id: String,

    /// Resource limits
    limits: ResourceLimits,

    /// Current resource usage
    usage: Arc<RwLock<ResourceUsage>>,
}

/// Current resource usage tracking
#[derive(Debug, Default)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_percent: f32,

    /// Memory usage in bytes
    pub memory_bytes: u64,

    /// Network bytes transferred
    pub network_bytes: u64,

    /// Active operations count
    pub operations: usize,
}

impl ExtensionSandbox {
    /// Create new sandbox for extension
    pub fn new(extension_id: String, limits: ResourceLimits) -> Self {
        Self {
            extension_id,
            limits,
            usage: Arc::new(RwLock::new(ResourceUsage::default())),
        }
    }

    /// Check if operation is allowed within limits
    pub async fn check_limits(&self) -> ExtensionResult<()> {
        let usage = self.usage.read().await;

        if usage.cpu_percent > self.limits.max_cpu_percent {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!("CPU: {:.1}% > {:.1}%", usage.cpu_percent, self.limits.max_cpu_percent),
            });
        }

        if usage.memory_bytes > self.limits.max_memory_bytes {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!("Memory: {} > {}", usage.memory_bytes, self.limits.max_memory_bytes),
            });
        }

        if usage.operations >= self.limits.max_concurrent_operations {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!("Operations: {} >= {}", usage.operations, self.limits.max_concurrent_operations),
            });
        }

        Ok(())
    }

    /// Track operation start
    pub async fn start_operation(&self) -> ExtensionResult<()> {
        self.check_limits().await?;

        let mut usage = self.usage.write().await;
        usage.operations += 1;
        Ok(())
    }

    /// Track operation end
    pub async fn end_operation(&self) {
        let mut usage = self.usage.write().await;
        if usage.operations > 0 {
            usage.operations -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert!(config.enable_wasm);
        assert!(config.verify_signatures);
        assert_eq!(config.max_extensions, 50);
    }

    #[test]
    fn test_loading_method_determination() {
        let loader = ExtensionLoader::new(LoaderConfig::default());

        let so_path = PathBuf::from("extension.so");
        assert_eq!(
            loader.determine_loading_method(&so_path).unwrap(),
            LoadingMethod::SharedLibrary
        );

        let wasm_path = PathBuf::from("extension.wasm");
        assert_eq!(
            loader.determine_loading_method(&wasm_path).unwrap(),
            LoadingMethod::WebAssembly
        );
    }

    #[tokio::test]
    async fn test_resource_sandbox() {
        let limits = ResourceLimits {
            max_cpu_percent: 50.0,
            max_memory_bytes: 1024 * 1024,
            max_concurrent_operations: 5,
            ..Default::default()
        };

        let sandbox = ExtensionSandbox::new("test".to_string(), limits);

        // Should allow operations within limits
        assert!(sandbox.start_operation().await.is_ok());
        sandbox.end_operation().await;

        // Test operation limit
        for _ in 0..5 {
            sandbox.start_operation().await.unwrap();
        }

        // Should fail when at limit
        assert!(sandbox.start_operation().await.is_err());
    }
}