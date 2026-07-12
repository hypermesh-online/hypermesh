// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration test: Two-node shard exchange via StoqShardTransport.
//!
//! - Starts two STOQ transports on different localhost ports (OS-assigned)
//! - Creates StoqShardTransport + ShardStore for each node
//! - Registers addresses, sends shard A->B, verifies B stored it
//! - Fetches shard from B back to A, verifies round-trip

use std::sync::Arc;

use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::{
    handle_incoming_shard_stream, ShardTransport, StoqShardTransport,
};
use hypermesh_lib::{ContentHash, NodeId};

/// Build a default STOQ transport config bound to localhost with OS-assigned port.
fn localhost_config() -> stoq::TransportConfig {
    stoq::TransportConfig {
        port: 0,
        bind_address: std::net::Ipv6Addr::LOCALHOST,
        ..Default::default()
    }
}

/// Try to create a StoqTransport. Returns None if socket binding fails
/// (CI/sandboxed environments).
async fn try_create_transport() -> Option<Arc<stoq::StoqTransport>> {
    match stoq::StoqTransport::new(localhost_config()).await {
        Ok(t) => Some(Arc::new(t)),
        Err(e) => {
            eprintln!("Skipping: STOQ transport init failed: {e}");
            None
        }
    }
}

/// Spawn a background task that accepts one connection and handles incoming
/// shard streams on it. The task processes up to `max_streams` streams, then
/// exits. Returns a JoinHandle that can be aborted for cleanup.
fn spawn_shard_acceptor(
    transport: Arc<stoq::StoqTransport>,
    store: Arc<ShardStore>,
    max_streams: usize,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let connection = match transport.accept().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Acceptor: failed to accept connection: {e}");
                return;
            }
        };

        for i in 0..max_streams {
            match connection.accept_stream().await {
                Ok(mut stream) => {
                    let s = store.clone();
                    if let Err(e) = handle_incoming_shard_stream(&mut stream, &s).await {
                        eprintln!("Acceptor: stream {i} handler error: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("Acceptor: failed to accept stream {i}: {e}");
                    break;
                }
            }
        }
    })
}

#[tokio::test]
async fn two_node_send_shard() {
    // Ensure crypto provider is installed (idempotent)
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Create two STOQ transports (skip if unavailable)
    let transport_a = match try_create_transport().await {
        Some(t) => t,
        None => return,
    };
    let transport_b = match try_create_transport().await {
        Some(t) => t,
        None => return,
    };

    // Resolve actual bound addresses
    let _addr_a = transport_a
        .local_addr()
        .expect("test: transport A local addr");
    let addr_b = transport_b
        .local_addr()
        .expect("test: transport B local addr");

    // Build shard-level infrastructure for each node
    let shard_transport_a = Arc::new(StoqShardTransport::new(transport_a.clone()));
    let store_a = Arc::new(ShardStore::new());
    let store_b = Arc::new(ShardStore::new());

    // Node identities
    let node_b_id = NodeId::from_bytes([0xBB; 32]);

    // A needs to know B's address to auto-dial
    shard_transport_a
        .register_node_address(&node_b_id, addr_b)
        .await;

    // Spawn acceptor on B that will handle 1 incoming stream (the SHARD_SEND)
    let acceptor = spawn_shard_acceptor(transport_b.clone(), store_b.clone(), 1);

    // Give the acceptor a moment to start listening
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Prepare test shard.
    //
    // The shard_id MUST be the content hash of the shard data: the receive
    // path (`handle_incoming_shard_stream`) enforces the content-validity
    // invariant (`BLAKE3(shard_data) == shard_id`) and rejects any shard whose
    // claimed id does not match its bytes. Sending a content-valid shard is
    // exactly what this test now validates.
    let shard_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
    let shard_id = ContentHash(*blake3::hash(&shard_data).as_bytes());

    // A sends shard to B
    shard_transport_a
        .send_shard(&node_b_id, &shard_id, &shard_data)
        .await
        .expect("test: send_shard A->B should succeed");

    // Wait for B's acceptor to finish processing
    let timeout = tokio::time::Duration::from_secs(5);
    match tokio::time::timeout(timeout, acceptor).await {
        Ok(Ok(())) => {} // acceptor finished normally
        Ok(Err(e)) => panic!("test: acceptor task panicked: {e}"),
        Err(_) => panic!("test: acceptor did not finish within 5s"),
    }

    // Verify B has the shard
    assert!(
        store_b.has(&shard_id).await,
        "Node B should have the shard after A sent it"
    );
    let stored = store_b
        .get(&shard_id)
        .await
        .expect("test: B should return shard data");
    assert_eq!(stored, shard_data, "Shard data should match what A sent");
    assert_eq!(store_b.count().await, 1, "B should have exactly 1 shard");

    // Verify A's store is still empty (A sent, didn't store locally)
    assert_eq!(store_a.count().await, 0, "A should have 0 shards");
}

#[tokio::test]
async fn two_node_fetch_shard() {
    // Ensure crypto provider is installed
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Create two STOQ transports
    let transport_a = match try_create_transport().await {
        Some(t) => t,
        None => return,
    };
    let transport_b = match try_create_transport().await {
        Some(t) => t,
        None => return,
    };

    let addr_b = transport_b
        .local_addr()
        .expect("test: transport B local addr");

    let shard_transport_a = Arc::new(StoqShardTransport::new(transport_a.clone()));
    let store_b = Arc::new(ShardStore::new());

    let node_b_id = NodeId::from_bytes([0xBB; 32]);

    // A knows how to reach B
    shard_transport_a
        .register_node_address(&node_b_id, addr_b)
        .await;

    // Pre-populate B's store with a shard that A will fetch
    let shard_id = ContentHash([0x77; 32]);
    let shard_data: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE];
    store_b.store(shard_id, shard_data.clone()).await;

    // Spawn acceptor on B: handle 1 stream for the SHARD_FETCH request
    let acceptor = spawn_shard_acceptor(transport_b.clone(), store_b.clone(), 1);

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // A fetches shard from B
    let fetched = shard_transport_a
        .fetch_shard(&node_b_id, &shard_id)
        .await
        .expect("test: fetch_shard from B should succeed");

    assert_eq!(
        fetched, shard_data,
        "Fetched shard data should match what B had"
    );

    // Clean up acceptor
    let timeout = tokio::time::Duration::from_secs(5);
    match tokio::time::timeout(timeout, acceptor).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("test: acceptor panicked: {e}"),
        Err(_) => {
            // fetch_shard only uses 1 stream, so acceptor should have finished
            eprintln!("test: acceptor timed out (non-fatal, aborting)");
        }
    }
}
