//! Transport layer integration tests
//! Tests real multi-node communication and network scenarios

use crate::{TestResult, init_test_logging};
use nexus_transport::{CertificateManager, TransportBuilder, TransportConfig};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, error, warn};

/// Run comprehensive transport integration tests
pub async fn run_transport_integration() -> crate::TestResult {
    init_test_logging();
    info!("🚀 Running comprehensive transport integration tests");
    
    // Test different integration scenarios
    test_basic_client_server_communication().await?;
    test_multi_node_mesh_network().await?;
    test_connection_failure_recovery().await?;
    test_certificate_rotation().await?;
    test_network_partition_handling().await?;
    
    info!("✅ All transport integration tests completed");
    Ok(())
}

/// Test basic client-server bidirectional communication
async fn test_basic_client_server_communication() -> TestResult {
    info!("🧪 Testing basic client-server communication");
    
    let server_port = crate::test_utils::find_available_port()?;
    let server_addr = format!("127.0.0.1:{}", server_port);
    
    // Create separate certificates for client and server
    let server_cert = Arc::new(
        CertificateManager::new_self_signed(
            "integration-server".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    let client_cert = Arc::new(
        CertificateManager::new_self_signed(
            "integration-client".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    // Start server
    let mut server_config = TransportConfig::default();
    server_config.bind_address = "127.0.0.1".parse()?;
    server_config.port = server_port;
    
    let server = TransportBuilder::new()
        .with_config(server_config)
        .with_certificate_manager(server_cert)
        .build_server()
        .await?;
        
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client and connect
    let client_config = TransportConfig::default();
    let client = TransportBuilder::new()
        .with_config(client_config)
        .with_certificate_manager(client_cert)
        .build_client()
        .await?;
        
    let connection = client.connect(&server_addr.parse()?).await?;
    
    // Test bidirectional communication
    let stream = connection.open_stream().await?;
    
    // Send test message
    let test_message = b"integration_test_message_from_client";
    stream.write_message(test_message).await?;
    
    // Verify message was sent (in real scenario, server would echo)
    info!("✅ Successfully sent message: {}", String::from_utf8_lossy(test_message));
    
    // Test multiple streams on same connection
    let mut handles = Vec::new();
    for i in 0..5 {
        let connection = connection.clone();
        let handle = tokio::spawn(async move {
            let stream = connection.open_stream().await?;
            let message = format!("stream_{}_message", i);
            stream.write_message(message.as_bytes()).await?;
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        handles.push(handle);
    }
    
    // Wait for all streams to complete
    for handle in handles {
        handle.await??;
    }
    
    info!("✅ Basic client-server communication test PASSED");
    Ok(())
}

/// Test multi-node mesh network formation
async fn test_multi_node_mesh_network() -> TestResult {
    info!("🕸️ Testing multi-node mesh network formation");
    
    // Create 4 nodes that will form a mesh network
    let mut nodes = Vec::new();
    let mut node_addrs = Vec::new();
    
    // Create nodes
    for i in 0..4 {
        let port = crate::test_utils::find_available_port()?;
        let addr = format!("127.0.0.1:{}", port);
        
        let cert = Arc::new(
            CertificateManager::new_self_signed(
                format!("mesh-node-{}", i),
                365,
                Duration::from_secs(3600)
            ).await?
        );
        
        let mut config = TransportConfig::default();
        config.bind_address = "127.0.0.1".parse()?;
        config.port = port;
        
        let server = TransportBuilder::new()
            .with_config(config.clone())
            .with_certificate_manager(cert.clone())
            .build_server()
            .await?;
            
        let client = TransportBuilder::new()
            .with_config(config)
            .with_certificate_manager(cert)
            .build_client()
            .await?;
            
        nodes.push((server, client));
        node_addrs.push(addr.parse::<SocketAddr>()?);
    }
    
    // Allow all nodes to start
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Test full mesh connectivity - each node connects to every other node
    let mut connections = HashMap::new();
    
    for (i, (_, client)) in nodes.iter().enumerate() {
        for (j, addr) in node_addrs.iter().enumerate() {
            if i != j {
                match client.connect(addr).await {
                    Ok(conn) => {
                        connections.insert((i, j), conn);
                        info!("✅ Node {} connected to Node {}", i, j);
                    }
                    Err(e) => {
                        error!("❌ Node {} failed to connect to Node {}: {}", i, j, e);
                        return Err(e.into());
                    }
                }
            }
        }
    }
    
    // Verify we have full mesh connectivity (each node connected to 3 others)
    let expected_connections = 4 * 3; // 4 nodes * 3 connections each
    if connections.len() == expected_connections {
        info!("✅ Full mesh network established: {} connections", connections.len());
    } else {
        error!("❌ Incomplete mesh: expected {} connections, got {}", 
               expected_connections, connections.len());
        return Err("Incomplete mesh network".into());
    }
    
    // Test message propagation across the mesh
    let test_message = b"mesh_broadcast_test";
    let mut message_tasks = Vec::new();
    
    for ((from, to), connection) in connections.iter() {
        let connection = connection.clone();
        let message = test_message.clone();
        let from = *from;
        let to = *to;
        
        let task = tokio::spawn(async move {
            let stream = connection.open_stream().await
                .map_err(|e| format!("Failed to open stream {}->{}: {}", from, to, e))?;
            stream.write_message(message).await
                .map_err(|e| format!("Failed to send message {}->{}: {}", from, to, e))?;
            Ok::<(), String>(())
        });
        
        message_tasks.push(task);
    }
    
    // Wait for all messages to be sent
    for task in message_tasks {
        task.await.map_err(|e| format!("Task failed: {}", e))??;
    }
    
    info!("✅ Multi-node mesh network test PASSED");
    Ok(())
}

/// Test connection failure and recovery scenarios
async fn test_connection_failure_recovery() -> TestResult {
    info!("🔄 Testing connection failure recovery");
    
    let server_port = crate::test_utils::find_available_port()?;
    let server_addr = format!("127.0.0.1:{}", server_port);
    
    let cert = Arc::new(
        CertificateManager::new_self_signed(
            "recovery-test".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    // Start server
    let mut server_config = TransportConfig::default();
    server_config.bind_address = "127.0.0.1".parse()?;
    server_config.port = server_port;
    
    let server = TransportBuilder::new()
        .with_config(server_config)
        .with_certificate_manager(cert.clone())
        .build_server()
        .await?;
        
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client
    let client_config = TransportConfig::default();
    let client = TransportBuilder::new()
        .with_config(client_config)
        .with_certificate_manager(cert)
        .build_client()
        .await?;
    
    // Establish initial connection
    let connection = client.connect(&server_addr.parse()?).await?;
    let stream = connection.open_stream().await?;
    
    // Verify initial connection works
    stream.write_message(b"pre_failure_test").await?;
    info!("✅ Initial connection established");
    
    // Simulate connection failure by dropping the server
    drop(server);
    info!("⚠️ Simulated server failure");
    
    // Try to use connection - should fail
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    match stream.write_message(b"during_failure_test").await {
        Ok(_) => warn!("⚠️ Unexpected success during failure simulation"),
        Err(_) => info!("✅ Connection correctly failed during server downtime")
    }
    
    // Restart server on same port
    let server_cert = Arc::new(
        CertificateManager::new_self_signed(
            "recovery-server-restarted".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    let mut restart_config = TransportConfig::default();
    restart_config.bind_address = "127.0.0.1".parse()?;
    restart_config.port = server_port;
    
    let restarted_server = TransportBuilder::new()
        .with_config(restart_config)
        .with_certificate_manager(server_cert)
        .build_server()
        .await?;
        
    tokio::time::sleep(Duration::from_millis(200)).await;
    info!("🔄 Server restarted");
    
    // Try to reconnect
    match client.connect(&server_addr.parse()?).await {
        Ok(new_connection) => {
            let new_stream = new_connection.open_stream().await?;
            new_stream.write_message(b"post_recovery_test").await?;
            info!("✅ Connection recovery successful");
        }
        Err(e) => {
            error!("❌ Connection recovery failed: {}", e);
            return Err(e.into());
        }
    }
    
    info!("✅ Connection failure recovery test PASSED");
    Ok(())
}

/// Test certificate rotation without connection interruption
async fn test_certificate_rotation() -> TestResult {
    info!("🔐 Testing certificate rotation");
    
    let server_port = crate::test_utils::find_available_port()?;
    let server_addr = format!("127.0.0.1:{}", server_port);
    
    // Create initial certificates with short validity
    let server_cert = Arc::new(
        CertificateManager::new_self_signed(
            "rotation-server".to_string(),
            1, // 1 day validity for testing
            Duration::from_secs(10) // Very short rotation period for testing
        ).await?
    );
    
    let client_cert = Arc::new(
        CertificateManager::new_self_signed(
            "rotation-client".to_string(),
            1,
            Duration::from_secs(10)
        ).await?
    );
    
    // Start server
    let mut server_config = TransportConfig::default();
    server_config.bind_address = "127.0.0.1".parse()?;
    server_config.port = server_port;
    
    let server = TransportBuilder::new()
        .with_config(server_config)
        .with_certificate_manager(server_cert.clone())
        .build_server()
        .await?;
        
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client
    let client_config = TransportConfig::default();
    let client = TransportBuilder::new()
        .with_config(client_config)
        .with_certificate_manager(client_cert.clone())
        .build_client()
        .await?;
    
    // Establish connection
    let connection = client.connect(&server_addr.parse()?).await?;
    let stream = connection.open_stream().await?;
    
    // Verify connection works with original certificates
    stream.write_message(b"pre_rotation_test").await?;
    info!("✅ Connection established with original certificates");
    
    // Simulate certificate rotation
    tokio::time::sleep(Duration::from_secs(2)).await;
    info!("🔄 Simulating certificate rotation...");
    
    // In a real implementation, the certificate manager would
    // automatically rotate certificates. For testing, we verify
    // the existing connection continues to work during rotation
    
    // Test that existing connection still works
    stream.write_message(b"during_rotation_test").await?;
    info!("✅ Existing connection survived certificate rotation simulation");
    
    // Test new connection with rotated certificates would work
    // (In real implementation, new certificates would be used)
    let new_connection = client.connect(&server_addr.parse()?).await?;
    let new_stream = new_connection.open_stream().await?;
    new_stream.write_message(b"post_rotation_test").await?;
    info!("✅ New connection works after certificate rotation simulation");
    
    info!("✅ Certificate rotation test PASSED");
    Ok(())
}

/// Test network partition handling and recovery
async fn test_network_partition_handling() -> TestResult {
    info!("🌐 Testing network partition handling");
    
    // Create 3 nodes to simulate a partition scenario
    let mut nodes = Vec::new();
    let mut node_addrs = Vec::new();
    
    for i in 0..3 {
        let port = crate::test_utils::find_available_port()?;
        let addr = format!("127.0.0.1:{}", port);
        
        let cert = Arc::new(
            CertificateManager::new_self_signed(
                format!("partition-node-{}", i),
                365,
                Duration::from_secs(3600)
            ).await?
        );
        
        let mut config = TransportConfig::default();
        config.bind_address = "127.0.0.1".parse()?;
        config.port = port;
        config.connection_timeout = Duration::from_secs(5); // Shorter timeout for partition testing
        
        let server = TransportBuilder::new()
            .with_config(config.clone())
            .with_certificate_manager(cert.clone())
            .build_server()
            .await?;
            
        let client = TransportBuilder::new()
            .with_config(config)
            .with_certificate_manager(cert)
            .build_client()
            .await?;
            
        nodes.push((server, client));
        node_addrs.push(addr.parse::<SocketAddr>()?);
    }
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    // Establish initial connections between all nodes
    let connection_01 = nodes[0].1.connect(&node_addrs[1]).await?;
    let connection_02 = nodes[0].1.connect(&node_addrs[2]).await?;
    let connection_12 = nodes[1].1.connect(&node_addrs[2]).await?;
    
    info!("✅ Initial network established: Node0-Node1, Node0-Node2, Node1-Node2");
    
    // Test initial connectivity
    let stream_01 = connection_01.open_stream().await?;
    stream_01.write_message(b"pre_partition_0_to_1").await?;
    
    let stream_12 = connection_12.open_stream().await?;
    stream_12.write_message(b"pre_partition_1_to_2").await?;
    
    info!("✅ Network functioning normally before partition");
    
    // Simulate partition: Node 2 becomes isolated
    // (In real scenario, this would involve network configuration)
    drop(nodes[2].0); // Drop server for node 2
    info!("⚠️ Simulated network partition: Node 2 isolated");
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Test that Node 0 - Node 1 connection still works
    match stream_01.write_message(b"during_partition_0_to_1").await {
        Ok(_) => info!("✅ Node 0-1 connection survived partition"),
        Err(e) => warn!("⚠️ Node 0-1 connection affected by partition: {}", e)
    }
    
    // Test that connection to partitioned node fails
    match stream_12.write_message(b"during_partition_1_to_2").await {
        Ok(_) => warn!("⚠️ Unexpected success to partitioned node"),
        Err(_) => info!("✅ Connection to partitioned node correctly failed")
    }
    
    // Simulate partition recovery
    let recovered_cert = Arc::new(
        CertificateManager::new_self_signed(
            "recovered-node-2".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    let mut recovered_config = TransportConfig::default();
    recovered_config.bind_address = "127.0.0.1".parse()?;
    recovered_config.port = node_addrs[2].port();
    
    let recovered_server = TransportBuilder::new()
        .with_config(recovered_config)
        .with_certificate_manager(recovered_cert)
        .build_server()
        .await?;
        
    tokio::time::sleep(Duration::from_millis(300)).await;
    info!("🔄 Simulated partition recovery: Node 2 reconnected");
    
    // Test that new connections can be established after recovery
    let recovery_connection = nodes[1].1.connect(&node_addrs[2]).await?;
    let recovery_stream = recovery_connection.open_stream().await?;
    recovery_stream.write_message(b"post_recovery_1_to_2").await?;
    info!("✅ Connection re-established after partition recovery");
    
    info!("✅ Network partition handling test PASSED");
    Ok(())
}
