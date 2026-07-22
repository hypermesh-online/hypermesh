// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Unit tests for the `sync_dispatch` submodule.

#![cfg(test)]

use std::collections::HashMap;
use std::net::SocketAddr;

use crate::blockchain::block::Block;
use crate::blockchain::node_chain::NodeBlockchain;
use crate::blockchain::sync_manager::{BlockProvider, SyncConfig, SyncManager, SyncMessage};
use crate::bootstrap::PrivacyMode;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::reflector_pool::{Reflector, ReflectorConfig, ReflectorPool};
use crate::network::stoq_integration::MatrixMessage;
use hypermesh_lib::MatrixPosition;

use super::dispatcher::{DispatchResponse, SyncDispatcher};
use super::transport_sync_driver::{filter_missing_hashes, resolve_reflector_addr, TransportSyncDriver};

/// A trivial BlockProvider that returns predictable hashes.
struct FakeBlockProvider {
    chain_height: u64,
}

impl BlockProvider for FakeBlockProvider {
    fn get_block_hashes(&self, from_height: u64, max_blocks: u32) -> (Vec<String>, u64) {
        let end = (from_height + max_blocks as u64).min(self.chain_height);
        let hashes: Vec<String> = (from_height..end).map(|h| format!("hash_{h}")).collect();
        (hashes, self.chain_height)
    }
}

fn make_sync_manager() -> SyncManager {
    SyncManager::new("device-chain".to_string(), SyncConfig::default())
}

fn make_reflector_pool() -> ReflectorPool {
    ReflectorPool::new(ReflectorConfig::default())
}

fn zero_position() -> MatrixPosition {
    MatrixPosition {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }
}

// ------------------------------------------------------------------
// Gap 5 tests: Message dispatch routing
// ------------------------------------------------------------------

#[test]
fn test_dispatch_sync_request_without_provider() {
    let mut sm = make_sync_manager();
    sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join");

    let mut rp = make_reflector_pool();

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    let msg = MatrixMessage::SyncRequest {
        network_id: "net-1".to_string(),
        from_height: 0,
        max_blocks: 10,
    };

    let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
    match resp {
        DispatchResponse::Reply(MatrixMessage::SyncResponse {
            network_id,
            block_hashes,
            peer_height,
        }) => {
            assert_eq!(network_id, "net-1");
            assert!(block_hashes.is_empty());
            assert_eq!(peer_height, 0);
        }
        other => unreachable!("test: expected SyncResponse, got {:?}", other),
    }
}

#[test]
fn test_dispatch_sync_request_with_provider() {
    let mut sm = make_sync_manager();
    sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join");

    let mut rp = make_reflector_pool();
    let provider = FakeBlockProvider { chain_height: 20 };

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: Some(&provider),
    };

    let msg = MatrixMessage::SyncRequest {
        network_id: "net-1".to_string(),
        from_height: 5,
        max_blocks: 10,
    };

    let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
    match resp {
        DispatchResponse::Reply(MatrixMessage::SyncResponse {
            block_hashes,
            peer_height,
            ..
        }) => {
            assert_eq!(block_hashes.len(), 10);
            assert_eq!(block_hashes[0], "hash_5");
            assert_eq!(peer_height, 20);
        }
        other => unreachable!("test: expected SyncResponse, got {:?}", other),
    }
}

#[test]
fn test_dispatch_sync_request_unknown_network() {
    let mut sm = make_sync_manager();
    let mut rp = make_reflector_pool();

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    let msg = MatrixMessage::SyncRequest {
        network_id: "unknown".to_string(),
        from_height: 0,
        max_blocks: 5,
    };

    let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
    assert!(matches!(resp, DispatchResponse::None));
}

#[test]
fn test_dispatch_sync_response_updates_state() {
    let mut sm = make_sync_manager();
    sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join");

    let mut rp = make_reflector_pool();

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    let msg = MatrixMessage::SyncResponse {
        network_id: "net-1".to_string(),
        block_hashes: Vec::new(),
        peer_height: 42,
    };

    let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
    assert!(matches!(resp, DispatchResponse::None));

    // SyncManager should now be Synchronized at height 42
    use crate::blockchain::sync_manager::SyncState;
    assert_eq!(
        sm.sync_state("net-1"),
        Some(&SyncState::Synchronized {
            last_block_height: 42
        })
    );
}

#[test]
fn test_dispatch_sync_announce_triggers_resync() {
    let config = SyncConfig {
        max_block_lag: 5,
        ..SyncConfig::default()
    };
    let mut sm = SyncManager::new("dev".to_string(), config);
    sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join");
    sm.update_sync_state(
        "net-1",
        crate::blockchain::sync_manager::SyncState::Synchronized {
            last_block_height: 10,
        },
    )
    .expect("test: set synced");

    let mut rp = make_reflector_pool();

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    let msg = MatrixMessage::SyncAnnounce {
        network_id: "net-1".to_string(),
        block_height: 100,
        block_hash: "abc".to_string(),
    };

    let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
    assert!(matches!(resp, DispatchResponse::None));

    // Should have transitioned to Syncing
    use crate::blockchain::sync_manager::SyncState;
    assert!(matches!(
        sm.sync_state("net-1"),
        Some(SyncState::Syncing { .. })
    ));
}

// ------------------------------------------------------------------
// Gap 2 tests: ReflectorPool receives heartbeats
// ------------------------------------------------------------------

#[test]
fn test_dispatch_reflector_heartbeat_registers() {
    let mut sm = make_sync_manager();
    let mut rp = make_reflector_pool();

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    let msg = MatrixMessage::ReflectorHeartbeat {
        network_id: "net-1".to_string(),
        block_height: 50,
        health_score: 0.8,
    };

    let pos = MatrixPosition {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };

    let resp = dispatcher.dispatch(msg, "reflector-node-1", pos);
    assert!(matches!(resp, DispatchResponse::None));

    assert_eq!(rp.total_count("net-1"), 1);
    let best = rp.get_best_reflectors("net-1", 1);
    assert_eq!(best.len(), 1);
    assert_eq!(best[0].node_id, "reflector-node-1");
    assert_eq!(best[0].block_height, 50);
}

#[test]
fn test_dispatch_reflector_heartbeat_updates_existing() {
    let mut sm = make_sync_manager();
    let mut rp = make_reflector_pool();

    let pos = MatrixPosition {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };

    // First heartbeat
    {
        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };
        let msg = MatrixMessage::ReflectorHeartbeat {
            network_id: "net-1".to_string(),
            block_height: 10,
            health_score: 0.5,
        };
        dispatcher.dispatch(msg, "node-A", pos);
    }

    // Second heartbeat with updated data
    {
        let mut dispatcher = SyncDispatcher {
            sync_manager: &mut sm,
            reflector_pool: &mut rp,
            block_provider: None,
        };
        let msg = MatrixMessage::ReflectorHeartbeat {
            network_id: "net-1".to_string(),
            block_height: 25,
            health_score: 0.9,
        };
        dispatcher.dispatch(msg, "node-A", pos);
    }

    // Still one reflector, with updated values
    assert_eq!(rp.total_count("net-1"), 1);
    let best = rp.get_best_reflectors("net-1", 1);
    assert_eq!(best[0].block_height, 25);
    assert!((best[0].health_score - 0.9).abs() < f64::EPSILON);
}

/// S3.0/B3: a hash-only provider can no longer answer a GenesisRequest at all.
/// It used to reply with `hash_0` in a field called `genesis_block_json` — a
/// response no peer could verify or adopt. Declining is the correct answer.
#[test]
fn test_dispatch_genesis_request_hash_only_provider_declines() {
    let mut sm = make_sync_manager();
    sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join");

    let mut rp = make_reflector_pool();
    let provider = FakeBlockProvider { chain_height: 10 };

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: Some(&provider),
    };

    let msg = MatrixMessage::GenesisRequest {
        network_id: "net-1".to_string(),
    };

    assert!(matches!(
        dispatcher.dispatch(msg, "peer-1", zero_position()),
        DispatchResponse::None
    ));
}

/// S3.0/B3: the full round trip. Node A serves its REAL genesis block; node B
/// dispatches the response, deserializes it, verifies the hash and records it
/// as the network's root — without touching its own chain.
#[test]
fn test_genesis_request_response_round_trips_a_real_block() {
    let coord_a = MatrixCoordinate::new(3, 1, 4).expect("test: valid coord");
    let genesis_a = Block::genesis(coord_a);
    let provider = crate::blockchain::sync_manager::NodeBlockchainBlockProvider::from_blocks(
        std::slice::from_ref(&genesis_a),
    );

    // --- Node A: serve the genesis ---
    let mut sm_a = make_sync_manager();
    sm_a.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join");
    let mut rp_a = make_reflector_pool();
    let mut dispatcher_a = SyncDispatcher {
        sync_manager: &mut sm_a,
        reflector_pool: &mut rp_a,
        block_provider: Some(&provider),
    };

    let reply = dispatcher_a.dispatch(
        MatrixMessage::GenesisRequest {
            network_id: "net-1".to_string(),
        },
        "peer-b",
        zero_position(),
    );

    let (network_id, genesis_block_json) = match reply {
        DispatchResponse::Reply(MatrixMessage::GenesisResponse {
            network_id,
            genesis_block_json,
        }) => (network_id, genesis_block_json),
        other => unreachable!("test: expected GenesisResponse, got {:?}", other),
    };

    // The payload is a real Block, not a hash.
    let wire: Block =
        serde_json::from_str(&genesis_block_json).expect("test: response carries a Block");
    assert_eq!(wire.hash, genesis_a.hash);
    assert!(wire.verify_hash());
    assert!(wire.is_genesis());

    // --- Node B: receive it ---
    let coord_b = MatrixCoordinate::new(-2, 7, 0).expect("test: valid coord");
    let own_genesis = Block::genesis(coord_b);
    assert_ne!(own_genesis.hash, genesis_a.hash);

    let mut sm_b = make_sync_manager();
    let mut rp_b = make_reflector_pool();
    let mut dispatcher_b = SyncDispatcher {
        sync_manager: &mut sm_b,
        reflector_pool: &mut rp_b,
        block_provider: None,
    };

    assert!(matches!(
        dispatcher_b.dispatch(
            MatrixMessage::GenesisResponse {
                network_id: network_id.clone(),
                genesis_block_json,
            },
            "peer-a",
            zero_position(),
        ),
        DispatchResponse::None
    ));

    let recorded = sm_b
        .network_genesis(&network_id)
        .expect("test: B recorded the network genesis");
    assert_eq!(recorded.hash, genesis_a.hash);
}

/// S3.0/B3: a tampered genesis is rejected on receipt, not recorded.
#[test]
fn test_genesis_response_with_tampered_block_is_rejected() {
    let coord = MatrixCoordinate::new(3, 1, 4).expect("test: valid coord");
    let mut tampered = Block::genesis(coord);
    tampered.hash = "0".repeat(64);

    let mut sm = make_sync_manager();
    let mut rp = make_reflector_pool();
    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    dispatcher.dispatch(
        MatrixMessage::GenesisResponse {
            network_id: "net-1".to_string(),
            genesis_block_json: serde_json::to_string(&tampered).expect("test: serialize"),
        },
        "peer-a",
        zero_position(),
    );

    assert!(
        sm.network_genesis("net-1").is_none(),
        "a block whose hash does not verify must never be recorded as a root",
    );
}

#[test]
fn test_dispatch_genesis_request_without_provider() {
    let mut sm = make_sync_manager();
    let mut rp = make_reflector_pool();

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    let msg = MatrixMessage::GenesisRequest {
        network_id: "net-1".to_string(),
    };

    let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
    assert!(matches!(resp, DispatchResponse::None));
}

#[test]
fn test_dispatch_header_request_with_provider() {
    let mut sm = make_sync_manager();
    sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join");

    let mut rp = make_reflector_pool();
    let provider = FakeBlockProvider { chain_height: 20 };

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: Some(&provider),
    };

    let msg = MatrixMessage::HeaderRequest {
        network_id: "net-1".to_string(),
        from_height: 5,
        max_count: 3,
    };

    let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
    match resp {
        DispatchResponse::Reply(MatrixMessage::HeaderResponse {
            network_id,
            headers_json,
            peer_height,
        }) => {
            assert_eq!(network_id, "net-1");
            assert_eq!(headers_json.len(), 3);
            assert_eq!(peer_height, 20);
        }
        other => unreachable!("test: expected HeaderResponse, got {:?}", other),
    }
}

#[test]
fn test_dispatch_sync_block_request() {
    let mut sm = make_sync_manager();
    let mut rp = make_reflector_pool();

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    let msg = MatrixMessage::SyncBlockRequest {
        network_id: "net-1".to_string(),
        block_hashes: vec!["hash_a".to_string()],
    };

    let resp = dispatcher.dispatch(msg, "peer-1", zero_position());
    match resp {
        DispatchResponse::Reply(MatrixMessage::SyncBlockResponse {
            network_id,
            blocks_json,
        }) => {
            assert_eq!(network_id, "net-1");
            assert!(blocks_json.is_empty());
        }
        other => unreachable!("test: expected SyncBlockResponse, got {:?}", other),
    }
}

#[test]
fn test_dispatch_response_variants_return_none() {
    let mut sm = make_sync_manager();
    let mut rp = make_reflector_pool();

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    // GenesisResponse
    let msg = MatrixMessage::GenesisResponse {
        network_id: "net-1".to_string(),
        genesis_block_json: "{}".to_string(),
    };
    assert!(matches!(
        dispatcher.dispatch(msg, "peer-1", zero_position()),
        DispatchResponse::None
    ));

    // HeaderResponse
    let msg = MatrixMessage::HeaderResponse {
        network_id: "net-1".to_string(),
        headers_json: vec![],
        peer_height: 0,
    };
    assert!(matches!(
        dispatcher.dispatch(msg, "peer-1", zero_position()),
        DispatchResponse::None
    ));

    // SyncBlockResponse
    let msg = MatrixMessage::SyncBlockResponse {
        network_id: "net-1".to_string(),
        blocks_json: vec![],
    };
    assert!(matches!(
        dispatcher.dispatch(msg, "peer-1", zero_position()),
        DispatchResponse::None
    ));
}

#[test]
fn test_dispatch_ignores_non_sync_messages() {
    let mut sm = make_sync_manager();
    let mut rp = make_reflector_pool();

    let mut dispatcher = SyncDispatcher {
        sync_manager: &mut sm,
        reflector_pool: &mut rp,
        block_provider: None,
    };

    let msg = MatrixMessage::Heartbeat {
        coordinate: crate::matrix::coordinate::MatrixCoordinate::new(0, 0, 0)
            .expect("test: valid coord"),
        timestamp: 12345,
    };

    let resp = dispatcher.dispatch(msg, "peer", zero_position());
    assert!(matches!(resp, DispatchResponse::None));
}

// ------------------------------------------------------------------
// Gap 4 tests: SyncObserver notification
// ------------------------------------------------------------------

#[test]
fn test_sync_observer_notified_on_completion() {
    use crate::blockchain::sync_manager::SyncObserver;
    use std::sync::{Arc, Mutex};

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
    let observer = TestObserver {
        events: events.clone(),
    };

    let mut sm = make_sync_manager();
    sm.set_observer(Box::new(observer));
    sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join");

    // Process an empty SyncResponse (triggers Synchronized)
    sm.process_sync_message(SyncMessage::Response {
        network_id: "net-1".to_string(),
        block_hashes: Vec::new(),
        peer_height: 99,
    });

    let captured = events.lock().expect("test: lock");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].0, "net-1");
    assert_eq!(captured[0].1, 99);
}

// ------------------------------------------------------------------
// Gap 1 tests: SyncManager uses BlockProvider
// ------------------------------------------------------------------

#[test]
fn test_sync_manager_with_block_provider() {
    let mut sm = make_sync_manager();
    sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join");

    let provider = FakeBlockProvider { chain_height: 50 };

    let request = SyncMessage::Request {
        network_id: "net-1".to_string(),
        from_height: 10,
        max_blocks: 20,
    };

    let response = sm.process_sync_message_with_provider(request, Some(&provider));
    match response {
        Some(SyncMessage::Response {
            block_hashes,
            peer_height,
            ..
        }) => {
            assert_eq!(block_hashes.len(), 20);
            assert_eq!(block_hashes[0], "hash_10");
            assert_eq!(block_hashes[19], "hash_29");
            assert_eq!(peer_height, 50);
        }
        other => unreachable!("test: expected Response, got {:?}", other),
    }
}

// ------------------------------------------------------------------
// TransportSyncDriver helper tests
// ------------------------------------------------------------------

#[test]
fn test_resolve_reflector_addr_found() {
    let mut node_map = HashMap::new();
    let addr: SocketAddr = "[::1]:9292".parse().expect("test: parse addr");
    node_map.insert("1,2,3".to_string(), ("node-abc".to_string(), addr));

    let reflector = Reflector {
        node_id: "node-abc".to_string(),
        position: zero_position(),
        last_seen: 0,
        block_height: 0,
        health_score: 1.0,
        privacy_mode: PrivacyMode::PUBLIC,
    };

    let result = resolve_reflector_addr(&reflector, &node_map);
    assert!(result.is_ok());
    assert_eq!(result.expect("test: addr"), addr);
}

#[test]
fn test_resolve_reflector_addr_not_found() {
    let node_map = HashMap::new();

    let reflector = Reflector {
        node_id: "missing-node".to_string(),
        position: zero_position(),
        last_seen: 0,
        block_height: 0,
        health_score: 1.0,
        privacy_mode: PrivacyMode::PUBLIC,
    };

    let result = resolve_reflector_addr(&reflector, &node_map);
    assert!(result.is_err());
}

#[test]
fn test_block_fetch_request_serialization() {
    let msg = MatrixMessage::BlockFetchRequest {
        block_hashes: vec!["abc123".to_string(), "def456".to_string()],
    };
    let json = serde_json::to_string(&msg).expect("test: serialize");
    let parsed: MatrixMessage =
        serde_json::from_str(&json).expect("test: deserialize");
    match parsed {
        MatrixMessage::BlockFetchRequest { block_hashes } => {
            assert_eq!(block_hashes.len(), 2);
            assert_eq!(block_hashes[0], "abc123");
            assert_eq!(block_hashes[1], "def456");
        }
        other => unreachable!("test: expected BlockFetchRequest, got {:?}", other),
    }
}

#[test]
fn test_block_fetch_response_serialization() {
    let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
    let block = Block::genesis(coord);
    let block_json =
        serde_json::to_string(&block).expect("test: serialize block");

    let msg = MatrixMessage::BlockFetchResponse {
        blocks: vec![block_json.clone()],
    };
    let json = serde_json::to_string(&msg).expect("test: serialize");
    let parsed: MatrixMessage =
        serde_json::from_str(&json).expect("test: deserialize");
    match parsed {
        MatrixMessage::BlockFetchResponse { blocks } => {
            assert_eq!(blocks.len(), 1);
            let deserialized: Block =
                serde_json::from_str(&blocks[0]).expect("test: deserialize block");
            assert_eq!(deserialized.index, block.index);
            assert!(deserialized.verify_hash());
        }
        other => unreachable!("test: expected BlockFetchResponse, got {:?}", other),
    }
}

#[tokio::test]
async fn test_filter_missing_hashes() {
    let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
    let blockchain = NodeBlockchain::new(coord);
    let chain = blockchain.get_chain().await;
    let genesis_hash = chain.first().map(|b| b.hash.clone()).unwrap_or_default();

    let hashes = vec![
        genesis_hash.clone(),
        "nonexistent_hash_1".to_string(),
        "nonexistent_hash_2".to_string(),
    ];

    let missing = filter_missing_hashes(&hashes, &blockchain).await;
    // Genesis hash exists, the other two do not
    assert_eq!(missing.len(), 2);
    assert!(!missing.contains(&genesis_hash));
    assert!(missing.contains(&"nonexistent_hash_1".to_string()));
    assert!(missing.contains(&"nonexistent_hash_2".to_string()));
}

#[tokio::test]
async fn test_run_sync_round_no_reflectors_returns_empty() {
    let mut sm = make_sync_manager();
    sm.join_network("net-1".to_string(), PrivacyMode::PUBLIC, 100)
        .expect("test: join");

    let mut rp = make_reflector_pool();
    let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
    let blockchain = NodeBlockchain::new(coord);
    let node_map = HashMap::new();

    // Cannot create a real StoqTransport in unit tests without binding,
    // but with no reflectors the driver never connects.
    let config = stoq::TransportConfig {
        port: 0,
        bind_address: std::net::Ipv6Addr::LOCALHOST,
        ..stoq::TransportConfig::default()
    };
    let transport = match stoq::StoqTransport::new(config).await {
        Ok(t) => t,
        Err(_) => return, // Skip if socket binding fails
    };

    let blocks = TransportSyncDriver::run_sync_round(
        &mut sm,
        &mut rp,
        &blockchain,
        &transport,
        &node_map,
        &coord,
    )
    .await;

    assert!(blocks.is_empty(), "No reflectors means no blocks fetched");
}
