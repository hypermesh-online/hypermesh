use super::*;

impl NetworkManager {
    pub(super) fn peer_endpoint(&self, addr: SocketAddr) -> Result<stoq::Endpoint> {
        // Always use the NetworkManager handshake path (flat JSON).
        // MatrixStoqIntegration::connect_to_node sends MatrixMessage::Announcement
        // which is incompatible with handle_incoming_handshake on the accept side.
        let ipv6 = match addr {
            SocketAddr::V6(v6) => *v6.ip(),
            SocketAddr::V4(v4) => v4.ip().to_ipv6_mapped(),
        };

        // WAN address validation: reject loopback when WAN mode is active
        let wan_enabled = self.transport.wan_enabled();
        if wan_enabled && !is_valid_wan_address(&ipv6) {
            return Err(anyhow!(
                "Cannot connect to loopback address {} in WAN mode",
                ipv6
            ));
        }

        // Warn about link-local addresses over WAN — they may not route
        if wan_enabled && (ipv6.segments()[0] & 0xffc0) == 0xfe80 {
            warn!(
                "Connecting to link-local address {} over WAN — may not route across subnets",
                ipv6
            );
        }
        let endpoint = stoq::Endpoint::new(ipv6, addr.port());

        Ok(endpoint)
    }

    pub(super) async fn store_connected_peer(
        &self,
        node_info: &NetworkNode,
        handshake: &HandshakeData,
    ) {
        super::connected_peer_routes::publish_node(
            &self.nodes,
            &self.connected_routes,
            node_info.clone(),
        )
        .await;

        info!(
            "Successfully connected to node {} at ({},{},{}) — bilateral PoS verified (proof={} bytes, pubkey={} bytes)",
            node_info.node_id,
            node_info.coordinate.x,
            node_info.coordinate.y,
            node_info.coordinate.z,
            handshake.peer_proof.len(),
            handshake.peer_pubkey.len(),
        );
    }

    pub(super) async fn register_connected_peer(
        &self,
        node_info: &NetworkNode,
        handshake: HandshakeData,
        connection: &stoq::Connection,
    ) -> Result<()> {
        let registered = peer_auth::register_authenticated_peer(
            &self.authenticated_peers,
            peer_auth::AuthenticatedPeer {
                node_id: node_info.node_id.clone(),
                connection_id: connection.id(),
                pubkey: handshake.peer_pubkey,
                coordinate: (
                    node_info.coordinate.x as i32,
                    node_info.coordinate.y as i32,
                    node_info.coordinate.z as i32,
                ),
                network_id: handshake.peer_network_id,
                authenticated_at: std::time::Instant::now(),
                proof_bytes: handshake.peer_proof,
            },
        )
        .await;

        if !registered {
            warn!(
                "Peer {} failed authentication registration — disconnecting",
                &node_info.node_id[..8.min(node_info.node_id.len())]
            );
            super::connected_peer_routes::remove_connection(
                &self.nodes,
                &self.connected_routes,
                &node_info.node_id,
                &connection.id(),
            )
            .await;
            return Err(anyhow!(
                "Peer {} bilateral PoS verification incomplete — proof or pubkey missing",
                node_info.node_id
            ));
        }

        Ok(())
    }

    /// Perform bilateral handshake via STOQ protocol layer.
    ///
    /// Opens a stream, writes a `CONN_TYPE_HANDSHAKE` discriminator byte,
    /// then delegates to `stoq::initiate_handshake_on_stream()` which
    /// implements the 3-message challenge-response protocol (R11) using
    /// NodeSigner and StateProofProvider traits.
    ///
    /// After the STOQ crypto handshake, exchanges a blockmatrix-level
    /// metadata message carrying `network_id` (for multi-node sync).
    ///
    /// Returns both the `NetworkNode` and the raw proof/pubkey data so
    /// the caller can register a fully-verified `AuthenticatedPeer`.
    pub(super) async fn exchange_node_info(
        &self,
        connection: &Arc<stoq::Connection>,
        our_network_id: &str,
    ) -> Result<HandshakeData> {
        let coord = self.local_coordinate;
        let local_coord = (coord.x, coord.y, coord.z);

        // Open stream and write connection-type discriminator before handshake
        let mut stream = connection.open_stream().await?;
        stream.write_discriminator(CONN_TYPE_HANDSHAKE).await?;

        let result = stoq::initiate_handshake_on_stream(
            &mut stream,
            self.signer.as_ref(),
            self.proof_provider.as_ref(),
            local_coord,
        )
        .await?;

        let peer_network_id = self
            .exchange_peer_metadata(&mut stream, our_network_id)
            .await?;

        let _ = stream.finish_send();

        let coordinate = MatrixCoordinate::new(
            result.peer_coordinate.0,
            result.peer_coordinate.1,
            result.peer_coordinate.2,
        )
        .map_err(|e| anyhow!("Invalid peer coordinate: {e}"))?;

        Ok(HandshakeData {
            node: NetworkNode {
                coordinate,
                address: connection.endpoint().to_socket_addr(),
                node_id: result.peer_node_id,
                privacy_mode: PrivacyMode::PUBLIC, // Will be negotiated later
                connection: Some(connection.clone()),
            },
            peer_pubkey: result.peer_pubkey,
            peer_proof: result.peer_proof,
            // Ingress boundary: the peer's advertised wire string becomes its
            // canonical NetworkId here (and only here on this path).
            peer_network_id: NetworkId::from_wire_str(&peer_network_id),
        })
    }

    async fn exchange_peer_metadata(
        &self,
        stream: &mut stoq::Stream,
        our_network_id: &str,
    ) -> Result<String> {
        // Post-handshake metadata exchange (blockmatrix layer).
        // Initiator sends metadata first, then reads peer's metadata.
        let our_meta = HandshakeMetadata {
            network_id: our_network_id.to_string(),
        };
        let meta_bytes = serde_json::to_vec(&our_meta)
            .map_err(|e| anyhow!("Failed to serialize handshake metadata: {e}"))?;
        stream.write_msg(&meta_bytes).await?;

        Ok(Self::read_peer_metadata(stream).await)
    }

    async fn read_peer_metadata(stream: &mut stoq::Stream) -> String {
        // Read peer's metadata (with timeout tolerance for old nodes).
        match tokio::time::timeout(std::time::Duration::from_secs(5), stream.read_msg()).await {
            Ok(Ok(peer_meta_bytes)) => {
                let peer_meta: HandshakeMetadata =
                    serde_json::from_slice(&peer_meta_bytes).unwrap_or_default();
                info!(
                    "Received peer network_id from metadata: '{}'",
                    peer_meta.network_id
                );
                peer_meta.network_id
            }
            Ok(Err(e)) => {
                warn!("Peer did not send handshake metadata: {e}");
                String::new()
            }
            Err(_) => {
                warn!("Timeout waiting for peer handshake metadata — assuming old node");
                String::new()
            }
        }
    }
}
