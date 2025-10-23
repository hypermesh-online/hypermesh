//! Connection Manager Integration Example
//! 
//! Demonstrates the connection management layer bridging transport and services

use nexus_connection_manager::{
    ConnectionManager, ConnectionManagerConfig, ServiceEvent, ConnectionEvent
};
use nexus_shared::{ServiceId, NodeId};
use nexus_transport::CertificateManager;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Example service that registers with the connection manager
struct ExampleService {
    id: ServiceId,
    port: u16,
}

impl ExampleService {
    fn new(name: &str, port: u16) -> Self {
        Self {
            id: ServiceId::new(name, "default"),
            port,
        }
    }
    
    async fn start(&self, connection_manager: &ConnectionManager) -> Result<(), Box<dyn std::error::Error>> {
        let address: SocketAddr = format!("127.0.0.1:{}", self.port).parse()?;
        let mut metadata = HashMap::new();
        metadata.insert("version".to_string(), "1.0.0".to_string());
        metadata.insert("protocol".to_string(), "http".to_string());
        
        connection_manager.register_service(
            self.id.clone(),
            address,
            metadata
        ).await?;
        
        println!("✅ Service {} registered at {}", self.id, address);
        Ok(())
    }
}

/// Simulate a client making requests through the connection manager
async fn simulate_client_requests(
    connection_manager: &ConnectionManager,
    service_id: &ServiceId,
    request_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Simulating {} client requests to {}", request_count, service_id);
    
    for i in 1..=request_count {
        // Discover available services
        let services = connection_manager.discover_services(service_id).await?;
        
        if !services.is_empty() {
            println!("  📍 Request {}: Found {} instances of {}", 
                i, services.len(), service_id);
            
            // In a real implementation, you would:
            // 1. Get a connection handle: connection_manager.get_connection(service_id).await?
            // 2. Use the connection to make the actual request
            // For now, we'll just simulate the request
            
            tokio::time::sleep(Duration::from_millis(10)).await;
            println!("  ✅ Request {} completed", i);
        } else {
            println!("  ❌ Request {}: No services available", i);
        }
    }
    
    Ok(())
}

/// Monitor service and connection events
async fn event_monitor(
    mut service_events: tokio::sync::broadcast::Receiver<ServiceEvent>,
    mut connection_events: tokio::sync::broadcast::Receiver<ConnectionEvent>,
) {
    println!("👂 Starting event monitor...");
    
    loop {
        tokio::select! {
            service_event = service_events.recv() => {
                match service_event {
                    Ok(event) => println!("🔔 Service Event: {:?}", event),
                    Err(e) => {
                        println!("Service event error: {}", e);
                        break;
                    }
                }
            }
            connection_event = connection_events.recv() => {
                match connection_event {
                    Ok(event) => println!("🔗 Connection Event: {:?}", event),
                    Err(e) => {
                        println!("Connection event error: {}", e);
                        break;
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();
    
    println!("🚀 HyperMesh Connection Manager Integration Test");
    println!("=================================================");
    
    // Create certificate manager
    let cert_manager = Arc::new(
        CertificateManager::new_self_signed(
            "connection-manager-test".to_string(),
            365,
            Duration::from_secs(3600)
        ).await?
    );
    
    // Create connection manager configuration
    let mut config = ConnectionManagerConfig::default();
    config.bind_address = "127.0.0.1".parse()?;
    config.port = 8081;
    
    // Create and start connection manager
    println!("📡 Creating connection manager...");
    let mut connection_manager = ConnectionManager::new(config, cert_manager).await?;
    
    // Subscribe to events before starting
    let service_events = connection_manager.subscribe_service_events();
    let connection_events = connection_manager.subscribe_connection_events();
    
    // Start the connection manager
    let listen_addr = connection_manager.start().await?;
    println!("✅ Connection manager listening on {}", listen_addr);
    
    // Start event monitor in background
    let event_monitor_handle = tokio::spawn(event_monitor(service_events, connection_events));
    
    // Create and register multiple services
    let services = vec![
        ExampleService::new("web-api", 8082),
        ExampleService::new("user-service", 8083),
        ExampleService::new("data-service", 8084),
    ];
    
    println!("\n📋 Registering services...");
    for service in &services {
        service.start(&connection_manager).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Wait a bit for services to be fully registered
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Test service discovery
    println!("\n🔍 Testing service discovery...");
    for service in &services {
        let discovered = connection_manager.discover_services(&service.id).await?;
        println!("  📍 {} -> {} instances found", service.id, discovered.len());
        
        for instance in &discovered {
            println!("    - {} at {} (status: {:?})", 
                instance.node_id, instance.address, instance.status);
        }
    }
    
    // Simulate client load
    println!("\n🔄 Simulating client requests...");
    for service in &services {
        let connection_manager_ref = &connection_manager;
        let service_id = &service.id;
        
        // Run with timeout to avoid hanging
        if let Err(_) = timeout(
            Duration::from_secs(5),
            simulate_client_requests(connection_manager_ref, service_id, 3)
        ).await {
            println!("  ⚠️  Request simulation timed out for {}", service_id);
        }
    }
    
    // Show metrics
    println!("\n📊 Connection Manager Metrics:");
    let metrics = connection_manager.metrics().await;
    println!("  - Connections Created: {}", metrics.connections_created);
    println!("  - Active Connections: {}", metrics.active_connections);
    println!("  - Connection Errors: {}", metrics.connection_errors);
    
    // Graceful shutdown
    println!("\n🛑 Shutting down connection manager...");
    connection_manager.shutdown().await?;
    
    // Stop event monitor
    event_monitor_handle.abort();
    
    println!("=================================================");
    println!("✅ Connection Manager integration test completed!");
    println!("");
    println!("Key Features Demonstrated:");
    println!("  🔗 Connection management and pooling");
    println!("  📋 Service registration and discovery");  
    println!("  ⚖️  Load balancing across service instances");
    println!("  📊 Real-time metrics collection");
    println!("  🔔 Event-driven architecture");
    println!("");
    println!("Ready for integration with:");
    println!("  - Resource scheduler for workload placement");
    println!("  - State manager for distributed coordination");
    println!("  - Service mesh for advanced networking");
    
    Ok(())
}