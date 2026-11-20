// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Real loopback STOQ acceptance lifecycle; anonymous transport, no auth claim.

use super::{NetworkManager, PrivacyMode};
use crate::matrix::coordinate::MatrixCoordinate;
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::Duration;
use stoq::transport::NetworkType;

#[tokio::test]
async fn idle_accept_waits_and_endpoint_close_finishes_once() {
    let config = stoq::TransportConfig {
        port: 0,
        bind_address: Ipv6Addr::LOCALHOST,
        enable_kernel_acceleration: false,
        ..stoq::TransportConfig::default()
    };
    let transport = Arc::new(
        stoq::StoqTransport::new_for_network(config, NetworkType::Anonymous)
            .await
            .expect("anonymous loopback STOQ binds"),
    );
    let coord = MatrixCoordinate::new(0, 0, 0).expect("valid coordinate");
    let identity = crate::identity::FalconIdentity::generate();
    let signer: Arc<dyn hypermesh_lib::NodeSigner> = Arc::new(identity);
    let proof: Arc<dyn hypermesh_lib::StateProofProvider> =
        Arc::new(crate::proof_of_state::BlockMatrixProofProvider::new(
            signer.node_id().to_string(),
            signer.clone(),
        ));
    let manager = Arc::new(
        NetworkManager::new(
            coord,
            transport.clone(),
            PrivacyMode::ANONYMOUS,
            vec![],
            signer,
            proof,
            "accept-lifecycle".to_string(),
        )
        .await
        .expect("manager initializes"),
    );
    let mut task = tokio::spawn(async move { manager.accept_connections(None).await });
    assert!(tokio::time::timeout(Duration::from_millis(100), &mut task)
        .await
        .is_err());
    transport.shutdown().await;
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .expect("closed endpoint stops accept loop")
        .expect("accept task joins")
        .expect("terminal close is orderly");
}
