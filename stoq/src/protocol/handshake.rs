//! STOQ QUIC Handshake Integration with FALCON Quantum-Resistant Cryptography
//!
//! This module integrates FALCON signatures into the QUIC handshake process
//! for quantum-resistant authentication at the transport layer.

use bytes::{Bytes, BytesMut, BufMut, Buf};
use quinn::crypto::Session;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tracing::{debug, info, warn};

use crate::transport::falcon::{FalconTransport, FalconPublicKey};
use super::transport_params;

/// STOQ handshake extension for QUIC
pub struct StoqHandshakeExtension {
    /// FALCON transport for quantum-resistant crypto
    falcon_transport: Option<Arc<parking_lot::RwLock<FalconTransport>>>,

    /// Cached peer public keys
    peer_keys: Arc<dashmap::DashMap<String, FalconPublicKey>>,

    /// Whether to require FALCON authentication
    require_falcon: bool,

    /// Whether to use hybrid mode (FALCON + traditional)
    hybrid_mode: bool,
}

impl StoqHandshakeExtension {
    /// Create new handshake extension
    pub fn new(
        falcon_transport: Option<Arc<parking_lot::RwLock<FalconTransport>>>,
        require_falcon: bool,
        hybrid_mode: bool,
    ) -> Self {
        Self {
            falcon_transport,
            peer_keys: Arc::new(dashmap::DashMap::new()),
            require_falcon,
            hybrid_mode,
        }
    }

    /// Add FALCON signature to handshake
    pub fn add_falcon_signature(&self, handshake_data: &[u8]) -> Result<Vec<u8>> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            let signature = falcon_guard.sign_handshake_data(handshake_data)?;
            let exported = falcon_guard.export_signature(&signature);

            debug!("Added FALCON signature to handshake: {} bytes", exported.len());
            Ok(exported)
        } else if self.require_falcon {
            Err(anyhow!("FALCON required but not available"))
        } else {
            Ok(Vec::new())
        }
    }

    /// Verify FALCON signature in handshake
    pub fn verify_falcon_signature(
        &self,
        peer_id: &str,
        signature_data: &[u8],
        handshake_data: &[u8],
    ) -> Result<bool> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            let signature = falcon_guard.import_signature(signature_data)?;

            // Check if we have the peer's public key
            if let Some(peer_key) = self.peer_keys.get(peer_id) {
                let engine = crate::transport::falcon::FalconEngine::new(peer_key.variant);
                let valid = engine.verify(&peer_key, &signature, handshake_data)?;

                if valid {
                    info!("FALCON signature verified for peer: {}", peer_id);
                } else {
                    warn!("FALCON signature verification failed for peer: {}", peer_id);
                }

                Ok(valid)
            } else {
                warn!("No public key for peer: {}", peer_id);
                Ok(false)
            }
        } else if self.require_falcon {
            Err(anyhow!("FALCON required but not available"))
        } else {
            Ok(true) // Pass if FALCON not required
        }
    }

    /// Export local FALCON public key for transport parameters
    pub fn export_public_key(&self) -> Result<Option<Vec<u8>>> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            if let Some(public_key) = falcon_guard.get_local_public_key() {
                let mut buf = BytesMut::new();

                // Export key format
                buf.put_u8(match public_key.variant {
                    crate::transport::falcon::FalconVariant::Falcon512 => 0,
                    crate::transport::falcon::FalconVariant::Falcon1024 => 1,
                });
                buf.put_u32(public_key.key_data.len() as u32);
                buf.put_slice(&public_key.key_data);
                buf.put_u64(public_key.created_at);

                // Optional key ID
                if let Some(ref key_id) = public_key.key_id {
                    buf.put_u8(1); // Has key ID
                    buf.put_u32(key_id.len() as u32);
                    buf.put_slice(key_id.as_bytes());
                } else {
                    buf.put_u8(0); // No key ID
                }

                Ok(Some(buf.freeze().to_vec()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Import peer's FALCON public key from transport parameters
    pub fn import_peer_key(&self, peer_id: String, key_data: &[u8]) -> Result<()> {
        if key_data.len() < 14 { // Minimum size
            return Err(anyhow!("Public key data too short"));
        }

        let mut buf = Bytes::copy_from_slice(key_data);

        let variant = match buf.get_u8() {
            0 => crate::transport::falcon::FalconVariant::Falcon512,
            1 => crate::transport::falcon::FalconVariant::Falcon1024,
            v => return Err(anyhow!("Unknown FALCON variant: {}", v)),
        };

        let key_len = buf.get_u32() as usize;
        if buf.len() < key_len + 9 { // key_len + 8 (timestamp) + 1 (key_id flag)
            return Err(anyhow!("Public key truncated"));
        }

        let key_bytes = buf.split_to(key_len).to_vec();
        let created_at = buf.get_u64();

        let mut public_key = FalconPublicKey::new(variant, key_bytes)?;
        public_key.created_at = created_at;

        // Read optional key ID
        if buf.has_remaining() && buf.get_u8() == 1 {
            if buf.remaining() >= 4 {
                let id_len = buf.get_u32() as usize;
                if buf.len() >= id_len {
                    let key_id = String::from_utf8_lossy(&buf.split_to(id_len)).to_string();
                    public_key = public_key.with_key_id(key_id);
                }
            }
        }

        self.peer_keys.insert(peer_id.clone(), public_key);
        debug!("Imported FALCON public key for peer: {}", peer_id);

        Ok(())
    }

    /// Create handshake authenticator combining traditional TLS and FALCON
    pub fn create_hybrid_authenticator(&self, tls_data: &[u8]) -> Result<Vec<u8>> {
        let mut auth = BytesMut::new();

        // Add traditional TLS data
        auth.put_u32(tls_data.len() as u32);
        auth.put_slice(tls_data);

        // Add FALCON signature if available
        if let Some(falcon_sig) = self.add_falcon_signature(tls_data).ok() {
            auth.put_u8(1); // Has FALCON
            auth.put_u32(falcon_sig.len() as u32);
            auth.put_slice(&falcon_sig);
        } else {
            auth.put_u8(0); // No FALCON
        }

        Ok(auth.freeze().to_vec())
    }

    /// Verify hybrid authenticator
    pub fn verify_hybrid_authenticator(
        &self,
        peer_id: &str,
        auth_data: &[u8],
        expected_tls: &[u8],
    ) -> Result<bool> {
        if auth_data.len() < 5 {
            return Err(anyhow!("Authenticator too short"));
        }

        let mut buf = Bytes::copy_from_slice(auth_data);

        // Verify TLS data
        let tls_len = buf.get_u32() as usize;
        if buf.len() < tls_len + 1 {
            return Err(anyhow!("TLS data truncated"));
        }

        let tls_data = buf.split_to(tls_len);
        if tls_data != expected_tls {
            return Ok(false); // TLS mismatch
        }

        // Check for FALCON signature
        let has_falcon = buf.get_u8() == 1;
        if has_falcon {
            if buf.remaining() < 4 {
                return Err(anyhow!("FALCON signature header truncated"));
            }

            let falcon_len = buf.get_u32() as usize;
            if buf.len() < falcon_len {
                return Err(anyhow!("FALCON signature truncated"));
            }

            let falcon_sig = buf.split_to(falcon_len);
            let valid = self.verify_falcon_signature(peer_id, &falcon_sig, &tls_data)?;

            if self.hybrid_mode {
                // In hybrid mode, both must be valid
                Ok(valid)
            } else {
                // In non-hybrid mode, FALCON is optional enhancement
                Ok(true)
            }
        } else if self.require_falcon {
            Ok(false) // FALCON required but not present
        } else {
            Ok(true) // FALCON not required
        }
    }
}

/// STOQ-enhanced QUIC crypto session
pub struct StoqCryptoSession {
    /// Base crypto session
    inner: Box<dyn Session>,

    /// STOQ handshake extension
    extension: Arc<StoqHandshakeExtension>,

    /// Connection ID
    conn_id: String,
}

impl StoqCryptoSession {
    /// Create new STOQ crypto session
    pub fn new(
        inner: Box<dyn Session>,
        extension: Arc<StoqHandshakeExtension>,
        conn_id: String,
    ) -> Self {
        Self {
            inner,
            extension,
            conn_id,
        }
    }

    /// Process handshake with STOQ extensions
    pub fn process_handshake(&mut self, data: &[u8]) -> Result<()> {
        // Let base session process first
        // Note: This is simplified - actual integration would hook into quinn's crypto traits

        // Add FALCON verification if enabled
        if self.extension.falcon_transport.is_some() {
            // Extract peer ID from connection
            let peer_id = self.conn_id.clone(); // Simplified

            // Look for FALCON signature in handshake
            // In real implementation, this would parse TLS extensions
            if data.len() > 100 {
                // Simplified check
                let sig_start = data.len().saturating_sub(100);
                let potential_sig = &data[sig_start..];

                // Try to verify as FALCON signature
                if let Ok(valid) = self.extension.verify_falcon_signature(
                    &peer_id,
                    potential_sig,
                    &data[..sig_start],
                ) {
                    if valid {
                        info!("FALCON handshake verification succeeded for {}", peer_id);
                    } else if self.extension.require_falcon {
                        return Err(anyhow!("FALCON verification required but failed"));
                    }
                }
            }
        }

        Ok(())
    }
}

/// Transport parameters builder for STOQ
pub struct StoqTransportParams {
    params: Vec<(u64, Vec<u8>)>,
}

impl StoqTransportParams {
    /// Create new transport parameters
    pub fn new() -> Self {
        Self {
            params: Vec::new(),
        }
    }

    /// Add STOQ extensions enabled flag
    pub fn with_stoq_extensions(mut self, enabled: bool) -> Self {
        self.params.push((
            transport_params::STOQ_EXTENSIONS_ENABLED,
            vec![if enabled { 1 } else { 0 }],
        ));
        self
    }

    /// Add FALCON enabled flag
    pub fn with_falcon(mut self, enabled: bool) -> Self {
        self.params.push((
            transport_params::FALCON_ENABLED,
            vec![if enabled { 1 } else { 0 }],
        ));
        self
    }

    /// Add FALCON public key
    pub fn with_falcon_key(mut self, key: Vec<u8>) -> Self {
        self.params.push((
            transport_params::FALCON_PUBLIC_KEY,
            key,
        ));
        self
    }

    /// Add maximum shard size
    pub fn with_max_shard_size(mut self, size: u32) -> Self {
        let mut buf = Vec::with_capacity(4);
        buf.extend_from_slice(&size.to_be_bytes());
        self.params.push((
            transport_params::MAX_SHARD_SIZE,
            buf,
        ));
        self
    }

    /// Build the parameters
    pub fn build(self) -> Vec<(u64, Vec<u8>)> {
        self.params
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::falcon::FalconVariant;

    #[tokio::test]
    async fn test_handshake_extension() -> Result<()> {
        // Create FALCON transport
        let mut falcon = FalconTransport::new(FalconVariant::Falcon1024);
        falcon.generate_local_keypair()?;
        let falcon = Arc::new(parking_lot::RwLock::new(falcon));

        // Create handshake extension
        let extension = StoqHandshakeExtension::new(
            Some(falcon.clone()),
            false, // Don't require FALCON
            false, // No hybrid mode
        );

        // Test adding signature
        let handshake_data = b"test handshake data";
        let sig = extension.add_falcon_signature(handshake_data)?;
        assert!(!sig.is_empty());

        // Test exporting public key
        let key = extension.export_public_key()?;
        assert!(key.is_some());

        // Test importing peer key
        extension.import_peer_key("peer1".to_string(), &key.unwrap())?;

        Ok(())
    }

    #[test]
    fn test_transport_params() {
        let params = StoqTransportParams::new()
            .with_stoq_extensions(true)
            .with_falcon(true)
            .with_max_shard_size(1400)
            .build();

        assert_eq!(params.len(), 3);
        assert_eq!(params[0].0, transport_params::STOQ_EXTENSIONS_ENABLED);
        assert_eq!(params[1].0, transport_params::FALCON_ENABLED);
        assert_eq!(params[2].0, transport_params::MAX_SHARD_SIZE);
    }
}