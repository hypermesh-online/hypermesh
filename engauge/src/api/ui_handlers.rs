// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! UI-facing Engauge STOQ API handlers.
//!
//! Handlers for endpoints expected by the UI TypeScript interfaces in
//! `ui/frontend/lib/api/services/EngaugeAPI.ts` that go beyond the core
//! analytics handlers (health, metrics, capacity, traffic, swarm).
//!
//! Includes: trending, throttle, routing advisory, marketplace (pools,
//! leases, pricing), metrics stream, and lease creation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use super::stoq_api::{
    ApiError, ApiHandler, ApiRequest, ApiResponse, CreateLeaseRequest, EngaugeAppState,
    LeaseContractEntry, MetricsFrameEntry, PricingInfoEntry, ResourcePoolEntry,
    RoutingAdvisoryResponse, ThrottleStatusResponse, TrendingMetricEntry,
};

// ---------------------------------------------------------------------------
// Trending handler
// ---------------------------------------------------------------------------

pub struct TrendingHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for TrendingHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/trending: {}", request.id);

        let active = self
            .state
            .active_nodes
            .load(std::sync::atomic::Ordering::Relaxed);

        let metrics = vec![
            TrendingMetricEntry {
                metric_name: "active_nodes".to_string(),
                current_value: active as f64,
                previous_value: active as f64,
                trend_direction: "stable".to_string(),
                change_percent: 0.0,
            },
            TrendingMetricEntry {
                metric_name: "bytes_served".to_string(),
                current_value: 0.0,
                previous_value: 0.0,
                trend_direction: "stable".to_string(),
                change_percent: 0.0,
            },
        ];

        serialize_response(&request.id, &metrics)
    }

    fn path(&self) -> &str {
        "engauge/trending"
    }
}

// ---------------------------------------------------------------------------
// Throttle handler
// ---------------------------------------------------------------------------

pub struct ThrottleHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for ThrottleHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/throttle: {}", request.id);

        let response = ThrottleStatusResponse {
            governor_signal: 0.0,
            is_throttled: false,
            reason: None,
        };

        serialize_response(&request.id, &response)
    }

    fn path(&self) -> &str {
        "engauge/throttle"
    }
}

// ---------------------------------------------------------------------------
// Routing advisory handler
// ---------------------------------------------------------------------------

pub struct RoutingAdvisoryHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for RoutingAdvisoryHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/routing_advisory: {}", request.id);

        let now = now_unix();

        let response = RoutingAdvisoryResponse {
            tensor_weight_modifier: 1.0,
            path_policy: "default".to_string(),
            congestion_forecast: 0.0,
            recommended_tier: "L0".to_string(),
            alternate_paths: 0,
            last_updated: now,
        };

        serialize_response(&request.id, &response)
    }

    fn path(&self) -> &str {
        "engauge/routing_advisory"
    }
}

// ---------------------------------------------------------------------------
// Marketplace handlers
// ---------------------------------------------------------------------------

pub struct MarketplacePoolsHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for MarketplacePoolsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/marketplace_pools: {}", request.id);

        let pools: Vec<ResourcePoolEntry> = Vec::new();
        serialize_response(&request.id, &pools)
    }

    fn path(&self) -> &str {
        "engauge/marketplace_pools"
    }
}

pub struct MarketplaceLeasesHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for MarketplaceLeasesHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/marketplace_leases: {}", request.id);

        let leases: Vec<LeaseContractEntry> = Vec::new();
        serialize_response(&request.id, &leases)
    }

    fn path(&self) -> &str {
        "engauge/marketplace_leases"
    }
}

pub struct MarketplacePricingHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for MarketplacePricingHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/marketplace_pricing: {}", request.id);

        let pricing = vec![
            PricingInfoEntry {
                tier: "L0".to_string(),
                multiplier: 1.0,
                base_price: 0.001,
                effective_price: 0.001,
            },
            PricingInfoEntry {
                tier: "L1".to_string(),
                multiplier: 0.8,
                base_price: 0.001,
                effective_price: 0.0008,
            },
            PricingInfoEntry {
                tier: "L2".to_string(),
                multiplier: 0.5,
                base_price: 0.001,
                effective_price: 0.0005,
            },
            PricingInfoEntry {
                tier: "L3".to_string(),
                multiplier: 0.2,
                base_price: 0.001,
                effective_price: 0.0002,
            },
        ];

        serialize_response(&request.id, &pricing)
    }

    fn path(&self) -> &str {
        "engauge/marketplace_pricing"
    }
}

// ---------------------------------------------------------------------------
// Metrics stream handler
// ---------------------------------------------------------------------------

pub struct MetricsStreamHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for MetricsStreamHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/metrics_stream: {}", request.id);

        let frames: Vec<MetricsFrameEntry> = Vec::new();
        serialize_response(&request.id, &frames)
    }

    fn path(&self) -> &str {
        "engauge/metrics_stream"
    }
}

// ---------------------------------------------------------------------------
// Create lease handler
// ---------------------------------------------------------------------------

pub struct CreateLeaseHandler {
    pub state: Arc<EngaugeAppState>,
}

#[async_trait]
impl ApiHandler for CreateLeaseHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling engauge/create_lease: {}", request.id);

        let req: CreateLeaseRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid create_lease request: {e}")))?;

        let now = now_unix();
        let expires_at = now + req.duration_seconds.unwrap_or(3600);

        let lease = LeaseContractEntry {
            lease_id: format!("lease-{}", uuid::Uuid::new_v4()),
            pool_id: req.pool_id,
            state: "Proposed".to_string(),
            units: req.units,
            cost_gg: 0.0,
            lessee: String::new(),
            created_at: now,
            expires_at,
        };

        serialize_response(&request.id, &lease)
    }

    fn path(&self) -> &str {
        "engauge/create_lease"
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn serialize_response<T: Serialize>(
    request_id: &str,
    body: &T,
) -> Result<ApiResponse, ApiError> {
    let payload = serde_json::to_vec(body)
        .map_err(|e| ApiError::SerializationError(e.to_string()))?;

    Ok(ApiResponse {
        request_id: request_id.to_string(),
        success: true,
        payload: payload.into(),
        error: None,
        metadata: HashMap::new(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn empty_request(id: &str) -> ApiRequest {
        ApiRequest {
            id: id.to_string(),
            service: "engauge".to_string(),
            method: String::new(),
            payload: Bytes::from("{}"),
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_throttle_handler() {
        let state = Arc::new(EngaugeAppState::new());
        let handler = ThrottleHandler { state };

        let resp = handler
            .handle(empty_request("test-throttle-1"))
            .await
            .expect("test: throttle handler should succeed");
        assert!(resp.success);

        let body: ThrottleStatusResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert!(!body.is_throttled);
        assert!(body.reason.is_none());
    }

    #[tokio::test]
    async fn test_trending_handler_returns_array() {
        let state = Arc::new(EngaugeAppState::new());
        state.set_active_nodes(3);
        let handler = TrendingHandler { state };

        let resp = handler
            .handle(empty_request("test-trending-1"))
            .await
            .expect("test: trending handler should succeed");
        assert!(resp.success);

        let body: Vec<TrendingMetricEntry> =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert!(!body.is_empty());
        assert_eq!(body[0].metric_name, "active_nodes");
    }

    #[tokio::test]
    async fn test_marketplace_pricing_handler() {
        let state = Arc::new(EngaugeAppState::new());
        let handler = MarketplacePricingHandler { state };

        let resp = handler
            .handle(empty_request("test-pricing-1"))
            .await
            .expect("test: pricing handler should succeed");
        assert!(resp.success);

        let body: Vec<PricingInfoEntry> =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.len(), 4);
        assert_eq!(body[0].tier, "L0");
    }

    #[tokio::test]
    async fn test_routing_advisory_handler() {
        let state = Arc::new(EngaugeAppState::new());
        let handler = RoutingAdvisoryHandler { state };

        let resp = handler
            .handle(empty_request("test-routing-1"))
            .await
            .expect("test: routing handler should succeed");
        assert!(resp.success);

        let body: RoutingAdvisoryResponse =
            serde_json::from_slice(&resp.payload).expect("test: deserialize");
        assert_eq!(body.recommended_tier, "L0");
    }

    #[test]
    fn test_handler_paths() {
        let state = Arc::new(EngaugeAppState::new());

        assert_eq!(
            TrendingHandler { state: state.clone() }.path(),
            "engauge/trending"
        );
        assert_eq!(
            ThrottleHandler { state: state.clone() }.path(),
            "engauge/throttle"
        );
        assert_eq!(
            RoutingAdvisoryHandler { state: state.clone() }.path(),
            "engauge/routing_advisory"
        );
        assert_eq!(
            MarketplacePoolsHandler { state: state.clone() }.path(),
            "engauge/marketplace_pools"
        );
        assert_eq!(
            MarketplaceLeasesHandler { state: state.clone() }.path(),
            "engauge/marketplace_leases"
        );
        assert_eq!(
            MarketplacePricingHandler { state: state.clone() }.path(),
            "engauge/marketplace_pricing"
        );
        assert_eq!(
            MetricsStreamHandler { state: state.clone() }.path(),
            "engauge/metrics_stream"
        );
        assert_eq!(
            CreateLeaseHandler { state }.path(),
            "engauge/create_lease"
        );
    }
}
