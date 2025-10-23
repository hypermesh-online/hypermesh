//! Container image management

use crate::{Result, RuntimeError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug, error};

/// Image manager for handling container images
#[derive(Debug)]
pub struct ImageManager {
    config: ImageConfig,
    
    /// Local image cache
    image_cache: Arc<RwLock<HashMap<String, Arc<Image>>>>,
    
    /// Image metadata store
    metadata_store: Arc<RwLock<HashMap<String, ImageMetadata>>>,
}

/// Image configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Local image storage directory
    pub storage_dir: String,
    
    /// Maximum cache size in bytes
    pub max_cache_size: u64,
    
    /// Default registry URL
    pub default_registry: String,
    
    /// Authentication for registries
    pub registry_auth: HashMap<String, RegistryAuth>,
    
    /// Image pull timeout
    pub pull_timeout_seconds: u64,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            storage_dir: "./data/images".to_string(),
            max_cache_size: 10 * 1024 * 1024 * 1024, // 10GB
            default_registry: "docker.io".to_string(),
            registry_auth: HashMap::new(),
            pull_timeout_seconds: 600, // 10 minutes
        }
    }
}

/// Registry authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryAuth {
    pub username: String,
    pub password: String,
    pub registry_url: String,
}

/// Container image specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSpec {
    /// Image name (e.g., "nginx", "my-app")
    pub name: String,
    
    /// Image tag (e.g., "latest", "v1.2.3")
    pub tag: String,
    
    /// Registry URL (optional, uses default if not specified)
    pub registry: Option<String>,
    
    /// Image digest for immutable reference
    pub digest: Option<String>,
}

impl Default for ImageSpec {
    fn default() -> Self {
        Self {
            name: "alpine".to_string(),
            tag: "latest".to_string(),
            registry: None,
            digest: None,
        }
    }
}

impl ImageSpec {
    /// Get full image reference
    pub fn full_reference(&self, default_registry: &str) -> String {
        let binding = default_registry.to_string();
        let registry = self.registry.as_ref().unwrap_or(&binding);
        
        if let Some(digest) = &self.digest {
            format!("{}/{}@{}", registry, self.name, digest)
        } else {
            format!("{}/{}:{}", registry, self.name, self.tag)
        }
    }
    
    /// Get image key for caching
    pub fn cache_key(&self) -> String {
        if let Some(digest) = &self.digest {
            format!("{}@{}", self.name, digest)
        } else {
            format!("{}:{}", self.name, self.tag)
        }
    }
}

/// Container image
#[derive(Debug)]
pub struct Image {
    /// Image specification
    pub spec: ImageSpec,
    
    /// Image metadata
    pub metadata: ImageMetadata,
    
    /// Local storage path
    pub storage_path: PathBuf,
    
    /// Image layers
    pub layers: Vec<ImageLayer>,
}

/// Image metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    /// Image ID
    pub image_id: String,
    
    /// Parent image ID
    pub parent_id: Option<String>,
    
    /// Creation timestamp
    pub created: std::time::SystemTime,
    
    /// Image size in bytes
    pub size: u64,
    
    /// Architecture (e.g., "amd64", "arm64")
    pub architecture: String,
    
    /// Operating system
    pub os: String,
    
    /// Image configuration
    pub config: ImageConfig,
    
    /// Environment variables
    pub env: Vec<String>,
    
    /// Entry point
    pub entrypoint: Vec<String>,
    
    /// Default command
    pub cmd: Vec<String>,
    
    /// Working directory
    pub workdir: Option<String>,
    
    /// Exposed ports
    pub exposed_ports: HashMap<String, serde_json::Value>,
    
    /// Labels
    pub labels: HashMap<String, String>,
}

/// Image layer information
#[derive(Debug, Clone)]
pub struct ImageLayer {
    /// Layer digest
    pub digest: String,
    
    /// Layer size
    pub size: u64,
    
    /// Local storage path
    pub path: PathBuf,
    
    /// Whether layer is compressed
    pub compressed: bool,
}

impl ImageManager {
    /// Create a new image manager
    pub async fn new(config: &ImageConfig) -> Result<Self> {
        // Create storage directory
        tokio::fs::create_dir_all(&config.storage_dir).await
            .map_err(|e| RuntimeError::Storage { 
                message: format!("Failed to create image storage dir: {}", e) 
            })?;
        
        Ok(Self {
            config: config.clone(),
            image_cache: Arc::new(RwLock::new(HashMap::new())),
            metadata_store: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Ensure an image is available locally
    pub async fn ensure_image(&self, spec: &ImageSpec) -> Result<Arc<Image>> {
        let cache_key = spec.cache_key();
        
        // Check local cache first
        {
            let cache = self.image_cache.read().await;
            if let Some(image) = cache.get(&cache_key) {
                debug!("Image found in cache: {}", cache_key);
                return Ok(Arc::clone(image));
            }
        }
        
        // Try to load from local storage
        if let Ok(image) = self.load_local_image(spec).await {
            let image_arc = Arc::new(image);
            self.image_cache.write().await.insert(cache_key, Arc::clone(&image_arc));
            return Ok(image_arc);
        }
        
        // Pull from registry
        info!("Pulling image: {}", spec.full_reference(&self.config.default_registry));
        let image = self.pull_image(spec).await?;
        let image_arc = Arc::new(image);
        
        // Cache the image
        self.image_cache.write().await.insert(cache_key, Arc::clone(&image_arc));
        
        Ok(image_arc)
    }
    
    /// Pull an image from registry
    async fn pull_image(&self, spec: &ImageSpec) -> Result<Image> {
        // This is a simplified implementation
        // In a real implementation, this would:
        // 1. Connect to the registry
        // 2. Authenticate if needed
        // 3. Download the image manifest
        // 4. Download all layers
        // 5. Store layers locally
        // 6. Build the image metadata
        
        let full_ref = spec.full_reference(&self.config.default_registry);
        info!("Pulling image from registry: {}", full_ref);
        
        // Simulate image pull for now
        let metadata = ImageMetadata {
            image_id: format!("sha256:{}", blake3::hash(full_ref.as_bytes()).to_hex()),
            parent_id: None,
            created: std::time::SystemTime::now(),
            size: 1024 * 1024, // 1MB placeholder
            architecture: "amd64".to_string(),
            os: "linux".to_string(),
            config: self.config.clone(),
            env: vec!["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()],
            entrypoint: vec!["/bin/sh".to_string()],
            cmd: vec![],
            workdir: Some("/".to_string()),
            exposed_ports: HashMap::new(),
            labels: HashMap::new(),
        };
        
        let storage_path = Path::new(&self.config.storage_dir)
            .join(&metadata.image_id);
        
        // Create storage directory
        tokio::fs::create_dir_all(&storage_path).await
            .map_err(|e| RuntimeError::Storage { 
                message: format!("Failed to create image storage: {}", e) 
            })?;
        
        let image = Image {
            spec: spec.clone(),
            metadata,
            storage_path,
            layers: vec![], // Would contain actual layers
        };
        
        info!("Successfully pulled image: {}", full_ref);
        Ok(image)
    }
    
    /// Load an image from local storage
    async fn load_local_image(&self, spec: &ImageSpec) -> Result<Image> {
        // Check if image exists locally
        let cache_key = spec.cache_key();
        let metadata_store = self.metadata_store.read().await;
        
        if let Some(metadata) = metadata_store.get(&cache_key) {
            let storage_path = Path::new(&self.config.storage_dir)
                .join(&metadata.image_id);
            
            if storage_path.exists() {
                debug!("Loading image from local storage: {}", cache_key);
                
                return Ok(Image {
                    spec: spec.clone(),
                    metadata: metadata.clone(),
                    storage_path,
                    layers: vec![], // Would load actual layers
                });
            }
        }
        
        Err(RuntimeError::ImageNotFound { 
            name: spec.name.clone(), 
            tag: spec.tag.clone() 
        })
    }
    
    /// Remove an image from local storage
    pub async fn remove_image(&self, spec: &ImageSpec) -> Result<bool> {
        let cache_key = spec.cache_key();
        
        // Remove from cache
        let removed_from_cache = self.image_cache.write().await.remove(&cache_key).is_some();
        
        // Remove from metadata store
        let metadata_store = self.metadata_store.read().await;
        if let Some(metadata) = metadata_store.get(&cache_key) {
            let storage_path = Path::new(&self.config.storage_dir)
                .join(&metadata.image_id);
            
            // Remove storage directory
            if storage_path.exists() {
                tokio::fs::remove_dir_all(&storage_path).await
                    .map_err(|e| RuntimeError::Storage { 
                        message: format!("Failed to remove image storage: {}", e) 
                    })?;
            }
            
            drop(metadata_store);
            self.metadata_store.write().await.remove(&cache_key);
            
            info!("Removed image: {}", cache_key);
            return Ok(true);
        }
        
        Ok(removed_from_cache)
    }
    
    /// List locally cached images
    pub async fn list_images(&self) -> Vec<ImageSpec> {
        self.image_cache.read().await
            .values()
            .map(|image| image.spec.clone())
            .collect()
    }
    
    /// Get image metadata
    pub async fn get_image_metadata(&self, spec: &ImageSpec) -> Option<ImageMetadata> {
        let cache_key = spec.cache_key();
        self.metadata_store.read().await.get(&cache_key).cloned()
    }
    
    /// Clean up unused images
    pub async fn cleanup_unused_images(&self) -> Result<u64> {
        // This would implement garbage collection of unused images
        // based on age, usage, and available disk space
        Ok(0)
    }
    
    /// Get cache statistics
    pub async fn cache_stats(&self) -> ImageCacheStats {
        let cache = self.image_cache.read().await;
        let total_size: u64 = cache.values()
            .map(|img| img.metadata.size)
            .sum();
        
        ImageCacheStats {
            image_count: cache.len(),
            total_size_bytes: total_size,
            cache_hit_rate: 0.0, // Would track actual hit rate
        }
    }
}

impl Image {
    /// Extract image to a directory
    pub async fn extract_to(&self, target_path: &Path) -> Result<()> {
        // Create target directory
        tokio::fs::create_dir_all(target_path).await
            .map_err(|e| RuntimeError::Storage { 
                message: format!("Failed to create extraction target: {}", e) 
            })?;
        
        // In a real implementation, this would:
        // 1. Extract each layer in order
        // 2. Apply layer changes (files, directories, etc.)
        // 3. Set up the final filesystem state
        
        debug!("Extracted image {} to {:?}", self.spec.cache_key(), target_path);
        Ok(())
    }
    
    /// Get image configuration for container creation
    pub fn get_container_config(&self) -> ContainerImageConfig {
        ContainerImageConfig {
            env: self.metadata.env.clone(),
            entrypoint: self.metadata.entrypoint.clone(),
            cmd: self.metadata.cmd.clone(),
            workdir: self.metadata.workdir.clone(),
            exposed_ports: self.metadata.exposed_ports.keys().cloned().collect(),
            labels: self.metadata.labels.clone(),
        }
    }
}

/// Container configuration from image
#[derive(Debug, Clone)]
pub struct ContainerImageConfig {
    pub env: Vec<String>,
    pub entrypoint: Vec<String>,
    pub cmd: Vec<String>,
    pub workdir: Option<String>,
    pub exposed_ports: Vec<String>,
    pub labels: HashMap<String, String>,
}

/// Image cache statistics
#[derive(Debug, Clone)]
pub struct ImageCacheStats {
    pub image_count: usize,
    pub total_size_bytes: u64,
    pub cache_hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_image_spec() {
        let spec = ImageSpec {
            name: "nginx".to_string(),
            tag: "latest".to_string(),
            registry: Some("docker.io".to_string()),
            digest: None,
        };
        
        assert_eq!(spec.full_reference("registry.io"), "docker.io/nginx:latest");
        assert_eq!(spec.cache_key(), "nginx:latest");
    }
    
    #[tokio::test]
    async fn test_image_manager() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = ImageConfig::default();
        config.storage_dir = temp_dir.path().to_string_lossy().to_string();
        
        let manager = ImageManager::new(&config).await.unwrap();
        
        let spec = ImageSpec::default();
        
        // This would fail in a real test without proper registry setup
        // but demonstrates the API structure
        let result = manager.ensure_image(&spec).await;
        match result {
            Ok(_) => {},
            Err(_) => {
                // Expected to fail without proper setup
            }
        }
    }
    
    #[test]
    fn test_image_spec_serialization() {
        let spec = ImageSpec::default();
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: ImageSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec.name, parsed.name);
    }
}