//! Container live migration implementation

use crate::{ContainerId, error::Result};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::time::Duration;
use tracing::{info, debug};

/// Migration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRequest {
    pub container_id: ContainerId,
    pub destination_node: String,
    pub migration_type: MigrationType,
    pub downtime_budget: Duration,
    pub bandwidth_limit: Option<u64>,
}

/// Migration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationType {
    /// Stop container, transfer, start
    Cold,
    /// Pre-copy memory, stop, transfer remaining, start
    Warm,
    /// Live migration with minimal downtime
    Hot,
}

/// Migration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub container_id: ContainerId,
    pub success: bool,
    pub downtime: Duration,
    pub transferred_bytes: u64,
    pub error_message: Option<String>,
}

/// Migration manager trait
#[async_trait]
pub trait MigrationManager: Send + Sync {
    async fn migrate(&self, request: MigrationRequest) -> Result<MigrationResult>;
    async fn prepare_migration(&self, container_id: ContainerId) -> Result<()>;
    async fn cancel_migration(&self, container_id: ContainerId) -> Result<()>;
    async fn get_migration_status(&self, container_id: ContainerId) -> Result<MigrationStatus>;
}

/// Migration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStatus {
    NotStarted,
    Preparing,
    Transferring,
    Finalizing,
    Complete,
    Failed(String),
}

/// Default migration manager implementation
pub struct DefaultMigrationManager {
    active_migrations: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<ContainerId, MigrationStatus>>>,
}

impl DefaultMigrationManager {
    pub fn new() -> Self {
        Self {
            active_migrations: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait]
impl MigrationManager for DefaultMigrationManager {
    async fn migrate(&self, request: MigrationRequest) -> Result<MigrationResult> {
        let start_time = std::time::Instant::now();
        let container_id = request.container_id;
        
        info!("Starting migration of container {} to {} using {:?}", 
             container_id, request.destination_node, request.migration_type);
        
        // Update status to preparing
        {
            let mut migrations = self.active_migrations.write().await;
            migrations.insert(container_id, MigrationStatus::Preparing);
        }
        
        // Simulate migration process
        match request.migration_type {
            MigrationType::Cold => {
                // Stop container
                tokio::time::sleep(Duration::from_millis(50)).await;
                
                // Transfer state
                self.update_status(container_id, MigrationStatus::Transferring).await;
                tokio::time::sleep(Duration::from_millis(200)).await;
                
                // Start on destination
                tokio::time::sleep(Duration::from_millis(50)).await;
            },
            MigrationType::Warm => {
                // Pre-copy phase
                self.update_status(container_id, MigrationStatus::Transferring).await;
                tokio::time::sleep(Duration::from_millis(150)).await;
                
                // Stop and final transfer
                self.update_status(container_id, MigrationStatus::Finalizing).await;
                tokio::time::sleep(Duration::from_millis(50)).await;
            },
            MigrationType::Hot => {
                // Live migration with minimal downtime
                self.update_status(container_id, MigrationStatus::Transferring).await;
                tokio::time::sleep(Duration::from_millis(80)).await;
                
                self.update_status(container_id, MigrationStatus::Finalizing).await;
                tokio::time::sleep(Duration::from_millis(20)).await;
            },
        }
        
        let downtime = start_time.elapsed();
        let result = MigrationResult {
            container_id,
            success: downtime <= request.downtime_budget,
            downtime,
            transferred_bytes: 1024 * 1024 * 100, // 100MB simulated
            error_message: None,
        };
        
        // Update final status
        if result.success {
            self.update_status(container_id, MigrationStatus::Complete).await;
            info!("Successfully migrated container {} in {:?}", container_id, downtime);
        } else {
            self.update_status(container_id, MigrationStatus::Failed("Downtime budget exceeded".to_string())).await;
        }
        
        Ok(result)
    }
    
    async fn prepare_migration(&self, container_id: ContainerId) -> Result<()> {
        self.update_status(container_id, MigrationStatus::Preparing).await;
        debug!("Prepared migration for container {}", container_id);
        Ok(())
    }
    
    async fn cancel_migration(&self, container_id: ContainerId) -> Result<()> {
        let mut migrations = self.active_migrations.write().await;
        migrations.remove(&container_id);
        debug!("Cancelled migration for container {}", container_id);
        Ok(())
    }
    
    async fn get_migration_status(&self, container_id: ContainerId) -> Result<MigrationStatus> {
        let migrations = self.active_migrations.read().await;
        Ok(migrations.get(&container_id)
           .cloned()
           .unwrap_or(MigrationStatus::NotStarted))
    }
}

impl DefaultMigrationManager {
    async fn update_status(&self, container_id: ContainerId, status: MigrationStatus) {
        let mut migrations = self.active_migrations.write().await;
        migrations.insert(container_id, status);
    }
}

impl Default for DefaultMigrationManager {
    fn default() -> Self {
        Self::new()
    }
}