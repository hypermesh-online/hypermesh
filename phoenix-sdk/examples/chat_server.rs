//! Phoenix Chat Server Example
//!
//! A simple chat server that demonstrates Phoenix SDK's ease of use.
//!
//! Run with: cargo run --example chat_server

use phoenix_sdk::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ChatMessage {
    Join { username: String },
    Leave { username: String },
    Message { username: String, text: String },
    ServerMessage { text: String },
}

type ClientMap = Arc<RwLock<HashMap<String, PhoenixConnection>>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Create Phoenix instance with zero configuration
    let phoenix = Phoenix::new("chat-server").await?;

    // Start listening on port 8080
    let listener = phoenix.listen(8080).await?;

    println!("🚀 Phoenix Chat Server listening on port 8080");
    println!("📊 Metrics available at: http://localhost:8080/metrics");

    // Shared client map
    let clients: ClientMap = Arc::new(RwLock::new(HashMap::new()));

    // Accept connections
    listener.handle(move |conn| {
        let clients = clients.clone();
        async move {
            handle_client(conn, clients).await
        }
    }).await?;

    Ok(())
}

async fn handle_client(conn: PhoenixConnection, clients: ClientMap) -> Result<()> {
    let conn_id = conn.id().to_string();
    println!("📥 New connection from: {}", conn.remote_addr());

    // Receive username
    let username: String = conn.receive().await?;
    println!("👤 User joined: {}", username);

    // Store connection
    clients.write().await.insert(username.clone(), conn.clone());

    // Broadcast join message
    broadcast(
        &clients,
        ChatMessage::Join { username: username.clone() },
        Some(&username),
    ).await;

    // Handle messages
    loop {
        match conn.receive::<String>().await {
            Ok(text) => {
                println!("💬 {}: {}", username, text);

                // Broadcast message to all clients
                broadcast(
                    &clients,
                    ChatMessage::Message {
                        username: username.clone(),
                        text,
                    },
                    Some(&username),
                ).await;
            }
            Err(_) => {
                // Connection closed
                println!("👋 User left: {}", username);
                break;
            }
        }
    }

    // Remove client and broadcast leave
    clients.write().await.remove(&username);
    broadcast(
        &clients,
        ChatMessage::Leave { username: username.clone() },
        None,
    ).await;

    Ok(())
}

async fn broadcast(
    clients: &ClientMap,
    message: ChatMessage,
    exclude: Option<&str>,
) {
    let clients = clients.read().await;

    for (name, conn) in clients.iter() {
        if exclude.map_or(true, |ex| ex != name) {
            if let Err(e) = conn.send(&message).await {
                eprintln!("Failed to send to {}: {}", name, e);
            }
        }
    }
}

// Helper function to display metrics
async fn show_metrics(phoenix: &Phoenix) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        let metrics = phoenix.metrics().await;
        println!("📊 Metrics: {} connections, {:.2} Gbps, {} ms latency",
            metrics.active_connections,
            metrics.throughput_gbps,
            metrics.avg_latency_us / 1000,
        );
    }
}