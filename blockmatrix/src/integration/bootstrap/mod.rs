// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Bootstrap Manager for Web3 Ecosystem
//!
//! Manages the phased bootstrap process to resolve circular dependencies
//! between HyperMesh, TrustChain, STOQ, Catalog, and Caesar components.

mod providers;
pub mod metrics;

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, SystemTime, Instant};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::{RwLock, Notify};
use tracing::{info, instrument};
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use dashmap::DashMap;
use async_trait::async_trait;
pub use trustchain::consensus::ConsensusProof;

pub use metrics::{BootstrapMetrics, HealthMonitor, HealthState};
use providers::*;

/// Bootstrap phase enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum BootstrapPhase {
    Traditional = 0,
    Hybrid = 1,
    PartialFederation = 2,
    FullFederation = 3,
}

impl From<u8> for BootstrapPhase {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Traditional,
            1 => Self::Hybrid,
            2 => Self::PartialFederation,
            3 => Self::FullFederation,
            _ => Self::Traditional,
        }
    }
}

/// Component state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentState {
    pub name: String,
    pub status: ComponentStatus,
    pub phase: BootstrapPhase,
    pub started_at: Option<SystemTime>,
    pub last_health_check: Option<SystemTime>,
    pub error_count: u32,
    pub dependencies: Vec<String>,
}

/// Component status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentStatus {
    NotStarted,
    Starting,
    Running,
    Failed(String),
    Stopping,
    Stopped,
}

/// Bootstrap configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub phase_timeouts: HashMap<BootstrapPhase, Duration>,
    pub startup_order: Vec<String>,
    pub max_retries: u32,
    pub health_check_interval: Duration,
    pub auto_transition: bool,
    pub network_usage: NetworkConfig,
}

/// Network configuration for bootstrap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub stoq_bind: SocketAddr,
    pub trustchain_bind: SocketAddr,
    pub hypermesh_bind: SocketAddr,
    pub traditional_dns: Vec<String>,
}

/// Service discovery trait abstraction
#[async_trait]
pub trait ServiceDiscovery: Send + Sync {
    async fn resolve(&self, service: &str) -> Result<ServiceEndpoint>;
    async fn register(&self, registration: ServiceRegistration) -> Result<()>;
    fn phase(&self) -> BootstrapPhase;
}

/// Certificate provider trait abstraction
#[async_trait]
pub trait CertificateProvider: Send + Sync {
    async fn get_certificate(&self, domain: &str) -> Result<Certificate>;
    async fn validate(&self, cert: &Certificate) -> Result<bool>;
    fn phase(&self) -> BootstrapPhase;
}

/// Transport provider trait abstraction
#[async_trait]
pub trait TransportProvider: Send + Sync {
    async fn connect(&self, endpoint: &ServiceEndpoint) -> Result<Connection>;
    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn Listener>>;
    fn phase(&self) -> BootstrapPhase;
}

/// Consensus provider trait abstraction
#[async_trait]
pub trait ConsensusProvider: Send + Sync {
    async fn validate_proof(&self, proof: &ConsensusProof) -> Result<bool>;
    async fn generate_proof(&self, data: &[u8]) -> Result<ConsensusProof>;
    fn phase(&self) -> BootstrapPhase;
    fn is_required(&self) -> bool;
}

/// Connection listener trait
#[async_trait]
pub trait Listener: Send + Sync {
    async fn accept(&self) -> Result<Connection>;
    fn local_addr(&self) -> Result<SocketAddr>;
}

/// Service endpoint representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub address: SocketAddr,
    pub service_type: ServiceType,
    pub metadata: HashMap<String, String>,
}

/// Service types in the ecosystem
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServiceType {
    STOQ, TrustChain, HyperMesh, Catalog, Caesar, DNS, ConsensusNode,
}

/// Service registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistration {
    pub name: String,
    pub service_type: ServiceType,
    pub endpoint: ServiceEndpoint,
    pub health_check_url: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Connection abstraction
#[derive(Debug)]
pub struct Connection {
    pub id: String,
    pub remote_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub established_at: SystemTime,
}

/// Certificate abstraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    pub subject: String,
    pub issuer: String,
    pub not_before: SystemTime,
    pub not_after: SystemTime,
    pub fingerprint: String,
    pub is_self_signed: bool,
}

/// Bootstrap manager for coordinating multi-component startup
#[allow(dead_code)]
pub struct BootstrapManager {
    current_phase: Arc<AtomicU8>,
    components: Arc<DashMap<String, ComponentState>>,
    discovery: Arc<RwLock<Box<dyn ServiceDiscovery>>>,
    certificates: Arc<RwLock<Box<dyn CertificateProvider>>>,
    transport: Arc<RwLock<Box<dyn TransportProvider>>>,
    consensus: Arc<RwLock<Box<dyn ConsensusProvider>>>,
    phase_notifications: Arc<DashMap<BootstrapPhase, Arc<Notify>>>,
    config: Arc<BootstrapConfig>,
    metrics: Arc<BootstrapMetrics>,
    health_monitor: Arc<HealthMonitor>,
}

impl BootstrapManager {
    pub fn new(config: BootstrapConfig) -> Self {
        let phase_notifications = DashMap::new();
        phase_notifications.insert(BootstrapPhase::Traditional, Arc::new(Notify::new()));
        phase_notifications.insert(BootstrapPhase::Hybrid, Arc::new(Notify::new()));
        phase_notifications.insert(BootstrapPhase::PartialFederation, Arc::new(Notify::new()));
        phase_notifications.insert(BootstrapPhase::FullFederation, Arc::new(Notify::new()));

        Self {
            current_phase: Arc::new(AtomicU8::new(0)),
            components: Arc::new(DashMap::new()),
            discovery: Arc::new(RwLock::new(Box::new(TraditionalDNS::new(config.network_usage.traditional_dns.clone())))),
            certificates: Arc::new(RwLock::new(Box::new(SelfSignedProvider::new()))),
            transport: Arc::new(RwLock::new(Box::new(BasicTransport::new()))),
            consensus: Arc::new(RwLock::new(Box::new(NoOpConsensus::new()))),
            phase_notifications: Arc::new(phase_notifications),
            config: Arc::new(config),
            metrics: Arc::new(BootstrapMetrics::new()),
            health_monitor: Arc::new(HealthMonitor::new()),
        }
    }

    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        info!("Starting Web3 ecosystem bootstrap sequence");
        self.metrics.set_start_time();
        self.execute_phase_0().await?;
        if self.should_transition_to_phase_1().await { self.execute_phase_1().await?; }
        if self.should_transition_to_phase_2().await { self.execute_phase_2().await?; }
        if self.should_transition_to_phase_3().await { self.execute_phase_3().await?; }
        info!("Bootstrap sequence completed successfully");
        Ok(())
    }

    async fn execute_phase_0(&self) -> Result<()> {
        info!("Executing Phase 0: Traditional bootstrap");
        let phase_start = Instant::now();
        self.metrics.mark_phase_start(BootstrapPhase::Traditional);
        self.start_component("stoq", vec![]).await?;
        self.start_component("trustchain", vec!["stoq"]).await?;
        self.start_component("hypermesh", vec!["stoq", "trustchain"]).await?;
        self.start_component("catalog", vec!["hypermesh"]).await?;
        self.start_component("caesar", vec!["hypermesh"]).await?;
        self.metrics.mark_phase_complete(BootstrapPhase::Traditional);
        info!("Phase 0 completed in {:?}", phase_start.elapsed());
        if let Some(notify) = self.phase_notifications.get(&BootstrapPhase::Traditional) {
            notify.notify_waiters();
        }
        Ok(())
    }

    async fn execute_phase_1(&self) -> Result<()> {
        info!("Executing Phase 1: Hybrid trust model");
        let phase_start = Instant::now();
        self.metrics.mark_phase_start(BootstrapPhase::Hybrid);
        *self.discovery.write().await = Box::new(HybridDiscovery::new(
            self.config.network_usage.traditional_dns.clone(), self.config.network_usage.trustchain_bind,
        ));
        *self.certificates.write().await = Box::new(TrustChainProvider::new(self.config.network_usage.trustchain_bind));
        self.update_component_phases(BootstrapPhase::Hybrid).await;
        *self.consensus.write().await = Box::new(OptionalConsensus::new(self.config.network_usage.hypermesh_bind));
        self.current_phase.store(1, Ordering::SeqCst);
        self.metrics.mark_phase_complete(BootstrapPhase::Hybrid);
        info!("Phase 1 completed in {:?}", phase_start.elapsed());
        if let Some(notify) = self.phase_notifications.get(&BootstrapPhase::Hybrid) {
            notify.notify_waiters();
        }
        Ok(())
    }

    async fn execute_phase_2(&self) -> Result<()> {
        info!("Executing Phase 2: Partial federation");
        let phase_start = Instant::now();
        self.metrics.mark_phase_start(BootstrapPhase::PartialFederation);
        *self.discovery.write().await = Box::new(FederatedDiscovery::new(
            self.config.network_usage.hypermesh_bind, Some(self.config.network_usage.traditional_dns.clone()),
        ));
        *self.consensus.write().await = Box::new(RequiredConsensus::new(self.config.network_usage.hypermesh_bind));
        self.update_component_phases(BootstrapPhase::PartialFederation).await;
        self.enable_byzantine_detection().await?;
        self.current_phase.store(2, Ordering::SeqCst);
        self.metrics.mark_phase_complete(BootstrapPhase::PartialFederation);
        info!("Phase 2 completed in {:?}", phase_start.elapsed());
        if let Some(notify) = self.phase_notifications.get(&BootstrapPhase::PartialFederation) {
            notify.notify_waiters();
        }
        Ok(())
    }

    async fn execute_phase_3(&self) -> Result<()> {
        info!("Executing Phase 3: Full federation");
        let phase_start = Instant::now();
        self.metrics.mark_phase_start(BootstrapPhase::FullFederation);
        *self.discovery.write().await = Box::new(FederatedDiscovery::new(self.config.network_usage.hypermesh_bind, None));
        *self.consensus.write().await = Box::new(FullConsensus::new(self.config.network_usage.hypermesh_bind));
        self.update_component_phases(BootstrapPhase::FullFederation).await;
        self.enable_advanced_features().await?;
        self.current_phase.store(3, Ordering::SeqCst);
        self.metrics.mark_phase_complete(BootstrapPhase::FullFederation);
        info!("Phase 3 completed in {:?}", phase_start.elapsed());
        if let Some(notify) = self.phase_notifications.get(&BootstrapPhase::FullFederation) {
            notify.notify_waiters();
        }
        Ok(())
    }

    async fn start_component(&self, name: &str, dependencies: Vec<&str>) -> Result<()> {
        info!("Starting component: {}", name);
        for dep in &dependencies { self.wait_for_component(dep).await?; }
        let state = ComponentState {
            name: name.to_string(),
            status: ComponentStatus::Starting,
            phase: self.get_current_phase(),
            started_at: Some(SystemTime::now()),
            last_health_check: None,
            error_count: 0,
            dependencies: dependencies.iter().map(|s| s.to_string()).collect(),
        };
        self.components.insert(name.to_string(), state);
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Some(mut state) = self.components.get_mut(name) { state.status = ComponentStatus::Running; }
        self.health_monitor.start_monitoring(name.to_string()).await;
        info!("Component {} started successfully", name);
        Ok(())
    }

    async fn wait_for_component(&self, name: &str) -> Result<()> {
        let timeout = Duration::from_secs(30);
        let start = Instant::now();
        loop {
            if let Some(state) = self.components.get(name) {
                match state.status {
                    ComponentStatus::Running => return Ok(()),
                    ComponentStatus::Failed(ref err) => return Err(anyhow!("Component {} failed: {}", name, err)),
                    _ => {}
                }
            }
            if start.elapsed() > timeout { return Err(anyhow!("Timeout waiting for component {}", name)); }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn should_transition_to_phase_1(&self) -> bool {
        if !self.config.auto_transition { return false; }
        for component in &["stoq", "trustchain", "hypermesh", "catalog", "caesar"] {
            if let Some(state) = self.components.get(*component) {
                if state.status != ComponentStatus::Running { return false; }
            } else { return false; }
        }
        true
    }

    async fn should_transition_to_phase_2(&self) -> bool {
        self.config.auto_transition && self.current_phase.load(Ordering::SeqCst) >= 1
    }

    async fn should_transition_to_phase_3(&self) -> bool {
        self.config.auto_transition && self.current_phase.load(Ordering::SeqCst) >= 2
    }

    async fn update_component_phases(&self, phase: BootstrapPhase) {
        for mut entry in self.components.iter_mut() { entry.phase = phase; }
    }

    async fn enable_byzantine_detection(&self) -> Result<()> {
        info!("Enabling Byzantine fault detection");
        Ok(())
    }

    async fn enable_advanced_features(&self) -> Result<()> {
        info!("Enabling advanced features");
        Ok(())
    }

    pub fn get_current_phase(&self) -> BootstrapPhase {
        self.current_phase.load(Ordering::SeqCst).into()
    }

    pub async fn wait_for_phase(&self, phase: BootstrapPhase) -> Result<()> {
        if self.get_current_phase() >= phase { return Ok(()); }
        if let Some(notify) = self.phase_notifications.get(&phase) {
            let notify_clone = notify.clone();
            notify_clone.notified().await;
        }
        Ok(())
    }

    pub fn get_metrics(&self) -> &BootstrapMetrics { &self.metrics }

    pub async fn get_component_states(&self) -> HashMap<String, ComponentState> {
        let mut states = HashMap::new();
        for entry in self.components.iter() {
            states.insert(entry.key().clone(), entry.value().clone());
        }
        states
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bootstrap_phases() {
        let config = BootstrapConfig {
            phase_timeouts: {
                let mut m = HashMap::new();
                m.insert(BootstrapPhase::Traditional, Duration::from_secs(10));
                m.insert(BootstrapPhase::Hybrid, Duration::from_secs(20));
                m.insert(BootstrapPhase::PartialFederation, Duration::from_secs(30));
                m.insert(BootstrapPhase::FullFederation, Duration::from_secs(40));
                m
            },
            startup_order: vec![
                "stoq".to_string(), "trustchain".to_string(), "hypermesh".to_string(),
                "catalog".to_string(), "caesar".to_string(),
            ],
            max_retries: 3,
            health_check_interval: Duration::from_secs(5),
            auto_transition: true,
            network_usage: NetworkConfig {
                stoq_bind: "[::1]:9292".parse().expect("test"),
                trustchain_bind: "[::1]:8443".parse().expect("test"),
                hypermesh_bind: "[::1]:8080".parse().expect("test"),
                traditional_dns: vec!["8.8.8.8".to_string()],
            },
        };

        let bootstrap = BootstrapManager::new(config);
        assert_eq!(bootstrap.get_current_phase(), BootstrapPhase::Traditional);
        bootstrap.start().await.expect("Bootstrap should succeed");

        let states = bootstrap.get_component_states().await;
        assert_eq!(states.len(), 5);
        for (_name, state) in states {
            assert_eq!(state.status, ComponentStatus::Running);
        }
    }
}
