//! Session-owned routing hints. None is discovery metadata, never admission.
use super::NetworkNode;
use crate::matrix::coordinate::MatrixCoordinate;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type NetworkNodes = Arc<RwLock<HashMap<String, NetworkNode>>>;
pub type ConnectedPeerRoutes = Arc<RwLock<HashMap<String, ConnectedRoute>>>;

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectedRoute {
    pub connection_id: Option<String>,
    pub coordinate: MatrixCoordinate,
}

pub fn new_connected_routes() -> ConnectedPeerRoutes {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Production session identity comes only from the local transport object.
pub async fn publish_node(nodes: &NetworkNodes, routes: &ConnectedPeerRoutes, node: NetworkNode) {
    let connection_id = node.connection.as_ref().map(|connection| connection.id());
    publish_with_identity(nodes, routes, node, connection_id).await;
}

async fn publish_with_identity(
    nodes: &NetworkNodes,
    routes: &ConnectedPeerRoutes,
    node: NetworkNode,
    connection_id: Option<String>,
) {
    let mut nodes = nodes.write().await;
    let mut routes = routes.write().await;
    routes.insert(
        node.node_id.clone(),
        ConnectedRoute {
            connection_id,
            coordinate: node.coordinate,
        },
    );
    nodes.insert(node.node_id.clone(), node);
}

/// Lock order is always source nodes then routes, including refresh/close.
/// A discovery-only projection must not downgrade an existing live owner.
pub async fn refresh_routes(nodes: &NetworkNodes, routes: &ConnectedPeerRoutes) {
    let nodes = nodes.read().await;
    let mut routes = routes.write().await;
    routes.retain(|node_id, _| nodes.contains_key(node_id));
    for (node_id, node) in nodes.iter() {
        let connection_id = node.connection.as_ref().map(|connection| connection.id());
        if connection_id.is_none()
            && routes
                .get(node_id)
                .is_some_and(|route| route.connection_id.is_some())
        {
            continue;
        }
        routes.insert(
            node_id.clone(),
            ConnectedRoute {
                connection_id,
                coordinate: node.coordinate,
            },
        );
    }
}

/// Remove only this session's route and matching live source metadata.
/// A discovery hint with no transport ownership remains independent.
pub async fn remove_connection(
    nodes: &NetworkNodes,
    routes: &ConnectedPeerRoutes,
    node_id: &str,
    connection_id: &str,
) {
    if connection_id.is_empty() {
        return;
    }
    let mut nodes = nodes.write().await;
    let mut routes = routes.write().await;
    if !routes
        .get(node_id)
        .is_some_and(|route| route.connection_id.as_deref() == Some(connection_id))
    {
        return;
    }
    routes.remove(node_id);
    let owns_node = nodes
        .get(node_id)
        .and_then(|node| node.connection.as_ref())
        .is_some_and(|connection| connection.id() == connection_id);
    if owns_node {
        nodes.remove(node_id);
    }
}

/// Return detached, deduplicated coordinates; no guard escapes to network IO.
pub async fn coordinate_snapshot(routes: &ConnectedPeerRoutes) -> Vec<MatrixCoordinate> {
    let mut coordinates: Vec<_> = routes
        .read()
        .await
        .values()
        .map(|route| route.coordinate)
        .collect();
    coordinates.sort_by_key(|coordinate| (coordinate.x, coordinate.y, coordinate.z));
    coordinates.dedup_by_key(|coordinate| (coordinate.x, coordinate.y, coordinate.z));
    coordinates
}

#[cfg(test)]
mod tests;
