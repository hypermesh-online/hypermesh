// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! eBPF Security Integration
//!
//! Delegates to the unified hypermesh-ebpf crate for all eBPF operations.
//! BlockMatrix is a CONFIGURATOR -- it sets policies and routing rules,
//! but does not implement eBPF programs directly.

use super::{config::EBPFConfig, error::Result, NetworkPacket, ProcessContext, SystemCall};
use hypermesh_ebpf::{EbpfConfig, HyperMeshEbpf, PacketDecision, ShardMetadata};
use hypermesh_lib::{ContentHash, NetworkId, PrivacyMode};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, info, warn};

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

/// Default network interface used for XDP attachment.
const DEFAULT_XDP_INTERFACE: &str = "lo";

/// Default denied syscalls for security policy.
const DEFAULT_DENIED_SYSCALLS: &[&str] = &[
    "ptrace",
    "kexec_load",
    "kexec_file_load",
    "init_module",
    "finit_module",
    "delete_module",
];

/// eBPF Security Manager -- configurator for the unified hypermesh-ebpf crate.
///
/// Sets security policies and routing rules. Does NOT implement eBPF programs.
/// All actual eBPF operations are delegated to [`HyperMeshEbpf`].
pub struct EBPFSecurityManager {
    /// Underlying hypermesh-ebpf orchestrator (RwLock for mutable XDP attach/detach)
    ebpf: Arc<RwLock<HyperMeshEbpf>>,
    /// Configuration
    config: EBPFConfig,
    /// Registered program metadata
    programs: Vec<EBPFProgram>,
    /// Collected security events
    events: Vec<SecurityEvent>,
    /// Interface name used for XDP attachment (None if not attached)
    attached_interface: Option<String>,
    /// Denied syscall names (userspace policy, not kernel kprobe)
    denied_syscalls: HashSet<String>,
    /// Monitored process PIDs
    monitored_pids: HashSet<u32>,
}

impl EBPFSecurityManager {
    /// Create a new eBPF security manager delegating to hypermesh-ebpf
    pub async fn new() -> Result<Self> {
        let config = EBPFConfig::default();
        let ebpf = HyperMeshEbpf::new(EbpfConfig::default()).map_err(|e| {
            super::error::SecurityError::EBPFError {
                message: e.to_string(),
            }
        })?;

        // Populate the default denied syscalls from config and hardcoded list
        let mut denied_syscalls: HashSet<String> = DEFAULT_DENIED_SYSCALLS
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        for name in &config.default_policies.syscall.blocked_syscalls {
            denied_syscalls.insert(name.clone());
        }

        info!("eBPF security manager initialized (delegating to hypermesh-ebpf)");

        Ok(Self {
            ebpf: Arc::new(RwLock::new(ebpf)),
            config,
            programs: Vec::new(),
            events: Vec::new(),
            attached_interface: None,
            denied_syscalls,
            monitored_pids: HashSet::new(),
        })
    }

    /// Get a shared reference to the underlying HyperMeshEbpf Arc.
    ///
    /// Callers must acquire the read or write lock as needed.
    pub fn ebpf_lock(&self) -> &Arc<RwLock<HyperMeshEbpf>> {
        &self.ebpf
    }

    /// Load default security programs and attach XDP to the configured interface.
    ///
    /// Attaches the XDP program to the first configured network program's
    /// attach point, or falls back to loopback. Also pushes default
    /// validation policies to the eBPF policy manager.
    pub async fn load_default_programs(&mut self) -> Result<()> {
        let interface = self
            .config
            .network_programs
            .first()
            .map(|p| p.attach_point.clone())
            .unwrap_or_else(|| DEFAULT_XDP_INTERFACE.to_string());

        // Attach XDP via the orchestrator
        {
            let mut ebpf = self.ebpf.write();
            ebpf.attach_xdp(&interface)
                .map_err(|e| super::error::SecurityError::EBPFError {
                    message: format!("XDP attach to '{interface}': {e}"),
                })?;
        }
        self.attached_interface = Some(interface.clone());

        // Register program metadata for bookkeeping
        self.programs.push(EBPFProgram {
            name: "hypermesh_xdp".to_string(),
            program_type: ProgramType::XDP,
            attach_point: interface.clone(),
            bytecode_path: PathBuf::from("(managed by hypermesh-ebpf)"),
            handle: 0,
            loaded_at: SystemTime::now(),
        });

        info!(
            "Default eBPF security programs loaded: XDP attached to '{}'",
            interface
        );
        Ok(())
    }

    /// Unload all programs and clean up eBPF state.
    ///
    /// Detaches XDP if attached, and resets local program metadata.
    pub async fn unload_all_programs(&mut self) -> Result<()> {
        if self.attached_interface.is_some() {
            let mut ebpf = self.ebpf.write();
            ebpf.detach_xdp()
                .map_err(|e| super::error::SecurityError::EBPFError {
                    message: format!("XDP detach: {e}"),
                })?;
        }
        self.attached_interface = None;
        self.programs.clear();
        self.events.clear();

        info!("eBPF security programs unloaded");
        Ok(())
    }

    /// Process network packet through eBPF validation.
    ///
    /// Synthesizes a byte representation of the packet metadata and delegates
    /// validation to the hypermesh-ebpf XDP pipeline.
    pub async fn process_packet(&self, packet: &NetworkPacket) -> Result<bool> {
        let repr = format!(
            "{}:{}->{}:{}",
            packet.src_addr, packet.src_port, packet.dst_addr, packet.dst_port
        );
        let decision = self.ebpf.read().validate_packet(repr.as_bytes());
        let allowed = matches!(decision, PacketDecision::Pass);
        debug!("Packet validation: {:?} -> allowed={}", decision, allowed);
        Ok(allowed)
    }

    /// Trace a system call against the deny list.
    ///
    /// Returns `Ok(true)` if the syscall is allowed, `Ok(false)` if denied.
    /// This is a userspace-level policy check, not an actual kernel kprobe.
    pub async fn trace_syscall(&self, syscall: &SystemCall) -> Result<bool> {
        if self.denied_syscalls.contains(&syscall.name) {
            warn!(
                "Syscall denied by policy: '{}' (pid={})",
                syscall.name, syscall.process.pid
            );
            return Ok(false);
        }
        debug!(
            "Syscall allowed: '{}' (pid={})",
            syscall.name, syscall.process.pid
        );
        Ok(true)
    }

    /// Monitor a process against the monitored PID set.
    ///
    /// Returns `Ok(true)` if the process is being monitored (known PID),
    /// `Ok(false)` if the PID is not in the monitored set.
    /// When no PIDs are registered, all processes are considered monitored
    /// (open monitoring policy).
    pub async fn monitor_process(&self, process: &ProcessContext) -> Result<bool> {
        if self.monitored_pids.is_empty() {
            debug!(
                "All-process monitoring active: pid={} name='{}'",
                process.pid, process.name
            );
            return Ok(true);
        }
        let is_monitored = self.monitored_pids.contains(&process.pid);
        debug!(
            "Process monitor check: pid={} name='{}' monitored={}",
            process.pid, process.name, is_monitored
        );
        Ok(is_monitored)
    }

    // -------------------------------------------------------------------
    // State push methods -- forward to hypermesh-ebpf orchestrator
    // -------------------------------------------------------------------

    /// Register an asset hash with shard metadata in the eBPF layer.
    ///
    /// Delegates to [`HyperMeshEbpf::register_asset_hash`] so the XDP
    /// pipeline can validate asset transfers at packet level.
    pub fn register_asset(&self, hash: ContentHash, metadata: ShardMetadata) -> Result<()> {
        self.ebpf
            .read()
            .register_asset_hash(hash, metadata)
            .map_err(|e| super::error::SecurityError::EBPFError {
                message: format!("register_asset_hash: {e}"),
            })
    }

    /// Set the privacy mode for a network in the eBPF policy layer.
    ///
    /// Delegates to [`HyperMeshEbpf::set_privacy_tier`] to configure
    /// per-network validation policies (PoS requirements, hash checks, etc.).
    pub fn set_privacy_mode(&self, network_id: NetworkId, mode: PrivacyMode) -> Result<()> {
        self.ebpf
            .read()
            .set_privacy_tier(network_id, mode)
            .map_err(|e| super::error::SecurityError::EBPFError {
                message: format!("set_privacy_tier: {e}"),
            })
    }

    /// Report a Proof of State validation result to the eBPF layer.
    ///
    /// Delegates to [`HyperMeshEbpf::set_pos_validation`] so subsequent
    /// packets referencing this hash can be validated at packet level.
    pub fn report_pos_validation(&self, hash: ContentHash, valid: bool) -> Result<()> {
        self.ebpf
            .read()
            .set_pos_validation(hash, valid)
            .map_err(|e| super::error::SecurityError::EBPFError {
                message: format!("set_pos_validation: {e}"),
            })
    }

    // -------------------------------------------------------------------
    // Deny list / monitor list management
    // -------------------------------------------------------------------

    /// Add a syscall name to the deny list.
    pub fn deny_syscall(&mut self, name: impl Into<String>) {
        self.denied_syscalls.insert(name.into());
    }

    /// Remove a syscall name from the deny list.
    pub fn allow_syscall(&mut self, name: &str) {
        self.denied_syscalls.remove(name);
    }

    /// Add a PID to the monitored set.
    pub fn add_monitored_pid(&mut self, pid: u32) {
        self.monitored_pids.insert(pid);
    }

    /// Remove a PID from the monitored set.
    pub fn remove_monitored_pid(&mut self, pid: u32) {
        self.monitored_pids.remove(&pid);
    }

    /// Check whether XDP is currently attached.
    pub fn is_xdp_attached(&self) -> bool {
        self.attached_interface.is_some()
    }

    /// Get the interface XDP is attached to, if any.
    pub fn attached_interface(&self) -> Option<&str> {
        self.attached_interface.as_deref()
    }

    // -------------------------------------------------------------------
    // Existing accessors
    // -------------------------------------------------------------------

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

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------
    // EBPFSecurityManager wiring tests
    // -------------------------------------------------------------------

    #[tokio::test]
    async fn test_ebpf_security_manager_creation() {
        let manager = EBPFSecurityManager::new().await;
        assert!(manager.is_ok());
        let manager = manager.expect("test: create EBPFSecurityManager");
        assert!(!manager.is_xdp_attached());
        assert_eq!(manager.active_program_count(), 0);
    }

    #[tokio::test]
    async fn test_syscall_deny_list() {
        let manager = EBPFSecurityManager::new()
            .await
            .expect("test: create EBPFSecurityManager");

        // ptrace should be denied by default
        let ptrace_call = SystemCall {
            number: 101,
            name: "ptrace".to_string(),
            args: vec![],
            return_value: None,
            process: ProcessContext {
                pid: 1234,
                name: "test_proc".to_string(),
                uid: 1000,
                gid: 1000,
                cmdline: "test".to_string(),
                ppid: 1,
            },
            timestamp: SystemTime::now(),
        };
        let result = manager
            .trace_syscall(&ptrace_call)
            .await
            .expect("test: trace ptrace");
        assert!(!result, "ptrace should be denied");

        // read should be allowed
        let read_call = SystemCall {
            number: 0,
            name: "read".to_string(),
            args: vec![],
            return_value: None,
            process: ProcessContext {
                pid: 1234,
                name: "test_proc".to_string(),
                uid: 1000,
                gid: 1000,
                cmdline: "test".to_string(),
                ppid: 1,
            },
            timestamp: SystemTime::now(),
        };
        let result = manager
            .trace_syscall(&read_call)
            .await
            .expect("test: trace read");
        assert!(result, "read should be allowed");
    }

    #[tokio::test]
    async fn test_monitor_process_open_policy() {
        let manager = EBPFSecurityManager::new()
            .await
            .expect("test: create EBPFSecurityManager");

        // With no monitored PIDs, all processes are monitored
        let proc_ctx = ProcessContext {
            pid: 9999,
            name: "any_process".to_string(),
            uid: 1000,
            gid: 1000,
            cmdline: "test".to_string(),
            ppid: 1,
        };
        let result = manager
            .monitor_process(&proc_ctx)
            .await
            .expect("test: monitor");
        assert!(result);
    }

    #[tokio::test]
    async fn test_monitor_process_with_pid_set() {
        let mut manager = EBPFSecurityManager::new()
            .await
            .expect("test: create EBPFSecurityManager");

        manager.add_monitored_pid(100);
        manager.add_monitored_pid(200);

        let monitored = ProcessContext {
            pid: 100,
            name: "monitored".to_string(),
            uid: 1000,
            gid: 1000,
            cmdline: "test".to_string(),
            ppid: 1,
        };
        let result = manager
            .monitor_process(&monitored)
            .await
            .expect("test: monitor");
        assert!(result, "PID 100 should be monitored");

        let not_monitored = ProcessContext {
            pid: 999,
            name: "unknown".to_string(),
            uid: 1000,
            gid: 1000,
            cmdline: "test".to_string(),
            ppid: 1,
        };
        let result = manager
            .monitor_process(&not_monitored)
            .await
            .expect("test: monitor");
        assert!(!result, "PID 999 should not be monitored");
    }

    #[tokio::test]
    async fn test_register_asset_delegates() {
        let manager = EBPFSecurityManager::new()
            .await
            .expect("test: create EBPFSecurityManager");

        let hash = ContentHash::from_bytes([0xAB; 32]);
        let metadata = ShardMetadata {
            shard_index: 0,
            shard_count: 10,
            position: hypermesh_lib::MatrixPosition {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        };

        let result = manager.register_asset(hash, metadata);
        assert!(result.is_ok());

        // Verify it was stored in the ebpf orchestrator
        let ebpf = manager.ebpf.read();
        assert_eq!(ebpf.asset_hash_count(), 1);
        let retrieved = ebpf.get_asset_hash(&hash);
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_set_privacy_mode_delegates() {
        let manager = EBPFSecurityManager::new()
            .await
            .expect("test: create EBPFSecurityManager");

        let network = NetworkId([0x01; 16]);
        let result = manager.set_privacy_mode(network, PrivacyMode::PUBLIC);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_report_pos_validation_delegates() {
        let manager = EBPFSecurityManager::new()
            .await
            .expect("test: create EBPFSecurityManager");

        let hash = ContentHash::from_bytes([0xCD; 32]);
        let result = manager.report_pos_validation(hash, true);
        assert!(result.is_ok());

        // Verify stored
        let ebpf = manager.ebpf.read();
        assert_eq!(ebpf.get_pos_validation(&hash), Some(true));
    }

    #[tokio::test]
    async fn test_load_and_unload_programs() {
        let mut manager = EBPFSecurityManager::new()
            .await
            .expect("test: create EBPFSecurityManager");

        // Load default programs
        let load_result = manager.load_default_programs().await;
        assert!(load_result.is_ok());
        assert!(manager.is_xdp_attached());
        assert_eq!(manager.active_program_count(), 1);
        assert!(manager.attached_interface().is_some());

        // Unload all programs
        let unload_result = manager.unload_all_programs().await;
        assert!(unload_result.is_ok());
        assert!(!manager.is_xdp_attached());
        assert_eq!(manager.active_program_count(), 0);
    }

    #[tokio::test]
    async fn test_deny_and_allow_syscall() {
        let mut manager = EBPFSecurityManager::new()
            .await
            .expect("test: create EBPFSecurityManager");

        // Add custom deny
        manager.deny_syscall("my_dangerous_call");

        let call = SystemCall {
            number: 999,
            name: "my_dangerous_call".to_string(),
            args: vec![],
            return_value: None,
            process: ProcessContext {
                pid: 1,
                name: "test".to_string(),
                uid: 0,
                gid: 0,
                cmdline: "test".to_string(),
                ppid: 0,
            },
            timestamp: SystemTime::now(),
        };
        let result = manager.trace_syscall(&call).await.expect("test: trace");
        assert!(!result, "custom deny should block");

        // Remove it
        manager.allow_syscall("my_dangerous_call");
        let result = manager.trace_syscall(&call).await.expect("test: trace");
        assert!(result, "after allow, should pass");
    }
}
