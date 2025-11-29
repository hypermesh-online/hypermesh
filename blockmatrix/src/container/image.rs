//! Container image management

use crate::{ContainerId, config::StorageConfig};
use super::error::{Result, ContainerError};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{info, debug, warn, error};

/// Container image metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerImage {
    /// Image ID (SHA256 digest)
    pub id: String,
    /// Image repository and tag
    pub reference: String,
    /// Image size in bytes
    pub size: u64,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Image layers
    pub layers: Vec<ImageLayer>,
    /// Image configuration
    pub config: ImageConfig,
    /// Image manifest
    pub manifest: ImageManifest,
}

/// Image layer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageLayer {
    /// Layer digest
    pub digest: String,
    /// Layer size in bytes
    pub size: u64,
    /// Layer compression
    pub compression: CompressionType,
    /// Layer media type
    pub media_type: String,
}

/// Image configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Default command
    pub cmd: Vec<String>,
    /// Entry point
    pub entrypoint: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Exposed ports
    pub exposed_ports: Vec<u16>,
    /// User ID
    pub user: Option<String>,
    /// Labels
    pub labels: HashMap<String, String>,
}

/// Image manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageManifest {
    /// Schema version
    pub schema_version: u32,
    /// Media type
    pub media_type: String,
    /// Config descriptor
    pub config: LayerDescriptor,
    /// Layer descriptors
    pub layers: Vec<LayerDescriptor>,
}

/// Layer descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDescriptor {
    /// Media type
    pub media_type: String,
    /// Size in bytes
    pub size: u64,
    /// Digest
    pub digest: String,
}

/// Compression types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// GZIP compression
    Gzip,
    /// ZSTD compression
    Zstd,
}

/// Image pull progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullProgress {
    /// Current layer being pulled
    pub layer: String,
    /// Bytes downloaded
    pub downloaded: u64,
    /// Total bytes to download
    pub total: u64,
    /// Pull status
    pub status: PullStatus,
}

/// Pull status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PullStatus {
    /// Starting pull
    Starting,
    /// Downloading layer
    Downloading,
    /// Extracting layer
    Extracting,
    /// Layer complete
    Complete,
    /// Pull failed
    Failed(String),
}

/// Image manager trait
#[async_trait]
pub trait ImageManager: Send + Sync {
    /// Pull an image from a registry
    async fn pull(&self, reference: &str) -> Result<ContainerImage>;
    
    /// Check if image exists locally
    async fn exists(&self, reference: &str) -> Result<bool>;
    
    /// Get image metadata
    async fn get(&self, reference: &str) -> Result<ContainerImage>;
    
    /// List available images
    async fn list(&self) -> Result<Vec<ContainerImage>>;
    
    /// Remove an image
    async fn remove(&self, reference: &str) -> Result<()>;
    
    /// Build an image from a context
    async fn build(&self, context: &Path, dockerfile: &Path, tag: &str) -> Result<ContainerImage>;
    
    /// Export image to tar archive
    async fn export(&self, reference: &str, output_path: &Path) -> Result<()>;
    
    /// Import image from tar archive
    async fn import(&self, input_path: &Path, reference: &str) -> Result<ContainerImage>;
    
    /// Get image layers for container creation
    async fn get_layers(&self, reference: &str) -> Result<Vec<PathBuf>>;
    
    /// Cleanup unused images
    async fn garbage_collect(&self) -> Result<Vec<String>>;
}

/// Default image manager implementation
pub struct DefaultImageManager {
    storage_config: StorageConfig,
    images: std::sync::Arc<tokio::sync::RwLock<HashMap<String, ContainerImage>>>,
}

impl DefaultImageManager {
    /// Create a new image manager
    pub fn new(storage_config: &StorageConfig) -> Result<Self> {
        // Ensure image directory exists
        std::fs::create_dir_all(&storage_config.images)
            .map_err(|e| ContainerError::image(
                format!("Failed to create image directory: {}", e)
            ))?;
        
        Ok(Self {
            storage_config: storage_config.clone(),
            images: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }
    
    /// Get image storage path
    fn image_path(&self, reference: &str) -> PathBuf {
        let safe_name = reference.replace(['/', ':'], "_");
        self.storage_config.images.join(safe_name)
    }
    
    /// Simulate image pull from registry
    async fn simulate_pull(&self, reference: &str) -> Result<ContainerImage> {
        info!("Pulling image: {}", reference);
        
        // Simulate network delay
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        
        // Create mock image layers
        let layers = vec![
            ImageLayer {
                digest: "sha256:base_layer".to_string(),
                size: 50 * 1024 * 1024, // 50MB
                compression: CompressionType::Gzip,
                media_type: "application/vnd.docker.image.rootfs.diff.tar.gzip".to_string(),
            },
            ImageLayer {
                digest: "sha256:app_layer".to_string(),
                size: 20 * 1024 * 1024, // 20MB
                compression: CompressionType::Gzip,
                media_type: "application/vnd.docker.image.rootfs.diff.tar.gzip".to_string(),
            },
        ];
        
        let total_size = layers.iter().map(|l| l.size).sum();
        
        let image = ContainerImage {
            id: format!("sha256:{}", sha256::digest(reference)),
            reference: reference.to_string(),
            size: total_size,
            created_at: SystemTime::now(),
            layers,
            config: ImageConfig {
                cmd: vec!["sh".to_string()],
                entrypoint: vec![],
                env: HashMap::new(),
                working_dir: Some("/".to_string()),
                exposed_ports: vec![],
                user: None,
                labels: HashMap::new(),
            },
            manifest: ImageManifest {
                schema_version: 2,
                media_type: "application/vnd.docker.distribution.manifest.v2+json".to_string(),
                config: LayerDescriptor {
                    media_type: "application/vnd.docker.container.image.v1+json".to_string(),
                    size: 1024,
                    digest: "sha256:config".to_string(),
                },
                layers: vec![
                    LayerDescriptor {
                        media_type: "application/vnd.docker.image.rootfs.diff.tar.gzip".to_string(),
                        size: 50 * 1024 * 1024,
                        digest: "sha256:base_layer".to_string(),
                    },
                    LayerDescriptor {
                        media_type: "application/vnd.docker.image.rootfs.diff.tar.gzip".to_string(),
                        size: 20 * 1024 * 1024,
                        digest: "sha256:app_layer".to_string(),
                    },
                ],
            },
        };
        
        // Create image directory and metadata
        let image_path = self.image_path(reference);
        std::fs::create_dir_all(&image_path)
            .map_err(|e| ContainerError::image(
                format!("Failed to create image path: {}", e)
            ))?;
        
        // Save image metadata
        let metadata_path = image_path.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&image)?;
        std::fs::write(metadata_path, metadata_json)
            .map_err(|e| ContainerError::image(
                format!("Failed to save image metadata: {}", e)
            ))?;
        
        info!("Successfully pulled image: {} ({})", reference, image.id);
        Ok(image)
    }
}

#[async_trait]
impl ImageManager for DefaultImageManager {
    async fn pull(&self, reference: &str) -> Result<ContainerImage> {
        let image = self.simulate_pull(reference).await?;
        
        // Store in cache
        let mut images = self.images.write().await;
        images.insert(reference.to_string(), image.clone());
        
        Ok(image)
    }
    
    async fn exists(&self, reference: &str) -> Result<bool> {
        // Check cache first
        let images = self.images.read().await;
        if images.contains_key(reference) {
            return Ok(true);
        }
        drop(images);
        
        // Check filesystem
        let image_path = self.image_path(reference);
        let metadata_path = image_path.join("metadata.json");
        Ok(metadata_path.exists())
    }
    
    async fn get(&self, reference: &str) -> Result<ContainerImage> {
        // Check cache first
        {
            let images = self.images.read().await;
            if let Some(image) = images.get(reference) {
                return Ok(image.clone());
            }
        }
        
        // Try to load from filesystem
        let image_path = self.image_path(reference);
        let metadata_path = image_path.join("metadata.json");
        
        if !metadata_path.exists() {
            return Err(ContainerError::image(
                format!("Image not found: {}", reference)
            ));
        }
        
        let metadata_json = std::fs::read_to_string(metadata_path)
            .map_err(|e| ContainerError::image(
                format!("Failed to read image metadata: {}", e)
            ))?;
        
        let image: ContainerImage = serde_json::from_str(&metadata_json)
            .map_err(|e| ContainerError::image(
                format!("Failed to parse image metadata: {}", e)
            ))?;
        
        // Cache the image
        let mut images = self.images.write().await;
        images.insert(reference.to_string(), image.clone());
        
        Ok(image)
    }
    
    async fn list(&self) -> Result<Vec<ContainerImage>> {
        let images = self.images.read().await;
        Ok(images.values().cloned().collect())
    }
    
    async fn remove(&self, reference: &str) -> Result<()> {
        // Remove from cache
        let mut images = self.images.write().await;
        images.remove(reference);
        drop(images);
        
        // Remove from filesystem
        let image_path = self.image_path(reference);
        if image_path.exists() {
            std::fs::remove_dir_all(&image_path)
                .map_err(|e| ContainerError::image(
                    format!("Failed to remove image directory: {}", e)
                ))?;
        }
        
        info!("Removed image: {}", reference);
        Ok(())
    }
    
    async fn build(&self, _context: &Path, _dockerfile: &Path, tag: &str) -> Result<ContainerImage> {
        // Simulate image build
        info!("Building image: {}", tag);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        
        // For now, just create a mock built image
        self.pull(tag).await
    }
    
    async fn export(&self, reference: &str, output_path: &Path) -> Result<()> {
        let image = self.get(reference).await?;
        
        // Simulate tar export
        info!("Exporting image {} to {:?}", reference, output_path);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        
        // Create mock tar file
        std::fs::write(output_path, format!("Mock tar export of {}", image.id))
            .map_err(|e| ContainerError::image(
                format!("Failed to export image: {}", e)
            ))?;
        
        Ok(())
    }
    
    async fn import(&self, input_path: &Path, reference: &str) -> Result<ContainerImage> {
        if !input_path.exists() {
            return Err(ContainerError::image(
                "Import file does not exist".to_string()
            ));
        }
        
        info!("Importing image from {:?} as {}", input_path, reference);
        
        // Simulate import (would normally extract tar and create image)
        self.pull(reference).await
    }
    
    async fn get_layers(&self, reference: &str) -> Result<Vec<PathBuf>> {
        let image = self.get(reference).await?;
        let image_path = self.image_path(reference);
        
        let mut layer_paths = Vec::new();
        for (i, layer) in image.layers.iter().enumerate() {
            let layer_path = image_path.join(format!("layer_{}.tar", i));
            
            // Simulate layer file creation if it doesn't exist
            if !layer_path.exists() {
                std::fs::write(&layer_path, format!("Mock layer data for {}", layer.digest))
                    .map_err(|e| ContainerError::image(
                        format!("Failed to create layer file: {}", e)
                    ))?;
            }
            
            layer_paths.push(layer_path);
        }
        
        Ok(layer_paths)
    }
    
    async fn garbage_collect(&self) -> Result<Vec<String>> {
        info!("Running image garbage collection");
        
        let mut removed_images = Vec::new();
        let cutoff_time = SystemTime::now() - self.storage_config.gc_policy.max_age;
        
        let images = self.images.read().await;
        let candidates: Vec<_> = images.iter()
            .filter(|(_, image)| image.created_at < cutoff_time)
            .map(|(ref_name, _)| ref_name.clone())
            .collect();
        drop(images);
        
        for reference in candidates {
            match self.remove(&reference).await {
                Ok(_) => {
                    removed_images.push(reference);
                },
                Err(e) => {
                    warn!("Failed to remove image during GC: {}: {}", reference, e);
                },
            }
        }
        
        info!("Garbage collection removed {} images", removed_images.len());
        Ok(removed_images)
    }
}

// Simple SHA256 digest for mock purposes
mod sha256 {
    pub fn digest(input: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        format!("{:016x}{:016x}", hasher.finish(), hasher.finish())
    }
}