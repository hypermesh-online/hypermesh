// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! eBPF lifecycle methods for Linux: validation, permissions, load/attach/metrics/unload.
//!
//! NOTE ON eBPF: These methods on the OsAbstraction trait are OS-level abstractions
//! with SIMULATED bytecode management. They do NOT delegate to the real hypermesh-ebpf
//! orchestrator (HyperMeshEbpf).
//!
//! For production HyperMesh eBPF operations, use `EBPFSecurityManager` which delegates
//! to the unified `hypermesh_ebpf::HyperMeshEbpf` crate.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;

use super::super::types::*;
use super::{EbpfProgramState, LinuxAbstraction};

impl LinuxAbstraction {
    /// Validate eBPF program bytecode (basic validation)
    pub(super) fn validate_ebpf_bytecode(&self, program: &[u8]) -> Result<()> {
        if program.is_empty() {
            return Err(anyhow::anyhow!("eBPF program is empty"));
        }

        if program.len() % 8 != 0 {
            return Err(anyhow::anyhow!(
                "eBPF program size must be multiple of 8 bytes (instruction size)"
            ));
        }

        const MAX_INSN_COUNT: usize = 1_000_000;
        if program.len() / 8 > MAX_INSN_COUNT {
            return Err(anyhow::anyhow!(
                "eBPF program too large: {} instructions (max {})",
                program.len() / 8,
                MAX_INSN_COUNT
            ));
        }

        Ok(())
    }

    /// Check if eBPF filesystem is mounted
    pub(super) fn check_bpf_fs(&self) -> bool {
        Path::new("/sys/fs/bpf").exists()
    }

    /// Check CAP_BPF or CAP_SYS_ADMIN capability
    pub(super) fn check_bpf_permissions(&self) -> bool {
        Path::new("/sys/kernel/debug/tracing").exists() || Path::new("/sys/kernel/tracing").exists()
    }

    /// SIMULATION ONLY -- does not load real eBPF bytecode into the kernel.
    pub(super) fn load_ebpf_program_impl(&self, program: &[u8]) -> Result<EbpfHandle> {
        if !self.kernel_supports_ebpf() {
            return Err(anyhow::anyhow!(
                "eBPF not supported: kernel version {:?} < 4.4",
                self.kernel_version
            ));
        }

        if !self.check_bpf_permissions() {
            return Err(anyhow::anyhow!(
                "Insufficient permissions to load eBPF programs. \
                 Requires CAP_BPF or CAP_SYS_ADMIN capability, or root access."
            ));
        }

        self.validate_ebpf_bytecode(program)?;

        let handle_id = self.next_handle.fetch_add(1, Ordering::SeqCst);
        let handle = EbpfHandle(handle_id);

        let state = EbpfProgramState {
            _program_type: EbpfProgramType::Generic,
            attached: false,
            attach_type: None,
            _bytecode: program.to_vec(),
            metrics: HashMap::new(),
            last_update: std::time::SystemTime::now(),
        };

        self.ebpf_programs
            .lock()
            .map_err(|_| anyhow::anyhow!("ebpf programs mutex poisoned"))?
            .insert(handle, state);

        tracing::info!(
            "eBPF program loaded: handle={}, size={} bytes ({} instructions), kernel={:?}",
            handle_id,
            program.len(),
            program.len() / 8,
            self.kernel_version
        );

        Ok(handle)
    }

    /// SIMULATION ONLY -- does not attach to real kernel hooks.
    pub(super) fn attach_ebpf_monitor_impl(
        &self,
        handle: EbpfHandle,
        attach_type: EbpfAttachType,
    ) -> Result<()> {
        let mut programs = self
            .ebpf_programs
            .lock()
            .map_err(|_| anyhow::anyhow!("ebpf programs mutex poisoned"))?;
        let state = programs
            .get_mut(&handle)
            .ok_or_else(|| anyhow::anyhow!("Invalid eBPF handle: {handle:?}"))?;

        if state.attached {
            return Err(anyhow::anyhow!(
                "eBPF program already attached to {:?}",
                state.attach_type
            ));
        }

        match &attach_type {
            EbpfAttachType::Xdp => {
                if !self.kernel_supports_ebpf() {
                    return Err(anyhow::anyhow!("XDP requires kernel >= 4.8"));
                }
            }
            EbpfAttachType::Lsm { .. } => {
                let (major, minor, _) = self.kernel_version;
                if major < 5 || (major == 5 && minor < 7) {
                    return Err(anyhow::anyhow!("LSM hooks require kernel >= 5.7"));
                }
            }
            _ => {}
        }

        state.attached = true;
        state.attach_type = Some(attach_type.clone());

        tracing::info!(
            "eBPF program attached: handle={:?}, type={:?}",
            handle,
            attach_type
        );

        Ok(())
    }

    /// SIMULATION ONLY -- returns synthetic metrics, not real kernel data.
    pub(super) fn read_ebpf_metrics_impl(&self, handle: EbpfHandle) -> Result<EbpfMetrics> {
        let mut programs = self
            .ebpf_programs
            .lock()
            .map_err(|_| anyhow::anyhow!("ebpf programs mutex poisoned"))?;
        let state = programs
            .get_mut(&handle)
            .ok_or_else(|| anyhow::anyhow!("Invalid eBPF handle: {handle:?}"))?;

        if !state.attached {
            return Err(anyhow::anyhow!(
                "eBPF program not attached, cannot read metrics"
            ));
        }

        state.last_update = std::time::SystemTime::now();

        let mut values = state.metrics.clone();
        let mut metadata = HashMap::new();

        if let Some(ref attach_type) = state.attach_type {
            metadata.insert("attach_type".to_string(), format!("{attach_type:?}"));

            match attach_type {
                EbpfAttachType::Xdp => {
                    values.insert("packets_processed".to_string(), 12345);
                    values.insert("bytes_processed".to_string(), 987654);
                    values.insert("packets_dropped".to_string(), 23);
                }
                EbpfAttachType::Kprobe { function } => {
                    values.insert("probe_hits".to_string(), 5678);
                    metadata.insert("function".to_string(), function.clone());
                }
                EbpfAttachType::Tracepoint { category, name } => {
                    values.insert("events".to_string(), 9012);
                    metadata.insert("category".to_string(), category.clone());
                    metadata.insert("name".to_string(), name.clone());
                }
                _ => {
                    values.insert("events".to_string(), 1000);
                }
            }
        }

        metadata.insert(
            "kernel_version".to_string(),
            format!("{:?}", self.kernel_version),
        );

        Ok(EbpfMetrics {
            name: format!("ebpf_program_{}", handle.0),
            metric_type: EbpfMetricType::Counter,
            values,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            metadata,
        })
    }

    /// SIMULATION ONLY -- removes local state but does not unload kernel programs.
    pub(super) fn unload_ebpf_program_impl(&self, handle: EbpfHandle) -> Result<()> {
        let mut programs = self
            .ebpf_programs
            .lock()
            .map_err(|_| anyhow::anyhow!("ebpf programs mutex poisoned"))?;

        let state = programs
            .remove(&handle)
            .ok_or_else(|| anyhow::anyhow!("Invalid eBPF handle: {handle:?}"))?;

        tracing::info!(
            "eBPF program unloaded: handle={:?}, was_attached={}, type={:?}",
            handle,
            state.attached,
            state.attach_type
        );

        Ok(())
    }
}
