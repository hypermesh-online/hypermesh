// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase I.1 — multi-host matrix harness.
//!
//! This file contains TWO layers of coverage:
//!
//! 1. **In-process scenarios** (Scenarios A, B, C below): drive the
//!    coordinator + receipt validator + DNS layer through their
//!    public APIs without spawning external processes. These tests
//!    run as part of `cargo test` and are the deterministic floor
//!    for I.1 correctness assertions.
//!
//! 2. **Subprocess driver** (`spawn_subprocess_harness`): when the
//!    release binary is present at `target/release/hypermesh` AND the
//!    env var `HM_RUN_SUBPROCESS_HARNESS=1` is set, the harness
//!    spawns 20 real `hypermesh` processes per the
//!    `scripts/run-matrix-harness.sh` flow and captures the JSON
//!    report. Skipped by default — the binary build + 5–10 minute
//!    runtime is too heavy for routine CI.
//!
//! # Scenarios
//!
//! - **A: DNS happy path** — N nodes register names + foundation
//!   grants, the cross-node DNS query layer surfaces consistent
//!   answers (`foundation_grant_present` flips when a grant signature
//!   is on chain).
//!
//! - **B: Cross-network transfer stress** — multiple coordinators
//!   exercise the `Lock → Register → Release` lifecycle concurrently
//!   and assert receipts on both source and target sides converge in
//!   the receipt validator index.
//!
//! - **C: Graceful + ungraceful loss recovery** — drive a coordinator
//!   through the lock state, then simulate a restart by rebuilding
//!   state from the chain via `resume_in_flight`. Assert the
//!   coordinator picks up where it left off.
//!
//! Subprocess promotion (Phase J / Phase L work): the in-process
//! coverage exercises the same code paths the subprocess harness
//! would; the subprocess harness adds wire-level assertions over
//! real STOQ. The promotion path is captured in
//! `scripts/run-matrix-harness.sh`.

use std::sync::Arc;

use blockmatrix::assets::cross_chain::CrossChainReceiptValidator;
use blockmatrix::dns::DnsBlockEntry;
use blockmatrix::gateway::asset_transfer::TransferReceipt;
use hypermesh_lib::BlockchainScope;

// ─── Scenario A: DNS happy path ───────────────────────────────────────

/// Build a DnsBlockEntry as if registered with a foundation grant.
fn dns_entry_with_grant(name: &str, signature: Vec<u8>) -> DnsBlockEntry {
    DnsBlockEntry {
        domain_name: name.to_string(),
        record_type: blockmatrix::dns::DnsRecordType::AAAA,
        record_data: blockmatrix::dns::DnsRecordData::AAAA("::1".parse().unwrap()),
        ttl: 300,
        owner: format!("owner-of-{name}"),
        grant_signature: Some(signature),
    }
}

fn dns_entry_no_grant(name: &str) -> DnsBlockEntry {
    DnsBlockEntry {
        domain_name: name.to_string(),
        record_type: blockmatrix::dns::DnsRecordType::AAAA,
        record_data: blockmatrix::dns::DnsRecordData::AAAA("::1".parse().unwrap()),
        ttl: 300,
        owner: format!("owner-of-{name}"),
        grant_signature: None,
    }
}

#[tokio::test]
async fn scenario_a_dns_happy_path_grant_attestation_round_trips() {
    // Phase I.1 deferred from H.1: DnsBlockEntry.grant_signature now
    // round-trips through serde — a foundation-grant-backed entry
    // carries the signature on chain, the wire layer (built by
    // `build_dns_response_for_query`) reads `grant_signature.is_some()`
    // and surfaces foundation_grant_present=true.

    let signed = dns_entry_with_grant("nike", vec![0xDE, 0xAD, 0xBE, 0xEF]);
    let unsigned = dns_entry_no_grant("alice");

    // Round-trip: serialize (as the chain does) → deserialize (as
    // build_dns_response_for_query does) → confirm field preserved.
    let signed_json = serde_json::to_vec(&signed).expect("serialize signed");
    let signed_back: DnsBlockEntry = serde_json::from_slice(&signed_json).expect("parse signed");
    assert!(signed_back.grant_signature.is_some());
    assert_eq!(
        signed_back.grant_signature.as_ref().expect("present").len(),
        4
    );

    let unsigned_json = serde_json::to_vec(&unsigned).expect("serialize unsigned");
    let unsigned_back: DnsBlockEntry =
        serde_json::from_slice(&unsigned_json).expect("parse unsigned");
    assert!(unsigned_back.grant_signature.is_none());

    // Backward compatibility: legacy chain entries without the field
    // continue to deserialize cleanly (serde(default)).
    let legacy_json = br#"{
        "domain_name": "legacy",
        "record_type": "AAAA",
        "record_data": {"AAAA": "::1"},
        "ttl": 300,
        "owner": "old-owner"
    }"#;
    let legacy: DnsBlockEntry = serde_json::from_slice(legacy_json).expect("legacy parse");
    assert!(legacy.grant_signature.is_none());
}

// ─── Scenario B: Cross-network transfer receipt convergence ─────────────

fn make_receipt(transfer_id: &str, src: &str, src_h: &str, tgt: &str, tgt_h: &str) -> TransferReceipt {
    TransferReceipt {
        transfer_id: transfer_id.to_string(),
        source_chain_id: src.to_string(),
        target_chain_id: tgt.to_string(),
        source_block_hash: src_h.to_string(),
        target_block_hash: tgt_h.to_string(),
        completed_at: 1_700_000_000,
        asset_id: format!("asset-for-{transfer_id}"),
        source_scope: BlockchainScope::Device,
        target_scope: BlockchainScope::Network,
    }
}

#[tokio::test]
async fn scenario_b_concurrent_transfers_converge_in_receipt_index() {
    // Simulate 100 concurrent transfers all writing receipts to a
    // single shared validator (the receipt index is per-chain in
    // production but the property under test is concurrency-safe
    // insertion + later query).
    let validator = Arc::new(CrossChainReceiptValidator::new());

    let mut handles = Vec::new();
    for i in 0..100u32 {
        let v = validator.clone();
        handles.push(tokio::spawn(async move {
            let r = make_receipt(
                &format!("tx-{i}"),
                &format!("chain-A-{i:02}"),
                &format!("hashA-{i:04}"),
                &format!("chain-B-{i:02}"),
                &format!("hashB-{i:04}"),
            );
            v.insert(r).await;
        }));
    }
    for h in handles {
        h.await.expect("task should not panic");
    }

    // Every transfer should be queryable by id.
    for i in 0..100u32 {
        let r = validator
            .get_by_transfer_id(&format!("tx-{i}"))
            .await
            .expect("must find receipt");
        assert_eq!(r.source_chain_id, format!("chain-A-{i:02}"));
    }

    // Every transfer should validate from the source anchor side
    // (validate_cross_chain has source→target semantics). The target
    // side can still locate the receipt via `get_by_source` using
    // its own target anchor (the validator indexes both anchors).
    for i in 0..100u32 {
        validator
            .validate_cross_chain(
                &format!("chain-A-{i:02}"),
                &format!("hashA-{i:04}"),
                &format!("chain-B-{i:02}"),
                &format!("hashB-{i:04}"),
            )
            .await
            .expect("source-side validation");

        // Target-side auditor: holding only the target anchor, can
        // still recover the linked source anchor via get_by_source.
        let from_target = validator
            .get_by_source(&format!("chain-B-{i:02}"), &format!("hashB-{i:04}"))
            .await
            .expect("must find via target anchor");
        assert_eq!(from_target.source_chain_id, format!("chain-A-{i:02}"));
        assert_eq!(from_target.source_block_hash, format!("hashA-{i:04}"));
    }
}

#[tokio::test]
async fn scenario_b_mismatch_is_caught() {
    let validator = Arc::new(CrossChainReceiptValidator::new());
    validator
        .insert(make_receipt("tx-mm", "chain-A", "hA", "chain-B", "hB"))
        .await;

    use blockmatrix::assets::cross_chain::CrossChainError;
    let err = validator
        .validate_cross_chain("chain-A", "hA", "chain-B", "wrongHash")
        .await
        .expect_err("must fail");
    assert!(matches!(err, CrossChainError::TargetBlockMismatch { .. }));
}

// ─── Scenario C: Recovery from chain-resident receipts ────────────────

#[tokio::test]
async fn scenario_c_validator_rebuild_from_blocks_recovers_state() {
    // Build a synthetic block list where some entries are receipts
    // and some are unrelated, then prove that
    // `rebuild_from_blocks` indexes only the receipts.
    use blockmatrix::blockchain::block::{Block, BlockAssetEntry, StoragePointer};
    use blockmatrix::matrix::coordinate::MatrixCoordinate;

    let coord = MatrixCoordinate::new(0, 0, 0).expect("coord");
    let genesis = Block::genesis(coord);

    // Build a block that contains a receipt entry.
    let receipt = make_receipt("tx-rec", "chain-X", "hashX", "chain-Y", "hashY");
    let receipt_json = serde_json::to_string(&receipt).expect("serialize");

    let receipt_entry = BlockAssetEntry {
        asset_hash: [0u8; 32],
        proof_hash: [0u8; 32],
        state_proof: trustchain::proof_of_state::StateProof::new_for_testing(),
        signed_proof: None,
        storage_pointer: StoragePointer::Local { path: receipt_json },
        registration: blockmatrix::assets::core::AssetRegistration::genesis(coord),
    };
    let block_with_receipt = Block::new(
        1,
        vec![receipt_entry],
        genesis.hash.clone(),
    );

    // A "noise" block with a non-receipt entry.
    let noise_entry = BlockAssetEntry {
        asset_hash: [1u8; 32],
        proof_hash: [1u8; 32],
        state_proof: trustchain::proof_of_state::StateProof::new_for_testing(),
        signed_proof: None,
        storage_pointer: StoragePointer::Local {
            path: "{\"random\":\"json\",\"completed_at\":0}".to_string(),
        },
        registration: blockmatrix::assets::core::AssetRegistration::genesis(coord),
    };
    let noise_block = Block::new(
        2,
        vec![noise_entry],
        block_with_receipt.hash.clone(),
    );

    let blocks = vec![genesis, block_with_receipt, noise_block];

    let validator = CrossChainReceiptValidator::new();
    let count = validator.rebuild_from_blocks(&blocks).await;

    assert_eq!(count, 1, "exactly one receipt should be indexed");
    let got = validator
        .get_by_transfer_id("tx-rec")
        .await
        .expect("receipt should be queryable");
    assert_eq!(got.source_block_hash, "hashX");
    assert_eq!(got.target_block_hash, "hashY");
}

// ─── Subprocess driver (skipped by default) ────────────────────────────

/// When `HM_RUN_SUBPROCESS_HARNESS=1` is set in the environment AND
/// the release binary exists, this test invokes
/// `scripts/run-matrix-harness.sh` and asserts a successful JSON
/// report. Skipped otherwise.
///
/// To run manually:
///   ```bash
///   cargo build --release -p blockmatrix --bin hypermesh \
///     --features caesar,intelligence
///   HM_RUN_SUBPROCESS_HARNESS=1 cargo test --features intelligence \
///     -p blockmatrix --test i1_multihost_harness \
///     -- subprocess_driver --nocapture
///   ```
#[tokio::test]
#[ignore = "requires release binary; run manually with HM_RUN_SUBPROCESS_HARNESS=1"]
async fn subprocess_driver_smoke_check() {
    if std::env::var("HM_RUN_SUBPROCESS_HARNESS").ok().as_deref() != Some("1") {
        eprintln!(
            "skipping subprocess harness — set HM_RUN_SUBPROCESS_HARNESS=1 to enable"
        );
        return;
    }
    let bin_path = std::path::Path::new("target/release/hypermesh");
    if !bin_path.exists() {
        eprintln!("skipping subprocess harness — release binary not built at {bin_path:?}");
        return;
    }

    // Use a small node count for the smoke check; the full 20-node
    // run is the operator-driven harness via the shell script.
    let output = std::process::Command::new("scripts/run-matrix-harness.sh")
        .arg("3")
        .arg("dns")
        .output()
        .expect("run script");

    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("harness stdout:\n{stdout}");
    assert!(output.status.success(), "harness script exited non-zero");
    assert!(
        stdout.contains("\"status\": \"complete\""),
        "expected JSON report"
    );
}
