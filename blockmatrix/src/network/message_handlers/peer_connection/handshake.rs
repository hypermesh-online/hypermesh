use super::*;

/// Run bilateral PoS handshake, register the peer, optionally spawn message loop.
pub(super) async fn handle_handshake_connection(
    connection: Arc<stoq::Connection>,
    stream: &mut stoq::Stream,
    nodes: Arc<RwLock<HashMap<String, NetworkNode>>>,
    local_coord: MatrixCoordinate,
    signer: Arc<dyn hypermesh_lib::NodeSigner>,
    proof_provider: Arc<dyn hypermesh_lib::StateProofProvider>,
    cert_manager: Arc<stoq::transport::certificates::CertificateManager>,
    peer_ctx: Option<Arc<PeerContext>>,
    authenticated_peers: AuthenticatedPeers,
    connected_routes: crate::network::connected_peer_routes::ConnectedPeerRoutes,
) -> Result<()> {
    debug!("Accepted incoming connection — handshake discriminator");
    let coord_tuple = (local_coord.x, local_coord.y, local_coord.z);

    let result = stoq::accept_handshake(
        stream,
        signer.as_ref(),
        proof_provider.as_ref(),
        coord_tuple,
    )
    .await?;

    let peer_network_id = exchange_acceptor_metadata(stream, peer_ctx.as_deref()).await;

    let (peer_node_id, coordinate) = store_accepted_peer(
        &connection,
        &nodes,
        &authenticated_peers,
        &connected_routes,
        &result,
        &peer_network_id,
    )
    .await?;

    finish_accepted_peer(
        connection,
        peer_node_id,
        coordinate,
        peer_ctx,
        cert_manager,
        signer,
        &result,
    )
    .await;

    Ok(())
}

/// Spawn CA enrollment on the acceptor side after a successful handshake.
fn spawn_acceptor_ca_enrollment(
    cert_manager: Arc<stoq::transport::certificates::CertificateManager>,
    node_id: String,
) {
    tokio::spawn(async move {
        enroll_acceptor_ca(cert_manager, node_id).await;
    });
}

async fn exchange_acceptor_metadata(
    stream: &mut stoq::Stream,
    peer_ctx: Option<&PeerContext>,
) -> String {
    let peer_network_id = read_acceptor_metadata(stream).await;
    // Send our metadata back to the initiator
    let our_network_id = peer_ctx.map(|c| c.network_id.as_str()).unwrap_or("");
    let our_meta = crate::network::HandshakeMetadata {
        network_id: our_network_id.to_string(),
    };
    if let Ok(meta_bytes) = serde_json::to_vec(&our_meta) {
        if let Err(e) = stream.write_msg(&meta_bytes).await {
            debug!("Failed to send handshake metadata to peer: {e}");
        }
    }

    peer_network_id
}

async fn read_acceptor_metadata(stream: &mut stoq::Stream) -> String {
    // Post-handshake metadata exchange (blockmatrix layer).
    // Acceptor reads initiator's metadata first, then sends its own.
    match tokio::time::timeout(std::time::Duration::from_secs(5), stream.read_msg()).await {
        Ok(Ok(peer_meta_bytes)) => {
            let peer_meta: crate::network::HandshakeMetadata =
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
async fn register_accepted_peer(
    authenticated_peers: &AuthenticatedPeers,
    connected_routes: &crate::network::connected_peer_routes::ConnectedPeerRoutes,
    nodes: &crate::network::connected_peer_routes::NetworkNodes,
    connection: &stoq::Connection,
    peer_node_id: &str,
    coordinate: MatrixCoordinate,
    result: &stoq::protocol::bilateral::HandshakeResult,
    peer_network_id: &str,
) -> Result<()> {
    // Register the accepted peer as authenticated (PoS handshake passed).
    // register_authenticated_peer enforces that proof_bytes and pubkey are
    // non-empty (R11 bilateral verification).
    // Use the network_id received from the peer during metadata exchange,
    // NOT our own network_id.
    let registered = peer_auth::register_authenticated_peer(
        authenticated_peers,
        peer_auth::AuthenticatedPeer {
            node_id: peer_node_id.to_string(),
            connection_id: connection.id(),
            pubkey: result.peer_pubkey.clone(),
            coordinate: (
                coordinate.x as i32,
                coordinate.y as i32,
                coordinate.z as i32,
            ),
            // Ingress boundary: peer's advertised wire string -> canonical NetworkId.
            network_id: hypermesh_lib::NetworkId::from_wire_str(&peer_network_id),
            authenticated_at: std::time::Instant::now(),
            proof_bytes: result.peer_proof.clone(),
        },
    )
    .await;

    if !registered {
        return reject_accepted_peer(nodes, connected_routes, connection, peer_node_id).await;
    }

    Ok(())
}
async fn finish_accepted_peer(
    connection: Arc<stoq::Connection>,
    peer_node_id: String,
    coordinate: MatrixCoordinate,
    peer_ctx: Option<Arc<PeerContext>>,
    cert_manager: Arc<stoq::transport::certificates::CertificateManager>,
    signer: Arc<dyn hypermesh_lib::NodeSigner>,
    result: &stoq::protocol::bilateral::HandshakeResult,
) {
    info!(
        "Bilateral verification complete — added authenticated node {} (proof={} bytes, pubkey={} bytes)",
        &peer_node_id[..8.min(peer_node_id.len())],
        result.peer_proof.len(),
        result.peer_pubkey.len(),
    );

    // P5 unification: mirror this authenticated peer into the kernel
    // fast-path maps, keyed on the SAME IPv6 source P1 just authorized.
    // Writes pos_header_map[src].validated=1 + policy_map[src].requires_pos=1
    // so the XDP gate admits this source's HyperMesh traffic. No-op unless
    // an XDP program is attached (graceful degradation).
    if let Some(ref ctx) = peer_ctx {
        mirror_peer_auth_to_kernel(ctx, &connection);
    }

    // Request CA certificate in background (Phase 2 bootstrap)
    spawn_acceptor_ca_enrollment(cert_manager, signer.node_id().to_string());

    // Register accepted peer as a reflector for proactive sync
    if let Some(ref ctx) = peer_ctx {
        register_peer_as_reflector(ctx, &peer_node_id, coordinate).await;
    }

    if let Some(ctx) = peer_ctx {
        tokio::spawn(async move {
            run_peer_message_loop(connection, peer_node_id, coordinate, ctx).await;
        });
    }
}

async fn store_accepted_peer(
    connection: &Arc<stoq::Connection>,
    nodes: &crate::network::connected_peer_routes::NetworkNodes,
    authenticated_peers: &AuthenticatedPeers,
    connected_routes: &crate::network::connected_peer_routes::ConnectedPeerRoutes,
    result: &stoq::HandshakeResult,
    peer_network_id: &str,
) -> Result<(String, MatrixCoordinate)> {
    let coordinate = MatrixCoordinate::new(
        result.peer_coordinate.0,
        result.peer_coordinate.1,
        result.peer_coordinate.2,
    )
    .map_err(|e| anyhow!("Invalid peer coordinate: {e}"))?;

    let peer_node_id = result.peer_node_id.clone();

    let node = NetworkNode {
        coordinate,
        address: connection.endpoint().to_socket_addr(),
        node_id: peer_node_id.clone(),
        privacy_mode: PrivacyMode::PUBLIC,
        connection: Some(connection.clone()),
    };

    crate::network::connected_peer_routes::publish_node(nodes, connected_routes, node).await;

    register_accepted_peer(
        &authenticated_peers,
        connected_routes,
        &nodes,
        &connection,
        &peer_node_id,
        coordinate,
        &result,
        &peer_network_id,
    )
    .await?;

    Ok((peer_node_id, coordinate))
}

async fn enroll_acceptor_ca(
    cert_manager: Arc<stoq::transport::certificates::CertificateManager>,
    node_id: String,
) {
    let state_proof = match crate::network::ca_enrollment::generate_node_state_proof(&node_id).await
    {
        Ok(sp) => sp,
        Err(e) => {
            warn!("CA enrollment (acceptor): state proof generation failed: {e}");
            return;
        }
    };
    crate::network::ca_enrollment::spawn_ca_enrollment(cert_manager, node_id, state_proof);
}

async fn reject_accepted_peer(
    nodes: &crate::network::connected_peer_routes::NetworkNodes,
    connected_routes: &crate::network::connected_peer_routes::ConnectedPeerRoutes,
    connection: &stoq::Connection,
    peer_node_id: &str,
) -> Result<()> {
    warn!(
        "Peer {} failed authentication registration — bilateral PoS incomplete, disconnecting",
        &peer_node_id[..8.min(peer_node_id.len())]
    );
    crate::network::connected_peer_routes::remove_connection(
        nodes,
        connected_routes,
        peer_node_id,
        &connection.id(),
    )
    .await;
    Err(anyhow!(
        "Peer {} bilateral PoS verification incomplete — proof or pubkey missing",
        peer_node_id
    ))
}
