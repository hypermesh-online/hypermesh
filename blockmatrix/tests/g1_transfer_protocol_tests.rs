// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase G.1 — cross-network transfer choreography integration tests.
//!
//! Covers the five wire-protocol scenarios described in the plan:
//!
//! 1. Happy path A → B with linked TransferReceipts on both chains.
//! 2. Target rejection rolls back, source's lock is followed by a
//!    rollback entry, original asset still accessible.
//! 3. Register-ack timeout rolls back automatically.
//! 4. In-flight state surfaces from `resume_in_flight` (Phase G.2 stub
//!    today — test guards the contract).
//! 5. Federation gating rejects unfederated peers before any wire
//!    traffic.
//!
//! Tests use the in-process `MockTransport` defined alongside
//! `TransferCoordinator` so they are self-contained and do not require
//! a multi-host harness (that is Phase I).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hypermesh_lib::{AssetId, BlockchainScope, ContentHash};
use tokio::sync::Mutex;
use trustchain::proof_of_state::StateProof;

use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::gateway::transfer_coordinator::{
    AllowAllFederation, DenyAllFederation, TransferCoordinator, TransferTransport,
};
use blockmatrix::gateway::transfer_protocol::{
    CoordinatorState, PeerCertFingerprint, ShardManifestEntry, TransferLockMessage,
    TransferRegisterAck, TransferRegisterRequest, TransferRelease, TransferRollback,
};
use blockmatrix::gateway::{GatewayError, RollbackReason};
use blockmatrix::matrix::coordinate::MatrixCoordinate;

// ---------------------------------------------------------------------------
// Shared test transport (in-process, captures sent messages).
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Mode {
    AcceptDefault,
    Reject(String),
    Silent,
}

struct ProgrammableTransport {
    mode: Mutex<Mode>,
    pub locks: Mutex<Vec<TransferLockMessage>>,
    pub releases: Mutex<Vec<TransferRelease>>,
    pub rollbacks: Mutex<Vec<TransferRollback>>,
    pub register_seen: Mutex<Vec<TransferRegisterRequest>>,
    pub target_chain: Mutex<Option<Arc<NodeBlockchain>>>,
}

impl ProgrammableTransport {
    fn new() -> Self {
        Self {
            mode: Mutex::new(Mode::AcceptDefault),
            locks: Mutex::new(Vec::new()),
            releases: Mutex::new(Vec::new()),
            rollbacks: Mutex::new(Vec::new()),
            register_seen: Mutex::new(Vec::new()),
            target_chain: Mutex::new(None),
        }
    }

    async fn set_mode(&self, m: Mode) {
        *self.mode.lock().await = m;
    }

    async fn attach_target_chain(&self, bc: Arc<NodeBlockchain>) {
        *self.target_chain.lock().await = Some(bc);
    }
}

#[async_trait]
impl TransferTransport for ProgrammableTransport {
    async fn broadcast_lock(&self, msg: TransferLockMessage) -> Result<(), GatewayError> {
        self.locks.lock().await.push(msg);
        Ok(())
    }

    async fn send_register_request(
        &self,
        _peer: &PeerCertFingerprint,
        req: TransferRegisterRequest,
        _deadline: Duration,
    ) -> Result<TransferRegisterAck, GatewayError> {
        self.register_seen.lock().await.push(req.clone());
        let mode = { self.mode.lock().await.clone() };
        match mode {
            Mode::AcceptDefault => {
                // Drive a separate target-chain coordinator if attached
                // — this proves cross-chain receipts land on both sides.
                if let Some(target) = self.target_chain.lock().await.as_ref().cloned() {
                    let target_transport: Arc<dyn TransferTransport> =
                        Arc::new(ProgrammableTransport::new());
                    let target_coord = TransferCoordinator::new(
                        target.clone(),
                        target_transport,
                        Arc::new(AllowAllFederation),
                        "tgt-chain".into(),
                    );
                    let ack = target_coord
                        .handle_register_request(req, StateProof::new_for_testing())
                        .await
                        .expect("test: target accepts");
                    return Ok(ack);
                }
                Ok(TransferRegisterAck {
                    transfer_id: req.transfer_id,
                    target_block_hash: "tgt-mock-hash".into(),
                    state_proof: StateProof::new_for_testing(),
                    accepted: true,
                    reason: None,
                    acked_at: 0,
                })
            }
            Mode::Reject(reason) => Ok(TransferRegisterAck {
                transfer_id: req.transfer_id,
                target_block_hash: String::new(),
                state_proof: StateProof::default(),
                accepted: false,
                reason: Some(reason),
                acked_at: 0,
            }),
            Mode::Silent => {
                // Sleep past any reasonable test deadline so the
                // coordinator's tokio::time::timeout fires.
                tokio::time::sleep(Duration::from_secs(60)).await;
                Err(GatewayError::TransportFailure {
                    transfer_id: req.transfer_id,
                    detail: "silent (should have timed out)".into(),
                })
            }
        }
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
// 1. Happy path: A→B transfer leaves linked receipts on both chains.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_happy_path_a_to_b_transfer() {
    let chain_a = make_chain(0, 0, 0);
    let chain_b = make_chain(1, 1, 1);
    let initial_a = chain_a.get_height().await;
    let initial_b = chain_b.get_height().await;

    let transport = Arc::new(ProgrammableTransport::new());
    transport.attach_target_chain(chain_b.clone()).await;

    let coord_a = TransferCoordinator::new(
        chain_a.clone(),
        transport.clone(),
        Arc::new(AllowAllFederation),
        "src-chain".into(),
    );

    let outcome = coord_a
        .initiate(
            AssetId::from("asset-happy"),
            "tgt-chain".into(),
            "peer-fp-AB".into(),
            BlockchainScope::Network,
            one_shard(),
            StateProof::new_for_testing(),
        )
        .await
        .expect("test: happy path");

    // Source chain: lock + release + receipt = 3 new blocks.
    let height_a = chain_a.get_height().await;
    assert_eq!(
        height_a,
        initial_a + 3,
        "source chain should have lock+release+receipt"
    );

    // Target chain: registration + receipt = 2 new blocks.
    let height_b = chain_b.get_height().await;
    assert_eq!(
        height_b, initial_b + 2,
        "target chain should have registration+receipt"
    );

    // Outcome encodes both block hashes so an auditor can walk either
    // chain and find the cross-link.
    assert!(!outcome.source_block_hash.is_empty());
    assert!(!outcome.target_block_hash.is_empty());

    // One lock broadcast, one release broadcast, no rollbacks.
    assert_eq!(transport.locks.lock().await.len(), 1);
    assert_eq!(transport.releases.lock().await.len(), 1);
    assert!(transport.rollbacks.lock().await.is_empty());

    let stored = coord_a
        .get_transfer(&outcome.transfer_id)
        .await
        .expect("test: tracked");
    assert_eq!(stored.state, CoordinatorState::Released);
}

// ---------------------------------------------------------------------------
// 2. Target rejection rolls back; source still has its lock + rollback entry.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_target_rejection_rolls_back() {
    let chain_a = make_chain(0, 0, 0);
    let initial_a = chain_a.get_height().await;
    let transport = Arc::new(ProgrammableTransport::new());
    transport.set_mode(Mode::Reject("hash mismatch".into())).await;

    let coord_a = TransferCoordinator::new(
        chain_a.clone(),
        transport.clone(),
        Arc::new(AllowAllFederation),
        "src-chain".into(),
    );

    let err = coord_a
        .initiate(
            AssetId::from("asset-reject"),
            "tgt-chain".into(),
            "peer-fp-X".into(),
            BlockchainScope::Network,
            one_shard(),
            StateProof::new_for_testing(),
        )
        .await
        .unwrap_err();

    match err {
        GatewayError::TargetRejected { reason, .. } => {
            assert!(reason.contains("hash mismatch"))
        }
        other => panic!("expected TargetRejected, got {other:?}"),
    }

    // Source: lock + release(rollback) = 2 new blocks. No receipt.
    let height_a = chain_a.get_height().await;
    assert_eq!(height_a, initial_a + 2, "source had only lock + release");

    // Rollback broadcast.
    let rbs = transport.rollbacks.lock().await.clone();
    assert_eq!(rbs.len(), 1);
    assert!(matches!(
        rbs[0].reason,
        RollbackReason::TargetRejected { .. }
    ));
}

// ---------------------------------------------------------------------------
// 3. Register-ack deadline elapses → automatic rollback.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_register_timeout_rolls_back() {
    let chain_a = make_chain(0, 0, 0);
    let initial_a = chain_a.get_height().await;
    let transport = Arc::new(ProgrammableTransport::new());
    transport.set_mode(Mode::Silent).await;

    let coord_a = TransferCoordinator::new(
        chain_a.clone(),
        transport.clone(),
        Arc::new(AllowAllFederation),
        "src-chain".into(),
    );
    coord_a
        .set_register_timeout(Duration::from_millis(150))
        .await;

    let err = coord_a
        .initiate(
            AssetId::from("asset-timeout"),
            "tgt-chain".into(),
            "peer-fp-T".into(),
            BlockchainScope::Network,
            one_shard(),
            StateProof::new_for_testing(),
        )
        .await
        .unwrap_err();

    match err {
        GatewayError::RegisterTimeout { elapsed_ms, .. } => {
            assert!(elapsed_ms >= 150, "expected ≥150ms, got {elapsed_ms}");
        }
        other => panic!("expected RegisterTimeout, got {other:?}"),
    }

    // Lock + rollback release on source.
    let height_a = chain_a.get_height().await;
    assert_eq!(height_a, initial_a + 2);

    let rbs = transport.rollbacks.lock().await.clone();
    assert_eq!(rbs.len(), 1);
    assert!(matches!(
        rbs[0].reason,
        RollbackReason::RegisterTimeout { .. }
    ));
}

// ---------------------------------------------------------------------------
// 4. Restart-recovery contract — Phase G.2 implements scan; G.1 stub returns empty.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_lock_state_persists_on_restart_g1_stub() {
    // G.1 contract: resume_in_flight returns Ok(empty) without scanning.
    // Phase G.2 will replace the stub with a real chain scan + state
    // restore. The test guards the contract: callers can rely on the
    // method existing and not panicking even before G.2.
    let chain = make_chain(0, 0, 0);
    let transport: Arc<dyn TransferTransport> = Arc::new(ProgrammableTransport::new());
    let coord = TransferCoordinator::new(
        chain,
        transport,
        Arc::new(AllowAllFederation),
        "any".into(),
    );
    let resumed = coord.resume_in_flight().await.expect("test: stub ok");
    assert!(resumed.is_empty(), "G.1 stub returns empty list");
}

// ---------------------------------------------------------------------------
// 5. Unfederated peer rejected before any wire traffic.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_unfederated_peer_rejected() {
    let chain_a = make_chain(0, 0, 0);
    let height_before = chain_a.get_height().await;
    let transport = Arc::new(ProgrammableTransport::new());

    let coord_a = TransferCoordinator::new(
        chain_a.clone(),
        transport.clone(),
        Arc::new(DenyAllFederation),
        "src-chain".into(),
    );

    let err = coord_a
        .initiate(
            AssetId::from("asset-deny"),
            "tgt-chain".into(),
            "peer-untrusted".into(),
            BlockchainScope::Network,
            one_shard(),
            StateProof::new_for_testing(),
        )
        .await
        .unwrap_err();

    assert!(matches!(err, GatewayError::FederationRejected { .. }));

    // No chain growth, no wire traffic.
    let height_after = chain_a.get_height().await;
    assert_eq!(height_after, height_before);
    assert!(transport.locks.lock().await.is_empty());
    assert!(transport.releases.lock().await.is_empty());
    assert!(transport.rollbacks.lock().await.is_empty());
}
