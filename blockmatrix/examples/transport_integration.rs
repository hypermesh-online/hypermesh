// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration example using the nexus-transport module

use nexus_transport::{
    TransportBuilder, TransportConfig, CertificateManager,
    QuicServer, QuicClient, TransportMessage, MessageType
};
use nexus_shared::NodeId;
use std::sync::Arc;
use std::time::Duration;

/// Simple transport integration test
async fn run_integration_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Transport Integration Test");

    // Create certificate manager
    let cert_manager = Arc::new(
        CertificateManager::new_self_signed(
            "test-node".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );

    // Create transport config
    let mut config = TransportConfig::default();
    config.bind_address = "127.0.0.1".parse()?;
    config.port = 5002;

    // Build server
    println!("📡 Creating server...");
    let server = TransportBuilder::new()
        .with_config(config.clone())
        .with_certificate_manager(cert_manager.clone())
        .build_server()
        .await?;

    // Build client  
    println!("📱 Creating client...");
    let client = TransportBuilder::new()
        .with_config(config)
        .with_certificate_manager(cert_manager)
        .build_client()
        .await?;

    println!("✅ Transport module integration successful!");
    println!("📊 Components created:");
    println!("   - Certificate Manager: ✅");
    println!("   - Transport Config: ✅");
    println!("   - QUIC Server: ✅");
    println!("   - QUIC Client: ✅");

    Ok(())
}

/// Message creation test
fn test_message_creation() -> Result<(), Box<dyn std::error::Error>> {
    println!("📨 Testing message creation...");

    let source_node = NodeId::random();
    let dest_node = NodeId::random();
    let payload = b"Hello, Transport!".to_vec();

    // Create message
    let message = TransportMessage::new(
        MessageType::Data,
        source_node,
        Some(dest_node),
        payload.clone()
    );

    // Test serialization
    let serialized = message.to_bytes()?;
    let deserialized = TransportMessage::from_bytes(&serialized)?;

    // Verify
    assert_eq!(message.payload, deserialized.payload);
    assert_eq!(message.source, deserialized.source);
    assert_eq!(message.destination, deserialized.destination);
    assert_eq!(message.message_type, deserialized.message_type);

    println!("✅ Message serialization/deserialization works!");
    println!("   - Message size: {} bytes", serialized.len());
    println!("   - Payload: {}", String::from_utf8_lossy(&payload));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    println!("🔧 HyperMesh Transport Integration Test");
    println!("=======================================");

    // Test message functionality (doesn't require network)
    if let Err(e) = test_message_creation() {
        eprintln!("❌ Message test failed: {}", e);
        return Err(e);
    }

    // Test transport module creation (doesn't require actual networking)
    if let Err(e) = run_integration_test().await {
        eprintln!("❌ Integration test failed: {}", e);
        return Err(e);
    }

    println!("=======================================");
    println!("🎉 All integration tests passed!");
    println!("");
    println!("Next steps:");
    println!("1. Implement connection management");
    println!("2. Add stream multiplexing");  
    println!("3. Integrate with service mesh");
    println!("4. Add P2P discovery");

    Ok(())
}