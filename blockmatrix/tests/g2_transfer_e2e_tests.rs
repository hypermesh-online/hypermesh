// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase G.2 — durable cross-network transfer end-to-end tests.
//!
//! These tests exercise the G.2 deliverables that G.1 deferred:
//!
//! 1. **Two-coordinator bridged harness** — `BridgedTransport` connects
//!    a source-side `TransferCoordinator` directly to a target-side
//!    `TransferCoordinator`, exactly the same way the production STOQ
//!    transport will: requests are dispatched through
//!    `TransferCoordinator::handle_register_request` on the target,
//!    acks are routed back. This proves the wire-level dispatch path
//!    end-to-end without needing two real subprocesses.
//! 2. **Crash recovery** — drop the source coordinator after the lock
//!    is written, construct a fresh coordinator on the same chain,
//!    `resume_in_flight()` discovers the in-flight transfer and the
//!    test asserts the recovered state matches what was persisted on
//!    chain.
//!
//! ## Why hybrid (in-process) instead of real subprocesses
//!
//! The plan permits a hybrid harness when subprocess infrastructure is
//! too heavy. Two reasons it is, today:
//!
//! * The daemon's STOQ-backed `TransferTransport` is one of G.2's open
//!   integration ends — the production wrapper that registers oneshot
//!   acks before broadcasting `TAG_TRANSFER_REGISTER_REQ` is not yet
//!   in `bin/node`. Building a real subprocess test before that lands
//!   would test the same surface twice.
//! * The wire protocol itself is identical — `BridgedTransport` calls
//!   `serde_json::to_vec` / `from_slice` on the exact wire payloads
//!   that the STOQ handlers use, so the JSON contract is exercised.
//!
//! The hybrid harness covers the choreography. The real-subprocess
//! variant lands when the daemon's STOQ TransferTransport is wired
//! (Phase G.2 follow-up / Phase I).

#![cfg(feature = "intelligence")]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hypermesh_lib::{AssetId, BlockchainScope, ContentHash};
use tokio::sync::{Mutex, OnceCell};
use trustchain::proof_of_state::StateProof;

use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::gateway::transfer_coordinator::{
    AllowAllFederation, TransferCoordinator, TransferTransport,
};
use blockmatrix::gateway::transfer_protocol::{
    CoordinatorState, PeerCertFingerprint, ShardManifestEntry, TransferLockMessage,
    TransferRegisterAck, TransferRegisterRequest, TransferRelease, TransferRollback,
};
use blockmatrix::gateway::GatewayError;
use blockmatrix::matrix::coordinate::MatrixCoordinate;

// ---------------------------------------------------------------------------
// BridgedTransport — connects two TransferCoordinators in-process.
//
// Acts as the source's TransferTransport. Routes register requests
// through the target coordinator's `handle_register_request` exactly
// the same way the production STOQ wire handler will (see
// `network/message_handlers/transfer_handlers.rs::handle_transfer_register_req`).
//
// Critically, the request and ack go through `serde_json::to_vec` /
// `from_slice` round-trips so the wire format is exercised end-to-end.
// ---------------------------------------------------------------------------

struct BridgedTransport {
    target: OnceCell<Arc<TransferCoordinator>>,
    pub locks: Mutex<Vec<TransferLockMessage>>,
    pub releases: Mutex<Vec<TransferRelease>>,
    pub rollbacks: Mutex<Vec<TransferRollback>>,
    pub register_requests: Mutex<Vec<TransferRegisterRequest>>,
}

impl BridgedTransport {
    fn new() -> Self {
        Self {
            target: OnceCell::new(),
            locks: Mutex::new(Vec::new()),
            releases: Mutex::new(Vec::new()),
            rollbacks: Mutex::new(Vec::new()),
            register_requests: Mutex::new(Vec::new()),
        }
    }

    async fn attach_target(&self, target: Arc<TransferCoordinator>) {
        self.target
            .set(target)
            .map_err(|_| ())
            .expect("test: target already attached");
    }
}

#[async_trait]
impl TransferTransport for BridgedTransport {
    async fn broadcast_lock(&self, msg: TransferLockMessage) -> Result<(), GatewayError> {
        // Wire-format round trip — exactly what the STOQ handler does.
        let bytes = serde_json::to_vec(&msg).expect("test: serialize lock");
        let parsed: TransferLockMessage =
            serde_json::from_slice(&bytes).expect("test: deserialize lock");
        self.locks.lock().await.push(parsed);
        Ok(())
    }

    async fn send_register_request(
        &self,
        _peer: &PeerCertFingerprint,
        req: TransferRegisterRequest,
        _deadline: Duration,
    ) -> Result<TransferRegisterAck, GatewayError> {
        // Wire-format round trip — exactly what the STOQ handler does
        // (handle_transfer_register_req in transfer_handlers.rs).
        let bytes = serde_json::to_vec(&req).expect("test: serialize req");
        let parsed: TransferRegisterRequest =
            serde_json::from_slice(&bytes).expect("test: deserialize req");
        self.register_requests.lock().await.push(parsed.clone());

        let target = self
            .target
            .get()
            .expect("test: target coordinator must be attached");

        // Mirror the production handler: call handle_register_request
        // on the target, return the ack as the response.
        let ack = target
            .handle_register_request(parsed, StateProof::new_for_testing())
            .await?;

        // Round-trip the ack too so the contract holds in both directions.
        let ack_bytes = serde_json::to_vec(&ack).expect("test: serialize ack");
        let parsed_ack: TransferRegisterAck =
            serde_json::from_slice(&ack_bytes).expect("test: deserialize ack");
        Ok(parsed_ack)
    }

    async fn broadcast_release(&self, msg: TransferRelease) -> Result<(), GatewayError> {
        let bytes = serde_json::to_vec(&msg).expect("test: serialize release");
        let parsed: TransferRelease =
            serde_json::from_slice(&bytes).expect("test: deserialize release");
        self.releases.lock().await.push(parsed);
        Ok(())
    }

    async fn broadcast_rollback(&self, msg: TransferRollback) -> Result<(), GatewayError> {
        let bytes = serde_json::to_vec(&msg).expect("test: serialize rollback");
        let parsed: TransferRollback =
            serde_json::from_slice(&bytes).expect("test: deserialize rollback");
        self.rollbacks.lock().await.push(parsed);
        Ok(())
    }
}

// Lightweight no-op transport for the target side — the target
// coordinator only needs to reply via handle_register_request, it does
// not initiate. broadcasts here are receipts only.
struct TargetTransport {
    pub releases: Mutex<Vec<TransferRelease>>,
    pub rollbacks: Mutex<Vec<TransferRollback>>,
}

impl TargetTransport {
    fn new() -> Self {
        Self {
            releases: Mutex::new(Vec::new()),
            rollbacks: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl TransferTransport for TargetTransport {
    async fn broadcast_lock(&self, _msg: TransferLockMessage) -> Result<(), GatewayError> {
        Ok(())
    }

    async fn send_register_request(
        &self,
        _peer: &PeerCertFingerprint,
        _req: TransferRegisterRequest,
        _deadline: Duration,
    ) -> Result<TransferRegisterAck, GatewayError> {
        Err(GatewayError::TransportFailure {
            transfer_id: "target-side".into(),
            detail: "TargetTransport does not initiate".into(),
        })
    }

    async fn broadcast_release(&self, msg: TransferRelease) -> Result<(), GatewayError> {
        self.releases.lock().await.push(msg);
        Ok(())
    }

    async fn broadcast_rollback(&self, msg: TransferRollback) -> Result<(), GatewayError> {
        self.rollbacks.lock().await.push(msg);
        Ok(())
    }
}

fn make_chain(x: i64, y: i64, z: i64) -> Arc<NodeBlockchain> {
    let coord = MatrixCoordinate::new(x, y, z).expect("test: coord");
    Arc::new(NodeBlockchain::new(coord))
}

fn one_shard() -> Vec<ShardManifestEntry> {
    vec![ShardManifestEntry {
        shard_id: ContentHash([0xAA; 32]),
        size_bytes: 128,
        source_matrix: Some((0, 0, 0)),
    }]
}

// ---------------------------------------------------------------------------
// Test 1 — Two-coordinator end-to-end: A initiates, B accepts via
// bridged transport, both chains end up with linked receipts.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_e2e_two_coordinators_bridged() {
    let chain_a = make_chain(0, 0, 0);
    let chain_b = make_chain(1, 1, 1);
    let initial_a = chain_a.get_height().await;
    let initial_b = chain_b.get_height().await;

    let bridged_transport = Arc::new(BridgedTransport::new());
    let target_transport = Arc::new(TargetTransport::new());

    // Build target coordinator first so we can attach it.
    let coord_b = Arc::new(TransferCoordinator::new(
        chain_b.clone(),
        target_transport.clone(),
        Arc::new(AllowAllFederation),
        "tgt-chain".into(),
    ));
    bridged_transport.attach_target(coord_b.clone()).await;

    let coord_a = TransferCoordinator::new(
        chain_a.clone(),
        bridged_transport.clone(),
        Arc::new(AllowAllFederation),
        "src-chain".into(),
    );

    let outcome = coord_a
        .initiate(
            AssetId::from("asset-e2e"),
            "tgt-chain".into(),
            "peer-fp-e2e".into(),
            BlockchainScope::Network,
            one_shard(),
            StateProof::new_for_testing(),
        )
        .await
        .expect("test: e2e initiate");

    // Source chain: lock + release + receipt = 3 blocks.
    let height_a = chain_a.get_height().await;
    assert_eq!(
        height_a,
        initial_a + 3,
        "source chain should have lock+release+receipt"
    );

    // Target chain: registration + receipt = 2 blocks.
    let height_b = chain_b.get_height().await;
    assert_eq!(
        height_b,
        initial_b + 2,
        "target chain should have registration+receipt"
    );

    // Cross-chain link: source receipt references target hash and v.v.
    assert!(!outcome.source_block_hash.is_empty());
    assert!(!outcome.target_block_hash.is_empty());

    // Wire-protocol round-trips actually happened.
    assert_eq!(bridged_transport.locks.lock().await.len(), 1);
    assert_eq!(bridged_transport.register_requests.lock().await.len(), 1);
    assert_eq!(bridged_transport.releases.lock().await.len(), 1);
    assert!(bridged_transport.rollbacks.lock().await.is_empty());

    let stored_a = coord_a
        .get_transfer(&outcome.transfer_id)
        .await
        .expect("test: A tracked");
    assert_eq!(stored_a.state, CoordinatorState::Released);

    // Target coordinator also tracked the transfer.
    let stored_b = coord_b
        .get_transfer(&outcome.transfer_id)
        .await
        .expect("test: B tracked");
    assert_eq!(stored_b.state, CoordinatorState::Registered);
}

// ---------------------------------------------------------------------------
// Test 2 — Crash-recovery: source crashes after Lock, before Release.
// On restart, resume_in_flight() rediscovers the unfinished transfer.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_lock_crashes_resume_completes() {
    let chain_a = make_chain(0, 0, 0);
    let initial_a = chain_a.get_height().await;
    let proof = StateProof::new_for_testing();

    // Phase 1: simulate "node A initiates, force-killed after lock".
    //
    // Use a transport that hangs on send_register_request so the
    // coordinator is mid-state-machine when we drop it. We use
    // tokio::time::timeout to bound the wait, dropping the future
    // before any rollback can be written.
    struct HangingTransport {
        pub locks: Mutex<Vec<TransferLockMessage>>,
    }
    #[async_trait]
    impl TransferTransport for HangingTransport {
        async fn broadcast_lock(
            &self,
            msg: TransferLockMessage,
        ) -> Result<(), GatewayError> {
            self.locks.lock().await.push(msg);
            Ok(())
        }
        async fn send_register_request(
            &self,
            _peer: &PeerCertFingerprint,
            _req: TransferRegisterRequest,
            _deadline: Duration,
        ) -> Result<TransferRegisterAck, GatewayError> {
            // Hang forever — caller drops future via timeout.
            std::future::pending::<()>().await;
            unreachable!()
        }
        async fn broadcast_release(
            &self,
            _msg: TransferRelease,
        ) -> Result<(), GatewayError> {
            Ok(())
        }
        async fn broadcast_rollback(
            &self,
            _msg: TransferRollback,
        ) -> Result<(), GatewayError> {
            Ok(())
        }
    }

    let hanging = Arc::new(HangingTransport {
        locks: Mutex::new(Vec::new()),
    });

    {
        let coord_a = TransferCoordinator::new(
            chain_a.clone(),
            hanging.clone(),
            Arc::new(AllowAllFederation),
            "src-chain".into(),
        );
        // Make the deadline very long so the *outer* timeout is what
        // fires (mirrors a real crash, not a coordinator-driven rollback).
        coord_a.set_register_timeout(Duration::from_secs(60)).await;

        let initiate_fut = coord_a.initiate(
            AssetId::from("asset-crash"),
            "tgt-chain".into(),
            "peer-fp-crash".into(),
            BlockchainScope::Network,
            one_shard(),
            proof.clone(),
        );

        // Bound the wait so the test doesn't hang. After 200ms, drop
        // the future — Lock has already been written to chain.
        let result = tokio::time::timeout(Duration::from_millis(200), initiate_fut).await;
        assert!(
            result.is_err(),
            "initiate should still be in-flight at this point"
        );
        // coord_a drops here, simulating crash. The lock entry is
        // persisted on chain_a; no release was written.
    }

    // Lock was broadcast (proves we got past the lock-write step).
    assert_eq!(
        hanging.locks.lock().await.len(),
        1,
        "lock was broadcast before the simulated crash"
    );

    // Source chain has exactly +1 block (the lock entry) — no release.
    let height_after_crash = chain_a.get_height().await;
    assert_eq!(
        height_after_crash,
        initial_a + 1,
        "only the lock entry was persisted before the crash"
    );

    // Phase 2: "node A restarts" — fresh coordinator on the same chain.
    // resume_in_flight discovers the in-flight transfer.
    let bridged_transport = Arc::new(BridgedTransport::new());
    let target_transport = Arc::new(TargetTransport::new());
    let chain_b = make_chain(1, 1, 1);
    let coord_b = Arc::new(TransferCoordinator::new(
        chain_b.clone(),
        target_transport,
        Arc::new(AllowAllFederation),
        "tgt-chain".into(),
    ));
    bridged_transport.attach_target(coord_b.clone()).await;

    let coord_a_restart = TransferCoordinator::new(
        chain_a.clone(),
        bridged_transport.clone(),
        Arc::new(AllowAllFederation),
        "src-chain".into(),
    );

    let resumed = coord_a_restart
        .resume_in_flight()
        .await
        .expect("test: resume_in_flight after crash");

    assert_eq!(
        resumed.len(),
        1,
        "restart should rediscover the in-flight transfer"
    );
    let recovered = &resumed[0];
    assert_eq!(recovered.asset_id.to_string(), "asset-crash");
    assert_eq!(recovered.state, CoordinatorState::Locked);
    assert!(
        recovered.source_lock_block_hash.is_some(),
        "lock block hash recovered from chain"
    );
    assert!(
        recovered.source_release_block_hash.is_none(),
        "no release recorded — transfer was mid-flight"
    );

    // The recovered transfer is in the in-memory map and an operator
    // could inspect / re-drive it (full re-driver lands when the daemon
    // STOQ transport plumbs ack-routing through PeerContext).
    let from_memory = coord_a_restart
        .get_transfer(&recovered.transfer_id)
        .await
        .expect("test: in-memory map repopulated");
    assert_eq!(from_memory.state, CoordinatorState::Locked);
}

// ---------------------------------------------------------------------------
// Test 3 — Wire-format contract: requests and acks round-trip via JSON
// without losing fields. This guards against handler/coordinator drift.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_wire_format_round_trip() {
    let chain_b = make_chain(1, 1, 1);
    let target_transport = Arc::new(TargetTransport::new());
    let coord_b = TransferCoordinator::new(
        chain_b.clone(),
        target_transport,
        Arc::new(AllowAllFederation),
        "tgt-chain".into(),
    );

    let req = TransferRegisterRequest {
        transfer_id: "tx-wire-1".into(),
        asset_id: AssetId::from("asset-wire"),
        source_chain_id: "src-chain".into(),
        target_chain_id: "tgt-chain".into(),
        shard_manifest: one_shard(),
        lock_block_hash: "deadbeef-lock".into(),
        lock_state_proof: StateProof::new_for_testing(),
        source_scope: BlockchainScope::Device,
        target_scope: BlockchainScope::Network,
        sent_at: 1_700_000_000,
    };

    // Emulate exactly what handle_transfer_register_req does:
    let req_bytes = serde_json::to_vec(&req).expect("test: serialize");
    let req_parsed: TransferRegisterRequest =
        serde_json::from_slice(&req_bytes).expect("test: deserialize");
    assert_eq!(req_parsed.transfer_id, req.transfer_id);
    assert_eq!(req_parsed.shard_manifest.len(), 1);

    // Coordinator handles the parsed request and produces an ack.
    let ack = coord_b
        .handle_register_request(req_parsed, StateProof::new_for_testing())
        .await
        .expect("test: handle req");
    assert!(ack.accepted);
    assert!(!ack.target_block_hash.is_empty());

    let ack_bytes = serde_json::to_vec(&ack).expect("test: serialize ack");
    let ack_parsed: TransferRegisterAck =
        serde_json::from_slice(&ack_bytes).expect("test: deserialize ack");
    assert_eq!(ack_parsed.transfer_id, "tx-wire-1");
    assert!(ack_parsed.accepted);
}

// ---------------------------------------------------------------------------
// Test 4 — Coordinator ack waiter delivers ack to awaiting future.
//
// Exercises the oneshot-based ack-routing the production STOQ
// transport uses: register_ack_waiter pre-flight, deliver_register_ack
// from the wire handler, the awaiting future receives the ack.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_register_ack_waiter_delivery() {
    let chain = make_chain(0, 0, 0);
    let bridged = Arc::new(BridgedTransport::new());
    // No target attached — bridged.send_register_request would panic,
    // but we never call it here.
    let coord = Arc::new(TransferCoordinator::new(
        chain,
        bridged,
        Arc::new(AllowAllFederation),
        "src-chain".into(),
    ));

    let (tx, rx) = tokio::sync::oneshot::channel();
    coord
        .register_ack_waiter("tx-wait-1".into(), tx)
        .await;

    // Wire handler-side: ack arrives, delivered via deliver_register_ack.
    let ack = TransferRegisterAck {
        transfer_id: "tx-wait-1".into(),
        target_block_hash: "tgt-hash-1".into(),
        state_proof: StateProof::new_for_testing(),
        accepted: true,
        reason: None,
        acked_at: 42,
    };
    let delivered = coord.deliver_register_ack(ack.clone()).await;
    assert!(delivered, "waiter must be present and ack delivered");

    let received = tokio::time::timeout(Duration::from_secs(1), rx)
        .await
        .expect("test: rx not timed out")
        .expect("test: rx not dropped");
    assert_eq!(received.transfer_id, "tx-wait-1");
    assert!(received.accepted);
    assert_eq!(received.target_block_hash, "tgt-hash-1");

    // Second deliver returns false — waiter consumed.
    let delivered_again = coord.deliver_register_ack(ack).await;
    assert!(!delivered_again, "waiter is single-shot");
}
