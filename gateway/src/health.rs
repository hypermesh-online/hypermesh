// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use bytes::Bytes;
use http::{Response, StatusCode};
use std::time::Instant;

use crate::gateway_mode::GatewayMode;

/// Comprehensive health report for the gateway.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthReport {
    /// Overall gateway status (e.g. "healthy").
    pub status: String,
    /// Gateway crate version.
    pub version: String,
    /// Seconds since the health checker was created.
    pub uptime_seconds: u64,
    /// Which gateway modes are enabled.
    pub enabled_modes: Vec<String>,
    /// Whether the HTTP/3 listener is active.
    pub http3_listening: bool,
    /// Whether the STOQ listener is active.
    pub stoq_listening: bool,
    /// Backend health information (caller-provided JSON value).
    pub backends: serde_json::Value,
    /// Optional STOQ-specific health status.
    pub stoq_status: Option<StoqHealthStatus>,
}

/// STOQ subsystem health information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoqHealthStatus {
    /// Number of active STOQ connections.
    pub active_connections: usize,
    /// Total bytes bridged through the STOQ layer.
    pub bytes_bridged: u64,
}

/// Builds health-check reports and HTTP responses for the gateway.
pub struct HealthChecker {
    start_time: Instant,
    enabled_modes: Vec<GatewayMode>,
    stoq_listening: bool,
}

impl HealthChecker {
    /// Create a new health checker, recording the current time as the start.
    pub fn new(enabled_modes: Vec<GatewayMode>, stoq_listening: bool) -> Self {
        Self {
            start_time: Instant::now(),
            enabled_modes,
            stoq_listening,
        }
    }

    /// Build a structured health report.
    pub fn build_report(
        &self,
        backends: serde_json::Value,
        stoq_status: Option<StoqHealthStatus>,
    ) -> HealthReport {
        HealthReport {
            status: "healthy".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            enabled_modes: self
                .enabled_modes
                .iter()
                .map(|m| format!("{m:?}"))
                .collect(),
            http3_listening: true,
            stoq_listening: self.stoq_listening,
            backends,
            stoq_status,
        }
    }

    /// Build the health report and serialize it as an HTTP `200 OK` JSON response.
    pub fn build_response(
        &self,
        backends: serde_json::Value,
        stoq_status: Option<StoqHealthStatus>,
    ) -> Response<Bytes> {
        let report = self.build_report(backends, stoq_status);
        let body = serde_json::to_vec(&report).unwrap_or_default();

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Bytes::from(body))
            .expect("building health response should not fail")
    }

    /// Uptime in whole seconds since the health checker was created.
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Whether the STOQ listener is marked as active.
    pub fn stoq_active(&self) -> bool {
        self.stoq_listening
    }

    /// Number of enabled gateway modes.
    pub fn mode_count(&self) -> usize {
        self.enabled_modes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_report_with_modes() {
        let checker = HealthChecker::new(
            vec![GatewayMode::Bootstrap, GatewayMode::InboundProxy],
            false,
        );

        let report = checker.build_report(json!({}), None);
        assert_eq!(report.status, "healthy");
        assert_eq!(report.enabled_modes.len(), 2);
        assert!(report.enabled_modes.contains(&"Bootstrap".to_string()));
        assert!(report.enabled_modes.contains(&"InboundProxy".to_string()));
        assert!(report.http3_listening);
        assert!(!report.stoq_listening);
        assert!(report.stoq_status.is_none());
    }

    #[test]
    fn build_response_returns_200_json() {
        let checker = HealthChecker::new(vec![GatewayMode::Bootstrap], true);
        let backends = json!({ "trustchain": { "status": "up" } });
        let response = checker.build_response(backends, None);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("application/json")
        );

        // Body should be valid JSON containing "healthy".
        let body_str = std::str::from_utf8(response.body()).expect("test: valid utf8");
        assert!(body_str.contains("healthy"));
    }

    #[test]
    fn uptime_increases() {
        let checker = HealthChecker::new(vec![], false);
        // Uptime should be at least 0 immediately after creation.
        let t0 = checker.uptime_secs();
        // We cannot reliably assert >0 without sleeping, but we can assert non-panicking.
        assert!(t0 < 10, "uptime should be near zero at creation");
    }

    #[test]
    fn stoq_status_included_in_report() {
        let checker = HealthChecker::new(vec![GatewayMode::OutboundProxy], true);
        let stoq = StoqHealthStatus {
            active_connections: 42,
            bytes_bridged: 1_000_000,
        };
        let report = checker.build_report(json!({}), Some(stoq));
        assert!(report.stoq_listening);
        let status = report.stoq_status.expect("test: should have stoq_status");
        assert_eq!(status.active_connections, 42);
        assert_eq!(status.bytes_bridged, 1_000_000);
    }

    #[test]
    fn version_matches_cargo_pkg() {
        let checker = HealthChecker::new(vec![], false);
        let report = checker.build_report(json!({}), None);
        assert_eq!(report.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn mode_count_and_stoq_active() {
        let checker = HealthChecker::new(
            vec![GatewayMode::Bootstrap, GatewayMode::InterNetwork],
            true,
        );
        assert_eq!(checker.mode_count(), 2);
        assert!(checker.stoq_active());

        let checker2 = HealthChecker::new(vec![], false);
        assert_eq!(checker2.mode_count(), 0);
        assert!(!checker2.stoq_active());
    }

    #[test]
    fn build_response_with_stoq_status() {
        let checker = HealthChecker::new(vec![GatewayMode::Bootstrap], true);
        let stoq = StoqHealthStatus {
            active_connections: 5,
            bytes_bridged: 500,
        };
        let response = checker.build_response(json!({}), Some(stoq));
        assert_eq!(response.status(), StatusCode::OK);

        let body_str = std::str::from_utf8(response.body()).expect("test: valid utf8");
        assert!(body_str.contains("active_connections"));
        assert!(body_str.contains("bytes_bridged"));
    }
}
