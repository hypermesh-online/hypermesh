//! Component lifecycle management

use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error};

use crate::{IntegrationResult, IntegrationError};

/// Component lifecycle management trait
#[async_trait::async_trait]
pub trait ComponentLifecycle {
    /// Initialize the component
    async fn initialize(&self) -> IntegrationResult<()>;
    
    /// Start the component
    async fn start(&self) -> IntegrationResult<()>;
    
    /// Stop the component
    async fn stop(&self) -> IntegrationResult<()>;
    
    /// Shutdown the component
    async fn shutdown(&self) -> IntegrationResult<()>;
    
    /// Get component health status
    async fn health_check(&self) -> ComponentHealth;
    
    /// Get component state
    async fn get_state(&self) -> ComponentState;
}

/// Component state information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentState {
    /// Component name
    pub name: String,
    /// Current status
    pub status: ComponentStatus,
    /// State timestamp
    pub timestamp: SystemTime,
    /// Initialization time
    pub initialized_at: Option<SystemTime>,
    /// Last health check
    pub last_health_check: Option<SystemTime>,
    /// Error message if any
    pub error: Option<String>,
    /// Component metrics
    pub metrics: ComponentMetrics,
}

/// Component status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentStatus {
    /// Component is uninitialized
    Uninitialized,
    /// Component is initializing
    Initializing,
    /// Component is ready but not started
    Ready,
    /// Component is starting
    Starting,
    /// Component is running normally
    Running,
    /// Component is stopping
    Stopping,
    /// Component is stopped
    Stopped,
    /// Component is in error state
    Error { message: String },
    /// Component is degraded but functional
    Degraded { reason: String },
}

/// Component health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Overall health status
    pub status: HealthStatus,
    /// Health check timestamp
    pub timestamp: SystemTime,
    /// Health details
    pub details: Vec<HealthCheck>,
    /// Overall health score (0.0 - 1.0)
    pub score: f64,
}

/// Health status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Component is healthy
    Healthy,
    /// Component is degraded but functional
    Degraded,
    /// Component is unhealthy
    Unhealthy,
    /// Health status unknown
    Unknown,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: HealthStatus,
    /// Check message
    pub message: Option<String>,
    /// Check duration
    pub duration: Duration,
}

/// Component metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Network bytes sent
    pub network_tx_bytes: u64,
    /// Network bytes received
    pub network_rx_bytes: u64,
    /// Request count
    pub request_count: u64,
    /// Error count
    pub error_count: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

impl ComponentState {
    /// Create a new component state
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: ComponentStatus::Uninitialized,
            timestamp: SystemTime::now(),
            initialized_at: None,
            last_health_check: None,
            error: None,
            metrics: ComponentMetrics::default(),
        }
    }
    
    /// Update component status
    pub fn update_status(&mut self, status: ComponentStatus) {
        self.status = status;
        self.timestamp = SystemTime::now();
    }
    
    /// Mark component as initialized
    pub fn mark_initialized(&mut self) {
        self.status = ComponentStatus::Ready;
        self.initialized_at = Some(SystemTime::now());
        self.timestamp = SystemTime::now();
    }
    
    /// Set error state
    pub fn set_error(&mut self, error_message: &str) {
        self.status = ComponentStatus::Error {
            message: error_message.to_string(),
        };
        self.error = Some(error_message.to_string());
        self.timestamp = SystemTime::now();
    }
    
    /// Check if component is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, ComponentStatus::Running | ComponentStatus::Ready)
    }
    
    /// Check if component is operational
    pub fn is_operational(&self) -> bool {
        matches!(
            self.status,
            ComponentStatus::Running | ComponentStatus::Ready | ComponentStatus::Degraded { .. }
        )
    }
    
    /// Get component uptime
    pub fn uptime(&self) -> Duration {
        if let Some(init_time) = self.initialized_at {
            SystemTime::now()
                .duration_since(init_time)
                .unwrap_or(Duration::from_secs(0))
        } else {
            Duration::from_secs(0)
        }
    }
}

impl ComponentHealth {
    /// Create a new health status
    pub fn new() -> Self {
        Self {
            status: HealthStatus::Unknown,
            timestamp: SystemTime::now(),
            details: Vec::new(),
            score: 0.0,
        }
    }
    
    /// Create healthy status
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            timestamp: SystemTime::now(),
            details: Vec::new(),
            score: 1.0,
        }
    }
    
    /// Create unhealthy status with message
    pub fn unhealthy(message: &str) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            timestamp: SystemTime::now(),
            details: vec![HealthCheck {
                name: "general".to_string(),
                status: HealthStatus::Unhealthy,
                message: Some(message.to_string()),
                duration: Duration::from_millis(0),
            }],
            score: 0.0,
        }
    }
    
    /// Add health check result
    pub fn add_check(&mut self, check: HealthCheck) {
        self.details.push(check);
        self.update_score();
    }
    
    /// Update overall health score based on individual checks
    pub fn update_score(&mut self) {
        if self.details.is_empty() {
            self.score = 0.0;
            self.status = HealthStatus::Unknown;
            return;
        }
        
        let total_score: f64 = self.details.iter().map(|check| {
            match check.status {
                HealthStatus::Healthy => 1.0,
                HealthStatus::Degraded => 0.5,
                HealthStatus::Unhealthy => 0.0,
                HealthStatus::Unknown => 0.0,
            }
        }).sum();
        
        self.score = total_score / self.details.len() as f64;
        
        // Determine overall status
        self.status = if self.score >= 0.8 {
            HealthStatus::Healthy
        } else if self.score >= 0.5 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };
    }
}

impl Default for ComponentMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0,
            network_tx_bytes: 0,
            network_rx_bytes: 0,
            request_count: 0,
            error_count: 0,
            avg_response_time_ms: 0.0,
            uptime_seconds: 0,
        }
    }
}

impl Default for ComponentHealth {
    fn default() -> Self {
        Self::new()
    }
}

/// Component lifecycle manager
/// Lifecycle manager (alias for ComponentLifecycleManager)
pub type LifecycleManager = ComponentLifecycleManager;

pub struct ComponentLifecycleManager {
    /// Managed components
    components: std::collections::HashMap<String, Box<dyn ComponentLifecycle + Send + Sync>>,
}

impl ComponentLifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        Self {
            components: std::collections::HashMap::new(),
        }
    }
    
    /// Register a component
    pub fn register_component<T>(&mut self, name: &str, component: T) 
    where
        T: ComponentLifecycle + Send + Sync + 'static,
    {
        self.components.insert(name.to_string(), Box::new(component));
    }
    
    /// Initialize all components
    pub async fn initialize_all(&self) -> IntegrationResult<()> {
        info!("Initializing all registered components");
        
        for (name, component) in &self.components {
            info!("Initializing component: {}", name);
            if let Err(e) = component.initialize().await {
                error!("Failed to initialize component {}: {}", name, e);
                return Err(e);
            }
        }
        
        info!("All components initialized successfully");
        Ok(())
    }
    
    /// Start all components
    pub async fn start_all(&self) -> IntegrationResult<()> {
        info!("Starting all registered components");
        
        for (name, component) in &self.components {
            info!("Starting component: {}", name);
            if let Err(e) = component.start().await {
                error!("Failed to start component {}: {}", name, e);
                return Err(e);
            }
        }
        
        info!("All components started successfully");
        Ok(())
    }
    
    /// Stop all components
    pub async fn stop_all(&self) -> IntegrationResult<()> {
        info!("Stopping all registered components");
        
        // Stop in reverse order of registration
        for (name, component) in self.components.iter().rev() {
            info!("Stopping component: {}", name);
            if let Err(e) = component.stop().await {
                warn!("Failed to stop component {}: {}", name, e);
                // Continue stopping other components
            }
        }
        
        info!("All components stop sequence completed");
        Ok(())
    }
    
    /// Shutdown all components
    pub async fn shutdown_all(&self) -> IntegrationResult<()> {
        info!("Shutting down all registered components");
        
        // Shutdown in reverse order of registration
        for (name, component) in self.components.iter().rev() {
            info!("Shutting down component: {}", name);
            if let Err(e) = component.shutdown().await {
                warn!("Failed to shutdown component {}: {}", name, e);
                // Continue shutting down other components
            }
        }
        
        info!("All components shutdown sequence completed");
        Ok(())
    }
    
    /// Get health status of all components
    pub async fn health_check_all(&self) -> Vec<(String, ComponentHealth)> {
        let mut results = Vec::new();
        
        for (name, component) in &self.components {
            let health = component.health_check().await;
            results.push((name.clone(), health));
        }
        
        results
    }
}

impl Default for ComponentLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}