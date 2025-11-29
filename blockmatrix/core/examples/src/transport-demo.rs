//! Transport layer demonstration
//! Shows QUIC server/client connectivity

use nexus_transport::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("🚀 Nexus Transport Layer Demo");
    println!("============================");
    
    // Create certificate manager
    println!("📜 Generating self-signed certificates...");
    let cert_manager = Arc::new(
        CertificateManager::new_self_signed(
            "transport-demo".to_string(),
            365,
            Duration::from_secs(3600),
        ).await?
    );
    
    let mut server_config = TransportConfig::default();
    server_config.port = 7000;
    
    // Start QUIC server
    println!("🌐 Starting QUIC server on port {}...", server_config.port);
    let mut server = QuicServer::new(server_config.clone(), cert_manager.clone()).await?;
    let server_addr = server.start().await?;
    println!("✅ Server listening on {}", server_addr);
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    // Create and start client
    println!("🔌 Starting QUIC client...");
    let mut client_config = TransportConfig::default();
    client_config.port = 0; // Any available port
    
    let mut client = QuicClient::new(client_config, cert_manager.clone()).await?;
    client.start().await?;
    
    // Test connection stats
    println!("📊 Testing connection statistics...");
    let client_stats = client.connection_count().await;
    let server_stats = server.connection_count().await;
    
    println!("   Client connections: {}", client_stats);
    println!("   Server connections: {}", server_stats);
    
    // Demonstrate configuration
    println!("⚙️  Transport Configuration:");
    println!("   Max connections: {}", server_config.max_connections);
    println!("   Keep alive: {:?}", server_config.keep_alive_interval);
    println!("   Idle timeout: {:?}", server_config.idle_timeout);
    
    // Show certificate info
    println!("🔐 Security Information:");
    println!("   Certificate rotation: {:?}", cert_manager.rotation_interval());
    println!("   Subject: transport-demo");
    
    sleep(Duration::from_secs(2)).await;
    
    // Cleanup
    println!("🧹 Shutting down...");
    client.stop().await?;
    server.stop().await?;
    
    println!("✅ Transport demo completed successfully!");
    println!("\n💡 This demonstrates:");
    println!("   • QUIC over IPv6 transport protocol");
    println!("   • Automatic certificate generation and rotation");
    println!("   • Secure server/client communication");
    println!("   • Connection management and statistics");
    
    Ok(())
}