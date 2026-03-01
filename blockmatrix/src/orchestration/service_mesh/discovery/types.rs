// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Type definitions for CPE-enhanced service discovery

use super::super::ServiceEndpoint;
use crate::ServiceId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};
use uuid::Uuid;

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceHealth {
    /// Service is healthy and available
    Healthy,
    /// Service is degraded but operational
    Degraded,
    /// Service has warnings but functional
    Warning,
    /// Service is unhealthy
    Unhealthy,
    /// Service status unknown
    Unknown,
}

/// Service discovery event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryEvent {
    /// Event ID
    pub id: Uuid,
    /// Event type
    pub event_type: DiscoveryEventType,
    /// Service affected
    pub service_id: ServiceId,
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Event details
    pub details: HashMap<String, String>,
    /// Whether event was predicted by CPE
    pub cpe_predicted: bool,
}

/// Types of discovery events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryEventType {
    /// Service registered
    ServiceRegistered,
    /// Service deregistered
    ServiceDeregistered,
    /// Endpoint added
    EndpointAdded,
    /// Endpoint removed
    EndpointRemoved,
    /// Health status changed
    HealthChanged,
    /// Service migrated
    ServiceMigrated,
    /// Load balancing updated
    LoadBalancingUpdated,
}

/// Health monitor for service
#[derive(Debug, Clone)]
pub struct HealthMonitor {
    /// Service being monitored
    pub service_id: ServiceId,
    /// Health check interval
    pub check_interval: Duration,
    /// Last health check
    pub last_check: SystemTime,
    /// Health check results history
    pub health_history: Vec<HealthCheckResult>,
    /// Predicted health trends
    pub health_predictions: Vec<HealthPrediction>,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Check timestamp
    pub timestamp: SystemTime,
    /// Health status
    pub status: ServiceHealth,
    /// Response time (ms)
    pub response_time_ms: f64,
    /// Error details if any
    pub error: Option<String>,
    /// Additional metrics
    pub metrics: HashMap<String, f64>,
}

/// Health prediction from CPE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthPrediction {
    /// Prediction timestamp
    pub timestamp: SystemTime,
    /// Predicted health status
    pub predicted_health: ServiceHealth,
    /// Confidence in prediction
    pub confidence: f64,
    /// Time horizon for prediction
    pub horizon_seconds: u64,
    /// Contributing factors
    pub factors: Vec<String>,
}

/// Registry metadata
#[derive(Debug, Clone)]
pub struct RegistryMetadata {
    /// Total services registered
    pub total_services: usize,
    /// Total endpoints
    pub total_endpoints: usize,
    /// Registry creation time
    pub created_at: SystemTime,
    /// Last update time
    pub last_updated: SystemTime,
}

/// Cached discovery result
#[derive(Debug, Clone)]
pub struct CachedDiscovery {
    /// Discovered endpoints
    pub endpoints: Vec<ServiceEndpoint>,
    /// Cache timestamp
    pub cached_at: Instant,
    /// Cache TTL
    pub ttl: Duration,
    /// Access count
    pub access_count: u32,
    /// Whether result was CPE-enhanced
    pub cpe_enhanced: bool,
}

/// Service prediction from CPE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePrediction {
    /// Service being predicted
    pub service_id: ServiceId,
    /// Predicted load patterns
    pub load_predictions: Vec<LoadPrediction>,
    /// Predicted health events
    pub health_predictions: Vec<HealthPrediction>,
    /// Predicted scaling needs
    pub scaling_predictions: Vec<ScalingPrediction>,
    /// Prediction confidence
    pub confidence: f64,
    /// Last updated
    pub last_updated: SystemTime,
}

/// Load prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadPrediction {
    /// Prediction timestamp
    pub timestamp: SystemTime,
    /// Predicted request rate
    pub predicted_request_rate: f64,
    /// Predicted response time
    pub predicted_response_time_ms: f64,
    /// Predicted error rate
    pub predicted_error_rate: f64,
    /// Confidence in prediction
    pub confidence: f64,
}

/// Scaling prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPrediction {
    /// Prediction timestamp
    pub timestamp: SystemTime,
    /// Predicted scaling action
    pub action: ScalingAction,
    /// Predicted instance count
    pub predicted_instances: u32,
    /// Trigger conditions
    pub trigger_conditions: Vec<String>,
    /// Confidence in prediction
    pub confidence: f64,
}

/// Scaling actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingAction {
    /// Scale up
    ScaleUp,
    /// Scale down
    ScaleDown,
    /// No scaling needed
    NoAction,
}

/// Discovery statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryStats {
    /// Total discovery operations
    pub total_discoveries: u64,
    /// CPE-enhanced discoveries
    pub cpe_enhanced_discoveries: u64,
    /// Average discovery latency (us)
    pub avg_discovery_latency_us: f64,
    /// IFR lookup percentage
    pub ifr_lookup_percentage: f64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Prediction accuracy
    pub prediction_accuracy: f64,
    /// Health check success rate
    pub health_check_success_rate: f64,
}

/// Service entry in registry
#[derive(Debug, Clone)]
pub struct ServiceEntry {
    /// Service identifier
    pub service_id: ServiceId,
    /// Available endpoints
    pub endpoints: Vec<ServiceEndpoint>,
    /// Service metadata
    pub metadata: HashMap<String, String>,
    /// Service health status
    pub health: ServiceHealth,
    /// Discovery events history
    pub events: Vec<DiscoveryEvent>,
    /// Last updated timestamp
    pub last_updated: SystemTime,
}
