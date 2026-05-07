// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase H.1 — Distributed DNS + Reserved Domains + Foundation Grant.
//!
//! Eight scenarios covering reserved-domain enforcement, foundation
//! grant signature verification, recipient binding, conflict resolution
//! across multiple chains, and wire-format stability for the new
//! `DistributedDnsQuery` / `DistributedDnsResponse` types.

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;

use blockmatrix::bootstrap::PrivacyMode;
use blockmatrix::dns::{
    is_reserved, reserved_count, DnsError, DnsPoolManager, DnsRegistrar, DnsValidator,
    FoundationGrant,
};
use blockmatrix::network::message_handlers::{
    select_canonical, DistributedDnsQuery, DistributedDnsResponse,
};
use blockmatrix::proof_of_state::proof_of_state_integration::{
    SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
};
use blockmatrix::proof_of_state::StateProof;
use hypermesh_lib::NodeSigner;
use trustchain::FalconIdentity;

// ── Helpers ────────────────────────────────────────────────────────────

fn test_proof() -> StateProof {
    let stake = StakeProof::new("test-holder".to_string(), "holder-id".to_string(), 1000);
    let time = TimeProof::new(Duration::from_secs(10));
    let space = SpaceProof::new(
        "test-node".to_string(),
        "/tmp/storage".to_string(),
        1024 * 1024,
    );
    let work = WorkProof::new(
        "test-owner".to_string(),
        "test-workload".to_string(),
        12345,
        100,
        WorkloadType::Compute,
        WorkState::Completed,
    );
    StateProof::new(stake, time, space, work)
}

fn make_registrar() -> DnsRegistrar {
    let pool_manager = Arc::new(DnsPoolManager::new());
    let validator = Arc::new(DnsValidator::new(false));
    DnsRegistrar::new(pool_manager, validator)
}

/// Sign a grant: foundation issues for `domain` to `recipient`.
fn issue_grant(
    foundation: &FalconIdentity,
    recipient: &FalconIdentity,
    domain: &str,
    valid_until: SystemTime,
) -> FoundationGrant {
    let mut g = FoundationGrant::new_unsigned(
        domain.to_string(),
        recipient.public_key.clone(),
        valid_until,
        valid_until,
    );
    g.foundation_signature = foundation
        .sign(&g.signing_payload())
        .expect("test: FALCON sign");
    g
}

// ── Sanity ─────────────────────────────────────────────────────────────

#[test]
fn reserved_registry_loads() {
    assert!(reserved_count() > 100, "reserved set should be substantive");
    assert!(is_reserved("nike"));
    assert!(is_reserved("hypermesh"));
    assert!(is_reserved("foundation"));
    assert!(!is_reserved("alephpt"));
    assert!(!is_reserved("myhomeserver"));
}

// ── 1. Reserved domain rejected without grant ──────────────────────────

#[tokio::test]
async fn test_reserved_domain_rejected_without_grant() {
    let registrar = make_registrar();
    let proof = test_proof();
    let result = registrar
        .register_domain(
            "nike",
            PrivacyMode::PUBLIC,
            "test-owner".into(),
            proof,
        )
        .await;
    match result {
        Err(DnsError::ReservedDomain { name }) => {
            assert_eq!(name, "nike");
        }
        other => panic!("expected ReservedDomain, got {:?}", other),
    }
}

// ── 2. Reserved domain accepted with valid grant ───────────────────────

#[tokio::test]
async fn test_reserved_domain_accepted_with_valid_grant() {
    let foundation = FalconIdentity::generate();
    let recipient = FalconIdentity::generate();

    let registrar = make_registrar();
    registrar
        .set_foundation_pubkey(Some(foundation.public_key.clone()))
        .await;

    let valid_until = SystemTime::now() + Duration::from_secs(365 * 24 * 60 * 60);
    let grant = issue_grant(&foundation, &recipient, "nike", valid_until);

    let result = registrar
        .register_domain_with_grant(
            "nike",
            PrivacyMode::PUBLIC,
            "test-owner".into(),
            test_proof(),
            &grant,
            &recipient.public_key,
        )
        .await;
    assert!(
        result.is_ok(),
        "registration with valid grant should succeed: {:?}",
        result
    );
    let reg = result.expect("test: registration");
    assert_eq!(reg.domain_name, "nike");
}

// ── 3. Tampered signature rejected ─────────────────────────────────────

#[tokio::test]
async fn test_reserved_domain_rejected_with_invalid_grant_signature() {
    let foundation = FalconIdentity::generate();
    let recipient = FalconIdentity::generate();

    let registrar = make_registrar();
    registrar
        .set_foundation_pubkey(Some(foundation.public_key.clone()))
        .await;

    let valid_until = SystemTime::now() + Duration::from_secs(365 * 24 * 60 * 60);
    let mut grant = issue_grant(&foundation, &recipient, "nike", valid_until);

    // Tamper with the signature.
    if !grant.foundation_signature.is_empty() {
        let mid = grant.foundation_signature.len() / 2;
        grant.foundation_signature[mid] ^= 0xff;
    }

    let result = registrar
        .register_domain_with_grant(
            "nike",
            PrivacyMode::PUBLIC,
            "test-owner".into(),
            test_proof(),
            &grant,
            &recipient.public_key,
        )
        .await;
    assert!(
        matches!(result, Err(DnsError::InvalidGrant)),
        "expected InvalidGrant, got {:?}",
        result
    );
}

// ── 4. Recipient mismatch rejected ─────────────────────────────────────

#[tokio::test]
async fn test_reserved_domain_rejected_when_grant_recipient_mismatch() {
    let foundation = FalconIdentity::generate();
    let intended_recipient = FalconIdentity::generate();
    let other_identity = FalconIdentity::generate();

    let registrar = make_registrar();
    registrar
        .set_foundation_pubkey(Some(foundation.public_key.clone()))
        .await;

    let valid_until = SystemTime::now() + Duration::from_secs(365 * 24 * 60 * 60);
    let grant = issue_grant(&foundation, &intended_recipient, "nike", valid_until);

    // Try to register with a DIFFERENT identity.
    let result = registrar
        .register_domain_with_grant(
            "nike",
            PrivacyMode::PUBLIC,
            "test-owner".into(),
            test_proof(),
            &grant,
            &other_identity.public_key,
        )
        .await;
    assert!(
        matches!(result, Err(DnsError::GrantRecipientMismatch)),
        "expected GrantRecipientMismatch, got {:?}",
        result
    );
}

// ── 5. Non-reserved domain works without grant ─────────────────────────

#[tokio::test]
async fn test_non_reserved_domain_works_without_grant() {
    let registrar = make_registrar();
    // "alephpt" is not on the reserved list.
    let result = registrar
        .register_domain(
            "alephpt",
            PrivacyMode::PUBLIC,
            "test-owner".into(),
            test_proof(),
        )
        .await;
    assert!(
        result.is_ok(),
        "non-reserved domain should register without grant: {:?}",
        result
    );
    let reg = result.expect("test: registration");
    assert_eq!(reg.domain_name, "alephpt");
}

// ── 6. Two simultaneous registrations resolve deterministically ────────

#[test]
fn test_two_simultaneous_registrations_resolve_deterministically() {
    // Two nodes register "alice" simultaneously on different chains.
    // Conflict resolution: older registration wins; tiebreak by chain
    // height. select_canonical chooses one deterministic answer.
    let r_node_a = mk_response("alice", 100, 5, false, 1, "chain-a");
    let r_node_b = mk_response("alice", 200, 50, false, 1, "chain-b");

    let responses = vec![r_node_a.clone(), r_node_b.clone()];
    let canonical = select_canonical(&responses).expect("test: non-empty");
    assert_eq!(
        canonical.chain_id, "chain-a",
        "older registration must win"
    );

    // And the result is order-independent.
    let responses_rev = vec![r_node_b, r_node_a];
    let canonical_rev = select_canonical(&responses_rev).expect("test: non-empty");
    assert_eq!(canonical_rev.chain_id, "chain-a");
}

// ── 7. Foundation grant beats local registration ───────────────────────

#[test]
fn test_foundation_grant_beats_local_registration() {
    // Node X has local "nike" entry from before reservation enforcement
    // (synthetic — represents a chain that was registered without a grant
    // because enforcement was not yet rolled out).
    let local = mk_response("nike", 50, 100, false, 1, "chain-x");
    // Node Y has foundation grant for "nike" — newer registration, lower
    // chain height, but foundation_grant_present beats both.
    let with_grant = mk_response("nike", 200, 1, true, 1, "chain-y");

    let responses = vec![local, with_grant];
    let canonical = select_canonical(&responses).expect("test: non-empty");
    assert_eq!(
        canonical.chain_id, "chain-y",
        "foundation grant must beat older non-grant registration"
    );
    assert!(canonical.foundation_grant_present);
}

// ── 8. Wire-format round-trip ──────────────────────────────────────────

#[test]
fn test_dns_query_response_round_trip() {
    let q = DistributedDnsQuery {
        query_id: uuid::Uuid::new_v4(),
        domain_name: "myserver.alephpt".to_string(),
    };
    let q_bytes = serde_json::to_vec(&q).expect("test: serialize");
    let q_decoded: DistributedDnsQuery =
        serde_json::from_slice(&q_bytes).expect("test: deserialize");
    assert_eq!(q.query_id, q_decoded.query_id);
    assert_eq!(q.domain_name, q_decoded.domain_name);

    let r = mk_response("myserver.alephpt", 1234, 7, true, 2, "chain-z");
    let r_bytes = serde_json::to_vec(&r).expect("test: serialize");
    let r_decoded: DistributedDnsResponse =
        serde_json::from_slice(&r_bytes).expect("test: deserialize");
    assert_eq!(r.query_id, r_decoded.query_id);
    assert_eq!(r.domain_name, r_decoded.domain_name);
    assert_eq!(r.chain_id, r_decoded.chain_id);
    assert_eq!(r.chain_height, r_decoded.chain_height);
    assert_eq!(r.registration_timestamp, r_decoded.registration_timestamp);
    assert_eq!(
        r.foundation_grant_present,
        r_decoded.foundation_grant_present
    );
    assert_eq!(r.records.len(), r_decoded.records.len());
}

// ── Bonus: grant expiry path ───────────────────────────────────────────

#[tokio::test]
async fn test_expired_grant_rejected() {
    let foundation = FalconIdentity::generate();
    let recipient = FalconIdentity::generate();

    let registrar = make_registrar();
    registrar
        .set_foundation_pubkey(Some(foundation.public_key.clone()))
        .await;

    // Issue a grant that expired one minute ago.
    let valid_until = SystemTime::now() - Duration::from_secs(60);
    let grant = issue_grant(&foundation, &recipient, "nike", valid_until);

    let result = registrar
        .register_domain_with_grant(
            "nike",
            PrivacyMode::PUBLIC,
            "test-owner".into(),
            test_proof(),
            &grant,
            &recipient.public_key,
        )
        .await;
    assert!(
        matches!(result, Err(DnsError::ExpiredGrant)),
        "expected ExpiredGrant, got {:?}",
        result
    );
}

// ── Bonus: foundation pubkey not configured ────────────────────────────

#[tokio::test]
async fn test_grant_path_without_configured_foundation_pubkey_rejects() {
    // Don't call set_foundation_pubkey — the registrar has no key.
    let foundation = FalconIdentity::generate();
    let recipient = FalconIdentity::generate();

    let registrar = make_registrar();

    let valid_until = SystemTime::now() + Duration::from_secs(365 * 24 * 60 * 60);
    let grant = issue_grant(&foundation, &recipient, "nike", valid_until);

    let result = registrar
        .register_domain_with_grant(
            "nike",
            PrivacyMode::PUBLIC,
            "test-owner".into(),
            test_proof(),
            &grant,
            &recipient.public_key,
        )
        .await;
    assert!(
        matches!(result, Err(DnsError::InvalidGrant)),
        "alpha-default inert: no foundation pubkey configured → InvalidGrant, got {:?}",
        result
    );
}

// ── Helpers shared across H.1 conflict-resolution tests ────────────────

fn mk_response(
    name: &str,
    timestamp_secs: u64,
    chain_height: u64,
    foundation_grant_present: bool,
    records: usize,
    chain_id: &str,
) -> DistributedDnsResponse {
    use blockmatrix::dns::{DnsRecord, DnsRecordData, DnsRecordType};
    use std::net::Ipv6Addr;

    let recs = (0..records)
        .map(|i| DnsRecord {
            domain: name.to_string(),
            record_type: DnsRecordType::AAAA,
            data: DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
            ttl: 300,
            created_at: UNIX_EPOCH + Duration::from_secs(timestamp_secs),
            expires_at: UNIX_EPOCH + Duration::from_secs(timestamp_secs + 300),
            owner: format!("owner-{i}"),
            tx_hash: Some(chain_id.to_string()),
        })
        .collect();
    DistributedDnsResponse {
        query_id: uuid::Uuid::new_v4(),
        domain_name: name.to_string(),
        records: recs,
        chain_id: chain_id.to_string(),
        chain_height,
        registration_timestamp: timestamp_secs,
        foundation_grant_present,
    }
}
