// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Process-based container isolation.
//!
//! Provides real process management for container workloads using
//! `std::process::Command`. No root privileges required -- resource
//! limits are tracked in-process and actual usage is read from
//! `/proc` on Linux with fallback estimates on other platforms.

use super::error::{ContainerError, Result};
use super::resources::ResourceUsage;
use super::types::ContainerId;

use std::collections::HashMap;
use std::process::Child;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Tracks a single managed process.
struct ManagedProcess {
    /// OS child process handle.
    child: Child,
    /// PID of the spawned process.
    pid: u32,
    /// Time the process was started.
    started_at: SystemTime,
    /// Memory budget (bytes) -- tracked, not enforced by OS.
    _memory_budget: u64,
    /// CPU budget (millicores) -- tracked, not enforced by OS.
    _cpu_budget: u64,
}

/// Resource budget entry for a registered (but not necessarily started) container.
#[derive(Debug, Clone)]
pub struct ResourceBudget {
    /// Allocated memory in bytes.
    pub memory_bytes: u64,
    /// Allocated CPU in millicores.
    pub cpu_millicores: u64,
    /// Allocated storage in bytes.
    pub storage_bytes: u64,
}

/// Process-based isolation manager.
///
/// Spawns child processes via `std::process::Command`, tracks them by
/// `ContainerId`, and reads real resource usage from `/proc` on Linux.
pub struct ProcessIsolation {
    /// Running processes keyed by container id.
    processes: Arc<RwLock<HashMap<ContainerId, ManagedProcess>>>,
    /// Resource budgets for registered containers (allocated at create time).
    budgets: Arc<RwLock<HashMap<ContainerId, ResourceBudget>>>,
    /// Total memory budget across all containers.
    total_memory_budget: Arc<RwLock<u64>>,
    /// Total CPU budget across all containers (millicores).
    total_cpu_budget: Arc<RwLock<u64>>,
    /// Capacity limits.
    max_memory: u64,
    max_cpu_millicores: u64,
}

impl ProcessIsolation {
    /// Create a new process isolation manager with the given capacity limits.
    pub fn new(max_memory: u64, max_cpu_millicores: u64) -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            budgets: Arc::new(RwLock::new(HashMap::new())),
            total_memory_budget: Arc::new(RwLock::new(0)),
            total_cpu_budget: Arc::new(RwLock::new(0)),
            max_memory,
            max_cpu_millicores,
        }
    }

    /// Register a container and allocate its resource budget.
    ///
    /// Returns an error if the requested resources exceed remaining capacity.
    pub async fn register(
        &self,
        id: ContainerId,
        memory_bytes: u64,
        cpu_millicores: u64,
        storage_bytes: u64,
    ) -> Result<()> {
        let mut budgets = self.budgets.write().await;
        if budgets.contains_key(&id) {
            return Err(ContainerError::AlreadyExists { id: id.to_string() });
        }

        // Check capacity
        let mut total_mem = self.total_memory_budget.write().await;
        let mut total_cpu = self.total_cpu_budget.write().await;

        if *total_mem + memory_bytes > self.max_memory {
            return Err(ContainerError::InsufficientResources {
                resource: format!(
                    "memory: need {} bytes, available {}",
                    memory_bytes,
                    self.max_memory.saturating_sub(*total_mem)
                ),
            });
        }
        if *total_cpu + cpu_millicores > self.max_cpu_millicores {
            return Err(ContainerError::InsufficientResources {
                resource: format!(
                    "cpu: need {}m, available {}m",
                    cpu_millicores,
                    self.max_cpu_millicores.saturating_sub(*total_cpu)
                ),
            });
        }

        *total_mem += memory_bytes;
        *total_cpu += cpu_millicores;

        budgets.insert(
            id,
            ResourceBudget {
                memory_bytes,
                cpu_millicores,
                storage_bytes,
            },
        );
        debug!(
            "Registered container {} with budget: {}MB mem, {}m cpu",
            id,
            memory_bytes / (1024 * 1024),
            cpu_millicores
        );
        Ok(())
    }

    /// Start a process for the given container.
    ///
    /// The `command` slice must have at least one element (the program).
    /// Environment variables are passed through to the child.
    pub async fn start(
        &self,
        id: &ContainerId,
        command: &[String],
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<u32> {
        // Must be registered
        let budgets = self.budgets.read().await;
        let budget = budgets
            .get(id)
            .ok_or_else(|| ContainerError::NotFound { id: id.to_string() })?;
        let memory_budget = budget.memory_bytes;
        let cpu_budget = budget.cpu_millicores;
        drop(budgets);

        // Must not already be running
        let processes = self.processes.read().await;
        if processes.contains_key(id) {
            return Err(ContainerError::InvalidState {
                expected: "not running".to_string(),
                actual: "running".to_string(),
            });
        }
        drop(processes);

        let program = command.first().ok_or_else(|| ContainerError::Config {
            message: "empty command".to_string(),
        })?;

        let mut cmd = std::process::Command::new(program);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }
        cmd.args(args);

        for (key, val) in env {
            cmd.env(key, val);
        }

        // Capture stdout/stderr
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Set up process group on Unix for clean shutdown
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let child = cmd.spawn().map_err(|e| ContainerError::Runtime {
            message: format!("failed to spawn process: {e}"),
        })?;

        let pid = child.id();
        info!("Started container {} as PID {}", id, pid);

        let managed = ManagedProcess {
            child,
            pid,
            started_at: SystemTime::now(),
            _memory_budget: memory_budget,
            _cpu_budget: cpu_budget,
        };

        let mut processes = self.processes.write().await;
        processes.insert(*id, managed);

        Ok(pid)
    }

    /// Stop a running container process.
    ///
    /// Sends SIGTERM (Unix) or `kill()` (Windows), waits up to `timeout`,
    /// then force-kills if the process has not exited.
    pub async fn stop(
        &self,
        id: &ContainerId,
        timeout: std::time::Duration,
    ) -> Result<Option<i32>> {
        let mut processes = self.processes.write().await;
        let managed = processes
            .get_mut(id)
            .ok_or_else(|| ContainerError::NotFound { id: id.to_string() })?;

        // Send graceful termination signal
        Self::send_term_signal(managed)?;

        // Wait with timeout for the process to exit
        let exit_code = Self::wait_with_timeout(&mut managed.child, timeout).await;

        let pid = managed.pid;
        processes.remove(id);
        info!(
            "Stopped container {} (PID {}), exit code: {:?}",
            id, pid, exit_code
        );
        Ok(exit_code)
    }

    /// Check if a container has a running process.
    pub async fn is_running(&self, id: &ContainerId) -> bool {
        let mut processes = self.processes.write().await;
        if let Some(managed) = processes.get_mut(id) {
            // Check if child actually exited
            match managed.child.try_wait() {
                Ok(Some(_)) => {
                    // Process exited -- remove it
                    processes.remove(id);
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Get the PID for a running container, if any.
    pub async fn pid(&self, id: &ContainerId) -> Option<u32> {
        let processes = self.processes.read().await;
        processes.get(id).map(|m| m.pid)
    }

    /// Unregister a container, releasing its resource budget.
    ///
    /// The container must not have a running process.
    pub async fn unregister(&self, id: &ContainerId) -> Result<()> {
        // Must not be running
        if self.is_running(id).await {
            return Err(ContainerError::InvalidState {
                expected: "not running".to_string(),
                actual: "running".to_string(),
            });
        }

        let mut budgets = self.budgets.write().await;
        if let Some(budget) = budgets.remove(id) {
            let mut total_mem = self.total_memory_budget.write().await;
            let mut total_cpu = self.total_cpu_budget.write().await;
            *total_mem = total_mem.saturating_sub(budget.memory_bytes);
            *total_cpu = total_cpu.saturating_sub(budget.cpu_millicores);
            debug!(
                "Unregistered container {}, released {}MB mem, {}m cpu",
                id,
                budget.memory_bytes / (1024 * 1024),
                budget.cpu_millicores
            );
        }
        Ok(())
    }

    /// Read actual resource usage for a running container.
    ///
    /// On Linux, reads from `/proc/{pid}/stat` and `/proc/{pid}/status`.
    /// On other platforms, returns estimates based on the budget.
    pub async fn get_usage(&self, id: &ContainerId) -> Result<ResourceUsage> {
        let processes = self.processes.read().await;
        if let Some(managed) = processes.get(id) {
            Self::read_process_usage(managed)
        } else {
            // Not running -- return zeroed usage
            let budgets = self.budgets.read().await;
            if budgets.contains_key(id) {
                Ok(Self::zero_usage())
            } else {
                Err(ContainerError::NotFound { id: id.to_string() })
            }
        }
    }

    /// Get the current total allocated memory budget.
    pub async fn total_memory_allocated(&self) -> u64 {
        *self.total_memory_budget.read().await
    }

    /// Get the current total allocated CPU budget.
    pub async fn total_cpu_allocated(&self) -> u64 {
        *self.total_cpu_budget.read().await
    }

    /// Check if a container is registered (has a budget).
    pub async fn is_registered(&self, id: &ContainerId) -> bool {
        self.budgets.read().await.contains_key(id)
    }

    // ---- Private helpers ----

    /// Send SIGTERM on Unix, kill() on other platforms.
    fn send_term_signal(managed: &mut ManagedProcess) -> Result<()> {
        #[cfg(unix)]
        {
            // Send SIGTERM to the process group
            let pgid = managed.pid as i32;
            // Safety: sending signal to a process group that this runtime owns.
            // libc::kill with a negative PID sends to the entire process group.
            #[allow(unsafe_code)]
            let ret = unsafe { libc::kill(-pgid, libc::SIGTERM) };
            if ret != 0 {
                // Fallback: try direct kill on the child
                managed.child.kill().map_err(|e| ContainerError::Runtime {
                    message: format!("failed to kill process: {e}"),
                })?;
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            managed.child.kill().map_err(|e| ContainerError::Runtime {
                message: format!("failed to kill process: {}", e),
            })?;
            Ok(())
        }
    }

    /// Wait for process exit with a timeout; force-kill if needed.
    async fn wait_with_timeout(child: &mut Child, timeout: std::time::Duration) -> Option<i32> {
        let deadline = tokio::time::Instant::now() + timeout;
        let poll_interval = std::time::Duration::from_millis(50);

        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status.code(),
                Ok(None) => {
                    if tokio::time::Instant::now() >= deadline {
                        warn!("Process did not exit within timeout, force-killing");
                        let _ = child.kill();
                        return child.wait().ok().and_then(|s| s.code());
                    }
                    tokio::time::sleep(poll_interval).await;
                }
                Err(e) => {
                    warn!("Error checking process status: {}", e);
                    return None;
                }
            }
        }
    }

    /// Read resource usage from /proc on Linux, estimate on others.
    fn read_process_usage(managed: &ManagedProcess) -> Result<ResourceUsage> {
        let pid = managed.pid;

        #[cfg(target_os = "linux")]
        {
            Self::read_linux_proc_usage(pid, managed)
        }

        #[cfg(not(target_os = "linux"))]
        {
            Ok(Self::estimate_usage(managed))
        }
    }

    /// Linux-specific: read from /proc/{pid}/stat and /proc/{pid}/status.
    #[cfg(target_os = "linux")]
    fn read_linux_proc_usage(pid: u32, managed: &ManagedProcess) -> Result<ResourceUsage> {
        use std::fs;

        let stat_path = format!("/proc/{pid}/stat");
        let status_path = format!("/proc/{pid}/status");

        let cpu_time_ns = match fs::read_to_string(&stat_path) {
            Ok(contents) => Self::parse_cpu_time_from_stat(&contents),
            Err(_) => 0,
        };

        let memory_usage = match fs::read_to_string(&status_path) {
            Ok(contents) => Self::parse_rss_from_status(&contents),
            Err(_) => 0,
        };

        let uptime = managed.started_at.elapsed().unwrap_or_default();
        let cpu_percent = if uptime.as_nanos() > 0 {
            (cpu_time_ns as f64 / uptime.as_nanos() as f64) * 100.0
        } else {
            0.0
        };

        Ok(ResourceUsage {
            memory_usage,
            memory_peak: memory_usage,
            cpu_usage_percent: cpu_percent,
            cpu_time_ns,
            io_bandwidth_current: 0,
            io_bytes_read: 0,
            io_bytes_written: 0,
            network_bandwidth_current: 0,
            network_bytes_rx: 0,
            network_bytes_tx: 0,
            file_descriptors_current: 0,
            processes_current: 1,
            disk_usage: 0,
            timestamp: SystemTime::now(),
        })
    }

    /// Parse total CPU time (user + system) from /proc/pid/stat.
    /// Fields 14 and 15 (0-indexed 13, 14) are utime and stime in clock ticks.
    #[cfg(target_os = "linux")]
    fn parse_cpu_time_from_stat(contents: &str) -> u64 {
        // Skip past the comm field (enclosed in parentheses)
        let after_comm = match contents.rfind(')') {
            Some(idx) => &contents[idx + 2..],
            None => return 0,
        };
        let fields: Vec<&str> = after_comm.split_whitespace().collect();
        // Fields after ')': state(0), ppid(1), ..., utime(11), stime(12)
        if fields.len() < 13 {
            return 0;
        }
        let utime: u64 = fields[11].parse().unwrap_or(0);
        let stime: u64 = fields[12].parse().unwrap_or(0);
        let ticks_per_sec: u64 = 100; // sysconf(_SC_CLK_TCK) default
        let total_ticks = utime + stime;
        total_ticks * 1_000_000_000 / ticks_per_sec
    }

    /// Parse VmRSS from /proc/pid/status (in kB, convert to bytes).
    #[cfg(target_os = "linux")]
    fn parse_rss_from_status(contents: &str) -> u64 {
        for line in contents.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                let trimmed = rest.trim();
                let kb_str = trimmed.split_whitespace().next().unwrap_or("0");
                let kb: u64 = kb_str.parse().unwrap_or(0);
                return kb * 1024;
            }
        }
        0
    }

    /// Fallback usage estimation for non-Linux platforms.
    #[cfg(not(target_os = "linux"))]
    fn estimate_usage(managed: &ManagedProcess) -> ResourceUsage {
        let uptime = managed.started_at.elapsed().unwrap_or_default();
        ResourceUsage {
            memory_usage: managed._memory_budget / 10, // ~10% estimate
            memory_peak: managed._memory_budget / 10,
            cpu_usage_percent: 1.0,
            cpu_time_ns: uptime.as_nanos() as u64 / 100, // ~1% CPU estimate
            io_bandwidth_current: 0,
            io_bytes_read: 0,
            io_bytes_written: 0,
            network_bandwidth_current: 0,
            network_bytes_rx: 0,
            network_bytes_tx: 0,
            file_descriptors_current: 3, // stdin/stdout/stderr
            processes_current: 1,
            disk_usage: 0,
            timestamp: SystemTime::now(),
        }
    }

    /// Return a zeroed ResourceUsage for stopped containers.
    fn zero_usage() -> ResourceUsage {
        ResourceUsage {
            memory_usage: 0,
            memory_peak: 0,
            cpu_usage_percent: 0.0,
            cpu_time_ns: 0,
            io_bandwidth_current: 0,
            io_bytes_read: 0,
            io_bytes_written: 0,
            network_bandwidth_current: 0,
            network_bytes_rx: 0,
            network_bytes_tx: 0,
            file_descriptors_current: 0,
            processes_current: 0,
            disk_usage: 0,
            timestamp: SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id() -> ContainerId {
        ContainerId::new()
    }

    #[tokio::test]
    async fn test_register_and_unregister() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        iso.register(id, 1024 * 1024 * 100, 1000, 0)
            .await
            .expect("test: register should succeed");
        assert!(iso.is_registered(&id).await);

        assert_eq!(iso.total_memory_allocated().await, 1024 * 1024 * 100);
        assert_eq!(iso.total_cpu_allocated().await, 1000);

        iso.unregister(&id)
            .await
            .expect("test: unregister should succeed");
        assert!(!iso.is_registered(&id).await);
        assert_eq!(iso.total_memory_allocated().await, 0);
    }

    #[tokio::test]
    async fn test_register_duplicate_rejected() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        iso.register(id, 1024 * 1024 * 10, 500, 0)
            .await
            .expect("test: first register");
        let err = iso.register(id, 1024 * 1024 * 10, 500, 0).await;
        assert!(err.is_err(), "duplicate register should fail");
    }

    #[tokio::test]
    async fn test_register_exceeds_capacity() {
        let iso = ProcessIsolation::new(1024 * 1024 * 100, 2000);
        let id = make_id();

        let err = iso.register(id, 1024 * 1024 * 200, 1000, 0).await;
        assert!(err.is_err(), "should fail when memory exceeds capacity");

        let id2 = make_id();
        let err = iso.register(id2, 1024 * 1024 * 50, 3000, 0).await;
        assert!(err.is_err(), "should fail when CPU exceeds capacity");
    }

    #[tokio::test]
    async fn test_start_spawns_process() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        iso.register(id, 1024 * 1024 * 100, 1000, 0)
            .await
            .expect("test: register");

        let cmd = vec!["sleep".to_string(), "10".to_string()];
        let pid = iso
            .start(&id, &cmd, &[], &HashMap::new())
            .await
            .expect("test: start");
        assert!(pid > 0, "PID should be positive");
        assert!(iso.is_running(&id).await, "should be running after start");

        // Cleanup
        iso.stop(&id, std::time::Duration::from_secs(2))
            .await
            .expect("test: stop");
    }

    #[tokio::test]
    async fn test_double_start_rejected() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        iso.register(id, 1024 * 1024 * 100, 1000, 0)
            .await
            .expect("test: register");
        let cmd = vec!["sleep".to_string(), "10".to_string()];
        iso.start(&id, &cmd, &[], &HashMap::new())
            .await
            .expect("test: start");

        let err = iso.start(&id, &cmd, &[], &HashMap::new()).await;
        assert!(err.is_err(), "double start should be rejected");

        iso.stop(&id, std::time::Duration::from_secs(2))
            .await
            .expect("test: stop");
    }

    #[tokio::test]
    async fn test_stop_and_exit_code() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        iso.register(id, 1024 * 1024 * 100, 1000, 0)
            .await
            .expect("test: register");

        // Start a process that sleeps
        let cmd = vec!["sleep".to_string(), "60".to_string()];
        iso.start(&id, &cmd, &[], &HashMap::new())
            .await
            .expect("test: start");

        // Stop it
        let exit_code = iso
            .stop(&id, std::time::Duration::from_secs(3))
            .await
            .expect("test: stop");
        // Terminated by signal -- exit code may be None or signal-based
        assert!(
            !iso.is_running(&id).await,
            "should not be running after stop"
        );
        // exit_code is platform-dependent when killed by signal, just verify no panic
        let _ = exit_code;
    }

    #[tokio::test]
    async fn test_stop_not_found() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        let err = iso.stop(&id, std::time::Duration::from_secs(1)).await;
        assert!(err.is_err(), "stopping unknown container should fail");
    }

    #[tokio::test]
    async fn test_unregister_running_rejected() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        iso.register(id, 1024 * 1024 * 100, 1000, 0)
            .await
            .expect("test: register");
        let cmd = vec!["sleep".to_string(), "60".to_string()];
        iso.start(&id, &cmd, &[], &HashMap::new())
            .await
            .expect("test: start");

        let err = iso.unregister(&id).await;
        assert!(err.is_err(), "unregister while running should fail");

        iso.stop(&id, std::time::Duration::from_secs(2))
            .await
            .expect("test: stop");
        iso.unregister(&id)
            .await
            .expect("test: unregister after stop");
    }

    #[tokio::test]
    async fn test_resource_usage_running() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        iso.register(id, 1024 * 1024 * 100, 1000, 0)
            .await
            .expect("test: register");
        let cmd = vec!["sleep".to_string(), "10".to_string()];
        iso.start(&id, &cmd, &[], &HashMap::new())
            .await
            .expect("test: start");

        // Brief sleep to let process settle
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let usage = iso.get_usage(&id).await.expect("test: get_usage");
        assert_eq!(usage.processes_current, 1);

        iso.stop(&id, std::time::Duration::from_secs(2))
            .await
            .expect("test: stop");
    }

    #[tokio::test]
    async fn test_resource_usage_stopped() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        iso.register(id, 1024 * 1024 * 100, 1000, 0)
            .await
            .expect("test: register");

        // Usage for registered but not-running container should be zero
        let usage = iso.get_usage(&id).await.expect("test: get_usage");
        assert_eq!(usage.memory_usage, 0);
        assert_eq!(usage.processes_current, 0);
    }

    #[tokio::test]
    async fn test_resource_usage_unknown() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        let err = iso.get_usage(&id).await;
        assert!(err.is_err(), "usage for unknown container should fail");
    }

    #[tokio::test]
    async fn test_full_lifecycle() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        // Register
        iso.register(id, 1024 * 1024 * 50, 500, 1024 * 1024)
            .await
            .expect("test: register");
        assert!(iso.is_registered(&id).await);

        // Start
        let cmd = vec!["sleep".to_string(), "10".to_string()];
        let pid = iso
            .start(&id, &cmd, &[], &HashMap::new())
            .await
            .expect("test: start");
        assert!(pid > 0);
        assert!(iso.is_running(&id).await);

        // Usage while running
        let usage = iso.get_usage(&id).await.expect("test: usage");
        assert_eq!(usage.processes_current, 1);

        // Stop
        iso.stop(&id, std::time::Duration::from_secs(2))
            .await
            .expect("test: stop");
        assert!(!iso.is_running(&id).await);

        // Unregister
        iso.unregister(&id).await.expect("test: unregister");
        assert!(!iso.is_registered(&id).await);
        assert_eq!(iso.total_memory_allocated().await, 0);
        assert_eq!(iso.total_cpu_allocated().await, 0);
    }

    #[tokio::test]
    async fn test_budget_tracking_multiple_containers() {
        let iso = ProcessIsolation::new(1024 * 1024 * 200, 4000);
        let id1 = make_id();
        let id2 = make_id();

        iso.register(id1, 1024 * 1024 * 80, 1500, 0)
            .await
            .expect("test: register id1");
        iso.register(id2, 1024 * 1024 * 80, 1500, 0)
            .await
            .expect("test: register id2");

        assert_eq!(iso.total_memory_allocated().await, 1024 * 1024 * 160);
        assert_eq!(iso.total_cpu_allocated().await, 3000);

        // Third container should fail -- not enough memory
        let id3 = make_id();
        let err = iso.register(id3, 1024 * 1024 * 80, 500, 0).await;
        assert!(err.is_err(), "should exceed memory capacity");

        iso.unregister(&id1).await.expect("test: unregister id1");
        assert_eq!(iso.total_memory_allocated().await, 1024 * 1024 * 80);

        // Now id3 should fit
        iso.register(id3, 1024 * 1024 * 80, 500, 0)
            .await
            .expect("test: register id3 after release");
    }

    #[tokio::test]
    async fn test_start_without_register() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        let cmd = vec!["sleep".to_string(), "1".to_string()];
        let err = iso.start(&id, &cmd, &[], &HashMap::new()).await;
        assert!(err.is_err(), "start without register should fail");
    }

    #[tokio::test]
    async fn test_process_with_env_vars() {
        let iso = ProcessIsolation::new(1024 * 1024 * 512, 4000);
        let id = make_id();

        iso.register(id, 1024 * 1024 * 50, 500, 0)
            .await
            .expect("test: register");

        let mut env = HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());

        // Use /bin/sh -c to echo and exit immediately
        let cmd = vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo $TEST_VAR".to_string(),
        ];
        let pid = iso
            .start(&id, &cmd, &[], &env)
            .await
            .expect("test: start with env");
        assert!(pid > 0);

        // Wait for short-lived process to exit
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert!(
            !iso.is_running(&id).await,
            "short process should have exited"
        );
    }
}
