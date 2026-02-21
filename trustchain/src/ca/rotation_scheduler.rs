// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Automated Certificate Rotation Scheduler
//!
//! Background task that periodically invokes `execute_scheduled_rotations()`
//! on the `CertificateRotationManager` at a configurable interval.
//! Uses `tokio_util::sync::CancellationToken` for graceful shutdown.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn, error};

use super::certificate_authority::{CertificateRotationManager, RotationResult};
use super::certificate_store::CertificateStore;

/// Configuration for the certificate rotation scheduler.
#[derive(Clone, Debug)]
pub struct RotationSchedulerConfig {
    /// Whether the scheduler is enabled.
    pub enabled: bool,
    /// How often to check for certificates needing rotation.
    pub check_interval: Duration,
    /// Renew certificates when this much time remains before expiry.
    pub renewal_threshold: Duration,
    /// Maximum number of concurrent rotations per cycle.
    pub max_concurrent_rotations: usize,
}

impl Default for RotationSchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: Duration::from_secs(60 * 60),       // 1 hour
            renewal_threshold: Duration::from_secs(6 * 60 * 60), // 6 hours
            max_concurrent_rotations: 10,
        }
    }
}

/// Metrics tracked by the rotation scheduler (lock-free atomic counters).
#[derive(Debug, Default)]
pub struct RotationSchedulerMetrics {
    /// Total rotation cycles attempted.
    pub rotations_attempted: AtomicU64,
    /// Total rotation cycles that completed successfully.
    pub rotations_succeeded: AtomicU64,
    /// Total rotation cycles that failed.
    pub rotations_failed: AtomicU64,
    /// Total individual certificates rotated across all cycles.
    pub certificates_rotated: AtomicU64,
}

impl RotationSchedulerMetrics {
    /// Snapshot all counters into a plain struct for reporting.
    pub fn snapshot(&self) -> RotationMetricsSnapshot {
        RotationMetricsSnapshot {
            rotations_attempted: self.rotations_attempted.load(Ordering::Relaxed),
            rotations_succeeded: self.rotations_succeeded.load(Ordering::Relaxed),
            rotations_failed: self.rotations_failed.load(Ordering::Relaxed),
            certificates_rotated: self.certificates_rotated.load(Ordering::Relaxed),
        }
    }
}

/// Immutable snapshot of rotation metrics for reporting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RotationMetricsSnapshot {
    pub rotations_attempted: u64,
    pub rotations_succeeded: u64,
    pub rotations_failed: u64,
    pub certificates_rotated: u64,
}

/// Background scheduler that drives automated certificate rotation.
///
/// Call [`start()`](Self::start) to spawn the background task and
/// [`stop()`](Self::stop) to request graceful shutdown.
pub struct CertificateRotationScheduler {
    config: RotationSchedulerConfig,
    rotation_manager: Arc<CertificateRotationManager>,
    certificate_store: Arc<CertificateStore>,
    metrics: Arc<RotationSchedulerMetrics>,
    cancel_token: CancellationToken,
    task_handle: Option<JoinHandle<()>>,
}

impl CertificateRotationScheduler {
    /// Create a new scheduler. Does **not** start the background task yet.
    pub fn new(
        config: RotationSchedulerConfig,
        rotation_manager: Arc<CertificateRotationManager>,
        certificate_store: Arc<CertificateStore>,
    ) -> Self {
        Self {
            config,
            rotation_manager,
            certificate_store,
            metrics: Arc::new(RotationSchedulerMetrics::default()),
            cancel_token: CancellationToken::new(),
            task_handle: None,
        }
    }

    /// Read-only access to live metrics.
    pub fn metrics(&self) -> &Arc<RotationSchedulerMetrics> {
        &self.metrics
    }

    /// Read-only access to configuration.
    pub fn config(&self) -> &RotationSchedulerConfig {
        &self.config
    }

    /// Returns `true` if the background task is currently running.
    pub fn is_running(&self) -> bool {
        self.task_handle
            .as_ref()
            .map_or(false, |h| !h.is_finished())
    }

    /// Spawn the background rotation loop. No-op if already running or disabled.
    pub fn start(&mut self) {
        if !self.config.enabled {
            info!("Certificate rotation scheduler is disabled by configuration");
            return;
        }

        if self.is_running() {
            warn!("Certificate rotation scheduler is already running");
            return;
        }

        let interval = self.config.check_interval;
        let rotation_manager = Arc::clone(&self.rotation_manager);
        let certificate_store = Arc::clone(&self.certificate_store);
        let metrics = Arc::clone(&self.metrics);
        let token = self.cancel_token.clone();

        info!(
            check_interval_secs = interval.as_secs(),
            "Starting certificate rotation scheduler"
        );

        let handle = tokio::spawn(async move {
            run_scheduler_loop(
                interval,
                rotation_manager,
                certificate_store,
                metrics,
                token,
            )
            .await;
        });

        self.task_handle = Some(handle);
    }

    /// Request graceful shutdown and wait for the background task to finish.
    pub async fn stop(&mut self) {
        self.cancel_token.cancel();

        if let Some(handle) = self.task_handle.take() {
            info!("Waiting for certificate rotation scheduler to stop");
            if let Err(e) = handle.await {
                error!(error = %e, "Rotation scheduler task panicked during shutdown");
            }
        }
    }
}

/// Core scheduler loop extracted as a free function for testability.
async fn run_scheduler_loop(
    check_interval: Duration,
    rotation_manager: Arc<CertificateRotationManager>,
    certificate_store: Arc<CertificateStore>,
    metrics: Arc<RotationSchedulerMetrics>,
    cancel_token: CancellationToken,
) {
    let mut interval = tokio::time::interval(check_interval);
    // The first tick completes immediately; consume it so we wait a full
    // period before the first rotation attempt.
    interval.tick().await;

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Certificate rotation scheduler received cancellation signal");
                break;
            }
            _ = interval.tick() => {
                execute_rotation_cycle(
                    &rotation_manager,
                    &certificate_store,
                    &metrics,
                ).await;
            }
        }
    }

    info!("Certificate rotation scheduler stopped");
}

/// Run a single rotation cycle, updating metrics accordingly.
async fn execute_rotation_cycle(
    rotation_manager: &CertificateRotationManager,
    certificate_store: &CertificateStore,
    metrics: &RotationSchedulerMetrics,
) {
    metrics.rotations_attempted.fetch_add(1, Ordering::Relaxed);

    match rotation_manager.execute_scheduled_rotations(certificate_store).await {
        Ok(RotationResult::Success { rotated_count }) => {
            metrics.rotations_succeeded.fetch_add(1, Ordering::Relaxed);
            metrics
                .certificates_rotated
                .fetch_add(u64::from(rotated_count), Ordering::Relaxed);
            info!(rotated_count, "Certificate rotation cycle completed successfully");
        }
        Ok(RotationResult::AlreadyInProgress) => {
            // Not counted as failure — a prior cycle is still running.
            warn!("Skipping rotation cycle: previous rotation still in progress");
        }
        Ok(RotationResult::Error { reason }) => {
            metrics.rotations_failed.fetch_add(1, Ordering::Relaxed);
            error!(reason = %reason, "Certificate rotation cycle failed");
        }
        Err(e) => {
            metrics.rotations_failed.fetch_add(1, Ordering::Relaxed);
            error!(error = %e, "Certificate rotation cycle encountered an error");
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    // ---- Config defaults ----

    #[test]
    fn test_config_defaults() {
        let cfg = RotationSchedulerConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.check_interval, Duration::from_secs(3600));
        assert_eq!(cfg.renewal_threshold, Duration::from_secs(21600));
        assert_eq!(cfg.max_concurrent_rotations, 10);
    }

    // ---- Metrics initialisation ----

    #[test]
    fn test_metrics_initial_values() {
        let m = RotationSchedulerMetrics::default();
        let snap = m.snapshot();
        assert_eq!(snap, RotationMetricsSnapshot::default());
        assert_eq!(snap.rotations_attempted, 0);
        assert_eq!(snap.rotations_succeeded, 0);
        assert_eq!(snap.rotations_failed, 0);
        assert_eq!(snap.certificates_rotated, 0);
    }

    #[test]
    fn test_metrics_snapshot_reflects_updates() {
        let m = RotationSchedulerMetrics::default();
        m.rotations_attempted.fetch_add(5, Ordering::Relaxed);
        m.rotations_succeeded.fetch_add(3, Ordering::Relaxed);
        m.rotations_failed.fetch_add(2, Ordering::Relaxed);
        m.certificates_rotated.fetch_add(12, Ordering::Relaxed);

        let snap = m.snapshot();
        assert_eq!(snap.rotations_attempted, 5);
        assert_eq!(snap.rotations_succeeded, 3);
        assert_eq!(snap.rotations_failed, 2);
        assert_eq!(snap.certificates_rotated, 12);
    }

    // ---- Scheduler lifecycle ----

    #[tokio::test]
    async fn test_scheduler_starts_and_stops_cleanly() {
        let rm = Arc::new(
            CertificateRotationManager::new()
                .await
                .expect("test: failed to create rotation manager"),
        );
        let cs = Arc::new(
            CertificateStore::new()
                .await
                .expect("test: failed to create certificate store"),
        );

        let cfg = RotationSchedulerConfig {
            enabled: true,
            check_interval: Duration::from_millis(50),
            ..Default::default()
        };

        let mut scheduler = CertificateRotationScheduler::new(cfg, rm, cs);
        assert!(!scheduler.is_running());

        scheduler.start();
        assert!(scheduler.is_running());

        // Let it run for a couple of ticks.
        tokio::time::sleep(Duration::from_millis(160)).await;
        assert!(scheduler.is_running());

        scheduler.stop().await;
        assert!(!scheduler.is_running());
    }

    #[tokio::test]
    async fn test_scheduler_respects_cancellation_token() {
        let rm = Arc::new(
            CertificateRotationManager::new()
                .await
                .expect("test: failed to create rotation manager"),
        );
        let cs = Arc::new(
            CertificateStore::new()
                .await
                .expect("test: failed to create certificate store"),
        );

        let cfg = RotationSchedulerConfig {
            enabled: true,
            check_interval: Duration::from_secs(3600), // long interval
            ..Default::default()
        };

        let mut scheduler = CertificateRotationScheduler::new(cfg, rm, cs);
        scheduler.start();
        assert!(scheduler.is_running());

        // Stop should return promptly even though the interval is 1 hour.
        let stop_start = tokio::time::Instant::now();
        scheduler.stop().await;
        let stop_duration = stop_start.elapsed();

        assert!(!scheduler.is_running());
        assert!(
            stop_duration < Duration::from_secs(2),
            "Stop took too long: {:?}",
            stop_duration
        );
    }

    #[tokio::test]
    async fn test_scheduler_disabled_does_not_start() {
        let rm = Arc::new(
            CertificateRotationManager::new()
                .await
                .expect("test: failed to create rotation manager"),
        );
        let cs = Arc::new(
            CertificateStore::new()
                .await
                .expect("test: failed to create certificate store"),
        );

        let cfg = RotationSchedulerConfig {
            enabled: false,
            ..Default::default()
        };

        let mut scheduler = CertificateRotationScheduler::new(cfg, rm, cs);
        scheduler.start();
        assert!(!scheduler.is_running());
    }

    #[tokio::test]
    async fn test_scheduler_metrics_increment_on_cycle() {
        let rm = Arc::new(
            CertificateRotationManager::new()
                .await
                .expect("test: failed to create rotation manager"),
        );
        let cs = Arc::new(
            CertificateStore::new()
                .await
                .expect("test: failed to create certificate store"),
        );

        let cfg = RotationSchedulerConfig {
            enabled: true,
            check_interval: Duration::from_millis(30),
            ..Default::default()
        };

        let mut scheduler = CertificateRotationScheduler::new(cfg, rm, cs);
        scheduler.start();

        // Allow a few cycles to execute.
        tokio::time::sleep(Duration::from_millis(200)).await;

        let snap = scheduler.metrics().snapshot();
        assert!(
            snap.rotations_attempted > 0,
            "Expected at least one rotation attempt, got {}",
            snap.rotations_attempted,
        );
        assert!(
            snap.rotations_succeeded > 0,
            "Expected at least one successful rotation, got {}",
            snap.rotations_succeeded,
        );
        assert_eq!(snap.rotations_failed, 0);

        scheduler.stop().await;
    }

    #[tokio::test]
    async fn test_double_start_is_noop() {
        let rm = Arc::new(
            CertificateRotationManager::new()
                .await
                .expect("test: failed to create rotation manager"),
        );
        let cs = Arc::new(
            CertificateStore::new()
                .await
                .expect("test: failed to create certificate store"),
        );

        let cfg = RotationSchedulerConfig {
            enabled: true,
            check_interval: Duration::from_secs(3600),
            ..Default::default()
        };

        let mut scheduler = CertificateRotationScheduler::new(cfg, rm, cs);
        scheduler.start();
        assert!(scheduler.is_running());

        // Second start should be a no-op (no panic, no second task).
        scheduler.start();
        assert!(scheduler.is_running());

        scheduler.stop().await;
    }
}
