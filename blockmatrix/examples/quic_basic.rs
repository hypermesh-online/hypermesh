//! Basic QUIC server and client example using quinn 0.10

use quinn::{Endpoint, ServerConfig, ClientConfig};
use std::error::Error;
use std::sync::Arc;
use std::net::SocketAddr;

/// Generate self-signed certificate for testing
fn generate_self_signed_cert() -> Result<(rustls::Certificate, rustls::PrivateKey), Box<dyn Error>> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let key = rustls::PrivateKey(cert.serialize_private_key_der());
    let cert = rustls::Certificate(cert.serialize_der()?);
    Ok((cert, key))
}

/// Configure server with self-signed certificate
fn configure_server() -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>> {
    let (cert, key) = generate_self_signed_cert()?;
    let cert_der = cert.0.clone();
    
    let mut server_config = ServerConfig::with_single_cert(vec![cert], key)?;
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(0_u8.into());
    
    Ok((server_config, cert_der))
}

/// Configure client to trust the server's self-signed certificate
fn configure_client(cert_der: &[u8]) -> Result<ClientConfig, Box<dyn Error>> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(&rustls::Certificate(cert_der.to_vec()))?;
    
    let client_config = ClientConfig::with_root_certificates(roots);
    Ok(client_config)
}

/// Run QUIC server
async fn run_server(addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let (server_config, cert_der) = configure_server()?;
    let endpoint = Endpoint::server(server_config, addr)?;
    
    println!("Server listening on {}", endpoint.local_addr()?);
    println!("Certificate (DER): {} bytes", cert_der.len());
    
    // Accept one connection
    if let Some(conn) = endpoint.accept().await {
        println!("Connection incoming...");
        let connection = conn.await?;
        println!("Connection established from {}", connection.remote_address());
        
        // Accept a bidirectional stream
        if let Ok((mut send, mut recv)) = connection.accept_bi().await {
            println!("Stream opened");
            
            // Read message from client
            let mut buf = [0u8; 1024];
            if let Ok(Some(n)) = recv.read(&mut buf).await {
                let msg = String::from_utf8_lossy(&buf[..n]);
                println!("Server received: {}", msg);
                
                // Send response
                let response = b"Hello from server!";
                send.write_all(response).await?;
                send.finish().await?;
                println!("Server sent response");
            }
        }
        
        // Keep connection alive briefly
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        connection.close(0u32.into(), b"done");
    }
    
    endpoint.close(0u32.into(), b"server shutdown");
    endpoint.wait_idle().await;
    
    Ok(())
}

/// Run QUIC client
async fn run_client(server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    // For this example, we'll use a pre-shared certificate
    // In production, you'd get this through a secure channel
    let cert_der = {
        let (cert, _key) = generate_self_signed_cert()?;
        cert.0.clone()
    };
    
    let client_config = configure_client(&cert_der)?;
    
    let mut endpoint = Endpoint::client("127.0.0.1:0".parse()?)?;
    endpoint.set_default_client_config(client_config);
    
    println!("Connecting to {}", server_addr);
    let connection = endpoint.connect(server_addr, "localhost")?.await?;
    println!("Connected to {}", connection.remote_address());
    
    // Open a bidirectional stream
    let (mut send, mut recv) = connection.open_bi().await?;
    println!("Stream opened");
    
    // Send message to server
    let message = b"Hello from client!";
    send.write_all(message).await?;
    send.finish().await?;
    println!("Client sent: {}", String::from_utf8_lossy(message));
    
    // Read response
    let mut buf = [0u8; 1024];
    if let Some(n) = recv.read(&mut buf).await? {
        let response = String::from_utf8_lossy(&buf[..n]);
        println!("Client received: {}", response);
    }
    
    // Close gracefully
    connection.close(0u32.into(), b"done");
    endpoint.wait_idle().await;
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Simple command-line handling
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} [server|client]", args[0]);
        std::process::exit(1);
    }
    
    let addr: SocketAddr = "127.0.0.1:5000".parse()?;
    
    match args[1].as_str() {
        "server" => run_server(addr).await?,
        "client" => {
            // Give server time to start if running locally
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            run_client(addr).await?
        },
        _ => {
            eprintln!("Invalid mode. Use 'server' or 'client'");
            std::process::exit(1);
        }
    }
    
    Ok(())
}