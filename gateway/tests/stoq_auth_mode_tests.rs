// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for the configurable STOQ gateway auth mode (F8).
//!
//! Two modes, driven by `StoqAuthMode` config:
//!
//! * **HTTP-proxy mode** — the listener accepts connections and runs the
//!   handler WITHOUT a bilateral PoS handshake (backwards-compatible
//!   reverse-proxy passthrough).
//! * **Full-STOQ-PoS mode** — every incoming connection MUST complete a
//!   bilateral PoS handshake (FALCON-1024 identity + four-proof state
//!   proof, inheriting the F2 signer↔identity binding). Connections that
//!   do not handshake are rejected (dropped).
//!
//! These tests use real STOQ QUIC transports on localhost (mirroring
//! `blockmatrix/tests/bilateral_e2e.rs`), a real FALCON-1024 gateway
//! identity, and a real `TrustChainProofProvider`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hypermesh_lib::{NodeSigner, StateProofProvider};
use stoq::transport::connection::Endpoint;
use stoq::{StoqTransport, TransportConfig};
use trustchain::identity::FalconIdentity;
use trustchain::proof_of_state::TrustChainProofProvider;

// The gateway exposes these modules via its `lib.rs`, so integration
// tests drive the real code paths through the crate's public surface.
use gateway::config::StoqAuthMode;
use gateway::stoq_bridge::{StoqBridge, StoqBridgeConfig};
use gateway::stoq_listener::StoqListener;

/// Build a STOQ transport bound to an OS-assigned localhost port.
async fn make_transport() -> Arc<StoqTransport> {
    let config = TransportConfig {
        port: 0,
        ..TransportConfig::default()
    };
    Arc::new(
        StoqTransport::new(config)
            .await
            .expect("test: build transport"),
    )
}

/// Generate a real FALCON-1024 signer + TrustChain proof provider.
fn make_pos_identity() -> (Arc<dyn NodeSigner>, Arc<dyn StateProofProvider>) {
    let identity = FalconIdentity::generate();
    let node_id = identity.node_id.clone();
    let signer: Arc<dyn NodeSigner> = Arc::new(identity);
    let proof_provider: Arc<dyn StateProofProvider> =
        Arc::new(TrustChainProofProvider::new(node_id, signer.clone()));
    (signer, proof_provider)
}

/// HTTP-proxy mode: a connection with NO handshake is accepted and the
/// handler runs. This is the backwards-compatible passthrough behavior.
#[tokio::test]
async fn http_proxy_mode_proxies_without_handshake() {
    let bridge_config = StoqBridgeConfig {
        bind_addr: "[::1]:0".parse().expect("test: addr"),
        auth_mode: StoqAuthMode::HttpProxy,
        ..StoqBridgeConfig::default()
    };
    let bridge = Arc::new(
        StoqBridge::new(bridge_config)
            .await
            .expect("test: http-proxy bridge builds without a signer"),
    );
    let bind_addr = bridge.local_addr().expect("test: local addr");

    // Handler records that it ran (proxy handoff would happen here).
    let handled = Arc::new(AtomicBool::new(false));
    let handled_clone = Arc::clone(&handled);

    let listener = StoqListener::new(Arc::clone(&bridge));
    let listener_handle = tokio::spawn(async move {
        let _ = listener
            .run(move |_info| {
                let handled = Arc::clone(&handled_clone);
                async move {
                    handled.store(true, Ordering::SeqCst);
                    Ok(())
                }
            })
            .await;
    });

    // Client connects with a plain STOQ transport and sends NO handshake.
    let client = make_transport().await;
    let endpoint = Endpoint::new(std::net::Ipv6Addr::LOCALHOST, bind_addr.port());
    let _conn = client
        .connect(&endpoint)
        .await
        .expect("test: client connects (no handshake)");

    // Give the listener a moment to accept + run the handler.
    let ran = wait_until(Duration::from_secs(5), || handled.load(Ordering::SeqCst)).await;
    assert!(
        ran,
        "http-proxy mode must run the handler without any PoS handshake"
    );

    listener_handle.abort();
    bridge.shutdown().await;
}

/// Full-STOQ-PoS mode: a connection that does NOT complete the bilateral
/// handshake is rejected — the handler must NEVER run for it.
#[tokio::test]
async fn full_stoq_pos_mode_rejects_unhandshaked_connection() {
    let (signer, proof_provider) = make_pos_identity();

    let bridge_config = StoqBridgeConfig {
        bind_addr: "[::1]:0".parse().expect("test: addr"),
        auth_mode: StoqAuthMode::FullStoqPos,
        local_coordinate: (5, 6, 7),
        ..StoqBridgeConfig::default()
    };
    let bridge = Arc::new(
        StoqBridge::new_with_pos(bridge_config, signer, proof_provider)
            .await
            .expect("test: full-stoq-pos bridge builds with signer"),
    );
    let bind_addr = bridge.local_addr().expect("test: local addr");

    // If the handler ever runs, that is a security failure.
    let handled = Arc::new(AtomicBool::new(false));
    let handled_clone = Arc::clone(&handled);

    let listener = StoqListener::new(Arc::clone(&bridge));
    let listener_handle = tokio::spawn(async move {
        let _ = listener
            .run(move |_info| {
                let handled = Arc::clone(&handled_clone);
                async move {
                    handled.store(true, Ordering::SeqCst);
                    Ok(())
                }
            })
            .await;
    });

    // Client connects but NEVER opens a handshake stream / sends Msg1.
    let client = make_transport().await;
    let endpoint = Endpoint::new(std::net::Ipv6Addr::LOCALHOST, bind_addr.port());
    let _conn = client
        .connect(&endpoint)
        .await
        .expect("test: client connects but does not handshake");

    // Wait long enough for the accept + a handshake attempt to fail. The
    // handshake waits on `accept_stream` which never arrives; the acceptor
    // side must reject and never run the handler. We assert the handler
    // stays un-run for a bounded window.
    let ran = wait_until(Duration::from_secs(3), || handled.load(Ordering::SeqCst)).await;
    assert!(
        !ran,
        "full-stoq-pos mode must NOT run the handler for an un-handshaked connection"
    );

    listener_handle.abort();
    bridge.shutdown().await;
}

/// Full-STOQ-PoS mode: a peer that DOES complete the bilateral handshake
/// is accepted and the handler runs. Proves the mode does not reject
/// legitimate authenticated peers.
#[tokio::test]
async fn full_stoq_pos_mode_accepts_valid_handshake() {
    let (gw_signer, gw_provider) = make_pos_identity();

    let bridge_config = StoqBridgeConfig {
        bind_addr: "[::1]:0".parse().expect("test: addr"),
        auth_mode: StoqAuthMode::FullStoqPos,
        local_coordinate: (1, 1, 1),
        ..StoqBridgeConfig::default()
    };
    let bridge = Arc::new(
        StoqBridge::new_with_pos(bridge_config, gw_signer, gw_provider)
            .await
            .expect("test: bridge builds"),
    );
    let bind_addr = bridge.local_addr().expect("test: local addr");

    let handled = Arc::new(AtomicBool::new(false));
    let handled_clone = Arc::clone(&handled);

    let listener = StoqListener::new(Arc::clone(&bridge));
    let listener_handle = tokio::spawn(async move {
        let _ = listener
            .run(move |_info| {
                let handled = Arc::clone(&handled_clone);
                async move {
                    handled.store(true, Ordering::SeqCst);
                    Ok(())
                }
            })
            .await;
    });

    // A real peer with its own identity completes the initiator side of the
    // bilateral handshake.
    let (peer_signer, peer_provider) = make_pos_identity();
    let client = make_transport().await;
    let endpoint = Endpoint::new(std::net::Ipv6Addr::LOCALHOST, bind_addr.port());
    let conn = client
        .connect(&endpoint)
        .await
        .expect("test: peer connects");

    let handshake =
        stoq::initiate_handshake(&conn, peer_signer.as_ref(), peer_provider.as_ref(), (9, 9, 9));
    let result = tokio::time::timeout(Duration::from_secs(10), handshake).await;
    assert!(
        matches!(result, Ok(Ok(_))),
        "peer's initiate_handshake should complete against the gateway: {result:?}"
    );

    let ran = wait_until(Duration::from_secs(5), || handled.load(Ordering::SeqCst)).await;
    assert!(
        ran,
        "full-stoq-pos mode must run the handler after a valid PoS handshake"
    );

    listener_handle.abort();
    bridge.shutdown().await;
}

/// Constructing a FullStoqPos bridge via `new` (no signer) is a config
/// error — the security-critical mode cannot be silently degraded to
/// passthrough.
#[tokio::test]
async fn full_stoq_pos_via_new_without_signer_is_error() {
    let bridge_config = StoqBridgeConfig {
        bind_addr: "[::1]:0".parse().expect("test: addr"),
        auth_mode: StoqAuthMode::FullStoqPos,
        ..StoqBridgeConfig::default()
    };
    let result = StoqBridge::new(bridge_config).await;
    assert!(
        result.is_err(),
        "full-stoq-pos mode must not build without a signer via new()"
    );
}

/// Poll `cond` until it is true or `timeout` elapses. Returns the final
/// value of `cond`.
async fn wait_until<F: Fn() -> bool>(timeout: Duration, cond: F) -> bool {
    let start = std::time::Instant::now();
    loop {
        if cond() {
            return true;
        }
        if start.elapsed() >= timeout {
            return cond();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
