// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Terminal-aware incoming connection loop.

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

use super::{message_handlers, NetworkManager, PeerContext};

impl NetworkManager {
    /// Accept peers until the STOQ endpoint closes or its driver is lost.
    /// Peer handshake failures remain per-connection errors, not a shutdown signal.
    pub async fn accept_connections(&self, peer_ctx: Option<Arc<PeerContext>>) -> Result<()> {
        info!("Starting to accept incoming connections");
        while self.accept_one(&peer_ctx).await {}
        Ok(())
    }

    async fn accept_one(&self, peer_ctx: &Option<Arc<PeerContext>>) -> bool {
        match self.transport.accept_next().await {
            Ok(Some(connection)) => self.dispatch_accepted(connection, peer_ctx).await,
            Ok(None) => {
                info!("STOQ endpoint closed; incoming accept loop stopped");
                false
            }
            Err(e) => {
                warn!("Failed to accept connection: {}", e);
                true
            }
        }
    }

    async fn dispatch_accepted(
        &self,
        connection: Arc<stoq::Connection>,
        peer_ctx: &Option<Arc<PeerContext>>,
    ) -> bool {
        if self.nodes.read().await.len() >= 50 {
            warn!("Max peers (50) reached, rejecting connection");
            return true;
        }
        self.spawn_incoming_handler(connection, peer_ctx.clone());
        true
    }

    fn spawn_incoming_handler(
        &self,
        connection: Arc<stoq::Connection>,
        peer_ctx: Option<Arc<PeerContext>>,
    ) {
        let nodes = self.nodes.clone();
        let connected_routes = self.connected_routes.clone();
        let local_coord = self.local_coordinate;
        let signer = self.signer.clone();
        let proof_provider = self.proof_provider.clone();
        let cert_manager = self.transport.cert_manager.clone();
        let auth_peers = self.authenticated_peers.clone();
        tokio::spawn(async move {
            if let Err(e) = message_handlers::handle_incoming_connection(
                connection,
                nodes,
                local_coord,
                signer,
                proof_provider,
                cert_manager,
                peer_ctx,
                auth_peers,
                connected_routes,
            )
            .await
            {
                warn!("Incoming connection handling failed: {e}");
            }
        });
    }
}
