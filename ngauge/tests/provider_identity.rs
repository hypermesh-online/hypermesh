//! Canonical provider text is a reversible identity, not authorization.
use hypermesh_lib::{ContentHash, MatrixPosition, NodeId, DEFAULT_NETWORK};
use ngauge::{DmsDriver, DmsPlan, ReplicaCandidate, ShardCandidates, SwarmAnalytics};

fn plan(ids: Vec<String>, positioned: Vec<ReplicaCandidate>) -> DmsPlan {
    let shard = ContentHash([9; 32]);
    let mut analytics = SwarmAnalytics::new();
    analytics.record_request_in_network(
        DEFAULT_NETWORK,
        shard,
        NodeId::from_bytes([2; 32]),
        MatrixPosition {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        0,
    );
    analytics.set_replica_count_in_network(DEFAULT_NETWORK, shard, 0);
    DmsDriver::plan(
        &analytics,
        DEFAULT_NETWORK,
        &[ShardCandidates {
            shard_id: shard,
            positioned,
            all_ids: ids,
        }],
        0.5,
    )
}

fn positioned(id: &str) -> ReplicaCandidate {
    ReplicaCandidate::new(
        id,
        MatrixPosition {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        1.0,
    )
}

fn invalid_ids() -> Vec<String> {
    vec![
        String::new(),
        "provider-1".into(),
        "a".repeat(63),
        "a".repeat(65),
        "AB".repeat(32),
        "gg".repeat(32),
        "00".repeat(32),
        "é".repeat(32),
        format!(" {}", "a".repeat(63)),
    ]
}

#[test]
fn canonical_id_preserves_all_bytes_without_rehashing() {
    let node = NodeId::from_bytes(std::array::from_fn(|i| i as u8));
    let action = plan(vec![node.to_hex()], vec![positioned(&node.to_hex())]);
    assert_eq!(action.mirror.len(), 1);
    assert_eq!(action.mirror[0].source, node);
    assert_ne!(
        action.mirror[0].source,
        NodeId::from_public_key(node.to_hex().as_bytes())
    );
}

#[test]
fn invalid_id_never_produces_an_action() {
    for id in invalid_ids() {
        assert!(
            plan(vec![id.clone()], vec![positioned(&id)])
                .mirror
                .is_empty(),
            "{id:?}"
        );
    }
    assert!(plan(Vec::new(), Vec::new()).mirror.is_empty());
}

#[test]
fn invalid_preferred_position_falls_back_to_valid_unpositioned_id() {
    let valid = NodeId::from_bytes([0xab; 32]);
    for invalid in invalid_ids() {
        let result = plan(
            vec![invalid.clone(), valid.to_hex()],
            vec![positioned(&invalid)],
        );
        assert_eq!(result.mirror.len(), 1);
        assert_eq!(result.mirror[0].source, valid);
    }
}

#[test]
fn invalid_unpositioned_id_cannot_win_deterministic_fallback() {
    let valid = NodeId::from_bytes([0xab; 32]);
    assert_eq!(
        plan(vec!["00".repeat(32), valid.to_hex()], Vec::new()).mirror[0].source,
        valid
    );
}

#[test]
fn invalid_positioned_candidate_does_not_skew_valid_geometry() {
    let valid = NodeId::from_bytes([0x12; 32]);
    let result = plan(
        vec![valid.to_hex()],
        vec![positioned("not-an-id"), positioned(&valid.to_hex())],
    );
    assert_eq!(result.mirror[0].source, valid);
}

#[test]
fn positioned_provider_must_belong_to_valid_indexed_ids() {
    let indexed = NodeId::from_bytes([0x12; 32]);
    let unindexed = NodeId::from_bytes([0xab; 32]);
    let result = plan(
        vec![indexed.to_hex()],
        vec![positioned(&unindexed.to_hex())],
    );
    assert_eq!(result.mirror.len(), 1);
    assert_eq!(result.mirror[0].source, indexed);
}
