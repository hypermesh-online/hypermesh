//! STOQ Integrated Echo Server Example
//!
//! This example demonstrates the fully integrated STOQ protocol system:
//! - Transport layer (QUIC over IPv6)
//! - Protocol layer (structured message handling)
//! - Application layer (echo server logic)

use std::net::Ipv6Addr;
use anyhow::Result;
use bytes::Bytes;
use tracing::{info, Level};
use tokio::time::Duration;

use stoq::{
    StoqServer, StoqServerConfig, StoqClient, StoqClientConfig,
    Endpoint, MessageHandler, StoqMessage, ConnectionInfo,
    ProtocolConfig, TransportConfig
};

/// Custom echo message handler that demonstrates protocol integration
struct IntegratedEchoHandler;

#[async_trait::async_trait]
impl MessageHandler<String> for IntegratedEchoHandler {
    async fn handle_message(
        &self, 
        message: StoqMessage<String>, 
        connection_info: &ConnectionInfo
    ) -> Result<Option<Bytes>> {
        info!(
            "Received message from {}: '{}'", 
            connection_info.remote_address, 
            message.payload
        );
        
        // Create echo response
        let response = format!("Echo: {}", message.payload);
        let response_bytes = bincode::serialize(&response)?;
        
        info!("Sending echo response: '{}'", response);
        Ok(Some(Bytes::from(response_bytes)))
    }
}

/// JSON message handler for structured data
struct JsonHandler;

#[async_trait::async_trait]
impl MessageHandler<serde_json::Value> for JsonHandler {
    async fn handle_message(
        &self,
        message: StoqMessage<serde_json::Value>,
        connection_info: &ConnectionInfo
    ) -> Result<Option<Bytes>> {
        info!(
            "Received JSON from {}: {}", 
            connection_info.remote_address, 
            message.payload
        );
        
        // Process JSON and create response
        let response = serde_json::json!({
            "status": "processed",
            "received": message.payload,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "connection_id": connection_info.connection_id
        });
        
        let response_bytes = bincode::serialize(&response)?;
        Ok(Some(Bytes::from(response_bytes)))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Initialize crypto provider
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("Failed to install crypto provider"))?;

    info!("Starting STOQ integrated echo server example...");

    // Server configuration
    let server_config = StoqServerConfig {
        bind_address: Ipv6Addr::LOCALHOST,
        port: 9292,
        transport: TransportConfig {
            bind_address: Ipv6Addr::LOCALHOST,
            port: 9292,
            max_concurrent_streams: 100,
            enable_zero_copy: true,
            max_connections: Some(100),
            ..Default::default()
        },
        protocol: ProtocolConfig {
            max_message_size: 1024 * 1024, // 1MB
            message_timeout: Duration::from_secs(10),
            enable_compression: true,
            enable_authentication: false, // Simplified for example
            ..Default::default()
        },
        max_connections: Some(100),
    };

    // Create and configure server
    let server = StoqServer::new(server_config).await?;
    
    // Register message handlers
    server.register_handler("echo".to_string(), IntegratedEchoHandler).await;
    server.register_handler("json".to_string(), JsonHandler).await;
    
    // Start server in background
    let server_clone = server.clone();
    let server_task = tokio::spawn(async move {
        if let Err(e) = server_clone.start().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait a moment for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    info!("Server started on [::1]:9292");

    // Client configuration
    let client_config = StoqClientConfig {
        transport: TransportConfig {
            bind_address: Ipv6Addr::LOCALHOST,
            port: 0, // Ephemeral port for client
            enable_zero_copy: true,
            ..Default::default()
        },
        protocol: ProtocolConfig {
            enable_authentication: false, // Simplified for example
            ..Default::default()
        },
        message_timeout: Duration::from_secs(5),
        ..Default::default()
    };

    // Create client
    let client = StoqClient::new(client_config).await?;
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);

    info!("Testing integrated protocol communication...");

    // Test 1: String echo
    info!("Test 1: String echo message");
    let test_message = "Hello, STOQ Protocol!".to_string();
    let echo_response: String = client.send_message_with_response(
        &endpoint,
        "echo".to_string(),
        test_message.clone()
    ).await?;
    info!("Echo response: '{}'", echo_response);

    // Test 2: JSON message
    info!("Test 2: JSON message");
    let json_request = serde_json::json!({
        "action": "test",
        "data": {
            "value": 42,
            "message": "Testing JSON protocol"
        }
    });
    
    let json_response: serde_json::Value = client.send_message_with_response(
        &endpoint,
        "json".to_string(),
        json_request.clone()
    ).await?;
    info!("JSON response: {}", serde_json::to_string_pretty(&json_response)?);

    // Test 3: Multiple rapid messages
    info!("Test 3: Multiple rapid messages");
    for i in 0..5 {
        let message = format!("Rapid message #{}", i);
        let response: String = client.send_message_with_response(
            &endpoint,
            "echo".to_string(),
            message.clone()
        ).await?;
        info!("Rapid #{}: {} -> {}", i, message, response);
    }

    // Test 4: Raw transport access (bypass protocol layer)
    info!("Test 4: Raw transport layer access");
    let raw_data = b"Raw transport data";
    client.send_raw_data(&endpoint, raw_data).await?;
    info!("Raw data sent successfully");

    // Get statistics
    let stats = server.stats().await;
    info!("Server statistics:");
    info!("  Bytes sent: {}", stats.bytes_sent);
    info!("  Bytes received: {}", stats.bytes_received);
    info!("  Active connections: {}", stats.active_connections);
    info!("  Throughput: {:.2} Mbps", stats.throughput_gbps * 1000.0);

    let client_stats = client.stats();
    info!("Client statistics:");
    info!("  Bytes sent: {}", client_stats.bytes_sent);
    info!("  Bytes received: {}", client_stats.bytes_received);

    // Cleanup
    info!("Shutting down...");
    client.shutdown().await?;
    server.stop().await?;
    server_task.abort();

    info!("STOQ integrated example completed successfully!");
    Ok(())
}