// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network discovery methods for `NetworkManager`.
//!
//! Contains privacy-mode-aware peer discovery, mDNS, and gossip
//! protocol initialization. Extracted from `network/mod.rs`.

use anyhow::Result;
use tracing::{debug, error, info, warn};

use crate::bootstrap::PrivacyMode;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::neighbors::{find_k_nearest, find_neighbors};
use crate::network::gossip;
use crate::network::discovery;

use super::message_handlers::TAG_GOSSIP;
use super::{NetworkManager, NetworkNode};

/// Discovery and gossip methods for `NetworkManager`.
impl NetworkManager {
    /// Start network discovery based on privacy mode.
    pub async fn start_discovery(&self) -> Result<()> {
        let mode = *self.privacy_mode.read().await;

        if mode == PrivacyMode::PRIVATE {
            self.discover_private_peers().await
        } else if mode == PrivacyMode::ANONYMOUS {
            info!("Anonymous mode: Starting ephemeral discovery");
            self.discover_ephemeral_peers().await
        } else if mode == PrivacyMode::PUBLIC {
            info!("Public mode: Joining network with full discovery");
            self.join_network().await
        } else {
            info!("Custom privacy mode ({:?}): Starting peer discovery", mode);
            self.discover_peers().await
        }
    }

    /// Private mode: connect to bounded peer list only.
    async fn discover_private_peers(&self) -> Result<()> {
        if self.bootstrap_nodes.is_empty() {
            info!("Private mode: No peers, running as local-only device");
        } else {
            info!(
                "Private mode: Connecting to {} bootstrap peer(s) (bounded network)",
                self.bootstrap_nodes.len()
            );
            for bootstrap_addr in &self.bootstrap_nodes {
                if let Err(e) = self.connect_to_peer(*bootstrap_addr, None).await {
                    warn!("Failed to connect to private peer {}: {}", bootstrap_addr, e);
                }
            }
        }
        Ok(())
    }

    /// Anonymous mode: accept incoming only, no active discovery.
    async fn discover_ephemeral_peers(&self) -> Result<()> {
        info!("Anonymous mode: Listening for ephemeral connections");
        Ok(())
    }

    /// P2P mode: bootstrap nodes + mDNS.
    async fn discover_peers(&self) -> Result<()> {
        info!("P2P mode: Discovering peers via bootstrap nodes");

        for bootstrap_addr in &self.bootstrap_nodes {
            if let Err(e) = self.connect_to_peer(*bootstrap_addr, None).await {
                warn!("Failed to connect to bootstrap node {}: {}", bootstrap_addr, e);
            }
        }

        self.start_mdns_discovery().await?;
        Ok(())
    }

    /// Public mode: full network participation.
    async fn join_network(&self) -> Result<()> {
        info!("Public mode: Joining network with full participation");

        for bootstrap_addr in &self.bootstrap_nodes {
            match self.connect_to_peer(*bootstrap_addr, None).await {
                Ok(node_id) => {
                    info!("Connected to bootstrap node {} ({})", bootstrap_addr, node_id);
                }
                Err(e) => {
                    error!("Failed to connect to bootstrap {}: {}", bootstrap_addr, e);
                }
            }
        }

        self.start_mdns_discovery().await?;
        self.start_gossip_protocol().await?;
        Ok(())
    }

    /// Find neighbors in the matrix topology within a given radius.
    pub async fn find_matrix_neighbors(&self, radius: f64) -> Vec<NetworkNode> {
        let nodes = self.nodes.read().await;
        let candidates: Vec<MatrixCoordinate> = nodes.values().map(|n| n.coordinate).collect();
        let neighbors = find_neighbors(&self.local_coordinate, &candidates, radius);

        nodes
            .values()
            .filter(|n| neighbors.contains(&n.coordinate))
            .cloned()
            .collect()
    }

    /// Find K nearest neighbors with their distances.
    pub async fn find_k_nearest_nodes(&self, k: usize) -> Vec<(NetworkNode, f64)> {
        let nodes = self.nodes.read().await;
        let candidates: Vec<MatrixCoordinate> = nodes.values().map(|n| n.coordinate).collect();
        let nearest = find_k_nearest(&self.local_coordinate, &candidates, k);

        nearest
            .into_iter()
            .filter_map(|(coord, dist)| {
                nodes
                    .values()
                    .find(|n| n.coordinate == coord)
                    .map(|n| (n.clone(), dist))
            })
            .collect()
    }

    /// Start mDNS discovery for local network.
    pub(super) async fn start_mdns_discovery(&self) -> Result<()> {
        let node_id = self.get_node_id();
        let stoq_port = self.transport.local_addr().ok().map(|a| a.port()).unwrap_or(9292);

        let mdns = discovery::MdnsDiscovery::new(node_id, self.local_coordinate, stoq_port);
        match mdns.start().await {
            Ok(()) => {
                info!("mDNS discovery started on _hypermesh._udp.local");
            }
            Err(e) => {
                warn!("mDNS discovery failed to start: {e}");
            }
        }
        Ok(())
    }

    /// Start gossip protocol for network state sharing.
    pub(super) async fn start_gossip_protocol(&self) -> Result<()> {
        let node_id = self.get_node_id();
        let mode = *self.privacy_mode.read().await;
        let privacy_str = format!("{mode:?}");
        let stoq_port = self.transport.local_addr().ok().map(|a| a.port()).unwrap_or(9292);

        let gossip_proto = gossip::GossipProtocol::new(
            node_id,
            self.local_coordinate,
            stoq_port,
            privacy_str,
        );
        gossip_proto.start().await;

        let nodes = self.nodes.clone();
        let gossip_state = gossip_proto.state();
        let interval = gossip_proto.gossip_interval();

        tokio::spawn(async move {
            gossip_broadcast_loop(nodes, gossip_state, interval).await;
        });

        info!("Gossip protocol started with {} second interval", interval.as_secs());
        Ok(())
    }
}

/// Background loop that periodically sends gossip to connected peers.
async fn gossip_broadcast_loop(
    nodes: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, super::NetworkNode>>>,
    gossip_state: std::sync::Arc<tokio::sync::RwLock<gossip::GossipState>>,
    interval: std::time::Duration,
) {
    loop {
        tokio::time::sleep(interval).await;

        let connected = nodes.read().await;
        if connected.is_empty() {
            continue;
        }

        let state = gossip_state.read().await;
        let message = state.build_message();
        drop(state);

        let json_data = match serde_json::to_vec(&message) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let mut data = Vec::with_capacity(1 + json_data.len());
        data.push(TAG_GOSSIP);
        data.extend_from_slice(&json_data);

        for (node_id, node) in connected.iter() {
            if let Some(ref conn) = node.connection {
                match conn.open_stream().await {
                    Ok(mut stream) => {
                        if let Err(e) = stream.send(&data).await {
                            debug!("Gossip send to {node_id} failed: {e}");
                        }
                    }
                    Err(e) => {
                        debug!("Gossip stream to {node_id} failed: {e}");
                    }
                }
            }
        }
    }
}
