//! Container filesystem implementation

use crate::{ContainerId, ContainerSpec, config::StorageConfig};
use super::error::{Result, ContainerError};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{info, debug};

/// Copy-on-Write filesystem layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CowLayer {
    pub id: String,
    pub parent: Option<String>,
    pub size: u64,
    pub created: SystemTime,
    pub modifications: BTreeMap<PathBuf, FileModification>,
}

/// File modification types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileModification {
    Created { content: Vec<u8>, permissions: u32 },
    Modified { delta: Vec<u8>, permissions: u32 },
    Deleted,
}

/// Container filesystem trait
#[async_trait]
pub trait ContainerFilesystem: Send + Sync {
    async fn create_container_filesystem(&self, id: ContainerId, spec: &ContainerSpec) -> Result<PathBuf>;
    async fn delete_container_filesystem(&self, id: ContainerId) -> Result<()>;
    async fn get_rootfs_path(&self, id: ContainerId) -> Result<PathBuf>;
    async fn create_layer(&self, id: ContainerId, parent: Option<String>) -> Result<CowLayer>;
    async fn apply_layer(&self, id: ContainerId, layer: &CowLayer) -> Result<()>;
}

/// Default container filesystem implementation
pub struct DefaultContainerFilesystem {
    storage_config: StorageConfig,
    containers: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<ContainerId, PathBuf>>>,
}

impl DefaultContainerFilesystem {
    pub fn new(storage_config: &StorageConfig) -> Result<Self> {
        std::fs::create_dir_all(&storage_config.containers)
            .map_err(|e| ContainerError::filesystem(
                format!("Failed to create containers directory: {}", e)
            ))?;
        
        Ok(Self {
            storage_config: storage_config.clone(),
            containers: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        })
    }
    
    fn container_path(&self, id: ContainerId) -> PathBuf {
        self.storage_config.containers.join(id.to_string())
    }
}

#[async_trait]
impl ContainerFilesystem for DefaultContainerFilesystem {
    async fn create_container_filesystem(&self, id: ContainerId, spec: &ContainerSpec) -> Result<PathBuf> {
        let container_path = self.container_path(id);
        let rootfs_path = container_path.join("rootfs");
        let work_path = container_path.join("work");
        let upper_path = container_path.join("upper");
        
        // Create directory structure
        for path in [&container_path, &rootfs_path, &work_path, &upper_path] {
            std::fs::create_dir_all(path)
                .map_err(|e| ContainerError::filesystem(
                    format!("Failed to create directory {:?}: {}", path, e)
                ))?;
        }
        
        // Create basic filesystem structure
        for dir in ["bin", "etc", "tmp", "var", "usr"] {
            std::fs::create_dir_all(rootfs_path.join(dir))
                .map_err(|e| ContainerError::filesystem(
                    format!("Failed to create directory {}: {}", dir, e)
                ))?;
        }
        
        let mut containers = self.containers.write().await;
        containers.insert(id, rootfs_path.clone());
        
        info!("Created container filesystem for {} at {:?}", id, rootfs_path);
        Ok(rootfs_path)
    }
    
    async fn delete_container_filesystem(&self, id: ContainerId) -> Result<()> {
        let mut containers = self.containers.write().await;
        containers.remove(&id);
        
        let container_path = self.container_path(id);
        if container_path.exists() {
            std::fs::remove_dir_all(&container_path)
                .map_err(|e| ContainerError::filesystem(
                    format!("Failed to delete container filesystem: {}", e)
                ))?;
        }
        
        info!("Deleted container filesystem for {}", id);
        Ok(())
    }
    
    async fn get_rootfs_path(&self, id: ContainerId) -> Result<PathBuf> {
        let containers = self.containers.read().await;
        containers.get(&id).cloned()
            .ok_or_else(|| ContainerError::filesystem("Container filesystem not found"))
    }
    
    async fn create_layer(&self, id: ContainerId, parent: Option<String>) -> Result<CowLayer> {
        let layer = CowLayer {
            id: format!("layer_{}", uuid::Uuid::new_v4()),
            parent,
            size: 0,
            created: SystemTime::now(),
            modifications: BTreeMap::new(),
        };
        
        debug!("Created COW layer {} for container {}", layer.id, id);
        Ok(layer)
    }
    
    async fn apply_layer(&self, id: ContainerId, layer: &CowLayer) -> Result<()> {
        let rootfs_path = self.get_rootfs_path(id).await?;
        
        for (path, modification) in &layer.modifications {
            let target_path = rootfs_path.join(path.strip_prefix("/").unwrap_or(path));
            
            match modification {
                FileModification::Created { content, permissions } => {
                    if let Some(parent) = target_path.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| ContainerError::filesystem(
                                format!("Failed to create parent directory: {}", e)
                            ))?;
                    }
                    std::fs::write(&target_path, content)
                        .map_err(|e| ContainerError::filesystem(
                            format!("Failed to write file: {}", e)
                        ))?;
                },
                FileModification::Modified { delta, permissions: _ } => {
                    if target_path.exists() {
                        let mut existing = std::fs::read(&target_path)
                            .map_err(|e| ContainerError::filesystem(
                                format!("Failed to read existing file: {}", e)
                            ))?;
                        existing.extend_from_slice(delta);
                        std::fs::write(&target_path, existing)
                            .map_err(|e| ContainerError::filesystem(
                                format!("Failed to write modified file: {}", e)
                            ))?;
                    }
                },
                FileModification::Deleted => {
                    if target_path.exists() {
                        std::fs::remove_file(&target_path)
                            .map_err(|e| ContainerError::filesystem(
                                format!("Failed to delete file: {}", e)
                            ))?;
                    }
                },
            }
        }
        
        debug!("Applied layer {} to container {}", layer.id, id);
        Ok(())
    }
}