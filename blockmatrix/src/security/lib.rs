//! HyperMesh Security Framework
//!
//! Comprehensive security framework providing defense-in-depth protection using
//! eBPF kernel-level enforcement, hardware-assisted virtualization, capability-based
//! security, and zero-trust architecture principles.
//!
//! # Key Features
//! - eBPF-based security enforcement at kernel level
//! - Real-time threat detection and response
//! - Hardware-assisted security isolation
//! - Capability-based access control
//! - Certificate and key management with HSM integration
//! - Machine learning-powered anomaly detection
//! - Automated incident response

#![warn(missing_docs)]
#![deny(unsafe_code)]

// Core security modules
pub mod ebpf;
pub mod capabilities;
pub mod certificates;
pub mod intrusion;
pub mod policies;
pub mod monitoring;
pub mod config;
pub mod error;

// Re-exports
pub use ebpf::{EBPFSecurityManager, EBPFProgram, SecurityEvent};
pub use capabilities::{CapabilitySystem, Capability, PermissionSet};
pub use certificates::{PKIManager, CertificateRotationManager};
pub use intrusion::{IntrusionDetectionSystem, ThreatIndicator};
pub use policies::{SecurityPolicy, PolicyEngine};
pub use monitoring::{SecurityMonitor, SecurityMetrics};
pub use config::SecurityConfig;
pub use error::{SecurityError, Result};

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// Security framework main orchestrator
pub struct HyperMeshSecurity {
    /// eBPF security manager
    pub ebpf_manager: EBPFSecurityManager,
    /// Capability system
    pub capability_system: CapabilitySystem,
    /// PKI manager
    pub pki_manager: PKIManager,
    /// Intrusion detection system
    pub ids: IntrusionDetectionSystem,
    /// Policy engine
    pub policy_engine: PolicyEngine,
    /// Security monitor
    pub monitor: SecurityMonitor,
    /// Configuration
    config: SecurityConfig,
}

impl HyperMeshSecurity {
    /// Create a new security framework instance
    pub async fn new(config: SecurityConfig) -> Result<Self> {
        let ebpf_manager = EBPFSecurityManager::new().await?;
        let capability_system = CapabilitySystem::new();
        let pki_manager = PKIManager::new(&config.certificates)?;
        let ids = IntrusionDetectionSystem::new();
        let policy_engine = PolicyEngine::new();
        let monitor = SecurityMonitor::new();
        
        Ok(Self {
            ebpf_manager,
            capability_system,
            pki_manager,
            ids,
            policy_engine,
            monitor,
            config,
        })
    }
    
    /// Initialize all security components
    pub async fn initialize(&mut self) -> Result<()> {
        // Load eBPF security programs
        self.ebpf_manager.load_default_programs().await?;
        
        // Initialize certificate infrastructure
        self.pki_manager.initialize().await?;
        
        // Start security monitoring
        self.monitor.start().await?;
        
        // Load security policies
        self.policy_engine.load_default_policies().await?;
        
        tracing::info!("HyperMesh security framework initialized");
        Ok(())
    }
    
    /// Shutdown security framework
    pub async fn shutdown(&mut self) -> Result<()> {
        self.monitor.stop().await?;
        self.ebpf_manager.unload_all_programs().await?;
        tracing::info!("HyperMesh security framework shutdown");
        Ok(())
    }
}

/// Security context for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Principal performing the operation
    pub principal: Principal,
    /// Resource being accessed
    pub resource: Resource,
    /// Operation being performed
    pub operation: Operation,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Additional context
    pub metadata: HashMap<String, String>,
}

/// Principal (user, process, service, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum Principal {
    /// User principal
    User { id: String, groups: Vec<String> },
    /// Process principal
    Process { pid: u32, owner: String },
    /// Service principal
    Service { name: String, instance_id: String },
    /// Node principal
    Node { id: String, cluster_id: String },
}

/// Resource being accessed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Resource {
    /// File system resource
    File { path: String },
    /// Network resource
    Network { address: String, port: Option<u16> },
    /// Process resource
    Process { pid: u32 },
    /// Memory resource
    Memory { address_range: (u64, u64) },
    /// Device resource
    Device { device_type: String, device_id: String },
    /// Service resource
    Service { service_name: String, method: Option<String> },
}

/// Operation being performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    /// Read operation
    Read,
    /// Write operation
    Write,
    /// Execute operation
    Execute,
    /// Delete operation
    Delete,
    /// Create operation
    Create,
    /// Connect operation
    Connect,
    /// Bind operation
    Bind,
    /// Listen operation
    Listen,
}

/// Access decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessDecision {
    /// Allow the operation
    Allow,
    /// Deny the operation
    Deny { reason: String },
    /// Allow with conditions
    ConditionalAllow { conditions: Vec<String> },
    /// Defer decision (needs more context)
    Defer { reason: String },
}

/// Security violation severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeverityLevel {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
    /// Warning severity
    Warning,
}

/// Network packet for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPacket {
    /// Source address
    pub src_addr: String,
    /// Destination address
    pub dst_addr: String,
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Protocol
    pub protocol: String,
    /// Payload size
    pub payload_size: usize,
    /// Packet flags
    pub flags: Vec<String>,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// System call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCall {
    /// System call number
    pub number: u64,
    /// System call name
    pub name: String,
    /// Arguments
    pub args: Vec<u64>,
    /// Return value
    pub return_value: Option<i64>,
    /// Process context
    pub process: ProcessContext,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// Process context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessContext {
    /// Process ID
    pub pid: u32,
    /// Process name
    pub name: String,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Command line
    pub cmdline: String,
    /// Parent process ID
    pub ppid: u32,
}

/// Default implementations
impl Default for SecurityConfig {
    fn default() -> Self {
        Self::new()
    }
}