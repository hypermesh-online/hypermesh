//! Integration Layer Metrics
//!
//! This module provides metrics collection and reporting for the integration layer,
//! tracking API calls, service health, bootstrap progress, and system performance.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Core metrics for the integration layer
#[derive(Debug, Clone)]
pub struct IntegrationMetrics {
    inner: Arc<MetricsInner>,
}

#[derive(Debug)]
struct MetricsInner {
    // API Bridge Metrics
    api_requests: AtomicU64,
    api_errors: AtomicU64,
    api_latency_sum: AtomicU64,
    api_latency_count: AtomicU64,

    // Bootstrap Metrics
    bootstrap_phase: AtomicUsize,
    bootstrap_duration: RwLock<Option<Duration>>,
    component_statuses: RwLock<HashMap<String, ComponentMetrics>>,

    // Service Registry Metrics
    services_registered: AtomicUsize,
    services_active: AtomicUsize,
    service_lookups: AtomicU64,

    // STOQ Bridge Metrics
    stoq_connections: AtomicUsize,
    stoq_messages_sent: AtomicU64,
    stoq_messages_received: AtomicU64,
    stoq_bytes_sent: AtomicU64,
    stoq_bytes_received: AtomicU64,

    // Coordinator Metrics
    coordinator_tasks: AtomicUsize,
    coordinator_errors: AtomicU64,

    // System Health
    start_time: Instant,
    last_health_check: RwLock<Instant>,
}

#[derive(Debug, Clone)]
struct ComponentMetrics {
    status: String,
    start_time: Instant,
    ready_time: Option<Instant>,
    error_count: u64,
}

impl IntegrationMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                api_requests: AtomicU64::new(0),
                api_errors: AtomicU64::new(0),
                api_latency_sum: AtomicU64::new(0),
                api_latency_count: AtomicU64::new(0),
                bootstrap_phase: AtomicUsize::new(0),
                bootstrap_duration: RwLock::new(None),
                component_statuses: RwLock::new(HashMap::new()),
                services_registered: AtomicUsize::new(0),
                services_active: AtomicUsize::new(0),
                service_lookups: AtomicU64::new(0),
                stoq_connections: AtomicUsize::new(0),
                stoq_messages_sent: AtomicU64::new(0),
                stoq_messages_received: AtomicU64::new(0),
                stoq_bytes_sent: AtomicU64::new(0),
                stoq_bytes_received: AtomicU64::new(0),
                coordinator_tasks: AtomicUsize::new(0),
                coordinator_errors: AtomicU64::new(0),
                start_time: Instant::now(),
                last_health_check: RwLock::new(Instant::now()),
            }),
        }
    }

    /// Record an API request
    pub fn record_api_request(&self, duration: Duration, success: bool) {
        self.inner.api_requests.fetch_add(1, Ordering::Relaxed);

        if !success {
            self.inner.api_errors.fetch_add(1, Ordering::Relaxed);
        }

        let micros = duration.as_micros() as u64;
        self.inner.api_latency_sum.fetch_add(micros, Ordering::Relaxed);
        self.inner.api_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Update bootstrap phase
    pub fn set_bootstrap_phase(&self, phase: usize) {
        self.inner.bootstrap_phase.store(phase, Ordering::Relaxed);
    }

    /// Record bootstrap completion
    pub async fn record_bootstrap_complete(&self, duration: Duration) {
        let mut dur = self.inner.bootstrap_duration.write().await;
        *dur = Some(duration);
    }

    /// Update component status
    pub async fn update_component_status(&self, name: String, status: String) {
        let mut statuses = self.inner.component_statuses.write().await;

        let component = statuses.entry(name).or_insert_with(|| ComponentMetrics {
            status: status.clone(),
            start_time: Instant::now(),
            ready_time: None,
            error_count: 0,
        });

        component.status = status.clone();

        if status == "ready" && component.ready_time.is_none() {
            component.ready_time = Some(Instant::now());
        }
    }

    /// Record service registration
    pub fn record_service_registration(&self) {
        self.inner.services_registered.fetch_add(1, Ordering::Relaxed);
        self.inner.services_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record service deregistration
    pub fn record_service_deregistration(&self) {
        self.inner.services_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record service lookup
    pub fn record_service_lookup(&self) {
        self.inner.service_lookups.fetch_add(1, Ordering::Relaxed);
    }

    /// Update STOQ connection count
    pub fn update_stoq_connections(&self, count: usize) {
        self.inner.stoq_connections.store(count, Ordering::Relaxed);
    }

    /// Record STOQ message sent
    pub fn record_stoq_send(&self, bytes: usize) {
        self.inner.stoq_messages_sent.fetch_add(1, Ordering::Relaxed);
        self.inner.stoq_bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Record STOQ message received
    pub fn record_stoq_receive(&self, bytes: usize) {
        self.inner.stoq_messages_received.fetch_add(1, Ordering::Relaxed);
        self.inner.stoq_bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Update coordinator task count
    pub fn update_coordinator_tasks(&self, count: usize) {
        self.inner.coordinator_tasks.store(count, Ordering::Relaxed);
    }

    /// Record coordinator error
    pub fn record_coordinator_error(&self) {
        self.inner.coordinator_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub async fn get_snapshot(&self) -> MetricsSnapshot {
        let uptime = self.inner.start_time.elapsed();

        let api_latency_avg = {
            let sum = self.inner.api_latency_sum.load(Ordering::Relaxed);
            let count = self.inner.api_latency_count.load(Ordering::Relaxed);
            if count > 0 {
                Duration::from_micros(sum / count)
            } else {
                Duration::ZERO
            }
        };

        let bootstrap_duration = *self.inner.bootstrap_duration.read().await;
        let component_count = self.inner.component_statuses.read().await.len();

        MetricsSnapshot {
            uptime,
            api_requests: self.inner.api_requests.load(Ordering::Relaxed),
            api_errors: self.inner.api_errors.load(Ordering::Relaxed),
            api_latency_avg,
            bootstrap_phase: self.inner.bootstrap_phase.load(Ordering::Relaxed),
            bootstrap_duration,
            component_count,
            services_registered: self.inner.services_registered.load(Ordering::Relaxed),
            services_active: self.inner.services_active.load(Ordering::Relaxed),
            service_lookups: self.inner.service_lookups.load(Ordering::Relaxed),
            stoq_connections: self.inner.stoq_connections.load(Ordering::Relaxed),
            stoq_messages_sent: self.inner.stoq_messages_sent.load(Ordering::Relaxed),
            stoq_messages_received: self.inner.stoq_messages_received.load(Ordering::Relaxed),
            stoq_bytes_sent: self.inner.stoq_bytes_sent.load(Ordering::Relaxed),
            stoq_bytes_received: self.inner.stoq_bytes_received.load(Ordering::Relaxed),
            coordinator_tasks: self.inner.coordinator_tasks.load(Ordering::Relaxed),
            coordinator_errors: self.inner.coordinator_errors.load(Ordering::Relaxed),
        }
    }

    /// Update health check timestamp
    pub async fn update_health_check(&self) {
        let mut last_check = self.inner.last_health_check.write().await;
        *last_check = Instant::now();
    }

    /// Get time since last health check
    pub async fn time_since_health_check(&self) -> Duration {
        self.inner.last_health_check.read().await.elapsed()
    }
}

impl Default for IntegrationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of current metrics
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub uptime: Duration,

    // API Metrics
    pub api_requests: u64,
    pub api_errors: u64,
    pub api_latency_avg: Duration,

    // Bootstrap Metrics
    pub bootstrap_phase: usize,
    pub bootstrap_duration: Option<Duration>,
    pub component_count: usize,

    // Service Metrics
    pub services_registered: usize,
    pub services_active: usize,
    pub service_lookups: u64,

    // STOQ Metrics
    pub stoq_connections: usize,
    pub stoq_messages_sent: u64,
    pub stoq_messages_received: u64,
    pub stoq_bytes_sent: u64,
    pub stoq_bytes_received: u64,

    // Coordinator Metrics
    pub coordinator_tasks: usize,
    pub coordinator_errors: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_creation() {
        let metrics = IntegrationMetrics::new();
        let snapshot = metrics.get_snapshot().await;

        assert_eq!(snapshot.api_requests, 0);
        assert_eq!(snapshot.api_errors, 0);
        assert_eq!(snapshot.services_active, 0);
    }

    #[tokio::test]
    async fn test_api_metrics() {
        let metrics = IntegrationMetrics::new();

        metrics.record_api_request(Duration::from_millis(10), true);
        metrics.record_api_request(Duration::from_millis(20), false);

        let snapshot = metrics.get_snapshot().await;
        assert_eq!(snapshot.api_requests, 2);
        assert_eq!(snapshot.api_errors, 1);
        assert!(snapshot.api_latency_avg.as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_service_metrics() {
        let metrics = IntegrationMetrics::new();

        metrics.record_service_registration();
        metrics.record_service_registration();
        metrics.record_service_deregistration();
        metrics.record_service_lookup();

        let snapshot = metrics.get_snapshot().await;
        assert_eq!(snapshot.services_registered, 2);
        assert_eq!(snapshot.services_active, 1);
        assert_eq!(snapshot.service_lookups, 1);
    }
}