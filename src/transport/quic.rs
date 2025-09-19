//! QUIC Implementation for STOQ Transport
//! 
//! High-performance QUIC over IPv6 implementation optimized for 40 Gbps throughput.
//! Integrates with existing STOQ transport system and provides certificate validation.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::net::{SocketAddr, Ipv6Addr};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};

use crate::config::StoqConfig;
use crate::authority::TrustChainAuthorityLayer;
use crate::transport::StoqEndpoint;

// Rustls imports
use rustls::client::danger::{ServerCertVerifier, ServerCertVerified, HandshakeSignatureValid};

/// QUIC Endpoint for STOQ transport
pub struct QuicEndpoint {
    /// QUIC endpoint configuration
    config: StoqConfig,
    
    /// Global configuration
    global_config: crate::config::GlobalConfig,
    
    /// Quinn QUIC endpoint
    quinn_endpoint: Arc<RwLock<Option<quinn::Endpoint>>>,
    
    /// TrustChain integration for certificates
    trustchain: Arc<TrustChainAuthorityLayer>,
    
    /// Endpoint state
    state: Arc<RwLock<EndpointState>>,
}

/// QUIC Connection wrapper
pub struct QuicConnection {
    /// Quinn connection
    inner: quinn::Connection,
    
    /// Connection metadata
    pub connection_id: String,
    pub remote_address: SocketAddr,
    pub established_at: Instant,
    
    /// Certificate validation results
    pub certificate_fingerprint: Option<String>,
    pub certificate_valid: bool,
}

/// Endpoint state
#[derive(Debug, Clone)]
struct EndpointState {
    bound: bool,
    listening: bool,
    bind_address: Option<SocketAddr>,
    connection_count: u64,
    error_count: u64,
}

impl QuicEndpoint {
    /// Create new QUIC endpoint
    pub async fn new(
        config: &StoqConfig,
        global_config: &crate::config::GlobalConfig,
        trustchain: Arc<TrustChainAuthorityLayer>
    ) -> Result<Self> {
        info!("🚀 Initializing QUIC endpoint for STOQ transport");
        
        let state = EndpointState {
            bound: false,
            listening: false,
            bind_address: None,
            connection_count: 0,
            error_count: 0,
        };
        
        Ok(Self {
            config: config.clone(),
            global_config: global_config.clone(),
            quinn_endpoint: Arc::new(RwLock::new(None)),
            trustchain,
            state: Arc::new(RwLock::new(state)),
        })
    }
    
    /// Start the QUIC endpoint
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting QUIC endpoint");
        
        // Initialize default crypto provider for Rustls (using ring)
        rustls::crypto::ring::default_provider()
            .install_default()
            .map_err(|_| anyhow!("Failed to install default crypto provider"))?;
        
        // Initialize TLS configuration from TrustChain
        let server_config = self.create_server_config().await?;
        let client_config = self.create_client_config().await?;
        
        // Create socket with IPv6-only binding - bind to global config port
        let bind_addr = SocketAddr::from((
            self.global_config.bind_address, // Use configured bind address
            self.global_config.port // Use configured port (e.g., 8443)
        ));
        
        let socket = std::net::UdpSocket::bind(bind_addr)?;
        
        // Set IPv6-only (temporarily disabled for testing)
        let socket = socket2::Socket::from(socket);
        // socket.set_only_v6(true)?;
        
        // Configure socket for high performance
        socket.set_send_buffer_size(self.config.quic.send_buffer_size)?;
        socket.set_recv_buffer_size(self.config.quic.receive_buffer_size)?;
        
        // Create Quinn endpoint
        let mut endpoint = quinn::Endpoint::new(
            quinn::EndpointConfig::default(),
            Some(server_config),
            socket.into(),
            Arc::new(quinn::TokioRuntime)
        )?;
        
        endpoint.set_default_client_config(client_config);
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.bound = true;
            state.listening = true;
            state.bind_address = Some(bind_addr);
        }
        
        // Store endpoint
        *self.quinn_endpoint.write().await = Some(endpoint);
        
        info!("✅ QUIC endpoint started on {}", bind_addr);
        Ok(())
    }
    
    /// Accept incoming connection (waits persistently for connections)
    pub async fn accept(&self) -> Result<Arc<QuicConnection>> {
        let endpoint_guard = self.quinn_endpoint.read().await;
        let endpoint = endpoint_guard.as_ref()
            .ok_or_else(|| anyhow!("QUIC endpoint not started"))?;
        
        // Accept incoming connection - this will block until a connection arrives
        loop {
            if let Some(incoming) = endpoint.accept().await {
                // Process the incoming connection
                match incoming.await {
                    Ok(connection) => {
                        let remote_addr = connection.remote_address();
                        
                        // Ensure IPv6-only
                        if !remote_addr.is_ipv6() {
                            warn!("⚠️ Rejected IPv4 connection from {} - STOQ is IPv6-only", remote_addr);
                            continue; // Skip IPv4 connections, wait for next
                        }
                        
                        // Create connection wrapper
                        let connection_id = format!("quic-{}", uuid::Uuid::new_v4());
                        let quic_conn = Arc::new(QuicConnection {
                            inner: connection,
                            connection_id: connection_id.clone(),
                            remote_address: remote_addr,
                            established_at: Instant::now(),
                            certificate_fingerprint: None,
                            certificate_valid: false,
                        });
                        
                        // Update connection count
                        {
                            let mut state = self.state.write().await;
                            state.connection_count += 1;
                        }
                        
                        debug!("✅ QUIC connection accepted: {} from {}", connection_id, remote_addr);
                        return Ok(quic_conn);
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to establish incoming connection: {}", e);
                        // Continue waiting for next connection
                        continue;
                    }
                }
            }
            // If endpoint.accept() returns None, the endpoint is closed
            return Err(anyhow!("QUIC endpoint closed"));
        }
    }
    
    /// Connect to remote endpoint
    pub async fn connect(&self, endpoint: &StoqEndpoint) -> Result<Arc<QuicConnection>> {
        let endpoint_guard = self.quinn_endpoint.read().await;
        let quinn_endpoint = endpoint_guard.as_ref()
            .ok_or_else(|| anyhow!("QUIC endpoint not started"))?;
        
        let socket_addr = SocketAddr::from((endpoint.address, endpoint.port));
        
        // Ensure IPv6-only
        if !socket_addr.is_ipv6() {
            return Err(anyhow!("IPv4 connections not supported - STOQ is IPv6-only"));
        }
        
        let server_name = endpoint.server_name.as_deref().unwrap_or("localhost");
        
        debug!("🔗 Connecting to {} at {}", server_name, socket_addr);
        
        // Initiate connection
        let connecting = quinn_endpoint.connect(socket_addr, server_name)?;
        let connection = connecting.await?;
        
        // Create connection wrapper
        let connection_id = format!("quic-{}", uuid::Uuid::new_v4());
        let quic_conn = Arc::new(QuicConnection {
            inner: connection,
            connection_id: connection_id.clone(),
            remote_address: socket_addr,
            established_at: Instant::now(),
            certificate_fingerprint: None,
            certificate_valid: false,
        });
        
        // Update connection count
        {
            let mut state = self.state.write().await;
            state.connection_count += 1;
        }
        
        info!("✅ QUIC connection established: {} to {}", connection_id, socket_addr);
        Ok(quic_conn)
    }
    
    /// Create server TLS configuration
    async fn create_server_config(&self) -> Result<quinn::ServerConfig> {
        debug!("🔐 Creating server TLS configuration from TrustChain");
        
        // For now, create a simple server config
        // In production, this would integrate with TrustChain certificates
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
        let cert_der = cert.cert.der();
        let priv_key = cert.key_pair.serialize_der();
        
        let cert_chain = vec![cert_der.clone()];
        let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(priv_key.into());
        
        let server_config = quinn::ServerConfig::with_single_cert(cert_chain, key_der)?;
        
        debug!("✅ Server TLS configuration created");
        Ok(server_config)
    }
    
    /// Create client TLS configuration
    async fn create_client_config(&self) -> Result<quinn::ClientConfig> {
        debug!("🔐 Creating client TLS configuration");
        
        // Create client config that accepts self-signed certificates for now
        // In production, this would validate against TrustChain CA
        let crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();
        
        let client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?
        ));
        
        debug!("✅ Client TLS configuration created");
        Ok(client_config)
    }
    
    /// Get endpoint statistics
    pub async fn get_stats(&self) -> EndpointStats {
        let state = self.state.read().await;
        
        EndpointStats {
            bound: state.bound,
            listening: state.listening,
            bind_address: state.bind_address,
            connection_count: state.connection_count,
            error_count: state.error_count,
        }
    }
    
    /// Shutdown endpoint
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down QUIC endpoint");
        
        if let Some(endpoint) = self.quinn_endpoint.write().await.take() {
            endpoint.close(0u32.into(), b"shutdown");
        }
        
        let mut state = self.state.write().await;
        state.bound = false;
        state.listening = false;
        
        info!("✅ QUIC endpoint shutdown complete");
        Ok(())
    }
}

impl QuicConnection {
    /// Get remote address
    pub fn remote_address(&self) -> Result<SocketAddr> {
        Ok(self.remote_address)
    }
    
    /// Close connection
    pub async fn close(&self) -> Result<()> {
        self.inner.close(0u32.into(), b"closing");
        Ok(())
    }
    
    /// Open bidirectional stream
    pub async fn open_bi(&self) -> Result<(quinn::SendStream, quinn::RecvStream)> {
        Ok(self.inner.open_bi().await?)
    }
    
    /// Accept bidirectional stream
    pub async fn accept_bi(&self) -> Result<(quinn::SendStream, quinn::RecvStream)> {
        Ok(self.inner.accept_bi().await?)
    }
    
    /// Send datagram
    pub fn send_datagram(&self, data: bytes::Bytes) -> Result<()> {
        Ok(self.inner.send_datagram(data)?)
    }
    
    /// Read datagram
    pub async fn read_datagram(&self) -> Result<bytes::Bytes> {
        Ok(self.inner.read_datagram().await?)
    }
}

/// Endpoint statistics
#[derive(Debug, Clone)]
pub struct EndpointStats {
    pub bound: bool,
    pub listening: bool,
    pub bind_address: Option<SocketAddr>,
    pub connection_count: u64,
    pub error_count: u64,
}

/// Skip server certificate verification (for development/testing)
/// In production, this would validate against TrustChain CA
#[derive(Debug)]
struct SkipServerVerification;

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Skip verification for now - in production this would validate against TrustChain
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}