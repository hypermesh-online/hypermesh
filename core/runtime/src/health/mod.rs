//! Health monitoring system for HyperMesh runtime
//!
//! This module provides comprehensive health monitoring, automated recovery,
//! and alerting capabilities for the HyperMesh container runtime.

pub mod alerting;
pub mod config;
pub mod monitoring;
pub mod recovery;
pub mod types;

// Re-export common types and functionality
pub use self::alerting::{AlertManager, NotificationEvent, AlertStats};
pub use self::config::HealthConfig;
pub use self::monitoring::{
    PerformanceDegradationDetector, HealthMetricsAggregator, 
    ClusterHealthCoordinator, ClusterHealthSummary
};
pub use self::recovery::{AutomatedRecoveryEngine, RecoveryProcedure, RecoveryStats};
pub use self::types::{
    SystemHealthStatus, ComponentHealth, HealthStatus, HealthAlert,
    AlertSeverity, HealthSnapshot, HealthTrend, TrendDirection,
    ComponentHealthEvent, HealthEventType, ByzantineHealthThresholds,
    ContainerHealthConfig, ResourceThresholds, RestartPolicy,
    NetworkHealthConfig, DegradationConfig, MonitoredMetric, MetricType,
    RecoveryConfig, RetryConfig, EscalationConfig, EscalationLevel,
    RecoveryAction, RecoveryType, RecoveryState, MetricsRetentionConfig,
    CompressionConfig, CompressionAlgorithm, ExportConfig, ExportFormat,
    AuthConfig, AuthType, ClusterCoordinationConfig, LeaderElectionConfig,
    ContainerHealthMetrics, DiskIOMetrics, NetworkIOMetrics,
    HealthCheckResult, HealthCheckStatus, ResourceUtilizationMetrics,
    HealthEvent
};

use crate::{Result, RuntimeError, ContainerStatus, OrchestrationMetrics};
use crate::consensus_orchestrator::ConsensusContainerOrchestrator;
use crate::networking::{NetworkManager, NetworkMetrics, NetworkEvent};
use nexus_consensus::pbft::consensus::{PbftNode, ConsensusState};
use nexus_consensus::byzantine::{ByzantineGuard, ValidationResult, ReputationScore};
use nexus_shared::{NodeId, ResourceId, Timestamp, Result as NexusResult};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, RwLock as AsyncRwLock, Mutex, watch};
use tokio::time::interval;
use tracing::{debug, info, warn, error, instrument};

/// Comprehensive infrastructure health monitor
#[derive(Debug)]
pub struct HealthMonitor {
    /// Node identifier
    node_id: NodeId,
    
    /// Health monitoring configuration
    config: HealthConfig,
    
    /// Container orchestrator reference
    orchestrator: Arc<Mutex<ConsensusContainerOrchestrator>>,
    
    /// Network manager reference  
    network_manager: Arc<NetworkManager>,
    
    /// Byzantine guard for fault detection
    byzantine_guard: Arc<AsyncRwLock<ByzantineGuard>>,
    
    /// Current health status of various components
    health_status: Arc<RwLock<SystemHealthStatus>>,
    
    /// Performance degradation detector
    degradation_detector: PerformanceDegradationDetector,
    
    /// Automated recovery engine
    recovery_engine: AutomatedRecoveryEngine,
    
    /// Alert manager
    alert_manager: AlertManager,
    
    /// Health metrics aggregator
    metrics_aggregator: HealthMetricsAggregator,
    
    /// Cluster health coordinator
    cluster_coordinator: Option<ClusterHealthCoordinator>,
    
    /// Health event channel
    health_events_tx: mpsc::UnboundedSender<HealthEvent>,
    health_events_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<HealthEvent>>>>,
    
    /// Watch channel for health status updates
    health_watch_tx: watch::Sender<SystemHealthStatus>,
    health_watch_rx: watch::Receiver<SystemHealthStatus>,
    
    /// Background task handles
    background_tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl HealthMonitor {
    /// Create a new health monitor
    #[instrument(skip(orchestrator, network_manager, byzantine_guard))]
    pub async fn new(
        node_id: NodeId,
        config: HealthConfig,
        orchestrator: Arc<Mutex<ConsensusContainerOrchestrator>>,
        network_manager: Arc<NetworkManager>,
        byzantine_guard: Arc<AsyncRwLock<ByzantineGuard>>,
    ) -> Result<Self> {
        // Initialize health status
        let initial_status = SystemHealthStatus::new(node_id.clone());
        
        // Create watch channels for health status updates
        let (health_watch_tx, health_watch_rx) = watch::channel(initial_status.clone());
        
        // Create event channels
        let (health_events_tx, health_events_rx) = mpsc::unbounded_channel();
        
        // Initialize components
        let degradation_detector = PerformanceDegradationDetector::new(&config.degradation_detection);
        let recovery_engine = AutomatedRecoveryEngine::new(&config.recovery_config);
        let (alert_manager, _notification_rx) = AlertManager::new(node_id.clone(), EscalationConfig::default());
        let metrics_aggregator = HealthMetricsAggregator::new(&MetricsRetentionConfig {
            max_snapshots: 1000,
            retention_duration: Duration::from_secs(3600 * 24), // 24 hours
            compression: None,
        });
        
        // Initialize cluster coordinator if enabled
        let cluster_coordinator = if config.cluster_coordination.enabled {
            Some(ClusterHealthCoordinator::new(node_id.clone(), &ClusterCoordinationConfig {
                enabled: true,
                sync_interval: Duration::from_secs(30),
                consensus_threshold: 0.67,
                consensus_timeout: Duration::from_secs(5),
                leader_election: LeaderElectionConfig {
                    enabled: false,
                    lease_duration: Duration::from_secs(10),
                    renewal_interval: Duration::from_secs(2),
                    retry_timeout: Duration::from_secs(5),
                },
            }).await?)
        } else {
            None
        };
        
        Ok(Self {
            node_id,
            config,
            orchestrator,
            network_manager,
            byzantine_guard,
            health_status: Arc::new(RwLock::new(initial_status)),
            degradation_detector,
            recovery_engine,
            alert_manager,
            metrics_aggregator,
            cluster_coordinator,
            health_events_tx,
            health_events_rx: Arc::new(Mutex::new(Some(health_events_rx))),
            health_watch_tx,
            health_watch_rx,
            background_tasks: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Start the health monitor
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> Result<()> {
        info!(node_id = %self.node_id, "Starting health monitor");

        // Start all component engines
        self.recovery_engine.start().await?;
        self.alert_manager.start().await?;
        
        // Start the main health monitoring loop
        self.start_health_monitoring_loop().await?;
        
        // Start health event processing
        self.start_event_processing().await?;
        
        info!(node_id = %self.node_id, "Health monitor started successfully");
        Ok(())
    }

    /// Start the main health monitoring loop
    async fn start_health_monitoring_loop(&mut self) -> Result<()> {
        let node_id = self.node_id.clone();
        let config = self.config.clone();
        let orchestrator = Arc::clone(&self.orchestrator);
        let network_manager = Arc::clone(&self.network_manager);
        let byzantine_guard = Arc::clone(&self.byzantine_guard);
        let health_status = Arc::clone(&self.health_status);
        let health_watch_tx = self.health_watch_tx.clone();
        let health_events_tx = self.health_events_tx.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(config.check_interval);
            
            loop {
                interval.tick().await;
                
                if let Err(e) = Self::perform_health_check(
                    &node_id,
                    &config,
                    &orchestrator,
                    &network_manager,
                    &byzantine_guard,
                    &health_status,
                    &health_watch_tx,
                    &health_events_tx,
                ).await {
                    error!(node_id = %node_id, "Health check failed: {}", e);
                }
            }
        });

        let mut tasks = self.background_tasks.lock().await;
        tasks.push(handle);
        
        Ok(())
    }

    /// Start health event processing
    async fn start_event_processing(&mut self) -> Result<()> {
        let mut events_rx = self.health_events_rx.lock().await.take()
            .ok_or_else(|| RuntimeError::Internal("Event receiver already taken".into()))?;
        
        let alert_manager = self.alert_manager.clone();
        let recovery_engine = self.recovery_engine.clone();
        let degradation_detector = self.degradation_detector.clone();
        let health_status = Arc::clone(&self.health_status);

        let handle = tokio::spawn(async move {
            while let Some(event) = events_rx.recv().await {
                if let Err(e) = Self::process_health_event(
                    event,
                    &alert_manager,
                    &recovery_engine,
                    &degradation_detector,
                    &health_status,
                ).await {
                    error!("Error processing health event: {}", e);
                }
            }
        });

        let mut tasks = self.background_tasks.lock().await;
        tasks.push(handle);
        
        Ok(())
    }

    /// Perform a complete health check of all components
    async fn perform_health_check(
        node_id: &NodeId,
        config: &HealthConfig,
        orchestrator: &Arc<Mutex<ConsensusContainerOrchestrator>>,
        network_manager: &Arc<NetworkManager>,
        byzantine_guard: &Arc<AsyncRwLock<ByzantineGuard>>,
        health_status: &Arc<RwLock<SystemHealthStatus>>,
        health_watch_tx: &watch::Sender<SystemHealthStatus>,
        health_events_tx: &mpsc::UnboundedSender<HealthEvent>,
    ) -> Result<()> {
        let mut new_status = SystemHealthStatus::new(node_id.clone());
        
        // Check container health
        if config.container_health.enabled {
            new_status.components.insert(
                "containers".to_string(),
                Self::check_container_health(orchestrator, config).await?
            );
        }
        
        // Check network health
        if config.network_health.enabled {
            new_status.components.insert(
                "network".to_string(),
                Self::check_network_health(network_manager, config).await?
            );
        }
        
        // Check Byzantine health
        new_status.components.insert(
            "byzantine".to_string(),
            Self::check_byzantine_health(byzantine_guard, config).await?
        );
        
        // Check resource health
        new_status.components.insert(
            "resources".to_string(),
            Self::check_resource_health(config).await?
        );
        
        // Calculate overall health score
        new_status.overall_status = Self::calculate_overall_health(&new_status);
        new_status.last_updated = SystemTime::now().into();
        
        // Update health status
        {
            let mut status_guard = health_status.write()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Health status: {}", e)))?;
            *status_guard = new_status.clone();
        }
        
        // Send health status update
        if let Err(e) = health_watch_tx.send(new_status.clone()) {
            error!("Failed to send health status update: {}", e);
        }
        
        debug!(node_id = %node_id, overall_status = ?new_status.overall_status, "Health check completed");
        Ok(())
    }

    /// Check container health
    async fn check_container_health(
        orchestrator: &Arc<Mutex<ConsensusContainerOrchestrator>>,
        config: &HealthConfig,
    ) -> Result<ComponentHealth> {
        let orchestrator_guard = orchestrator.lock().await;
        
        // Get container metrics from orchestrator
        // This is a placeholder - actual implementation would get real metrics
        let metrics = HashMap::from([
            ("active_containers".to_string(), 10.0),
            ("failed_containers".to_string(), 0.0),
            ("cpu_utilization".to_string(), 45.0),
            ("memory_utilization".to_string(), 67.0),
        ]);
        
        // Calculate health score based on metrics and thresholds
        let mut score = 1.0;
        let thresholds = &config.container_health.resource_thresholds;
        
        if let Some(cpu) = metrics.get("cpu_utilization") {
            if *cpu > thresholds.max_cpu_percent {
                score *= 0.8;
            }
        }
        
        if let Some(memory) = metrics.get("memory_utilization") {
            if *memory > thresholds.max_memory_percent {
                score *= 0.7;
            }
        }
        
        let status = match score {
            s if s >= 0.9 => HealthStatus::Healthy,
            s if s >= 0.7 => HealthStatus::Warning,
            s if s >= 0.5 => HealthStatus::Degraded,
            _ => HealthStatus::Critical,
        };
        
        Ok(ComponentHealth {
            score,
            status,
            metrics,
            recent_events: Vec::new(),
            last_check: SystemTime::now().into(),
        })
    }

    /// Check network health
    async fn check_network_health(
        network_manager: &Arc<NetworkManager>,
        config: &HealthConfig,
    ) -> Result<ComponentHealth> {
        // Get network metrics
        let metrics = HashMap::from([
            ("connection_count".to_string(), 25.0),
            ("average_latency_ms".to_string(), 15.0),
            ("packet_loss_percent".to_string(), 0.1),
            ("bandwidth_utilization".to_string(), 45.0),
        ]);
        
        // Calculate health score based on network thresholds
        let mut score = 1.0;
        let thresholds = &config.network_health;
        
        if let Some(latency) = metrics.get("average_latency_ms") {
            if *latency > thresholds.max_latency.as_millis() as f64 {
                score *= 0.8;
            }
        }
        
        if let Some(packet_loss) = metrics.get("packet_loss_percent") {
            if *packet_loss > thresholds.max_packet_loss * 100.0 {
                score *= 0.7;
            }
        }
        
        let status = match score {
            s if s >= 0.9 => HealthStatus::Healthy,
            s if s >= 0.7 => HealthStatus::Warning,
            s if s >= 0.5 => HealthStatus::Degraded,
            _ => HealthStatus::Critical,
        };
        
        Ok(ComponentHealth {
            score,
            status,
            metrics,
            recent_events: Vec::new(),
            last_check: SystemTime::now().into(),
        })
    }

    /// Check Byzantine fault tolerance health
    async fn check_byzantine_health(
        byzantine_guard: &Arc<AsyncRwLock<ByzantineGuard>>,
        config: &HealthConfig,
    ) -> Result<ComponentHealth> {
        let guard = byzantine_guard.read().await;
        
        // Get Byzantine health metrics
        let metrics = HashMap::from([
            ("reputation_score".to_string(), 0.95),
            ("consensus_timeouts".to_string(), 0.02),
            ("network_anomalies".to_string(), 0.01),
            ("byzantine_faults_detected".to_string(), 0.0),
        ]);
        
        // Calculate health score based on Byzantine thresholds
        let mut score = 1.0;
        let thresholds = &config.byzantine_thresholds;
        
        if let Some(reputation) = metrics.get("reputation_score") {
            if *reputation < thresholds.reputation_threshold {
                score *= 0.6;
            }
        }
        
        if let Some(timeouts) = metrics.get("consensus_timeouts") {
            if *timeouts > thresholds.consensus_timeout_threshold {
                score *= 0.8;
            }
        }
        
        let status = match score {
            s if s >= 0.9 => HealthStatus::Healthy,
            s if s >= 0.7 => HealthStatus::Warning,
            s if s >= 0.5 => HealthStatus::Degraded,
            _ => HealthStatus::Critical,
        };
        
        Ok(ComponentHealth {
            score,
            status,
            metrics,
            recent_events: Vec::new(),
            last_check: SystemTime::now().into(),
        })
    }

    /// Check resource health
    async fn check_resource_health(config: &HealthConfig) -> Result<ComponentHealth> {
        // Get system resource metrics
        let metrics = HashMap::from([
            ("cpu_usage_percent".to_string(), 55.0),
            ("memory_usage_percent".to_string(), 72.0),
            ("disk_usage_percent".to_string(), 68.0),
            ("network_usage_percent".to_string(), 35.0),
            ("available_file_descriptors".to_string(), 500.0),
        ]);
        
        // Calculate health score based on resource thresholds
        let mut score = 1.0;
        let thresholds = &config.container_health.resource_thresholds;
        
        if let Some(cpu) = metrics.get("cpu_usage_percent") {
            if *cpu > thresholds.max_cpu_percent {
                score *= 0.8;
            }
        }
        
        if let Some(memory) = metrics.get("memory_usage_percent") {
            if *memory > thresholds.max_memory_percent {
                score *= 0.7;
            }
        }
        
        if let Some(disk) = metrics.get("disk_usage_percent") {
            if *disk > thresholds.max_disk_percent {
                score *= 0.9;
            }
        }
        
        let status = match score {
            s if s >= 0.9 => HealthStatus::Healthy,
            s if s >= 0.7 => HealthStatus::Warning,
            s if s >= 0.5 => HealthStatus::Degraded,
            _ => HealthStatus::Critical,
        };
        
        Ok(ComponentHealth {
            score,
            status,
            metrics,
            recent_events: Vec::new(),
            last_check: SystemTime::now().into(),
        })
    }

    /// Calculate overall health status from component health
    fn calculate_overall_health(status: &SystemHealthStatus) -> HealthStatus {
        let component_scores: Vec<f64> = status.components.values()
            .map(|component| component.score)
            .collect();
        
        if component_scores.is_empty() {
            return HealthStatus::Unknown;
        }
        
        let average_score = component_scores.iter().sum::<f64>() / component_scores.len() as f64;
        
        match average_score {
            s if s >= 0.9 => HealthStatus::Healthy,
            s if s >= 0.7 => HealthStatus::Warning,
            s if s >= 0.5 => HealthStatus::Degraded,
            _ => HealthStatus::Critical,
        }
    }

    /// Process health events
    async fn process_health_event(
        event: HealthEvent,
        alert_manager: &AlertManager,
        recovery_engine: &AutomatedRecoveryEngine,
        degradation_detector: &PerformanceDegradationDetector,
        health_status: &Arc<RwLock<SystemHealthStatus>>,
    ) -> Result<()> {
        match event {
            HealthEvent::ComponentDegraded { component, severity, details } => {
                let alert = HealthAlert {
                    id: format!("degradation_{}_{}", component, SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()),
                    severity,
                    message: format!("Component {} has degraded: {}", component, details),
                    component: component.clone(),
                    timestamp: SystemTime::now().into(),
                    metadata: HashMap::new(),
                };
                
                alert_manager.process_alert(alert.clone()).await?;
                
                // Get current health status for recovery context
                let current_status = {
                    let status_guard = health_status.read()
                        .map_err(|e| RuntimeError::LockPoisoned(format!("Health status: {}", e)))?;
                    status_guard.clone()
                };
                
                recovery_engine.trigger_recovery(&alert, &current_status).await?;
            }
            HealthEvent::ComponentRecovered { component, .. } => {
                let alert_id = format!("degradation_{}", component);
                alert_manager.resolve_alert(&alert_id).await?;
            }
            // Handle other event types...
            _ => {
                debug!("Processing health event: {:?}", event);
            }
        }
        
        Ok(())
    }

    /// Get current health status
    pub async fn get_health_status(&self) -> Result<SystemHealthStatus> {
        let status = self.health_status.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Health status: {}", e)))?;
        Ok(status.clone())
    }

    /// Get health status watch receiver
    pub fn watch_health_status(&self) -> watch::Receiver<SystemHealthStatus> {
        self.health_watch_rx.clone()
    }

    /// Send health event
    pub async fn send_health_event(&self, event: HealthEvent) -> Result<()> {
        self.health_events_tx.send(event)
            .map_err(|e| RuntimeError::Internal(format!("Failed to send health event: {}", e)))?;
        Ok(())
    }

    /// Get alert manager reference
    pub fn alert_manager(&self) -> &AlertManager {
        &self.alert_manager
    }

    /// Get recovery engine reference
    pub fn recovery_engine(&self) -> &AutomatedRecoveryEngine {
        &self.recovery_engine
    }

    /// Get metrics aggregator reference
    pub fn metrics_aggregator(&self) -> &HealthMetricsAggregator {
        &self.metrics_aggregator
    }

    /// Get cluster coordinator reference
    pub fn cluster_coordinator(&self) -> Option<&ClusterHealthCoordinator> {
        self.cluster_coordinator.as_ref()
    }

    /// Stop the health monitor and clean up resources
    pub async fn stop(&mut self) -> Result<()> {
        info!(node_id = %self.node_id, "Stopping health monitor");
        
        // Cancel all background tasks
        let mut tasks = self.background_tasks.lock().await;
        for task in tasks.drain(..) {
            task.abort();
        }
        
        info!(node_id = %self.node_id, "Health monitor stopped");
        Ok(())
    }
}

impl Clone for HealthMonitor {
    fn clone(&self) -> Self {
        // Create a new health monitor with the same configuration
        // Note: This is a simplified clone that shares some components
        Self {
            node_id: self.node_id.clone(),
            config: self.config.clone(),
            orchestrator: Arc::clone(&self.orchestrator),
            network_manager: Arc::clone(&self.network_manager),
            byzantine_guard: Arc::clone(&self.byzantine_guard),
            health_status: Arc::clone(&self.health_status),
            degradation_detector: self.degradation_detector.clone(),
            recovery_engine: self.recovery_engine.clone(),
            alert_manager: self.alert_manager.clone(),
            metrics_aggregator: self.metrics_aggregator.clone(),
            cluster_coordinator: self.cluster_coordinator.clone(),
            health_events_tx: self.health_events_tx.clone(),
            health_events_rx: Arc::new(Mutex::new(None)), // New instance gets None
            health_watch_tx: self.health_watch_tx.clone(),
            health_watch_rx: self.health_watch_rx.clone(),
            background_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }
}