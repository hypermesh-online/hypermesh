//! Platform coordination and lifecycle management

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{RwLock, Notify};
use tracing::{info, warn, error, instrument};
use serde::{Serialize, Deserialize};

use crate::{IntegrationResult, IntegrationError};

/// Integration coordinator (alias for PlatformCoordinator)
pub type IntegrationCoordinator = PlatformCoordinator;

/// Platform coordinator manages component lifecycle and orchestration
pub struct PlatformCoordinator {
    /// Phase execution state
    phases: Arc<RwLock<HashMap<String, PhaseState>>>,
    /// Global coordination state
    coordination_state: Arc<RwLock<CoordinationState>>,
    /// Phase completion notifications
    notifications: Arc<RwLock<HashMap<String, Arc<Notify>>>>,
}

/// Coordination state for the platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationState {
    /// Current coordination phase
    pub current_phase: Option<String>,
    /// Platform startup timestamp
    pub startup_time: SystemTime,
    /// Phases execution history
    pub phase_history: Vec<PhaseExecution>,
    /// Global platform state
    pub platform_state: PlatformState,
}

/// Platform state enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlatformState {
    /// Platform is starting up
    Starting,
    /// Platform is running normally
    Running,
    /// Platform is shutting down
    Stopping,
    /// Platform has stopped
    Stopped,
    /// Platform is in error state
    Error { message: String },
}

/// Phase execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseState {
    /// Phase name
    pub name: String,
    /// Phase status
    pub status: PhaseStatus,
    /// Start time
    pub start_time: Option<SystemTime>,
    /// End time
    pub end_time: Option<SystemTime>,
    /// Phase dependencies
    pub dependencies: Vec<String>,
    /// Phase progress (0.0 - 1.0)
    pub progress: f64,
    /// Phase error if any
    pub error: Option<String>,
}

/// Phase status enumeration  
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PhaseStatus {
    /// Phase is pending execution
    Pending,
    /// Phase is currently executing
    InProgress,
    /// Phase completed successfully
    Completed,
    /// Phase failed with error
    Failed,
    /// Phase was skipped
    Skipped,
}

/// Phase execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseExecution {
    /// Phase name
    pub phase: String,
    /// Execution start time
    pub start_time: SystemTime,
    /// Execution duration
    pub duration: Duration,
    /// Execution result
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
}

impl PlatformCoordinator {
    /// Create a new platform coordinator
    pub fn new() -> Self {
        Self {
            phases: Arc::new(RwLock::new(HashMap::new())),
            coordination_state: Arc::new(RwLock::new(CoordinationState {
                current_phase: None,
                startup_time: SystemTime::now(),
                phase_history: Vec::new(),
                platform_state: PlatformState::Starting,
            })),
            notifications: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Initialize coordination phases
    #[instrument(skip(self))]
    pub async fn initialize_phases(&self, phase_definitions: Vec<PhaseDefinition>) -> IntegrationResult<()> {
        info!("Initializing coordination phases");
        
        let mut phases = self.phases.write().await;
        let mut notifications = self.notifications.write().await;
        
        for def in phase_definitions {
            let phase_state = PhaseState {
                name: def.name.clone(),
                status: PhaseStatus::Pending,
                start_time: None,
                end_time: None,
                dependencies: def.dependencies,
                progress: 0.0,
                error: None,
            };
            
            phases.insert(def.name.clone(), phase_state);
            notifications.insert(def.name.clone(), Arc::new(Notify::new()));
        }
        
        info!("Initialized {} coordination phases", phases.len());
        Ok(())
    }
    
    /// Start a coordination phase
    #[instrument(skip(self))]
    pub async fn start_phase(&self, phase_name: &str) -> IntegrationResult<()> {
        info!("Starting coordination phase: {}", phase_name);
        
        let mut phases = self.phases.write().await;
        let mut coord_state = self.coordination_state.write().await;
        
        if let Some(phase) = phases.get_mut(phase_name) {
            // Check dependencies
            for dep in &phase.dependencies {
                if let Some(dep_phase) = phases.get(dep) {
                    if dep_phase.status != PhaseStatus::Completed {
                        return Err(IntegrationError::Lifecycle {
                            phase: phase_name.to_string(),
                            message: format!("Dependency '{}' not completed", dep),
                        });
                    }
                }
            }
            
            phase.status = PhaseStatus::InProgress;
            phase.start_time = Some(SystemTime::now());
            coord_state.current_phase = Some(phase_name.to_string());
            
            info!("Phase '{}' started successfully", phase_name);
        } else {
            return Err(IntegrationError::Lifecycle {
                phase: phase_name.to_string(),
                message: "Phase not found".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Complete a coordination phase
    #[instrument(skip(self))]
    pub async fn complete_phase(&self, phase_name: &str) -> IntegrationResult<()> {
        info!("Completing coordination phase: {}", phase_name);
        
        let mut phases = self.phases.write().await;
        let mut coord_state = self.coordination_state.write().await;
        let notifications = self.notifications.read().await;
        
        if let Some(phase) = phases.get_mut(phase_name) {
            let start_time = phase.start_time.unwrap_or(SystemTime::now());
            let end_time = SystemTime::now();
            let duration = end_time.duration_since(start_time).unwrap_or(Duration::from_secs(0));
            
            phase.status = PhaseStatus::Completed;
            phase.end_time = Some(end_time);
            phase.progress = 1.0;
            
            // Record phase execution
            coord_state.phase_history.push(PhaseExecution {
                phase: phase_name.to_string(),
                start_time,
                duration,
                success: true,
                error_message: None,
            });
            
            // Clear current phase if this was it
            if coord_state.current_phase.as_ref() == Some(&phase_name.to_string()) {
                coord_state.current_phase = None;
            }
            
            // Notify waiters
            if let Some(notify) = notifications.get(phase_name) {
                notify.notify_waiters();
            }
            
            info!("Phase '{}' completed successfully in {:?}", phase_name, duration);
        } else {
            return Err(IntegrationError::Lifecycle {
                phase: phase_name.to_string(),
                message: "Phase not found".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Fail a coordination phase
    #[instrument(skip(self))]
    pub async fn fail_phase(&self, phase_name: &str, error_message: &str) -> IntegrationResult<()> {
        error!("Failing coordination phase: {} - {}", phase_name, error_message);
        
        let mut phases = self.phases.write().await;
        let mut coord_state = self.coordination_state.write().await;
        let notifications = self.notifications.read().await;
        
        if let Some(phase) = phases.get_mut(phase_name) {
            let start_time = phase.start_time.unwrap_or(SystemTime::now());
            let end_time = SystemTime::now();
            let duration = end_time.duration_since(start_time).unwrap_or(Duration::from_secs(0));
            
            phase.status = PhaseStatus::Failed;
            phase.end_time = Some(end_time);
            phase.error = Some(error_message.to_string());
            
            // Record phase execution
            coord_state.phase_history.push(PhaseExecution {
                phase: phase_name.to_string(),
                start_time,
                duration,
                success: false,
                error_message: Some(error_message.to_string()),
            });
            
            // Set platform to error state
            coord_state.platform_state = PlatformState::Error {
                message: format!("Phase '{}' failed: {}", phase_name, error_message),
            };
            
            // Clear current phase
            coord_state.current_phase = None;
            
            // Notify waiters
            if let Some(notify) = notifications.get(phase_name) {
                notify.notify_waiters();
            }
            
            error!("Phase '{}' failed after {:?}: {}", phase_name, duration, error_message);
        }
        
        Ok(())
    }
    
    /// Update phase progress
    #[instrument(skip(self))]
    pub async fn update_progress(&self, phase_name: &str, progress: f64) -> IntegrationResult<()> {
        let mut phases = self.phases.write().await;
        
        if let Some(phase) = phases.get_mut(phase_name) {
            phase.progress = progress.clamp(0.0, 1.0);
        }
        
        Ok(())
    }
    
    /// Wait for a phase to complete
    #[instrument(skip(self))]
    pub async fn wait_for_phase(&self, phase_name: &str, timeout: Duration) -> IntegrationResult<()> {
        let notifications = self.notifications.read().await;
        
        if let Some(notify) = notifications.get(phase_name) {
            let notify_clone = notify.clone();
            drop(notifications);
            
            // Wait for notification with timeout
            tokio::time::timeout(timeout, notify_clone.notified()).await
                .map_err(|_| IntegrationError::Lifecycle {
                    phase: phase_name.to_string(),
                    message: "Phase wait timeout".to_string(),
                })?;
            
            // Check if phase actually completed successfully
            let phases = self.phases.read().await;
            if let Some(phase) = phases.get(phase_name) {
                match phase.status {
                    PhaseStatus::Completed => Ok(()),
                    PhaseStatus::Failed => Err(IntegrationError::Lifecycle {
                        phase: phase_name.to_string(),
                        message: phase.error.clone().unwrap_or_else(|| "Phase failed".to_string()),
                    }),
                    _ => Err(IntegrationError::Lifecycle {
                        phase: phase_name.to_string(),
                        message: "Phase not completed".to_string(),
                    }),
                }
            } else {
                Err(IntegrationError::Lifecycle {
                    phase: phase_name.to_string(),
                    message: "Phase not found".to_string(),
                })
            }
        } else {
            Err(IntegrationError::Lifecycle {
                phase: phase_name.to_string(),
                message: "Phase notification not found".to_string(),
            })
        }
    }
    
    /// Get current coordination state
    pub async fn get_state(&self) -> CoordinationState {
        self.coordination_state.read().await.clone()
    }
    
    /// Get all phase states
    pub async fn get_phases(&self) -> HashMap<String, PhaseState> {
        self.phases.read().await.clone()
    }
    
    /// Set platform state
    pub async fn set_platform_state(&self, state: PlatformState) {
        let mut coord_state = self.coordination_state.write().await;
        coord_state.platform_state = state;
    }
    
    /// Get platform uptime
    pub async fn get_uptime(&self) -> Duration {
        let coord_state = self.coordination_state.read().await;
        SystemTime::now()
            .duration_since(coord_state.startup_time)
            .unwrap_or(Duration::from_secs(0))
    }
}

/// Phase definition for coordinator initialization
#[derive(Debug, Clone)]
pub struct PhaseDefinition {
    /// Phase name
    pub name: String,
    /// Phase dependencies (phases that must complete first)
    pub dependencies: Vec<String>,
    /// Expected phase duration
    pub expected_duration: Option<Duration>,
}

impl PhaseDefinition {
    /// Create a new phase definition
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            dependencies: Vec::new(),
            expected_duration: None,
        }
    }
    
    /// Add dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
    
    /// Set expected duration
    pub fn with_expected_duration(mut self, duration: Duration) -> Self {
        self.expected_duration = Some(duration);
        self
    }
}

impl Default for PlatformCoordinator {
    fn default() -> Self {
        Self::new()
    }
}