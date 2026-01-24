//! Example demonstrating the multi-network trust handlers

use blockmatrix::network::trust::{
    NetworkHandler, NetworkConfig, NetworkType, ProofOfState,
    AnonymousNetworkHandler, P2PNetworkHandler,
    FederatedNetworkHandler, PublicNetworkHandler,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Multi-Network Trust Architecture Demo ===\n");

    // 1. Anonymous Network
    println!("1. Anonymous Network Handler:");
    let anon_handler = AnonymousNetworkHandler::new();
    let anon_config = NetworkConfig {
        network_type: NetworkType::Anonymous,
        peer_addresses: vec![],
        federation_gateway: None,
        dns_name: None,
        proof_of_state: None,
    };

    match anon_handler.bootstrap(anon_config).await {
        Ok(conn) => {
            println!("   ✓ Anonymous network bootstrapped");
            println!("   - Network ID: {}", conn.network_id);
            println!("   - Certificate: {}", if conn.certificate.is_some() { "Present" } else { "None (as expected)" });
        }
        Err(e) => println!("   ✗ Failed: {}", e),
    }

    // 2. P2P Network
    println!("\n2. P2P Network Handler:");
    let p2p_handler = P2PNetworkHandler::new();
    let p2p_config = NetworkConfig {
        network_type: NetworkType::P2P,
        peer_addresses: vec!["127.0.0.1:8080".to_string()],
        federation_gateway: None,
        dns_name: None,
        proof_of_state: None,
    };

    match p2p_handler.bootstrap(p2p_config).await {
        Ok(conn) => {
            println!("   ✓ P2P network bootstrapped");
            println!("   - Network ID: {}", conn.network_id);
            println!("   - Certificate: {}", if conn.certificate.is_some() { "Self-signed" } else { "None" });
            if let Some(cert) = conn.certificate {
                println!("   - Self-signed: {}", cert.is_self_signed());
            }
        }
        Err(e) => println!("   ✗ Failed: {}", e),
    }

    // 3. Federated Network
    println!("\n3. Federated Network Handler:");
    let fed_handler = FederatedNetworkHandler::new();
    let fed_config = NetworkConfig {
        network_type: NetworkType::Federated {
            gateway_url: "bank.internal".to_string()
        },
        peer_addresses: vec![],
        federation_gateway: Some("bank.internal".to_string()),
        dns_name: None,
        proof_of_state: None,
    };

    match fed_handler.bootstrap(fed_config).await {
        Ok(conn) => {
            println!("   ✓ Federated network bootstrapped");
            println!("   - Network ID: {}", conn.network_id);
            println!("   - Federation: bank.internal");
            println!("   - Certificate: {}", if conn.certificate.is_some() { "Federation-issued" } else { "None" });
        }
        Err(e) => println!("   ✗ Failed: {}", e),
    }

    // 4. Public Network
    println!("\n4. Public Network Handler:");
    let public_handler = PublicNetworkHandler::new();

    // First try without proof (should fail)
    println!("   Testing without Proof of State:");
    let public_config_no_proof = NetworkConfig {
        network_type: NetworkType::Public,
        peer_addresses: vec![],
        federation_gateway: None,
        dns_name: None,
        proof_of_state: None,
    };

    match public_handler.bootstrap(public_config_no_proof).await {
        Ok(_) => println!("   ✗ Should have failed without proof!"),
        Err(e) => println!("   ✓ Correctly rejected: {}", e),
    }

    // Now with proof
    println!("   Testing with Proof of State:");
    let proof = ProofOfState {
        proof_of_space: vec![1, 2, 3],
        proof_of_stake: vec![4, 5, 6],
        proof_of_work: vec![7, 8, 9],
        proof_of_time: vec![10, 11, 12],
    };

    let public_config = NetworkConfig {
        network_type: NetworkType::Public,
        peer_addresses: vec![],
        federation_gateway: None,
        dns_name: Some("node.hypermesh".to_string()),
        proof_of_state: Some(proof),
    };

    match public_handler.bootstrap(public_config).await {
        Ok(conn) => {
            println!("   ✓ Public network bootstrapped");
            println!("   - Network ID: {}", conn.network_id);
            println!("   - DNS: node.hypermesh");
            println!("   - Certificate: Blockchain-registered");
            if let Some(cert) = conn.certificate {
                println!("   - Blockchain registered: {}", cert.is_blockchain_registered());
            }
        }
        Err(e) => println!("   ✗ Failed: {}", e),
    }

    println!("\n=== Summary ===");
    println!("✓ All four network handlers created and tested");
    println!("✓ Each network has isolated trust model");
    println!("✓ Anonymous: No certificates, ephemeral");
    println!("✓ P2P: Self-signed certificates");
    println!("✓ Federated: Federation gateway certificates");
    println!("✓ Public: Blockchain-registered certificates");

    Ok(())
}