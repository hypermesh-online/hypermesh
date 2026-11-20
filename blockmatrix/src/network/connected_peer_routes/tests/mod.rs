use super::*;

fn node(node_id: &str) -> NetworkNode {
    NetworkNode {
        node_id: node_id.into(),
        coordinate: coord(),
        address: "[::1]:1".parse().unwrap(),
        privacy_mode: crate::bootstrap::PrivacyMode::ANONYMOUS,
        connection: None,
    }
}
fn coord() -> MatrixCoordinate {
    MatrixCoordinate::new(2, 2, 2).unwrap()
}
fn nodes() -> NetworkNodes {
    Arc::new(RwLock::new(HashMap::new()))
}

async fn publish(nodes: &NetworkNodes, routes: &ConnectedPeerRoutes, id: &str, connection: &str) {
    // Synthetic trusted local IDs exercise the exact publish primitive called
    // by publish_node, which obtains its ID from the real STOQ object.
    publish_with_identity(nodes, routes, node(id), Some(connection.into())).await;
}

#[tokio::test]
async fn both_registration_cleanup_orders_preserve_b() {
    for replace_first in [true, false] {
        let nodes = nodes();
        let routes = new_connected_routes();
        publish(&nodes, &routes, "same-node", "A").await;
        if !replace_first {
            remove_connection(&nodes, &routes, "same-node", "A").await;
        }
        publish(&nodes, &routes, "same-node", "B").await;
        if replace_first {
            remove_connection(&nodes, &routes, "same-node", "A").await;
        }
        refresh_routes(&nodes, &routes).await;
        assert_eq!(
            routes.read().await["same-node"].connection_id.as_deref(),
            Some("B")
        );
        assert_eq!(coordinate_snapshot(&routes).await, vec![coord()]);
    }
}

#[tokio::test]
async fn current_close_wrong_close_and_coordinate_dedup() {
    let nodes = nodes();
    let routes = new_connected_routes();
    publish(&nodes, &routes, "one", "A").await;
    publish(&nodes, &routes, "two", "B").await;
    assert_eq!(coordinate_snapshot(&routes).await, vec![coord()]);
    remove_connection(&nodes, &routes, "two", "A").await;
    remove_connection(&nodes, &routes, "two", "").await;
    assert_eq!(routes.read().await.len(), 2);
    remove_connection(&nodes, &routes, "one", "A").await;
    assert!(!routes.read().await.contains_key("one"));
    assert_eq!(
        routes.read().await["two"].connection_id.as_deref(),
        Some("B")
    );
    assert_eq!(coordinate_snapshot(&routes).await, vec![coord()]);
}

#[tokio::test]
async fn discovery_anonymous_hints_are_routable_but_not_authentication() {
    let nodes = nodes();
    let routes = new_connected_routes();
    publish_node(&nodes, &routes, node("discovered")).await;
    refresh_routes(&nodes, &routes).await;
    assert_eq!(routes.read().await["discovered"].connection_id, None);
    remove_connection(&nodes, &routes, "discovered", "untrusted").await;
    assert_eq!(coordinate_snapshot(&routes).await, vec![coord()]);
    let peers = crate::network::peer_auth::new_authenticated_peers();
    assert!(
        !crate::network::peer_auth::verify_peer_access(&peers, "discovered", "untrusted", "net-1")
            .await
    );
}

#[tokio::test]
async fn detached_snapshot_releases_routing_guard() {
    let nodes = nodes();
    let routes = new_connected_routes();
    publish(&nodes, &routes, "one", "A").await;
    let snapshot = coordinate_snapshot(&routes).await;
    assert!(routes.try_write().is_ok());
    assert_eq!(snapshot, vec![coord()]);
}

mod interleavings;
mod propagation;
