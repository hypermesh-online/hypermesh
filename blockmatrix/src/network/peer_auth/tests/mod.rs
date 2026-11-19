use super::*;

fn make_peer(node_id: &str, network_id: &str) -> AuthenticatedPeer {
    AuthenticatedPeer {
        node_id: node_id.to_string(),
        connection_id: "session-current".into(),
        pubkey: vec![1, 2, 3],
        coordinate: (0, 0, 0),
        // AuthenticatedPeer.network_id is now the canonical NetworkId, mapped
        // from the wire label through the single ingress converter.
        network_id: NetworkId::from_wire_str(network_id),
        authenticated_at: std::time::Instant::now(),
        proof_bytes: vec![4, 5, 6],
    }
}

#[tokio::test]
async fn test_register_and_check() {
    let peers = new_authenticated_peers();
    let peer = make_peer("abc123def456", "net-1");

    assert!(!is_authenticated(&peers, "abc123def456").await);
    assert!(register_authenticated_peer(&peers, peer).await);
    assert!(is_authenticated(&peers, "abc123def456").await);
}

#[tokio::test]
async fn test_register_rejects_empty_proof() {
    let peers = new_authenticated_peers();
    let mut peer = make_peer("emptyproof1", "net-1");
    peer.proof_bytes = Vec::new();

    assert!(!register_authenticated_peer(&peers, peer).await);
    assert!(!is_authenticated(&peers, "emptyproof1").await);
}

#[tokio::test]
async fn test_register_rejects_empty_pubkey() {
    let peers = new_authenticated_peers();
    let mut peer = make_peer("emptypubk1", "net-1");
    peer.pubkey = Vec::new();

    assert!(!register_authenticated_peer(&peers, peer).await);
    assert!(!is_authenticated(&peers, "emptypubk1").await);
}

#[tokio::test]
async fn test_register_rejects_empty_node_id() {
    let peers = new_authenticated_peers();
    let peer = AuthenticatedPeer {
        node_id: String::new(),
        connection_id: "session-current".into(),
        pubkey: vec![1, 2, 3],
        coordinate: (0, 0, 0),
        network_id: NetworkId::from_wire_str("net-1"),
        authenticated_at: std::time::Instant::now(),
        proof_bytes: vec![4, 5, 6],
    };

    assert!(!register_authenticated_peer(&peers, peer).await);
}

#[tokio::test]
async fn test_remove_peer() {
    let peers = new_authenticated_peers();
    assert!(register_authenticated_peer(&peers, make_peer("node1234", "net-1")).await);
    assert!(is_authenticated(&peers, "node1234").await);

    remove_authenticated_peer(&peers, "node1234", "session-current").await;
    assert!(!is_authenticated(&peers, "node1234").await);
}

#[tokio::test]
async fn test_network_scoping() {
    let peers = new_authenticated_peers();
    assert!(register_authenticated_peer(&peers, make_peer("nodeAAAA", "net-alpha")).await);

    assert!(is_same_network(&peers, "nodeAAAA", "net-alpha").await);
    assert!(!is_same_network(&peers, "nodeAAAA", "net-beta").await);
}

#[tokio::test]
async fn test_verify_peer_access_authenticated_same_network() {
    let peers = new_authenticated_peers();
    assert!(register_authenticated_peer(&peers, make_peer("nodeX123", "main")).await);

    assert!(verify_peer_access(&peers, "nodeX123", "session-current", "main").await);
}

#[tokio::test]
async fn test_verify_peer_access_unauthenticated() {
    let peers = new_authenticated_peers();
    assert!(!verify_peer_access(&peers, "unknown1", "session-current", "main").await);
}

#[tokio::test]
async fn test_verify_peer_access_wrong_network() {
    let peers = new_authenticated_peers();
    assert!(register_authenticated_peer(&peers, make_peer("nodeY456", "other-net")).await);

    assert!(!verify_peer_access(&peers, "nodeY456", "session-current", "main").await);
}

// ── F6: per-asset shard-fetch proof binding ─────────────────────────

#[tokio::test]
async fn test_shard_proof_binding_ok_for_authenticated_peer() {
    let peers = new_authenticated_peers();
    // make_peer sets non-empty proof_bytes (vec![4,5,6]).
    assert!(register_authenticated_peer(&peers, make_peer("shardOK01", "main")).await);
    assert!(verify_shard_proof_binding(&peers, "shardOK01", "session-current", "main").await);
}

#[tokio::test]
async fn test_shard_proof_binding_rejects_unauthenticated() {
    let peers = new_authenticated_peers();
    // Never registered → no proof binding.
    assert!(!verify_shard_proof_binding(&peers, "ghostpeer", "session-current", "main").await);
}

#[tokio::test]
async fn test_shard_proof_binding_rejects_wrong_network() {
    let peers = new_authenticated_peers();
    assert!(register_authenticated_peer(&peers, make_peer("shardWN01", "other")).await);
    // Authenticated, but for a different network → refused.
    assert!(!verify_shard_proof_binding(&peers, "shardWN01", "session-current", "main").await);
}

#[tokio::test]
async fn test_shard_proof_binding_rejects_empty_proof() {
    // A peer inserted DIRECTLY (bypassing register_authenticated_peer's
    // non-empty-proof guard) with empty proof_bytes must be refused by
    // the F6 binding check — membership alone is not enough.
    let peers = new_authenticated_peers();
    let mut peer = make_peer("noproof01", "main");
    peer.proof_bytes = Vec::new();
    peers.write().await.insert(peer.node_id.clone(), peer);

    assert!(!verify_shard_proof_binding(&peers, "noproof01", "session-current", "main").await);
}

#[tokio::test]
async fn expired_auth_is_rejected() {
    let peers = new_authenticated_peers();
    let mut peer = make_peer("expiredAA", "net-1");
    peer.authenticated_at = std::time::Instant::now() - std::time::Duration::from_secs(7200);

    peers.write().await.insert(peer.node_id.clone(), peer);

    assert!(!verify_peer_access(&peers, "expiredAA", "session-current", "net-1").await);
    assert!(!peers.read().await.contains_key("expiredAA"));
}

// ── Key continuity tests ────────────────────────────────────────

#[tokio::test]
async fn test_validate_key_continuity_no_history() {
    let chains = tokio::sync::RwLock::new(HashMap::new());
    // No rotation chain at all → accept any key
    assert!(validate_key_continuity(&chains, "node-aaa", "fp-xyz").await);
}

#[tokio::test]
async fn test_validate_key_continuity_empty_chain() {
    let chains = tokio::sync::RwLock::new(HashMap::new());
    chains
        .write()
        .await
        .insert("node-bbb".to_string(), Vec::new());
    // Empty chain → accept any key
    assert!(validate_key_continuity(&chains, "node-bbb", "fp-xyz").await);
}

#[tokio::test]
async fn test_validate_key_continuity_matches_latest() {
    let chains = tokio::sync::RwLock::new(HashMap::new());
    let entry = serde_json::json!({
        "old_key_fingerprint": "fp-A",
        "new_key_fingerprint": "fp-B",
        "reason": "Scheduled",
    });
    chains
        .write()
        .await
        .insert("node-ccc".to_string(), vec![entry]);
    // Claimed key matches latest rotation's new_key → true
    assert!(validate_key_continuity(&chains, "node-ccc", "fp-B").await);
}

#[tokio::test]
async fn test_validate_key_continuity_stale_key() {
    let chains = tokio::sync::RwLock::new(HashMap::new());
    let entry = serde_json::json!({
        "old_key_fingerprint": "fp-A",
        "new_key_fingerprint": "fp-B",
        "reason": "Scheduled",
    });
    chains
        .write()
        .await
        .insert("node-ddd".to_string(), vec![entry]);
    // Claimed key is the old key, not the new one → false
    assert!(!validate_key_continuity(&chains, "node-ddd", "fp-A").await);
}

#[tokio::test]
async fn test_validate_key_continuity_multi_rotation() {
    let chains = tokio::sync::RwLock::new(HashMap::new());
    let entry1 = serde_json::json!({
        "old_key_fingerprint": "fp-A",
        "new_key_fingerprint": "fp-B",
    });
    let entry2 = serde_json::json!({
        "old_key_fingerprint": "fp-B",
        "new_key_fingerprint": "fp-C",
    });
    chains
        .write()
        .await
        .insert("node-eee".to_string(), vec![entry1, entry2]);
    // Only the latest new_key (fp-C) should pass
    assert!(validate_key_continuity(&chains, "node-eee", "fp-C").await);
    assert!(!validate_key_continuity(&chains, "node-eee", "fp-B").await);
    assert!(!validate_key_continuity(&chains, "node-eee", "fp-A").await);
}

mod session;
