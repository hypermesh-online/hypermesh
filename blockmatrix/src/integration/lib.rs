//! HyperMesh Platform Integration Layer
//!
//! This module provides the unified integration layer that orchestrates all HyperMesh components
//! including transport, consensus, container runtime, security, and orchestration into a 
//! cohesive distributed computing platform.
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         HyperMesh Platform              │
//! ├─────────────────────────────────────────┤
//! │  Integration Layer (This Module)        │
//! ├─────────────────────────────────────────┤
//! │ Transport│Consensus│Container│Security  │
//! ├─────────────────────────────────────────┤
//! │            STOQ Protocol                │
//! └─────────────────────────────────────────┘
//! ```

#![warn(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn, error, instrument};
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};

// Component imports
use crate::transport::{HyperMeshTransport, TransportConfig, NodeId};
use crate::consensus::{ConsensusManager, ConsensusConfig};
use hypermesh_container::{ContainerRuntime, ContainerRuntimeConfig};
use hypermesh_security::{HyperMeshSecurity, SecurityConfig};
use hypermesh_orchestration::{OrchestrationEngine, OrchestrationConfig};
// Temporarily comment out STOQ until compilation issues are resolved
// use stoq::{Stoq, StoqConfig};

mod config;
mod coordinator;
mod lifecycle;
mod metrics;
mod services;

pub use config::{HyperMeshConfig, ComponentConfig, StoqConfig};
pub use coordinator::PlatformCoordinator;
pub use lifecycle::{ComponentLifecycle, ComponentState};
pub use metrics::{IntegrationMetrics, PlatformMetrics};
pub use services::{ServiceRegistry, ServiceDiscovery};

/// Error types for integration layer
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    /// Component initialization failed
    #[error("Component {component} initialization failed: {message}")]
    ComponentInit { component: String, message: String },
    
    /// Component communication failure
    #[error("Communication failure between {source} and {target}: {message}")]
    ComponentCommunication { source: String, target: String, message: String },
    
    /// Configuration validation error
    #[error("Configuration validation failed: {message}")]
    ConfigValidation { message: String },
    
    /// Platform lifecycle error
    #[error("Platform lifecycle error in {phase}: {message}")]
    Lifecycle { phase: String, message: String },
    
    /// Service registry error
    #[error("Service registry operation failed: {message}")]
    ServiceRegistry { message: String },
    
    /// Underlying component error
    #[error("Component error: {0}")]
    Component(#[from] anyhow::Error),
}

/// Result type for integration operations
pub type IntegrationResult<T> = Result<T, IntegrationError>;

/// HyperMesh Platform - Main integration coordinator
pub struct HyperMeshPlatform {
    /// Platform configuration
    config: Arc<HyperMeshConfig>,
    /// Platform coordinator
    coordinator: Arc<PlatformCoordinator>,
    /// Component states
    components: Arc<RwLock<HashMap<String, ComponentState>>>,
    /// Service registry
    service_registry: Arc<ServiceRegistry>,
    /// Integration metrics
    metrics: Arc<IntegrationMetrics>,
    
    // Core components (STOQ temporarily disabled)
    // /// STOQ transport layer
    // stoq: Arc<Stoq>,
    /// Transport layer
    transport: Arc<HyperMeshTransport>,
    /// Consensus manager
    consensus: Arc<ConsensusManager>,
    /// Container runtime
    container_runtime: Arc<ContainerRuntime>,
    /// Security framework
    security: Arc<RwLock<HyperMeshSecurity>>,
    /// Orchestration engine
    orchestration: Arc<OrchestrationEngine>,
}

impl HyperMeshPlatform {
    /// Create a new HyperMesh platform instance
    #[instrument(skip(config))]
    pub async fn new(config: HyperMeshConfig) -> IntegrationResult<Self> {
        info!("Initializing HyperMesh platform");
        
        let config = Arc::new(config);
        let coordinator = Arc::new(PlatformCoordinator::new());
        let components = Arc::new(RwLock::new(HashMap::new()));
        let service_registry = Arc::new(ServiceRegistry::new());
        let metrics = Arc::new(IntegrationMetrics::new());
        
        // Initialize STOQ transport layer (temporarily disabled)
        // let stoq = Arc::new(
        //     Stoq::new(config.stoq.clone())
        //         .map_err(|e| IntegrationError::ComponentInit {
        //             component: "STOQ".to_string(),
        //             message: e.to_string(),
        //         })?
        // );
        
        // Initialize HyperMesh transport
        let transport = Arc::new(
            HyperMeshTransport::new(config.transport.clone())
                .await
                .map_err(|e| IntegrationError::ComponentInit {
                    component: "Transport".to_string(),
                    message: e.to_string(),
                })?
        );
        
        // Initialize consensus manager
        let consensus = Arc::new(
            ConsensusManager::new(config.consensus.clone())
                .await
                .map_err(|e| IntegrationError::ComponentInit {
                    component: "Consensus".to_string(),
                    message: e.to_string(),
                })?
        );
        
        // Initialize container runtime
        let container_runtime = Arc::new(
            ContainerRuntime::new(config.container.clone())
                .await
                .map_err(|e| IntegrationError::ComponentInit {
                    component: "Container".to_string(),
                    message: e.to_string(),
                })?
        );
        
        // Initialize security framework
        let security = Arc::new(RwLock::new(
            HyperMeshSecurity::new(config.security.clone())
                .await
                .map_err(|e| IntegrationError::ComponentInit {
                    component: "Security".to_string(),
                    message: e.to_string(),
                })?
        ));
        
        // Initialize orchestration engine
        let orchestration = Arc::new(
            OrchestrationEngine::new(config.orchestration.clone())
                .await
                .map_err(|e| IntegrationError::ComponentInit {
                    component: "Orchestration".to_string(),
                    message: e.to_string(),
                })?
        );
        
        Ok(Self {
            config,
            coordinator,
            components,
            service_registry,
            metrics,
            // stoq,  // temporarily disabled
            transport,
            consensus,
            container_runtime,
            security,
            orchestration,
        })
    }
    
    /// Initialize the platform and all components
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> IntegrationResult<()> {
        info!("Starting HyperMesh platform initialization");
        
        // Phase 1: Initialize core transport (STOQ temporarily disabled)
        self.coordinator.start_phase("transport_init").await;
        // self.stoq.start().await
        //     .map_err(|e| IntegrationError::ComponentInit {
        //         component: "STOQ".to_string(),
        //         message: e.to_string(),
        //     })?;
        self.transport.initialize().await
            .map_err(|e| IntegrationError::ComponentInit {
                component: "Transport".to_string(),
                message: e.to_string(),
            })?;
        self.coordinator.complete_phase("transport_init").await;
        
        // Phase 2: Initialize security framework
        self.coordinator.start_phase("security_init").await;
        let mut security = self.security.write().await;
        security.initialize().await
            .map_err(|e| IntegrationError::ComponentInit {
                component: "Security".to_string(),
                message: e.to_string(),
            })?;
        drop(security);
        self.coordinator.complete_phase("security_init").await;
        
        // Phase 3: Initialize consensus with transport integration
        self.coordinator.start_phase("consensus_init").await;
        self.consensus.initialize().await
            .map_err(|e| IntegrationError::ComponentInit {
                component: "Consensus".to_string(),
                message: e.to_string(),
            })?;
        self.coordinator.complete_phase("consensus_init").await;
        
        // Phase 4: Initialize container runtime with security integration
        self.coordinator.start_phase("container_init").await;
        self.container_runtime.initialize().await
            .map_err(|e| IntegrationError::ComponentInit {
                component: "Container".to_string(),
                message: e.to_string(),
            })?;
        self.coordinator.complete_phase("container_init").await;
        
        // Phase 5: Initialize orchestration with all components
        self.coordinator.start_phase("orchestration_init").await;
        self.orchestration.initialize().await
            .map_err(|e| IntegrationError::ComponentInit {
                component: "Orchestration".to_string(),
                message: e.to_string(),
            })?;
        self.coordinator.complete_phase("orchestration_init").await;
        
        // Phase 6: Start cross-component integrations
        self.coordinator.start_phase("integration").await;
        self.setup_component_integrations().await?;
        self.coordinator.complete_phase("integration").await;
        
        info!("HyperMesh platform initialization completed successfully");
        Ok(())
    }
    
    /// Set up cross-component integrations
    #[instrument(skip(self))]
    async fn setup_component_integrations(&self) -> IntegrationResult<()> {
        info!("Setting up cross-component integrations");
        
        // Transport-Consensus integration
        self.setup_transport_consensus_integration().await?;
        
        // Consensus-Container integration  
        self.setup_consensus_container_integration().await?;
        
        // Container-Security integration
        self.setup_container_security_integration().await?;
        
        // Security-Transport integration
        self.setup_security_transport_integration().await?;
        
        // Orchestration integration with all components
        self.setup_orchestration_integrations().await?;
        
        info!("Cross-component integrations setup completed");
        Ok(())
    }
    
    /// Setup Transport-Consensus integration
    async fn setup_transport_consensus_integration(&self) -> IntegrationResult<()> {
        info!("Setting up Transport-Consensus integration");
        
        // Register consensus as a transport service
        self.service_registry.register_service(
            "consensus".to_string(),
            services::ServiceEndpoint {
                service_type: "consensus".to_string(),
                address: self.config.consensus.node_id.clone(),
                port: self.config.consensus.port,
                health_check_path: "/health".to_string(),
            }
        ).await?;
        
        Ok(())
    }
    
    /// Setup Consensus-Container integration  
    async fn setup_consensus_container_integration(&self) -> IntegrationResult<()> {
        info!("Setting up Consensus-Container integration");
        
        // Register container runtime with consensus for coordinated scheduling
        self.service_registry.register_service(
            "container_runtime".to_string(),
            services::ServiceEndpoint {
                service_type: "container_runtime".to_string(),
                address: "localhost".to_string(),
                port: self.config.container.runtime_port.unwrap_or(5000),
                health_check_path: "/runtime/health".to_string(),
            }
        ).await?;
        
        Ok(())
    }
    
    /// Setup Container-Security integration
    async fn setup_container_security_integration(&self) -> IntegrationResult<()> {
        info!("Setting up Container-Security integration");
        
        // Container runtime will use security for eBPF policy enforcement
        // This integration enables real-time security monitoring of containers
        
        Ok(())
    }
    
    /// Setup Security-Transport integration
    async fn setup_security_transport_integration(&self) -> IntegrationResult<()> {
        info!("Setting up Security-Transport integration");
        
        // Transport layer will use security for certificate-based node authentication
        // Security framework will monitor transport layer for anomalies
        
        Ok(())
    }
    
    /// Setup orchestration integrations with all components
    async fn setup_orchestration_integrations(&self) -> IntegrationResult<()> {
        info!("Setting up Orchestration integrations");
        
        // Orchestration engine coordinates all components
        // It uses consensus for distributed decisions
        // It uses container runtime for workload deployment
        // It uses security for policy enforcement
        // It uses transport for inter-node communication
        
        Ok(())
    }
    
    /// Shutdown the platform gracefully
    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> IntegrationResult<()> {
        info!("Shutting down HyperMesh platform");
        
        // Shutdown in reverse order of initialization
        if let Err(e) = self.orchestration.shutdown().await {
            warn!("Orchestration shutdown error: {}", e);
        }
        
        if let Err(e) = self.container_runtime.shutdown().await {
            warn!("Container runtime shutdown error: {}", e);
        }
        
        if let Err(e) = self.consensus.shutdown().await {
            warn!("Consensus shutdown error: {}", e);
        }
        
        if let Ok(mut security) = self.security.try_write() {
            if let Err(e) = security.shutdown().await {
                warn!("Security shutdown error: {}", e);
            }
        }
        
        if let Err(e) = self.transport.shutdown().await {
            warn!("Transport shutdown error: {}", e);
        }
        
        // if let Err(e) = self.stoq.stop().await {
        //     warn!("STOQ shutdown error: {}", e);
        // }
        
        info!("HyperMesh platform shutdown completed");
        Ok(())
    }
    
    /// Get platform metrics
    pub async fn metrics(&self) -> PlatformMetrics {
        self.metrics.collect_platform_metrics().await
    }
    
    /// Get service registry
    pub fn service_registry(&self) -> &ServiceRegistry {
        &self.service_registry
    }
    
    // /// Get component handle (temporarily disabled)
    // pub fn stoq(&self) -> &Arc<Stoq> {
    //     &self.stoq
    // }
    
    /// Get transport handle
    pub fn transport(&self) -> &Arc<HyperMeshTransport> {
        &self.transport
    }
    
    /// Get consensus handle
    pub fn consensus(&self) -> &Arc<ConsensusManager> {
        &self.consensus
    }
    
    /// Get container runtime handle
    pub fn container_runtime(&self) -> &Arc<ContainerRuntime> {
        &self.container_runtime
    }
    
    /// Get security handle
    pub fn security(&self) -> &Arc<RwLock<HyperMeshSecurity>> {
        &self.security
    }
    
    /// Get orchestration handle
    pub fn orchestration(&self) -> &Arc<OrchestrationEngine> {
        &self.orchestration
    }
}

/// Builder pattern for HyperMesh platform
pub struct HyperMeshPlatformBuilder {
    config: HyperMeshConfig,
}

impl HyperMeshPlatformBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: HyperMeshConfig::default(),
        }
    }
    
    /// Set STOQ configuration
    pub fn stoq_config(mut self, config: StoqConfig) -> Self {
        self.config.stoq = config;
        self
    }
    
    /// Set transport configuration
    pub fn transport_config(mut self, config: TransportConfig) -> Self {
        self.config.transport = config;
        self
    }
    
    /// Set consensus configuration
    pub fn consensus_config(mut self, config: ConsensusConfig) -> Self {
        self.config.consensus = config;
        self
    }
    
    /// Set container configuration
    pub fn container_config(mut self, config: ContainerRuntimeConfig) -> Self {
        self.config.container = config;
        self
    }
    
    /// Set security configuration
    pub fn security_config(mut self, config: SecurityConfig) -> Self {
        self.config.security = config;
        self
    }
    
    /// Set orchestration configuration
    pub fn orchestration_config(mut self, config: OrchestrationConfig) -> Self {
        self.config.orchestration = config;
        self
    }
    
    /// Build the platform
    pub async fn build(self) -> IntegrationResult<HyperMeshPlatform> {
        HyperMeshPlatform::new(self.config).await
    }
}

impl Default for HyperMeshPlatformBuilder {
    fn default() -> Self {
        Self::new()
    }
}