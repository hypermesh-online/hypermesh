use super::super::*;
use super::fetch_fixture::{coordinate, peer};

#[tokio::test]
async fn old_close_preserves_replacement_route() {
    let peers = peer_auth::new_authenticated_peers();
    let coord = coordinate();
    let nodes = Arc::new(RwLock::new(HashMap::new()));
    let routes = crate::network::connected_peer_routes::new_connected_routes();
    routes.write().await.insert(
        "same-node".into(),
        crate::network::connected_peer_routes::ConnectedRoute {
            connection_id: Some("B".into()),
            coordinate: coord,
        },
    );
    assert!(peer_auth::register_authenticated_peer(&peers, peer("A")).await);
    assert!(peer_auth::register_authenticated_peer(&peers, peer("B")).await);
    assert_eq!(
        crate::network::connected_peer_routes::coordinate_snapshot(&routes).await,
        vec![coord]
    );
    let ctx = CleanupContext {
        authenticated_peers: &peers,
        connected_routes: &routes,
        connected_nodes: &nodes,
    };
    cleanup_peer_registration("same-node", "A", &ctx).await;
    assert!(peer_auth::verify_peer_access(&peers, "same-node", "B", "net-1").await);
    assert!(
        crate::network::connected_peer_routes::coordinate_snapshot(&routes)
            .await
            .contains(&coord),
        "old A close must preserve replacement B routing visibility"
    );
}
