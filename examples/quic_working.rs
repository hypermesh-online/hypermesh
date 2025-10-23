//! Working QUIC example with proper certificate handling

use quinn::{Endpoint, ServerConfig, ClientConfig};
use std::error::Error;
use std::sync::Arc;
use std::net::SocketAddr;
use std::path::Path;
use std::fs;

const CERT_PATH: &str = "/tmp/nexus_test_cert.der";

/// Generate and save self-signed certificate
fn generate_and_save_cert() -> Result<(rustls::Certificate, rustls::PrivateKey), Box<dyn Error>> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let key = rustls::PrivateKey(cert.serialize_private_key_der());
    let cert_der = cert.serialize_der()?;
    
    // Save certificate for client to use
    fs::write(CERT_PATH, &cert_der)?;
    println!("Certificate saved to {}", CERT_PATH);
    
    Ok((rustls::Certificate(cert_der), key))
}

/// Load certificate from disk
fn load_cert() -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(fs::read(CERT_PATH)?)
}

/// Configure server
fn configure_server() -> Result<ServerConfig, Box<dyn Error>> {
    let (cert, key) = generate_and_save_cert()?;
    
    let mut server_config = ServerConfig::with_single_cert(vec![cert], key)?;
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(0_u8.into());
    transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(5)));
    
    Ok(server_config)
}

/// Configure client
fn configure_client() -> Result<ClientConfig, Box<dyn Error>> {
    let cert_der = load_cert()?;
    let mut roots = rustls::RootCertStore::empty();
    roots.add(&rustls::Certificate(cert_der))?;
    
    let client_config = ClientConfig::with_root_certificates(roots);
    Ok(client_config)
}

/// Run server
async fn run_server(addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let server_config = configure_server()?;
    let endpoint = Endpoint::server(server_config, addr)?;
    
    println!("✅ Server listening on {}", endpoint.local_addr()?);
    
    loop {
        // Accept connections
        let conn = match endpoint.accept().await {
            Some(conn) => conn,
            None => break,
        };
        
        println!("📥 New connection incoming...");
        
        // Spawn handler for each connection
        tokio::spawn(async move {
            match conn.await {
                Ok(connection) => {
                    println!("✅ Connection established from {}", connection.remote_address());
                    handle_connection(connection).await;
                }
                Err(e) => {
                    println!("❌ Connection failed: {}", e);
                }
            }
        });
    }
    
    Ok(())
}

/// Handle a connection
async fn handle_connection(connection: quinn::Connection) {
    loop {
        match connection.accept_bi().await {
            Ok((mut send, mut recv)) => {
                println!("📨 Bidirectional stream opened");
                
                // Read message
                let mut buf = vec![0u8; 1024];
                match recv.read(&mut buf).await {
                    Ok(Some(n)) => {
                        buf.truncate(n);
                        let msg = String::from_utf8_lossy(&buf);
                        println!("📬 Server received: {}", msg);
                        
                        // Echo back with timestamp
                        let response = format!("Echo: {} [{}]", msg, chrono::Local::now().format("%H:%M:%S"));
                        send.write_all(response.as_bytes()).await.ok();
                        send.finish().await.ok();
                        println!("📤 Server sent: {}", response);
                    }
                    Ok(None) => {
                        println!("Stream closed by client");
                        break;
                    }
                    Err(e) => {
                        println!("Read error: {}", e);
                        break;
                    }
                }
            }
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                println!("Connection closed by client");
                break;
            }
            Err(e) => {
                println!("Connection error: {}", e);
                break;
            }
        }
    }
}

/// Run client
async fn run_client(server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    // Wait for certificate to be created
    for _ in 0..5 {
        if Path::new(CERT_PATH).exists() {
            break;
        }
        println!("⏳ Waiting for server certificate...");
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    let client_config = configure_client()?;
    let mut endpoint = Endpoint::client("127.0.0.1:0".parse()?)?;
    endpoint.set_default_client_config(client_config);
    
    println!("🔗 Connecting to {}", server_addr);
    let connection = endpoint.connect(server_addr, "localhost")?.await?;
    println!("✅ Connected to {}", connection.remote_address());
    
    // Send multiple messages
    for i in 1..=3 {
        let (mut send, mut recv) = connection.open_bi().await?;
        
        let message = format!("Message #{} from client", i);
        send.write_all(message.as_bytes()).await?;
        send.finish().await?;
        println!("📤 Client sent: {}", message);
        
        // Read response
        let mut buf = vec![0u8; 1024];
        if let Some(n) = recv.read(&mut buf).await? {
            buf.truncate(n);
            let response = String::from_utf8_lossy(&buf);
            println!("📬 Client received: {}", response);
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    
    println!("✅ Client test completed");
    connection.close(0u32.into(), b"test complete");
    endpoint.wait_idle().await;
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} [server|client|both]", args[0]);
        std::process::exit(1);
    }
    
    let addr: SocketAddr = "127.0.0.1:5001".parse()?;
    
    match args[1].as_str() {
        "server" => {
            println!("🚀 Starting QUIC server");
            run_server(addr).await?
        },
        "client" => {
            println!("🚀 Starting QUIC client");
            run_client(addr).await?
        },
        "both" => {
            println!("🚀 Starting server and client test");
            
            // Start server in background
            let server_handle = tokio::spawn(async move {
                if let Err(e) = run_server(addr).await {
                    eprintln!("Server error: {}", e);
                }
            });
            
            // Wait a bit for server to start
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            
            // Run client
            if let Err(e) = run_client(addr).await {
                eprintln!("Client error: {}", e);
            }
            
            // Clean up
            server_handle.abort();
        },
        _ => {
            eprintln!("Invalid mode. Use 'server', 'client', or 'both'");
            std::process::exit(1);
        }
    }
    
    // Cleanup
    let _ = fs::remove_file(CERT_PATH);
    
    Ok(())
}