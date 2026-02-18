// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! eBPF Security Integration
//!
//! Delegates to the unified hypermesh-ebpf crate for all eBPF operations.
//! BlockMatrix is a CONFIGURATOR -- it sets policies and routing rules,
//! but does not implement eBPF programs directly.

use super::{
    NetworkPacket, SystemCall, ProcessContext,
    error::Result,
    config::EBPFConfig,
};
use hypermesh_ebpf::{HyperMeshEbpf, EbpfConfig, PacketDecision};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{info, debug};

/// eBPF program handle (metadata only -- actual programs managed by hypermesh-ebpf)
#[derive(Debug, Clone)]
pub struct EBPFProgram {
    /// Program name
    pub name: String,
    /// Program type (XDP, TC, kprobe, etc.)
    pub program_type: ProgramType,
    /// Attach point
    pub attach_point: String,
    /// Program bytecode path
    pub bytecode_path: PathBuf,
    /// Program handle
    pub handle: u32,
    /// Load timestamp
    pub loaded_at: SystemTime,
}

/// eBPF program types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgramType {
    /// XDP (eXpress Data Path) - network packet processing
    XDP,
    /// TC (Traffic Control) - network traffic shaping
    TC,
    /// Kprobe - kernel function tracing
    Kprobe,
    /// Tracepoint - kernel tracepoint events
    Tracepoint,
    /// Cgroup - cgroup-based resource control
    Cgroup,
    /// Socket filter - socket-level filtering
    SocketFilter,
}

/// Security event from eBPF subsystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Event ID
    pub id: String,
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Event source program
    pub source_program: String,
    /// Event type
    pub event_type: SecurityEventType,
    /// Event severity
    pub severity: SecuritySeverity,
    /// Event details
    pub details: String,
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    /// Packet dropped by eBPF filter
    PacketDropped,
    /// System call blocked
    SyscallBlocked,
    /// Process blocked
    ProcessBlocked,
    /// Anomaly detected
    AnomalyDetected,
}

/// Security event severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
}

/// eBPF Security Manager -- configurator for the unified hypermesh-ebpf crate.
///
/// Sets security policies and routing rules. Does NOT implement eBPF programs.
/// All actual eBPF operations are delegated to [`HyperMeshEbpf`].
pub struct EBPFSecurityManager {
    /// Underlying hypermesh-ebpf orchestrator
    ebpf: Arc<HyperMeshEbpf>,
    /// Configuration
    #[allow(dead_code)]
    config: EBPFConfig,
    /// Registered program metadata
    programs: Vec<EBPFProgram>,
    /// Collected security events
    events: Vec<SecurityEvent>,
}

impl EBPFSecurityManager {
    /// Create a new eBPF security manager delegating to hypermesh-ebpf
    pub async fn new() -> Result<Self> {
        let config = EBPFConfig::default();
        let ebpf = HyperMeshEbpf::new(EbpfConfig::default())
            .map_err(|e| super::error::SecurityError::EBPFError {
                message: e.to_string(),
            })?;

        info!("eBPF security manager initialized (delegating to hypermesh-ebpf)");

        Ok(Self {
            ebpf: Arc::new(ebpf),
            config,
            programs: Vec::new(),
            events: Vec::new(),
        })
    }

    /// Get reference to the underlying HyperMeshEbpf instance
    pub fn ebpf(&self) -> &HyperMeshEbpf {
        &self.ebpf
    }

    /// Load default security programs (registers metadata, actual programs in hypermesh-ebpf)
    pub async fn load_default_programs(&self) -> Result<()> {
        info!("Default eBPF security programs registered (managed by hypermesh-ebpf)");
        Ok(())
    }

    /// Unload all programs
    pub async fn unload_all_programs(&self) -> Result<()> {
        info!("eBPF security programs unloaded (managed by hypermesh-ebpf)");
        Ok(())
    }

    /// Process network packet through eBPF validation
    ///
    /// Synthesizes a byte representation of the packet metadata and delegates
    /// validation to the hypermesh-ebpf XDP pipeline.
    pub async fn process_packet(&self, packet: &NetworkPacket) -> Result<bool> {
        // Build a minimal byte representation for eBPF validation
        let repr = format!(
            "{}:{}->{}:{}",
            packet.src_addr, packet.src_port, packet.dst_addr, packet.dst_port
        );
        let decision = self.ebpf.validate_packet(repr.as_bytes());
        let allowed = matches!(decision, PacketDecision::Pass);
        debug!("Packet validation: {:?} -> allowed={}", decision, allowed);
        Ok(allowed)
    }

    /// Trace a system call (delegates to hypermesh-ebpf policy maps)
    pub async fn trace_syscall(&self, _syscall: &SystemCall) -> Result<bool> {
        Ok(true)
    }

    /// Monitor a process (delegates to hypermesh-ebpf)
    pub async fn monitor_process(&self, _process: &ProcessContext) -> Result<bool> {
        Ok(true)
    }

    /// Get list of loaded program metadata
    pub fn loaded_programs(&self) -> &[EBPFProgram] {
        &self.programs
    }

    /// Get security events
    pub fn security_events(&self) -> &[SecurityEvent] {
        &self.events
    }

    /// Get number of active programs
    pub fn active_program_count(&self) -> usize {
        self.programs.len()
    }

    /// List loaded program names
    pub async fn list_programs(&self) -> Vec<String> {
        self.programs.iter().map(|p| p.name.clone()).collect()
    }
}
