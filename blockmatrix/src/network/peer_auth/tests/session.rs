use super::*;

// Synthetic post-handshake registry sequence; not a crypto or network test.
#[tokio::test]
async fn old_connection_cleanup_preserves_replacement() {
    let peers = new_authenticated_peers();
    let mut first = make_peer("same-node", "net-1");
    let mut replacement = make_peer("same-node", "net-1");
    first.connection_id = "A".into();
    replacement.connection_id = "B".into();
    replacement.pubkey = vec![7, 8, 9];
    assert!(register_authenticated_peer(&peers, first).await);
    assert!(verify_peer_access(&peers, "same-node", "A", "net-1").await);
    assert!(register_authenticated_peer(&peers, replacement).await);
    assert!(verify_peer_access(&peers, "same-node", "B", "net-1").await);
    remove_authenticated_peer(&peers, "same-node", "A").await;
    assert!(
        is_authenticated(&peers, "same-node").await,
        "old connection A cleanup must preserve replacement B"
    );
}

fn session_peer(id: &str) -> AuthenticatedPeer {
    let mut peer = make_peer("same-node", "net-1");
    peer.connection_id = id.into();
    peer
}

#[tokio::test]
async fn current_session_only_can_authorize_and_remove() {
    let peers = new_authenticated_peers();
    assert!(register_authenticated_peer(&peers, session_peer("A")).await);
    assert!(register_authenticated_peer(&peers, session_peer("B")).await);
    for id in ["A", "C", ""] {
        assert!(!verify_peer_access(&peers, "same-node", id, "net-1").await);
        assert!(!verify_shard_proof_binding(&peers, "same-node", id, "net-1").await);
        remove_authenticated_peer(&peers, "same-node", id).await;
        assert_eq!(peers.read().await["same-node"].connection_id, "B");
    }
    assert!(verify_peer_access(&peers, "same-node", "B", "net-1").await);
    assert!(verify_shard_proof_binding(&peers, "same-node", "B", "net-1").await);
    remove_authenticated_peer(&peers, "same-node", "B").await;
    assert!(!is_authenticated(&peers, "same-node").await);
}

#[tokio::test]
async fn missing_connection_id_rejected_without_replacing_current() {
    let peers = new_authenticated_peers();
    assert!(register_authenticated_peer(&peers, session_peer("B")).await);
    assert!(!register_authenticated_peer(&peers, session_peer("")).await);
    assert_eq!(peers.read().await["same-node"].connection_id, "B");
}

#[tokio::test]
async fn matching_expiry_removes_but_wrong_session_or_network_does_not() {
    let peers = new_authenticated_peers();
    let mut expired = session_peer("A");
    expired.authenticated_at -= AUTH_TTL * 2;
    assert!(register_authenticated_peer(&peers, expired).await);
    assert!(!verify_peer_access(&peers, "same-node", "C", "net-1").await);
    assert!(!verify_shard_proof_binding(&peers, "same-node", "C", "net-1").await);
    assert!(!verify_peer_access(&peers, "same-node", "A", "other-net").await);
    assert!(!verify_shard_proof_binding(&peers, "same-node", "A", "other-net").await);
    assert!(is_authenticated(&peers, "same-node").await);
    assert!(!verify_shard_proof_binding(&peers, "same-node", "A", "net-1").await);
    assert!(!is_authenticated(&peers, "same-node").await);
}

#[tokio::test]
async fn queued_old_expiry_cannot_remove_fresh_replacement() {
    check_refresh_before_queued_access("A", "B").await;
}

#[tokio::test]
async fn queued_expiry_cannot_remove_same_session_refresh() {
    check_refresh_before_queued_access("A", "A").await;
}

// FIFO write-lock acquisition with yield_now, not sleeps: registration is
// queued first, followed by the formerly expired session's access check.
async fn check_refresh_before_queued_access(old_id: &str, new_id: &str) {
    let peers = new_authenticated_peers();
    let mut expired = session_peer(old_id);
    expired.authenticated_at -= AUTH_TTL * 2;
    assert!(register_authenticated_peer(&peers, expired).await);
    let guard = peers.write().await;
    let fresh = session_peer(new_id);
    let registry = peers.clone();
    let registration =
        tokio::spawn(async move { register_authenticated_peer(&registry, fresh).await });
    tokio::task::yield_now().await;
    let registry = peers.clone();
    let old_connection = old_id.to_string();
    let access = tokio::spawn(async move {
        verify_peer_access(&registry, "same-node", &old_connection, "net-1").await
    });
    tokio::task::yield_now().await;
    drop(guard);
    assert!(registration.await.unwrap());
    assert_eq!(access.await.unwrap(), old_id == new_id);
    assert_eq!(peers.read().await["same-node"].connection_id, new_id);
    assert!(verify_shard_proof_binding(&peers, "same-node", new_id, "net-1").await);
}

#[tokio::test]
async fn shard_binding_checks_current_row_not_previous_membership() {
    let peers = new_authenticated_peers();
    assert!(register_authenticated_peer(&peers, session_peer("A")).await);
    assert!(verify_peer_access(&peers, "same-node", "A", "net-1").await);
    let mut replacement = session_peer("B");
    replacement.proof_bytes.clear();
    // Direct insertion models a malformed replacement; registration rejects it.
    peers.write().await.insert("same-node".into(), replacement);
    assert!(!verify_shard_proof_binding(&peers, "same-node", "A", "net-1").await);
    assert!(!verify_shard_proof_binding(&peers, "same-node", "B", "net-1").await);
    assert!(is_authenticated(&peers, "same-node").await);
}

// A queued writer must not supply B's proof to an authorization of A. Tokio's
// fair RwLock and explicit yields establish order without network IO or sleeps.
#[tokio::test]
async fn queued_shard_authorization_uses_one_row_for_membership_and_proof() {
    let peers = new_authenticated_peers();
    let mut first = session_peer("A");
    first.proof_bytes.clear();
    peers.write().await.insert("same-node".into(), first);
    let guard = peers.write().await;
    let registry = peers.clone();
    let authorization = tokio::spawn(async move {
        verify_shard_proof_binding(&registry, "same-node", "A", "net-1").await
    });
    tokio::task::yield_now().await;
    let registry = peers.clone();
    let replacement =
        tokio::spawn(
            async move { register_authenticated_peer(&registry, session_peer("B")).await },
        );
    tokio::task::yield_now().await;
    drop(guard);
    assert!(!authorization.await.unwrap());
    assert!(replacement.await.unwrap());
    assert!(!verify_shard_proof_binding(&peers, "same-node", "A", "net-1").await);
    assert!(verify_shard_proof_binding(&peers, "same-node", "B", "net-1").await);
}
