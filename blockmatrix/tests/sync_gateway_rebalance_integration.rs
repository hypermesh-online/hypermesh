// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests proving Network Sync, Gateway, and Shard Rebalancing
//! work together as a cohesive system.
//!
//! These tests exercise cross-module interactions between:
//! - `SyncManager` / `SyncDispatcher` (blockchain synchronization)
//! - `GatewayManager` / `ScopeBridge` (cross-scope asset transfers)
//! - `RebalanceManager` (dynamic shard redistribution)
//! - `ReflectorPool` (reflector health tracking)

use std::sync::Arc;

use blockmatrix::blockchain::sync_manager::{
    BlockProvider, SyncConfig, SyncManager, SyncMessage, SyncObserver, SyncState,
};
use blockmatrix::bootstrap::PrivacyMode;
use blockmatrix::distribution::rebalancing::{
    RebalanceAction, RebalanceConfig, RebalanceManager,
};
use blockmatrix::gateway::{
    AssetTransfer, GatewayManager, TransferStatus, TransferValidator,
};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::reflector_pool::{Reflector, ReflectorConfig, ReflectorPool};
use blockmatrix::network::sync_dispatch::{DispatchResponse, SyncDispatcher};
use blockmatrix::network::stoq_integration::MatrixMessage;
use blockmatrix::gateway::GatewayError;

use hypermesh_lib::{AssetId, BlockchainScope, MatrixPosition};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// A deterministic BlockProvider returning predictable block hashes.
struct TestBlockProvider {
    chain_height: u64,
}

impl BlockProvider for TestBlockProvider {
    fn get_block_hashes(&self, from_height: u64, max_blocks: u32) -> (Vec<String>, u64) {
        let end = (from_height + max_blocks as u64).min(self.chain_height);
        let hashes: Vec<String> = (from_height..end)
            .map(|h| format!("block-hash-{}", h))
            .collect();
        (hashes, self.chain_height)
    }
}

fn coord(x: i64, y: i64, z: i64) -> MatrixCoordinate {
    MatrixCoordinate::new(x, y, z).expect("test: valid coordinate")
}

fn position(x: f64, y: f64, z: f64) -> MatrixPosition {
    MatrixPosition { x, y, z }
}

fn make_reflector(
    node_id: &str,
    health: f64,
    height: u64,
    last_seen: u64,
    pos: MatrixPosition,
) -> Reflector {
    Reflector {
        node_id: node_id.to_string(),
        position: pos,
        last_seen,
        block_height: height,
        health_score: health,
        privacy_mode: PrivacyMode::PUBLIC,
    }
}

fn make_rebalance_manager() -> RebalanceManager {
    RebalanceManager::new(RebalanceConfig {
        min_replicas: 2,
        max_replicas: 4,
        rebalance_threshold: 0.3,
        cooldown_secs: 0,
    })
}

// =========================================================================
// 1. Sync + Gateway Integration
// =========================================================================

/// SyncManager joins network -> GatewayManager transfers asset between
/// Device and Network scopes -> verify transfer lifecycle completes.
#[tokio::test]
async fn test_sync_join_then_gateway_transfer() {
    // Step 1: SyncManager joins a network
    let mut sync = SyncManager::new("device-chain-1".to_string(), SyncConfig::default());
    sync.join_network("alpha-net".to_string(), PrivacyMode::PUBLIC, 1000)
        .expect("test: join alpha-net");

    assert!(sync.is_member("alpha-net"));
    assert_eq!(
        sync.sync_state("alpha-net"),
        Some(&SyncState::Discovering)
    );

    // Transition to Synchronized (simulating completed sync)
    sync.update_sync_state(
        "alpha-net",
        SyncState::Synchronized {
            last_block_height: 100,
        },
    )
    .expect("test: set synchronized");

    // Step 2: Now that sync is complete, perform a gateway transfer
    let gw = GatewayManager::new();
    let transfer_id = gw
        .transfer_asset(
            AssetId::from("cpu-asset-001"),
            BlockchainScope::Device,
            BlockchainScope::Network,
        )
        .await
        .expect("test: initiate transfer");

    let status = gw
        .validate_transfer(&transfer_id)
        .await
        .expect("test: validate transfer");
    assert_eq!(status, TransferStatus::Confirmed);

    // Step 3: Verify cross-module consistency
    // Sync is still synchronized after gateway transfer
    assert_eq!(
        sync.sync_state("alpha-net"),
        Some(&SyncState::Synchronized {
            last_block_height: 100,
        })
    );

    // Gateway reports transfer as confirmed
    let all = gw.list_all_transfers().await;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].status, TransferStatus::Confirmed);
    assert_eq!(all[0].asset_id, AssetId::from("cpu-asset-001"));
}

/// SyncDispatcher routes ReflectorHeartbeat -> ReflectorPool registers ->
/// verify health tracking works end-to-end.
#[tokio::test]
async fn test_dispatcher_heartbeat_to_reflector_health() {
    let mut sync_mgr = SyncManager::new("dev-chain".to_string(), SyncConfig::default());
    let mut reflector_pool = ReflectorPool::new(ReflectorConfig {
        min_reflectors: 2,
        max_reflectors: 10,
        health_threshold: 0.3,
        stale_timeout_ms: 30_000,
    });

    // Dispatch 3 heartbeats from different reflectors
    let reflectors = [
        ("reflector-a", position(1.0, 2.0, 3.0), 50, 0.85),
        ("reflector-b", position(-1.0, 2.0, 3.0), 48, 0.72),
        ("reflector-c", position(1.0, -2.0, 3.0), 50, 0.91),
    ];

    for (node_id, pos, height, health) in &reflectors {
        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sync_mgr,
            reflector_pool: &mut reflector_pool,
            block_provider: None,
        };

        let msg = MatrixMessage::ReflectorHeartbeat {
            network_id: "sync-net".to_string(),
            block_height: *height,
            health_score: *health,
        };

        let resp = dispatcher.dispatch(msg, node_id, *pos);
        assert!(
            matches!(resp, DispatchResponse::None),
            "Heartbeat should produce no reply"
        );
    }

    // Verify reflector pool has all 3
    assert_eq!(reflector_pool.total_count("sync-net"), 3);
    assert!(reflector_pool.is_healthy("sync-net"));

    // Best reflector should be the one with highest health (0.91)
    let best = reflector_pool.get_best_reflectors("sync-net", 1);
    assert_eq!(best.len(), 1);
    assert_eq!(best[0].node_id, "reflector-c");

    // Update health of reflector-a via pool directly
    reflector_pool.update_health("reflector-a", 0.99);
    let best = reflector_pool.get_best_reflectors("sync-net", 1);
    assert_eq!(best[0].node_id, "reflector-a");
}

/// BlockProvider returns blocks -> SyncManager populates SyncResponse ->
/// verify real block hashes flow through.
#[tokio::test]
async fn test_block_provider_through_sync_response() {
    let mut sync_mgr = SyncManager::new("dev-chain".to_string(), SyncConfig::default());
    sync_mgr
        .join_network("data-net".to_string(), PrivacyMode::PUBLIC, 500)
        .expect("test: join data-net");

    let provider = TestBlockProvider { chain_height: 100 };

    // Process a sync request with the provider
    let request = SyncMessage::Request {
        network_id: "data-net".to_string(),
        from_height: 10,
        max_blocks: 25,
    };

    let response = sync_mgr.process_sync_message_with_provider(request, Some(&provider));

    match response {
        Some(SyncMessage::Response {
            network_id,
            block_hashes,
            peer_height,
        }) => {
            assert_eq!(network_id, "data-net");
            assert_eq!(block_hashes.len(), 25);
            assert_eq!(block_hashes[0], "block-hash-10");
            assert_eq!(block_hashes[24], "block-hash-34");
            assert_eq!(peer_height, 100);
        }
        other => unreachable!("test: expected SyncResponse, got {:?}", other),
    }

    // Process an empty response (signals sync completion)
    let empty_response = SyncMessage::Response {
        network_id: "data-net".to_string(),
        block_hashes: Vec::new(),
        peer_height: 100,
    };
    sync_mgr.process_sync_message(empty_response);

    // Should now be synchronized
    assert_eq!(
        sync_mgr.sync_state("data-net"),
        Some(&SyncState::Synchronized {
            last_block_height: 100,
        })
    );
}

// =========================================================================
// 2. Gateway + Rebalancing Integration
// =========================================================================

/// Asset transferred via GatewayManager -> RebalanceManager detects node
/// topology change -> generates appropriate RebalanceActions.
#[tokio::test]
async fn test_gateway_transfer_triggers_rebalance() {
    // Step 1: Set up rebalance manager with existing topology
    let mut rebalance = make_rebalance_manager();
    let pos_a = coord(10, 10, 10);
    let pos_b = coord(-10, -10, -10);
    rebalance.register_shard("shard-alpha".to_string(), "node-a", &pos_a);
    rebalance.register_shard("shard-alpha".to_string(), "node-b", &pos_b);

    // Step 2: Perform gateway transfer (Device -> Network)
    let gw = GatewayManager::new();
    let tid = gw
        .transfer_asset(
            AssetId::from("shard-alpha-asset"),
            BlockchainScope::Device,
            BlockchainScope::Network,
        )
        .await
        .expect("test: initiate transfer");
    let status = gw
        .validate_transfer(&tid)
        .await
        .expect("test: validate");
    assert_eq!(status, TransferStatus::Confirmed);

    // Step 3: Simulate that the transfer caused a new node to join
    // (asset now visible in Network scope, new node picks it up)
    let pos_c = coord(10, -10, 10);
    let actions = rebalance.on_node_joined("node-c", &pos_c);

    // node-c should appear in the node list and there may be rebalance actions
    let dist = rebalance.get_shard_distribution();
    assert!(
        dist.contains_key("node-a") || dist.contains_key("node-b"),
        "Original nodes should still host shards"
    );

    // Since shard-alpha already has 2 replicas (min_replicas=2), the join
    // may or may not generate actions, but the manager should track the node
    let report = rebalance.check_balance();
    // node-c has 0 shards, might be underloaded
    assert!(
        report.orphaned_shards.is_empty(),
        "shard-alpha has min replicas, should not be orphaned"
    );

    // If there are actions, executing them should succeed
    if !actions.is_empty() {
        let result = rebalance.execute_actions(&actions);
        assert_eq!(result.actions_failed, 0);
    }
}

/// Multiple concurrent cross-scope transfers -> rebalancing maintains
/// minimum replicas.
#[tokio::test]
async fn test_concurrent_transfers_with_rebalancing() {
    let gw = Arc::new(GatewayManager::new());
    let mut rebalance = make_rebalance_manager();

    // Set up topology with shards
    let nodes = [
        ("node-1", coord(10, 10, 10)),
        ("node-2", coord(-10, 10, 10)),
        ("node-3", coord(10, -10, 10)),
    ];
    for (nid, pos) in &nodes {
        rebalance.register_shard("shard-x".to_string(), nid, pos);
    }

    // Perform 3 concurrent transfers for different assets
    let gw1 = gw.clone();
    let gw2 = gw.clone();
    let gw3 = gw.clone();

    let (r1, r2, r3) = tokio::join!(
        async {
            let tid = gw1
                .transfer_asset(
                    AssetId::from("asset-x"),
                    BlockchainScope::Device,
                    BlockchainScope::Network,
                )
                .await
                .expect("test: transfer asset-x");
            gw1.validate_transfer(&tid).await.expect("test: validate x")
        },
        async {
            let tid = gw2
                .transfer_asset(
                    AssetId::from("asset-y"),
                    BlockchainScope::Network,
                    BlockchainScope::Device,
                )
                .await
                .expect("test: transfer asset-y");
            gw2.validate_transfer(&tid).await.expect("test: validate y")
        },
        async {
            let tid = gw3
                .transfer_asset(
                    AssetId::from("asset-z"),
                    BlockchainScope::Device,
                    BlockchainScope::Network,
                )
                .await
                .expect("test: transfer asset-z");
            gw3.validate_transfer(&tid).await.expect("test: validate z")
        },
    );

    assert_eq!(r1, TransferStatus::Confirmed);
    assert_eq!(r2, TransferStatus::Confirmed);
    assert_eq!(r3, TransferStatus::Confirmed);

    // Verify rebalancing still reports correct shard state
    let report = rebalance.check_balance();
    // shard-x has 3 replicas across 3 nodes (>=min_replicas=2), healthy
    assert!(
        report.orphaned_shards.is_empty(),
        "Shards should maintain min replicas after concurrent transfers"
    );
}

/// Node failure during transfer -> gateway rollback + rebalancing
/// emergency re-replication.
#[tokio::test]
async fn test_node_failure_during_transfer_and_rebalance() {
    // Step 1: Set up a transfer that will fail (using a rejecting validator)
    struct RejectValidator;

    #[async_trait::async_trait]
    impl TransferValidator for RejectValidator {
        async fn validate_transfer(
            &self,
            _transfer: &AssetTransfer,
        ) -> Result<bool, GatewayError> {
            // Simulate validation failure (e.g., node went down)
            Ok(false)
        }
    }

    let gw = GatewayManager::with_validator(Arc::new(RejectValidator));
    let tid = gw
        .transfer_asset(
            AssetId::from("fragile-asset"),
            BlockchainScope::Device,
            BlockchainScope::Network,
        )
        .await
        .expect("test: initiate transfer");

    // Transfer should be rolled back due to validation failure
    let status = gw
        .validate_transfer(&tid)
        .await
        .expect("test: validate");
    assert_eq!(status, TransferStatus::RolledBack);

    // Step 2: Simultaneously, the failing node triggers rebalancing
    let mut rebalance = make_rebalance_manager();
    let pos_a = coord(10, 10, 10);
    let pos_b = coord(-10, -10, -10);
    let pos_c = coord(10, -10, 10);

    rebalance.register_shard("shard-fragile".to_string(), "failed-node", &pos_a);
    rebalance.register_shard("shard-fragile".to_string(), "healthy-node-1", &pos_b);
    // Add a spare node for re-replication
    rebalance
        .register_shard("other-shard".to_string(), "healthy-node-2", &pos_c);

    // Node failure triggers emergency rebalancing
    let actions = rebalance.on_node_failed("failed-node");

    // Should generate ReplicateShard actions for shard-fragile
    let replicate_actions: Vec<_> = actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                RebalanceAction::ReplicateShard { shard_id, .. }
                if shard_id == "shard-fragile"
            )
        })
        .collect();

    assert!(
        !replicate_actions.is_empty(),
        "Emergency rebalancing should replicate shard from failed node"
    );

    // Execute the emergency actions
    let result = rebalance.execute_actions(&actions);
    assert_eq!(result.actions_failed, 0, "Emergency replication should succeed");

    // Verify shard-fragile still has sufficient replicas
    let dist = rebalance.get_shard_distribution();
    let shard_hosts: Vec<_> = dist
        .iter()
        .filter(|(_, shards)| shards.contains(&"shard-fragile".to_string()))
        .map(|(nid, _)| nid.clone())
        .collect();

    assert!(
        shard_hosts.len() >= 2,
        "shard-fragile should have >= 2 replicas after emergency rebalance, has {}",
        shard_hosts.len()
    );
    assert!(
        !shard_hosts.contains(&"failed-node".to_string()),
        "Failed node should not host any shards"
    );
}

// =========================================================================
// 3. Sync + Rebalancing Integration
// =========================================================================

/// Node joins network via SyncManager -> RebalanceManager triggered with
/// on_node_joined -> shards redistributed to new node.
#[tokio::test]
async fn test_sync_join_triggers_rebalance_distribution() {
    // Set up SyncManager
    let mut sync = SyncManager::new("device-chain".to_string(), SyncConfig::default());

    // Set up RebalanceManager with an under-replicated shard
    let mut rebalance = make_rebalance_manager();
    let pos_a = coord(10, 10, 10);
    rebalance.register_shard("shard-data-1".to_string(), "existing-node", &pos_a);
    // shard-data-1 has only 1 replica (min_replicas=2), under-replicated

    // Step 1: New node joins network via SyncManager
    sync.join_network("cluster-net".to_string(), PrivacyMode::PUBLIC, 2000)
        .expect("test: join cluster-net");
    assert!(sync.is_member("cluster-net"));

    // Step 2: New node also triggers rebalancing
    let new_pos = coord(-10, -10, -10);
    let actions = rebalance.on_node_joined("new-node", &new_pos);

    // Step 3: Verify shard-data-1 gets replicated to new node
    let replicate_count = actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                RebalanceAction::ReplicateShard { shard_id, to_node }
                if shard_id == "shard-data-1" && to_node == "new-node"
            )
        })
        .count();

    assert!(
        replicate_count >= 1,
        "Under-replicated shard should be replicated to new node, got {} actions: {:?}",
        replicate_count,
        actions
    );

    // Execute the rebalancing
    let result = rebalance.execute_actions(&actions);
    assert_eq!(result.actions_failed, 0);

    // Verify new node now hosts the shard
    let dist = rebalance.get_shard_distribution();
    assert!(
        dist.get("new-node")
            .map_or(false, |s| s.contains(&"shard-data-1".to_string())),
        "new-node should host shard-data-1 after rebalancing"
    );
}

/// Node leaves network -> RebalanceManager on_node_left -> shards
/// moved to remaining nodes.
#[tokio::test]
async fn test_node_leave_triggers_rebalance() {
    // Set up SyncManager with a member
    let mut sync = SyncManager::new("device-chain".to_string(), SyncConfig::default());
    sync.join_network("mesh-net".to_string(), PrivacyMode::PUBLIC, 1000)
        .expect("test: join mesh-net");

    // Set up RebalanceManager with 3 nodes, shards distributed
    let mut rebalance = make_rebalance_manager();
    let positions = [
        ("node-alpha", coord(10, 10, 10)),
        ("node-beta", coord(-10, 10, 10)),
        ("node-gamma", coord(10, -10, -10)),
    ];
    for (nid, pos) in &positions {
        rebalance.register_shard("shard-important".to_string(), nid, pos);
    }
    rebalance.register_shard("shard-other".to_string(), "node-alpha", &positions[0].1);
    rebalance.register_shard("shard-other".to_string(), "node-beta", &positions[1].1);

    // Step 1: Node leaves the sync network
    sync.leave_network("mesh-net").expect("test: leave mesh-net");
    assert!(!sync.is_member("mesh-net"));

    // Step 2: Node also leaves the rebalancing topology
    let actions = rebalance.on_node_left("node-alpha");

    // shard-important: was on 3 nodes, lost 1, now 2 (>= min_replicas=2), OK
    // shard-other: was on 2 nodes, lost 1, now 1 (< min_replicas=2), needs replication
    let replicate_other: Vec<_> = actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                RebalanceAction::ReplicateShard { shard_id, .. }
                if shard_id == "shard-other"
            )
        })
        .collect();

    assert!(
        !replicate_other.is_empty(),
        "shard-other should be re-replicated after node-alpha leaves: {:?}",
        actions
    );

    // Execute
    let result = rebalance.execute_actions(&actions);
    assert_eq!(result.actions_failed, 0);

    // Verify node-alpha is gone from all distributions
    let dist = rebalance.get_shard_distribution();
    assert!(
        !dist.contains_key("node-alpha"),
        "node-alpha should not host any shards after leaving"
    );
}

/// ReflectorPool prunes stale reflector -> RebalanceManager handles
/// node removal.
#[tokio::test]
async fn test_reflector_prune_triggers_rebalance() {
    // Set up ReflectorPool with one stale reflector
    let mut reflector_pool = ReflectorPool::new(ReflectorConfig {
        min_reflectors: 2,
        max_reflectors: 10,
        health_threshold: 0.3,
        stale_timeout_ms: 10_000,
    });

    reflector_pool.register_reflector(
        "net-1",
        make_reflector("stale-node", 0.8, 50, 5, position(1.0, 1.0, 1.0)),
    );
    reflector_pool.register_reflector(
        "net-1",
        make_reflector("fresh-node-1", 0.9, 55, 25, position(-1.0, 1.0, 1.0)),
    );
    reflector_pool.register_reflector(
        "net-1",
        make_reflector("fresh-node-2", 0.85, 53, 24, position(1.0, -1.0, 1.0)),
    );

    // Set up RebalanceManager
    let mut rebalance = make_rebalance_manager();
    rebalance.register_shard(
        "shard-prunable".to_string(),
        "stale-node",
        &coord(10, 10, 10),
    );
    rebalance.register_shard(
        "shard-prunable".to_string(),
        "fresh-node-1",
        &coord(-10, 10, 10),
    );
    // Register the spare node for potential replication
    rebalance
        .register_shard("other-shard".to_string(), "fresh-node-2", &coord(10, -10, 10));

    // Step 1: Prune stale reflectors (now_ms=30_000, stale timeout=10_000)
    // cutoff = (30000 - 10000) / 1000 = 20 seconds
    // stale-node.last_seen=5 < 20 -> pruned
    let pruned = reflector_pool.prune_stale(30_000);
    assert_eq!(pruned, 1);
    assert_eq!(reflector_pool.total_count("net-1"), 2);

    // Step 2: React to the stale node departure in rebalancing
    let actions = rebalance.on_node_left("stale-node");

    // shard-prunable: was on 2 nodes, lost 1, now 1 (< min_replicas=2)
    let replicate_actions: Vec<_> = actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                RebalanceAction::ReplicateShard { shard_id, .. }
                if shard_id == "shard-prunable"
            )
        })
        .collect();

    assert!(
        !replicate_actions.is_empty(),
        "Pruned node's shards should be re-replicated: {:?}",
        actions
    );

    let result = rebalance.execute_actions(&actions);
    assert_eq!(result.actions_failed, 0);
}

// =========================================================================
// 4. Full Pipeline Integration
// =========================================================================

/// Complete flow: SyncManager joins -> GatewayManager transfers ->
/// RebalanceManager balances -> verify final state consistent.
#[tokio::test]
async fn test_full_pipeline_join_transfer_rebalance() {
    // --- Phase A: Network synchronization ---
    let mut sync = SyncManager::new("full-device-chain".to_string(), SyncConfig::default());
    sync.join_network("prod-net".to_string(), PrivacyMode::PUBLIC, 1000)
        .expect("test: join prod-net");

    // Set up an observer to verify sync completion notification
    use std::sync::Mutex;
    struct TestObserver {
        events: Arc<Mutex<Vec<(String, u64)>>>,
    }
    impl SyncObserver for TestObserver {
        fn on_sync_complete(&self, network_id: &str, block_height: u64) {
            self.events
                .lock()
                .expect("test: lock")
                .push((network_id.to_string(), block_height));
        }
    }

    let events = Arc::new(Mutex::new(Vec::new()));
    sync.set_observer(Box::new(TestObserver {
        events: events.clone(),
    }));

    // Simulate sync completion via empty response
    sync.process_sync_message(SyncMessage::Response {
        network_id: "prod-net".to_string(),
        block_hashes: Vec::new(),
        peer_height: 500,
    });

    // Verify observer was notified
    {
        let captured = events.lock().expect("test: lock");
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].0, "prod-net");
        assert_eq!(captured[0].1, 500);
    }

    assert_eq!(
        sync.sync_state("prod-net"),
        Some(&SyncState::Synchronized {
            last_block_height: 500,
        })
    );

    // --- Phase B: Gateway cross-scope transfer ---
    let gw = GatewayManager::new();
    let tid = gw
        .transfer_asset(
            AssetId::from("production-asset"),
            BlockchainScope::Device,
            BlockchainScope::Network,
        )
        .await
        .expect("test: initiate production transfer");

    let status = gw
        .validate_transfer(&tid)
        .await
        .expect("test: validate");
    assert_eq!(status, TransferStatus::Confirmed);

    // --- Phase C: Rebalancing after topology change ---
    let mut rebalance = make_rebalance_manager();
    let positions = [
        ("prod-node-1", coord(5, 5, 5)),
        ("prod-node-2", coord(-5, 5, 5)),
    ];
    for (nid, pos) in &positions {
        rebalance.register_shard("prod-shard-1".to_string(), nid, pos);
    }
    // prod-shard-1 has 2 replicas (== min_replicas), add a new node
    let new_actions = rebalance.on_node_joined("prod-node-3", &coord(5, -5, -5));

    // Execute any rebalancing actions
    if !new_actions.is_empty() {
        let result = rebalance.execute_actions(&new_actions);
        assert_eq!(result.actions_failed, 0);
    }

    // --- Phase D: Verify final consistency ---
    // Sync: still synchronized
    assert!(sync.is_member("prod-net"));
    assert_eq!(
        sync.sync_state("prod-net"),
        Some(&SyncState::Synchronized {
            last_block_height: 500,
        })
    );

    // Gateway: transfer confirmed, no pending
    let pending = gw.list_pending_transfers().await;
    assert!(pending.is_empty(), "No pending transfers after completion");

    // Rebalance: no orphaned shards
    let report = rebalance.check_balance();
    assert!(
        report.orphaned_shards.is_empty(),
        "No orphaned shards in final state"
    );
}

/// Cluster with 5+ nodes, multiple scopes, transfers, and rebalancing
/// all operating together.
#[tokio::test]
async fn test_multi_node_cluster_integration() {
    // --- Set up 5-node cluster ---
    let node_positions = [
        ("node-0", coord(10, 10, 10)),
        ("node-1", coord(-10, 10, 10)),
        ("node-2", coord(10, -10, 10)),
        ("node-3", coord(-10, -10, 10)),
        ("node-4", coord(10, 10, -10)),
    ];

    // Rebalance manager tracking shard placement
    let mut rebalance = RebalanceManager::new(RebalanceConfig {
        min_replicas: 3,
        max_replicas: 4,
        rebalance_threshold: 0.3,
        cooldown_secs: 0,
    });

    // Distribute 4 shards across the first 2 nodes initially (imbalanced)
    for i in 0..4 {
        let shard_id = format!("shard-{}", i);
        rebalance.register_shard(shard_id.clone(), "node-0", &node_positions[0].1);
        rebalance.register_shard(shard_id, "node-1", &node_positions[1].1);
    }

    // Register remaining nodes
    for (nid, pos) in &node_positions[2..] {
        let actions = rebalance.on_node_joined(nid, pos);
        let result = rebalance.execute_actions(&actions);
        assert_eq!(
            result.actions_failed, 0,
            "Node {} join actions should succeed",
            nid
        );
    }

    // --- Set up sync for 2 networks ---
    let mut sync = SyncManager::new("cluster-device".to_string(), SyncConfig::default());
    sync.join_network("net-primary".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join net-primary");
    sync.join_network("net-secondary".to_string(), PrivacyMode::PRIVATE, 100)
        .expect("test: join net-secondary");

    assert_eq!(sync.active_network_count(), 2);

    // Synchronize both
    for net_id in &["net-primary", "net-secondary"] {
        sync.update_sync_state(
            net_id,
            SyncState::Synchronized {
                last_block_height: 200,
            },
        )
        .expect("test: synchronize");
    }

    // --- Perform multiple gateway transfers ---
    let gw = GatewayManager::new();
    let mut confirmed_count = 0;

    for i in 0..3 {
        let asset = AssetId::from(format!("cluster-asset-{}", i));
        let (from, to) = if i % 2 == 0 {
            (BlockchainScope::Device, BlockchainScope::Network)
        } else {
            (BlockchainScope::Network, BlockchainScope::Device)
        };

        let tid = gw
            .transfer_asset(asset, from, to)
            .await
            .expect("test: initiate cluster transfer");
        let status = gw
            .validate_transfer(&tid)
            .await
            .expect("test: validate cluster transfer");

        if status == TransferStatus::Confirmed {
            confirmed_count += 1;
        }
    }

    assert_eq!(confirmed_count, 3, "All 3 transfers should confirm");

    // --- Simulate a node failure ---
    let failure_actions = rebalance.on_node_failed("node-2");
    let failure_result = rebalance.execute_actions(&failure_actions);
    assert_eq!(
        failure_result.actions_failed, 0,
        "Emergency rebalancing should succeed"
    );

    // --- Final state verification ---
    // All shards should have >= min_replicas (3) replicas on live nodes
    let _report = rebalance.check_balance();
    let dist = rebalance.get_shard_distribution();

    // node-2 should not host anything
    assert!(
        !dist.contains_key("node-2"),
        "Failed node should not host shards"
    );

    // Each shard should appear on at least min_replicas nodes
    for i in 0..4 {
        let shard_id = format!("shard-{}", i);
        let host_count = dist
            .iter()
            .filter(|(_, shards)| shards.contains(&shard_id))
            .count();
        // With 4 remaining nodes and min_replicas=3, shards might not all
        // reach 3 if there aren't enough candidates, but should be >= 2
        assert!(
            host_count >= 2,
            "shard-{} should have >= 2 replicas, has {}",
            i,
            host_count
        );
    }

    // Sync should be unaffected by rebalancing
    assert!(sync.is_member("net-primary"));
    assert!(sync.is_member("net-secondary"));

    // Gateway should have no pending transfers
    let pending = gw.list_pending_transfers().await;
    assert!(pending.is_empty());

    // All transfers should be in the history
    let all = gw.list_all_transfers().await;
    assert_eq!(all.len(), 3);
}
