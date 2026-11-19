use super::*;

#[tokio::test]
async fn queued_registration_and_cleanup_are_serialized_in_both_orders() {
    for replacement_first in [true, false] {
        check_order(replacement_first).await;
    }
}

async fn check_order(replacement_first: bool) {
    let nodes = nodes();
    let routes = new_connected_routes();
    publish(&nodes, &routes, "same-node", "A").await;
    let guard = nodes.write().await;
    let first = spawn_action(&nodes, &routes, replacement_first);
    tokio::task::yield_now().await;
    let second = spawn_action(&nodes, &routes, !replacement_first);
    tokio::task::yield_now().await;
    drop(guard);
    first.await.unwrap();
    second.await.unwrap();
    assert_eq!(
        routes.read().await["same-node"].connection_id.as_deref(),
        Some("B")
    );
}

fn spawn_action(
    nodes: &NetworkNodes,
    routes: &ConnectedPeerRoutes,
    replacement: bool,
) -> tokio::task::JoinHandle<()> {
    let nodes = nodes.clone();
    let routes = routes.clone();
    tokio::spawn(async move {
        if replacement {
            publish(&nodes, &routes, "same-node", "B").await;
        } else {
            remove_connection(&nodes, &routes, "same-node", "A").await;
        }
    })
}

#[tokio::test]
async fn refresh_does_not_downgrade_session_ownership_or_replacement() {
    let nodes = nodes();
    let routes = new_connected_routes();
    publish(&nodes, &routes, "same-node", "B").await;
    let mut hint = node("same-node");
    hint.coordinate = MatrixCoordinate::new(9, 9, 9).unwrap();
    nodes.write().await.insert("same-node".into(), hint);
    let guard = nodes.write().await;
    let n = nodes.clone();
    let r = routes.clone();
    let refresh = tokio::spawn(async move {
        refresh_routes(&n, &r).await;
    });
    tokio::task::yield_now().await;
    let close = spawn_action(&nodes, &routes, false);
    tokio::task::yield_now().await;
    drop(guard);
    refresh.await.unwrap();
    close.await.unwrap();
    let map = routes.read().await;
    assert_eq!(map["same-node"].connection_id.as_deref(), Some("B"));
    assert_eq!(map["same-node"].coordinate, coord());
}
