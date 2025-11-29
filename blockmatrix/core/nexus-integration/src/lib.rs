//! Nexus Integration Layer
//!
//! This crate provides the integration layer that connects all Nexus core components,
//! orchestrates their interactions, and provides a unified interface for external systems.

use anyhow::Result;
use nexus_shared::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

pub mod cluster;
pub mod coordinator;
pub mod events;
pub mod health;

use coordinator::SystemCoordinator;

/// Main Nexus system that orchestrates all core components
pub struct NexusSystem {
    coordinator: Arc<SystemCoordinator>,
    config: NexusConfig,
    node_id: NodeId,
    state: Arc<RwLock<SystemState>>,
}

/// Current state of the Nexus system
#[derive(Debug, Clone)]
pub struct SystemState {
    pub status: SystemStatus,
    pub components: ComponentStates,
    pub metrics: SystemMetrics,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct ComponentStates {
    pub transport: ComponentStatus,
    pub runtime: ComponentStatus,
    pub state_manager: ComponentStatus,
    pub networking: ComponentStatus,
    pub scheduler: ComponentStatus,
}

#[derive(Debug, Clone)]
pub enum ComponentStatus {
    Starting,
    Running,
    Degraded,
    Failed,
    Stopped,
}

#[derive(Debug, Clone)]
pub enum SystemStatus {
    Initializing,
    Healthy,
    Degraded,
    Critical,
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub network_connections: u32,
    pub active_services: u32,
    pub cluster_nodes: u32,
    pub uptime_seconds: u64,
}

impl NexusSystem {
    /// Create a new Nexus system instance
    pub async fn new(config: NexusConfig, node_id: Option<NodeId>) -> Result<Self> {
        info!("🚀 Initializing Nexus System v{}", VERSION);

        let node_id = node_id.unwrap_or_else(NodeId::random);
        info!("🆔 Node ID: {}", node_id.to_hex());

        // Initialize the system coordinator
        let coordinator = Arc::new(SystemCoordinator::new(&config, node_id).await?);

        let state = Arc::new(RwLock::new(SystemState {
            status: SystemStatus::Initializing,
            components: ComponentStates {
                transport: ComponentStatus::Starting,
                runtime: ComponentStatus::Starting,
                state_manager: ComponentStatus::Starting,
                networking: ComponentStatus::Starting,
                scheduler: ComponentStatus::Starting,
            },
            metrics: SystemMetrics {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                network_connections: 0,
                active_services: 0,
                cluster_nodes: 0,
                uptime_seconds: 0,
            },
            last_updated: chrono::Utc::now(),
        }));

        Ok(Self {
            coordinator,
            config,
            node_id,
            state,
        })
    }

    /// Start all Nexus components
    pub async fn start(&self) -> Result<()> {
        info!("🔄 Starting Nexus system components...");

        // Start coordinator which will start all components
        self.coordinator.start().await?;

        // Update system state
        {
            let mut state = self.state.write().await;
            state.status = SystemStatus::Healthy;
            state.components = ComponentStates {
                transport: ComponentStatus::Running,
                runtime: ComponentStatus::Running,
                state_manager: ComponentStatus::Running,
                networking: ComponentStatus::Running,
                scheduler: ComponentStatus::Running,
            };
            state.last_updated = chrono::Utc::now();
        }

        info!("✅ Nexus system started successfully");
        Ok(())
    }

    /// Stop all Nexus components gracefully
    pub async fn stop(&self) -> Result<()> {
        info!("🛑 Stopping Nexus system...");

        // Update system state
        {
            let mut state = self.state.write().await;
            state.status = SystemStatus::Shutdown;
        }

        // Stop coordinator which will stop all components
        self.coordinator.stop().await?;

        info!("👋 Nexus system stopped");
        Ok(())
    }

    /// Get current system status
    pub async fn status(&self) -> SystemState {
        self.state.read().await.clone()
    }

    /// Get system health information
    pub async fn health(&self) -> health::HealthReport {
        self.coordinator.health_check().await
    }

    /// Deploy a service to the cluster
    pub async fn deploy_service(&self, spec: ServiceSpec) -> Result<ServiceStatus> {
        info!("🚀 Deploying service: {}", spec.name);
        self.coordinator.deploy_service(spec).await
    }

    /// Scale a service
    pub async fn scale_service(&self, name: &str, replicas: u32) -> Result<ServiceStatus> {
        info!("📊 Scaling service {} to {} replicas", name, replicas);
        self.coordinator.scale_service(name, replicas).await
    }

    /// Delete a service
    pub async fn delete_service(&self, name: &str) -> Result<()> {
        info!("🗑️  Deleting service: {}", name);
        self.coordinator.delete_service(name).await
    }

    /// List all services
    pub async fn list_services(&self) -> Result<Vec<ServiceStatus>> {
        self.coordinator.list_services().await
    }

    /// Get service status
    pub async fn get_service(&self, name: &str) -> Result<ServiceStatus> {
        self.coordinator.get_service(name).await
    }

    /// Create a new cluster node
    pub async fn join_cluster(&self, cluster_endpoint: &str) -> Result<()> {
        info!("🔗 Joining cluster at: {}", cluster_endpoint);
        self.coordinator.join_cluster(cluster_endpoint).await
    }

    /// Leave the cluster
    pub async fn leave_cluster(&self) -> Result<()> {
        info!("👋 Leaving cluster");
        self.coordinator.leave_cluster().await
    }

    /// Get cluster information
    pub async fn cluster_info(&self) -> Result<cluster::ClusterInfo> {
        self.coordinator.cluster_info().await
    }

    /// Get system metrics
    pub async fn metrics(&self) -> SystemMetrics {
        let state = self.state.read().await;
        state.metrics.clone()
    }

    /// Subscribe to system events
    pub async fn subscribe_events(&self) -> events::EventStream {
        self.coordinator.event_stream().await
    }
}

/// Service specification for deployment
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceSpec {
    pub name: String,
    pub image: String,
    pub replicas: u32,
    pub resources: ResourceRequirements,
    pub networking: NetworkingSpec,
    pub environment: std::collections::HashMap<String, String>,
    pub volumes: Vec<VolumeSpec>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: f64,
    pub memory_mb: u64,
    pub storage_gb: Option<u64>,
    pub gpu_count: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkingSpec {
    pub ports: Vec<PortSpec>,
    pub ingress: Option<IngressSpec>,
    pub service_mesh: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortSpec {
    pub name: String,
    pub port: u16,
    pub target_port: u16,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Protocol {
    TCP,
    UDP,
    QUIC,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngressSpec {
    pub host: String,
    pub path: String,
    pub tls: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VolumeSpec {
    pub name: String,
    pub mount_path: String,
    pub size_gb: u64,
    pub storage_class: String,
}

/// Current status of a deployed service
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub status: ServiceState,
    pub replicas: u32,
    pub ready_replicas: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub endpoints: Vec<ServiceEndpoint>,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ServiceState {
    Pending,
    Running,
    Scaling,
    Updating,
    Failed,
    Terminated,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceEndpoint {
    pub name: String,
    pub url: String,
    pub port: u16,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub network_tx: u64,
    pub network_rx: u64,
    pub storage_used: u64,
}

impl Default for ServiceSpec {
    fn default() -> Self {
        Self {
            name: "example-service".to_string(),
            image: "nginx:latest".to_string(),
            replicas: 1,
            resources: ResourceRequirements {
                cpu_cores: 0.1,
                memory_mb: 128,
                storage_gb: None,
                gpu_count: None,
            },
            networking: NetworkingSpec {
                ports: vec![PortSpec {
                    name: "http".to_string(),
                    port: 80,
                    target_port: 80,
                    protocol: Protocol::TCP,
                }],
                ingress: None,
                service_mesh: true,
            },
            environment: std::collections::HashMap::new(),
            volumes: Vec::new(),
        }
    }
}

/// Initialize the integration layer
pub async fn init(config: NexusConfig, node_id: Option<NodeId>) -> Result<Arc<NexusSystem>> {
    nexus_shared::init()?;
    
    let system = NexusSystem::new(config, node_id).await?;
    Ok(Arc::new(system))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_system_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = NexusConfig::default();
        config.data_dir = temp_dir.path().to_string_lossy().to_string();

        let system = NexusSystem::new(config, None).await.unwrap();
        let status = system.status().await;
        
        assert!(matches!(status.status, SystemStatus::Initializing));
    }

    #[tokio::test]
    async fn test_service_spec_default() {
        let spec = ServiceSpec::default();
        assert_eq!(spec.name, "example-service");
        assert_eq!(spec.replicas, 1);
        assert_eq!(spec.resources.cpu_cores, 0.1);
        assert_eq!(spec.resources.memory_mb, 128);
    }
}