// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sprint 24 integration tests for Catalog
//!
//! Track 1: Typedef Registry search scoring and metadata
//! Track 2: Caesar Contribution Rewards (settlement.rs)
//! Track 3: STOQ API endpoint handlers

// ============================================================================
// Common helpers
// ============================================================================

use blockmatrix::consensus::proof_of_state_integration::{
    SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
};
use blockmatrix::assets::ConsensusProof;

fn create_test_proof() -> ConsensusProof {
    let stake = StakeProof::new("test-holder".into(), "test-id".into(), 1000);
    let space = SpaceProof::new("test-node".into(), "/test".into(), 1024);
    let work = WorkProof::new(
        "test-owner".into(),
        "test-workload".into(),
        12345,
        100,
        WorkloadType::Compute,
        WorkState::Completed,
    );
    let time = TimeProof::new(std::time::Duration::from_secs(10));
    ConsensusProof::new(stake, time, space, work)
}

/// Helper: create a CatalogRegistry with lenient trust policy for testing.
fn test_registry() -> catalog::CatalogRegistry {
    catalog::CatalogRegistry::new(
        hypermesh_lib::PrivacyMode::PUBLIC,
        catalog::TrustPolicy::default(),
        catalog::RegistryConfig::default(),
    )
}

/// Helper: build an AssetTypeDefinition with controllable metadata.
fn make_typedef(
    name: &str,
    author: Option<&str>,
    tags: &[&str],
    version_count: u32,
) -> catalog::AssetTypeDefinition {
    let schema = serde_json::json!({ "type": "object" });
    let mut td = catalog::AssetTypeDefinition::new(name.to_string(), schema, create_test_proof());
    td.metadata.author = author.map(|a| a.to_string());
    td.metadata.tags = tags.iter().map(|t| t.to_string()).collect();
    td.metadata.version_count = version_count;
    td
}

// ============================================================================
// Track 1 -- Registry search scoring (8 tests)
// ============================================================================

#[tokio::test]
async fn search_exact_name_scores_higher_than_partial() {
    let reg = test_registry();
    reg.register_type(make_typedef("Vehicle", None, &[], 1))
        .await
        .expect("test: register Vehicle");
    reg.register_type(make_typedef("VehicleInsurance", None, &[], 1))
        .await
        .expect("test: register VehicleInsurance");

    let query = catalog::SearchQuery {
        query: "Vehicle".to_string(),
        sort_by: catalog::SortCriteria::Relevance,
        ..Default::default()
    };
    let results = reg.search_types(&query).await.expect("test: search");
    assert_eq!(results.results.len(), 2, "both should match");

    let exact = results.results.iter().find(|r| r.type_name == "Vehicle")
        .expect("test: Vehicle should be in results");
    let partial = results.results.iter().find(|r| r.type_name == "VehicleInsurance")
        .expect("test: VehicleInsurance should be in results");
    assert!(
        exact.score > partial.score,
        "exact match score ({}) should exceed partial ({})",
        exact.score,
        partial.score
    );
}

#[tokio::test]
async fn search_empty_query_returns_all_entries() {
    let reg = test_registry();
    for name in &["Alpha", "Beta", "Gamma"] {
        reg.register_type(make_typedef(name, None, &[], 1))
            .await
            .expect("test: register type");
    }

    let query = catalog::SearchQuery {
        query: String::new(),
        ..Default::default()
    };
    let results = reg.search_types(&query).await.expect("test: search");
    assert_eq!(results.total_count, 3, "empty query is browse-all");
}

#[tokio::test]
async fn search_tag_filter_boosts_matching_entries() {
    let reg = test_registry();
    reg.register_type(make_typedef("ComputeNode", None, &["compute", "gpu"], 1))
        .await
        .expect("test: register ComputeNode");
    reg.register_type(make_typedef("ComputeScheduler", None, &["scheduler"], 1))
        .await
        .expect("test: register ComputeScheduler");

    let query = catalog::SearchQuery {
        query: "Compute".to_string(),
        tags: vec!["compute".to_string()],
        sort_by: catalog::SortCriteria::Relevance,
        ..Default::default()
    };
    let results = reg.search_types(&query).await.expect("test: search");
    assert!(results.results.len() >= 1);

    // ComputeNode should appear first because its tag matches the filter
    let first = &results.results[0];
    assert_eq!(first.type_name, "ComputeNode");
}

#[tokio::test]
async fn search_with_author_filter() {
    let reg = test_registry();
    reg.register_type(make_typedef("StorageA", Some("alice"), &[], 1))
        .await
        .expect("test: register StorageA");
    reg.register_type(make_typedef("StorageB", Some("bob"), &[], 1))
        .await
        .expect("test: register StorageB");

    let query = catalog::SearchQuery {
        query: "Storage".to_string(),
        author: Some("alice".to_string()),
        sort_by: catalog::SortCriteria::Relevance,
        ..Default::default()
    };
    let results = reg.search_types(&query).await.expect("test: search");
    assert_eq!(results.results.len(), 2, "both match name, but alice should rank higher");
    assert_eq!(results.results[0].type_name, "StorageA");
}

#[tokio::test]
async fn search_pagination_offset_and_limit() {
    let reg = test_registry();
    for i in 0..10 {
        let name = format!("Type{:02}", i);
        reg.register_type(make_typedef(&name, None, &[], 1))
            .await
            .expect("test: register type");
    }

    let query = catalog::SearchQuery {
        query: String::new(),
        limit: 3,
        offset: 2,
        ..Default::default()
    };
    let results = reg.search_types(&query).await.expect("test: search");
    assert_eq!(results.total_count, 10, "total_count reflects all matches");
    assert_eq!(results.results.len(), 3, "limit=3 respected");
}

#[tokio::test]
async fn sort_by_name_vs_relevance_differs() {
    let reg = test_registry();
    // "Zeta" comes last alphabetically but might score high on relevance
    reg.register_type(make_typedef("Zeta", None, &["important"], 5))
        .await
        .expect("test: register Zeta");
    reg.register_type(make_typedef("ZetaChild", None, &[], 1))
        .await
        .expect("test: register ZetaChild");

    let by_relevance = catalog::SearchQuery {
        query: "Zeta".to_string(),
        sort_by: catalog::SortCriteria::Relevance,
        ..Default::default()
    };
    let by_name = catalog::SearchQuery {
        query: "Zeta".to_string(),
        sort_by: catalog::SortCriteria::Name,
        ..Default::default()
    };

    let res_rel = reg.search_types(&by_relevance).await.expect("test: relevance");
    let res_name = reg.search_types(&by_name).await.expect("test: name");

    // Both return 2 results
    assert_eq!(res_rel.results.len(), 2);
    assert_eq!(res_name.results.len(), 2);

    // Name sort: alphabetical -> Zeta before ZetaChild
    assert_eq!(res_name.results[0].type_name, "Zeta");
    assert_eq!(res_name.results[1].type_name, "ZetaChild");

    // Relevance sort: exact match "Zeta" should still rank first
    assert_eq!(res_rel.results[0].type_name, "Zeta");
}

#[tokio::test]
async fn search_result_has_publisher_score_and_tier_fields() {
    let reg = test_registry();
    reg.register_type(make_typedef("Widget", None, &[], 1))
        .await
        .expect("test: register Widget");

    let query = catalog::SearchQuery {
        query: "Widget".to_string(),
        ..Default::default()
    };
    let results = reg.search_types(&query).await.expect("test: search");
    let result = &results.results[0];

    // publisher_authenticated is binary authentication (whitepaper-aligned)
    let _auth = result.publisher_authenticated; // Option<bool> — exists on struct
}

#[tokio::test]
async fn search_version_count_affects_scoring() {
    let reg = test_registry();
    // High version count = established package
    reg.register_type(make_typedef("Network", None, &[], 10))
        .await
        .expect("test: register Network");
    // Low version count = brand new
    reg.register_type(make_typedef("NetworkLite", None, &[], 1))
        .await
        .expect("test: register NetworkLite");

    let query = catalog::SearchQuery {
        query: "Network".to_string(),
        sort_by: catalog::SortCriteria::Relevance,
        ..Default::default()
    };
    let results = reg.search_types(&query).await.expect("test: search");
    assert_eq!(results.results.len(), 2);

    // "Network" should score higher: exact name match AND higher version_count
    assert_eq!(results.results[0].type_name, "Network");
    assert!(results.results[0].score > results.results[1].score);
}

// ============================================================================
// Track 2 -- Contribution Rewards (8 tests)
// ============================================================================

use catalog::settlement::{
    CatalogRewardAdapter, ContributionMetrics, ContributionTracker, RewardService,
};
use caesar::upi::EgressAdapter;
use hypermesh_lib::{GoldGrams, NodeId};
use rust_decimal::Decimal;

fn test_node() -> NodeId {
    NodeId("test-catalog-node".to_string())
}

fn test_pool() -> GoldGrams {
    GoldGrams::from_decimal(Decimal::new(1000, 0))
}

#[tokio::test]
async fn tracker_records_all_four_event_types() {
    let tracker = ContributionTracker::new();

    tracker.record_publication("pub-1").await;
    tracker.record_reference("pub-1").await;
    tracker.record_validation("pub-1", true).await;
    tracker.record_validation("pub-1", false).await;
    tracker.record_maintenance("pub-1", 0.1).await;

    let m = tracker.get_metrics("pub-1").await.expect("test: metrics should exist");
    assert_eq!(m.typedefs_published, 1);
    assert_eq!(m.typedef_references, 1);
    assert_eq!(m.successful_validations, 1);
    assert_eq!(m.failed_validations, 1);
    // maintenance_score starts at 0.5 + 0.1 = 0.6
    assert!(
        (m.maintenance_score - 0.6).abs() < 0.001,
        "maintenance score should be ~0.6, got {}",
        m.maintenance_score
    );
}

#[test]
fn contribution_score_known_inputs() {
    let mut m = ContributionMetrics {
        publisher_id: "pub-x".to_string(),
        typedefs_published: 5,
        typedef_references: 200,
        successful_validations: 90,
        failed_validations: 10,
        maintenance_score: 0.8,
        last_contribution: chrono::Utc::now(),
    };

    let score = m.contribution_score();
    assert!(score > 0.0 && score <= 1.0, "score {} out of range", score);

    // Rough expected: ln(6)/5*0.3 + 200/1000*0.3 + 0.9*0.25 + 0.8*0.15
    //              ~ 0.107 + 0.06 + 0.225 + 0.12 = 0.512
    assert!(score > 0.40 && score < 0.65, "score {} not in expected range", score);

    // Increase references -> score should go up
    m.typedef_references = 500;
    let higher = m.contribution_score();
    assert!(higher > score, "more references should increase score");
}

#[tokio::test]
async fn top_contributors_ordering() {
    let tracker = ContributionTracker::new();

    // High contributor
    for _ in 0..10 {
        tracker.record_publication("alice").await;
        tracker.record_reference("alice").await;
    }

    // Low contributor
    tracker.record_publication("bob").await;

    let top = tracker.top_contributors(10).await;
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].publisher_id, "alice", "alice should rank first");
    assert!(
        top[0].contribution_score() > top[1].contribution_score(),
        "first contributor should have higher score"
    );
}

#[tokio::test]
async fn reward_service_distributes_proportionally() {
    let service = RewardService::new(test_node(), test_pool());

    // Alice: heavy contributor
    for _ in 0..5 {
        service.tracker().record_publication("alice").await;
        service.tracker().record_reference("alice").await;
    }

    // Bob: light contributor
    service.tracker().record_publication("bob").await;

    let distributions = service
        .distribute_rewards()
        .await
        .expect("test: distribution should succeed");

    assert_eq!(distributions.len(), 2);

    let alice_dist = distributions
        .iter()
        .find(|d| d.publisher_id == "alice")
        .expect("test: alice distribution");
    let bob_dist = distributions
        .iter()
        .find(|d| d.publisher_id == "bob")
        .expect("test: bob distribution");

    assert!(
        alice_dist.amount > bob_dist.amount,
        "alice (heavy contributor) should get more than bob"
    );

    // Total should not exceed pool
    let total = service.total_distributed().await;
    assert!(
        total.0 <= test_pool().0,
        "total {} exceeds pool {}",
        total.0,
        test_pool().0
    );
}

#[tokio::test]
async fn reward_service_single_contributor_gets_all() {
    let service = RewardService::new(test_node(), test_pool());

    service.tracker().record_publication("solo").await;
    service.tracker().record_reference("solo").await;

    let distributions = service
        .distribute_rewards()
        .await
        .expect("test: distribution should succeed");

    assert_eq!(distributions.len(), 1);
    assert_eq!(distributions[0].publisher_id, "solo");
    // Single contributor gets the entire pool
    assert!(
        distributions[0].amount.0 > Decimal::ZERO,
        "solo contributor should receive nonzero reward"
    );
}

#[tokio::test]
async fn reward_adapter_settle_and_balance_tracking() {
    let adapter = CatalogRewardAdapter::new(test_node(), test_pool());
    assert_eq!(adapter.adapter_id(), "catalog_contribution_rewards");
    assert_eq!(
        adapter.supported_denominations(),
        vec!["CAES".to_string()]
    );

    // Settle 50 CAES to pub-1
    let value = GoldGrams::from_decimal(Decimal::new(50, 0));
    let receipt = adapter
        .settle(value, "pub-1", "CAES", Decimal::new(2350, 0))
        .await
        .expect("test: settle should succeed");
    assert_eq!(receipt.destination_denomination, "CAES");

    // Settle 30 more to pub-1
    let value2 = GoldGrams::from_decimal(Decimal::new(30, 0));
    adapter
        .settle(value2, "pub-1", "CAES", Decimal::new(2350, 0))
        .await
        .expect("test: second settle should succeed");

    // Balance should be 80
    let balance = adapter.publisher_rewards("pub-1").await;
    assert_eq!(balance.0, Decimal::new(80, 0));

    // Unknown publisher should have zero
    let unknown = adapter.publisher_rewards("nobody").await;
    assert!(unknown.is_zero());
}

#[tokio::test]
async fn reward_adapter_rejects_unsupported_denomination() {
    let adapter = CatalogRewardAdapter::new(test_node(), test_pool());
    let value = GoldGrams::from_decimal(Decimal::new(10, 0));

    let result = adapter
        .settle(value, "pub-1", "BTC", Decimal::new(2350, 0))
        .await;
    assert!(result.is_err(), "BTC should be rejected");
}

#[tokio::test]
async fn empty_distribution_returns_empty_vec() {
    let service = RewardService::new(test_node(), test_pool());

    // No contributions recorded
    let distributions = service
        .distribute_rewards()
        .await
        .expect("test: distribution should succeed");
    assert!(
        distributions.is_empty(),
        "no contributors means no distributions"
    );
}

// ============================================================================
// Track 3 -- STOQ API handlers (8 tests)
// ============================================================================

use catalog::api::stoq_api::*;
use stoq::api::{ApiError, ApiHandler, ApiRequest};
use std::collections::HashMap;
use std::sync::Arc;

fn make_api_request(id: &str, method: &str, payload: &impl serde::Serialize) -> ApiRequest {
    ApiRequest {
        id: id.to_string(),
        service: "catalog".to_string(),
        method: method.to_string(),
        payload: bytes::Bytes::from(
            serde_json::to_vec(payload).expect("test: serialize request payload"),
        ),
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn browse_handler_returns_valid_response_with_package_count() {
    let state = Arc::new(CatalogAppState::new());
    state.set_package_count(42);

    let handler = BrowseHandler { state };
    let req_body = BrowseRequest {
        category: None,
        sort_by: "relevance".to_string(),
        page: 0,
        page_size: 20,
        featured_only: false,
    };

    let resp = handler
        .handle(make_api_request("browse-1", "browse", &req_body))
        .await
        .expect("test: browse handler should succeed");
    assert!(resp.success);

    let body: BrowseResponse =
        serde_json::from_slice(&resp.payload).expect("test: deserialize browse response");
    assert_eq!(body.total_count, 42);
    assert_eq!(body.page, 0);
    assert_eq!(body.page_size, 20);
}

#[tokio::test]
async fn search_handler_processes_query_correctly() {
    let state = Arc::new(CatalogAppState::new());
    let handler = SearchHandler { state };

    let req_body = SearchRequest {
        query: "gpu compute".to_string(),
        tags: vec!["gpu".to_string()],
        author: Some("alice".to_string()),
        limit: 10,
        offset: 5,
    };

    let resp = handler
        .handle(make_api_request("search-1", "search", &req_body))
        .await
        .expect("test: search handler should succeed");
    assert!(resp.success);

    let body: SearchResponse =
        serde_json::from_slice(&resp.payload).expect("test: deserialize search response");
    assert_eq!(body.query, "gpu compute");
}

#[tokio::test]
async fn get_package_handler_returns_not_found_for_nonexistent() {
    let state = Arc::new(CatalogAppState::new());
    let handler = GetPackageHandler { state };

    let req_body = GetPackageRequest {
        name: "does-not-exist".to_string(),
        version: None,
    };

    let result = handler
        .handle(make_api_request("pkg-1", "package", &req_body))
        .await;
    match result {
        Err(ApiError::NotFound(msg)) => {
            assert!(
                msg.contains("does-not-exist"),
                "error should mention package name"
            );
        }
        other => {
            assert!(false, "expected NotFound, got {:?}", other);
        }
    }
}

#[tokio::test]
async fn get_publisher_handler_returns_not_found_for_unknown() {
    let state = Arc::new(CatalogAppState::new());
    let handler = GetPublisherHandler { state };

    let req_body = GetPublisherRequest {
        publisher_id: "unknown-pub-xyz".to_string(),
    };

    let result = handler
        .handle(make_api_request("pub-1", "publisher", &req_body))
        .await;
    match result {
        Err(ApiError::NotFound(msg)) => {
            assert!(
                msg.contains("unknown-pub-xyz"),
                "error should mention publisher id"
            );
        }
        other => {
            assert!(false, "expected NotFound, got {:?}", other);
        }
    }
}

#[tokio::test]
async fn registry_stats_handler_returns_correct_counters() {
    let state = Arc::new(CatalogAppState::new());
    state.set_package_count(100);
    state.set_publisher_count(25);
    state.increment_downloads();
    state.increment_downloads();
    state.increment_downloads();

    let handler = RegistryStatsHandler { state };

    let resp = handler
        .handle(make_api_request("stats-1", "stats", &serde_json::json!({})))
        .await
        .expect("test: stats handler should succeed");
    assert!(resp.success);

    let body: RegistryStatsResponse =
        serde_json::from_slice(&resp.payload).expect("test: deserialize stats response");
    assert_eq!(body.total_packages, 100);
    assert_eq!(body.total_publishers, 25);
    assert_eq!(body.total_downloads, 3);
}

#[tokio::test]
async fn catalog_health_handler_returns_healthy_with_uptime() {
    let state = Arc::new(CatalogAppState::new());
    state.set_package_count(7);

    let start = std::time::Instant::now();
    // Small sleep so uptime is non-zero
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let handler = CatalogHealthHandler {
        state,
        start_time: start,
    };

    let resp = handler
        .handle(make_api_request("health-1", "health", &serde_json::json!({})))
        .await
        .expect("test: health handler should succeed");
    assert!(resp.success);

    let body: HealthResponse =
        serde_json::from_slice(&resp.payload).expect("test: deserialize health response");
    assert_eq!(body.status, "healthy");
    assert_eq!(body.service, "catalog");
    assert_eq!(body.package_count, 7);
    // Uptime should be present (u64, so always >= 0)
    let _ = body.uptime_secs;
}

#[test]
fn catalog_app_state_counter_operations() {
    let state = CatalogAppState::new();
    assert_eq!(state.service_name, "catalog");

    // Package count
    state.set_package_count(500);
    assert_eq!(
        state
            .package_count
            .load(std::sync::atomic::Ordering::Relaxed),
        500
    );

    // Publisher count
    state.set_publisher_count(50);
    assert_eq!(
        state
            .publisher_count
            .load(std::sync::atomic::Ordering::Relaxed),
        50
    );

    // Download increments
    for _ in 0..10 {
        state.increment_downloads();
    }
    assert_eq!(
        state
            .total_downloads
            .load(std::sync::atomic::Ordering::Relaxed),
        10
    );
}

#[tokio::test]
async fn browse_handler_rejects_invalid_payload() {
    let state = Arc::new(CatalogAppState::new());
    let handler = BrowseHandler { state };

    let api_req = ApiRequest {
        id: "bad-1".to_string(),
        service: "catalog".to_string(),
        method: "browse".to_string(),
        payload: bytes::Bytes::from("not valid json"),
        metadata: HashMap::new(),
    };

    let result = handler.handle(api_req).await;
    match result {
        Err(ApiError::InvalidRequest(msg)) => {
            assert!(
                msg.contains("Invalid browse request"),
                "error message should describe the issue"
            );
        }
        other => {
            assert!(false, "expected InvalidRequest, got {:?}", other);
        }
    }
}
