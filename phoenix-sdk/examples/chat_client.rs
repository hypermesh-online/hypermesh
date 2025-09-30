//! Phoenix Chat Client Example
//!
//! A simple chat client that connects to the Phoenix chat server.
//!
//! Run with: cargo run --example chat_client

use phoenix_sdk::prelude::*;
use serde::{Serialize, Deserialize};
use std::io::{self, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ChatMessage {
    Join { username: String },
    Leave { username: String },
    Message { username: String, text: String },
    ServerMessage { text: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Get username
    print!("Enter your username: ");
    io::stdout().flush().unwrap();
    let mut username = String::new();
    io::stdin().read_line(&mut username).unwrap();
    let username = username.trim().to_string();

    // Create Phoenix instance
    let phoenix = Phoenix::new("chat-client").await?;

    // Connect to server
    println!("🔌 Connecting to chat server...");
    let conn = phoenix.connect("localhost:8080").await?;
    println!("✅ Connected!");

    // Send username
    conn.send(&username).await?;

    // Spawn task to receive messages
    let conn_recv = conn.clone();
    tokio::spawn(async move {
        loop {
            match conn_recv.receive::<ChatMessage>().await {
                Ok(msg) => display_message(msg),
                Err(_) => {
                    println!("❌ Disconnected from server");
                    break;
                }
            }
        }
    });

    // Read and send messages
    println!("📝 Type messages (or 'quit' to exit):");
    let stdin = io::stdin();
    let mut buffer = String::new();

    loop {
        buffer.clear();
        stdin.read_line(&mut buffer).unwrap();
        let text = buffer.trim();

        if text == "quit" {
            break;
        }

        if !text.is_empty() {
            if let Err(e) = conn.send(&text.to_string()).await {
                eprintln!("Failed to send message: {}", e);
                break;
            }
        }
    }

    // Cleanup
    conn.close().await?;
    phoenix.shutdown().await?;

    Ok(())
}

fn display_message(msg: ChatMessage) {
    match msg {
        ChatMessage::Join { username } => {
            println!("➡️  {} joined the chat", username);
        }
        ChatMessage::Leave { username } => {
            println!("⬅️  {} left the chat", username);
        }
        ChatMessage::Message { username, text } => {
            println!("💬 {}: {}", username, text);
        }
        ChatMessage::ServerMessage { text } => {
            println!("📢 Server: {}", text);
        }
    }
}