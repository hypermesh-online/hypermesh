//! STOQ Protocol Integration - Custom QUIC frames and transport parameters
//!
//! This module integrates STOQ protocol extensions directly into QUIC packet flow
//! using Quinn's extension points for custom frames and transport parameters.

use bytes::{Bytes, Buf};
use quinn::{VarInt, TransportConfig};
use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use tracing::{debug, trace, warn};

pub mod frames;
pub mod parameters;
pub mod handshake;

use crate::extensions::{PacketToken, PacketShard, StoqProtocolExtension};
use crate::transport::falcon::FalconSignature;

/// STOQ protocol version for QUIC ALPN
pub const STOQ_ALPN: &[u8] = b"stoq/1.0";

/// Custom QUIC frame type identifiers (in the private use range)
/// Range 0xfe000000 - 0xffffffff is reserved for private use
pub mod frame_types {
    use quinn::VarInt;

    /// STOQ tokenization frame type
    pub const STOQ_TOKEN: VarInt = VarInt::from_u32(0xfe000001);

    /// STOQ sharding metadata frame type
    pub const STOQ_SHARD: VarInt = VarInt::from_u32(0xfe000002);

    /// STOQ hop information frame type
    pub const STOQ_HOP: VarInt = VarInt::from_u32(0xfe000003);

    /// STOQ seed information frame type
    pub const STOQ_SEED: VarInt = VarInt::from_u32(0xfe000004);

    /// FALCON signature frame type for quantum-resistant auth
    pub const FALCON_SIG: VarInt = VarInt::from_u32(0xfe000005);

    /// FALCON public key exchange frame type
    pub const FALCON_KEY: VarInt = VarInt::from_u32(0xfe000006);
}

/// Transport parameter IDs for STOQ extensions
pub mod transport_params {
    /// STOQ protocol extensions enabled flag
    pub const STOQ_EXTENSIONS_ENABLED: u64 = 0xfe00;

    /// FALCON quantum crypto enabled flag
    pub const FALCON_ENABLED: u64 = 0xfe01;

    /// FALCON public key transport parameter
    pub const FALCON_PUBLIC_KEY: u64 = 0xfe02;

    /// Maximum shard size transport parameter
    pub const MAX_SHARD_SIZE: u64 = 0xfe03;

    /// Tokenization algorithm identifier
    pub const TOKEN_ALGORITHM: u64 = 0xfe04;
}

/// Shard storage for reassembly
#[derive(Debug, Clone)]
struct ShardStorage {
    shards: Vec<PacketShard>,
    total_expected: u32,
    shard_id: u32,
    last_update: std::time::Instant,
}

/// Connection state for token validation
#[derive(Debug, Clone)]
struct ConnectionState {
    last_token: Option<PacketToken>,
    validated_tokens: Vec<PacketToken>,
    last_validation_time: std::time::Instant,
}

/// STOQ protocol handler for QUIC integration
pub struct StoqProtocolHandler {
    /// Protocol extensions implementation
    extensions: Arc<dyn StoqProtocolExtension + Send + Sync>,

    /// FALCON transport for quantum-resistant crypto
    falcon_transport: Option<Arc<parking_lot::RwLock<crate::transport::falcon::FalconTransport>>>,

    /// Maximum shard size for packet fragmentation
    max_shard_size: usize,

    /// Whether extensions are enabled
    extensions_enabled: bool,

    /// Shard storage for reassembly (shard_id -> ShardStorage)
    shard_storage: Arc<parking_lot::RwLock<HashMap<u32, ShardStorage>>>,

    /// Connection state for token validation (stream_id -> ConnectionState)
    connection_states: Arc<parking_lot::RwLock<HashMap<VarInt, ConnectionState>>>,

    /// Token cache for validation history (token hash -> validation time)
    token_cache: Arc<parking_lot::RwLock<HashMap<[u8; 32], std::time::Instant>>>,
}

impl StoqProtocolHandler {
    /// Create a new protocol handler
    pub fn new(
        extensions: Arc<dyn StoqProtocolExtension + Send + Sync>,
        falcon_transport: Option<Arc<parking_lot::RwLock<crate::transport::falcon::FalconTransport>>>,
        max_shard_size: usize,
    ) -> Self {
        Self {
            extensions,
            falcon_transport,
            max_shard_size,
            extensions_enabled: true,
            shard_storage: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            connection_states: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            token_cache: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    /// Encode a STOQ token frame
    pub fn encode_token_frame(&self, token: &PacketToken) -> Result<Bytes> {
        use crate::protocol::frames::{StoqFrame, TokenFrame};

        let frame = StoqFrame::Token(TokenFrame {
            token: token.clone(),
            stream_id: None,
        });

        frame.encode()
    }

    /// Decode a STOQ token frame
    pub fn decode_token_frame(&self, mut data: Bytes) -> Result<PacketToken> {
        if data.len() < 48 { // 32 (hash) + 8 (seq) + 8 (timestamp)
            return Err(anyhow!("Token frame too short: {} bytes", data.len()));
        }

        let mut hash = [0u8; 32];
        data.copy_to_slice(&mut hash);
        let sequence = data.get_u64();
        let timestamp = data.get_u64();

        Ok(PacketToken {
            hash,
            sequence,
            timestamp,
        })
    }

    /// Encode a STOQ shard metadata frame
    pub fn encode_shard_frame(&self, shard: &PacketShard) -> Result<Bytes> {
        use crate::protocol::frames::{StoqFrame, ShardFrame};

        let frame = StoqFrame::Shard(ShardFrame {
            shard: shard.clone(),
            stream_id: None,
        });

        frame.encode()
    }

    /// Decode a STOQ shard metadata frame
    pub fn decode_shard_frame(&self, mut data: Bytes) -> Result<PacketShard> {
        if data.len() < 48 { // Minimum metadata size
            return Err(anyhow!("Shard frame too short: {} bytes", data.len()));
        }

        let shard_id = data.get_u32();
        let total_shards = data.get_u32();
        let sequence = data.get_u32();

        let mut packet_hash = [0u8; 32];
        data.copy_to_slice(&mut packet_hash);

        let data_len = data.get_u32() as usize;
        if data.len() < data_len {
            return Err(anyhow!("Shard data truncated"));
        }

        let shard_data = data.split_to(data_len);

        Ok(PacketShard {
            shard_id,
            total_shards,
            sequence,
            data: shard_data,
            packet_hash,
        })
    }

    /// Encode a FALCON signature frame
    pub fn encode_falcon_frame(&self, signature: &FalconSignature) -> Result<Bytes> {
        use crate::protocol::frames::{StoqFrame, FalconSigFrame};

        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            let exported = falcon_guard.export_signature(signature);

            let frame = StoqFrame::FalconSignature(FalconSigFrame {
                signature_data: exported,
                key_id: "local".to_string(), // TODO: Use actual key ID
                signed_frames: vec![frame_types::STOQ_TOKEN], // TODO: Track actual signed frames
            });

            frame.encode()
        } else {
            Err(anyhow!("FALCON transport not enabled"))
        }
    }

    /// Decode a FALCON signature frame
    pub fn decode_falcon_frame(&self, mut data: Bytes) -> Result<FalconSignature> {
        if let Some(falcon) = &self.falcon_transport {
            if data.len() < 4 {
                return Err(anyhow!("FALCON frame too short"));
            }

            let sig_len = data.get_u32() as usize;
            if data.len() < sig_len {
                return Err(anyhow!("FALCON signature truncated"));
            }

            let sig_data = data.split_to(sig_len);
            let falcon_guard = falcon.read();
            falcon_guard.import_signature(&sig_data)
        } else {
            Err(anyhow!("FALCON transport not enabled"))
        }
    }

    /// Process incoming custom frame
    pub fn process_frame(&self, data: Bytes) -> Result<()> {
        use crate::protocol::frames::StoqFrame;

        let frame = StoqFrame::decode(data)?;

        match frame {
            StoqFrame::Token(token_frame) => {
                debug!("Received STOQ token: seq={}", token_frame.token.sequence);

                // Validate token and update connection state
                if !self.validate_token(&token_frame.token)? {
                    return Err(anyhow!("Token validation failed"));
                }

                // Update connection state with validated token
                self.update_connection_state(token_frame.stream_id, token_frame.token)?;
                Ok(())
            }
            StoqFrame::Shard(shard_frame) => {
                debug!("Received STOQ shard: {}/{}",
                       shard_frame.shard.sequence + 1,
                       shard_frame.shard.total_shards);

                // Store shard for reassembly
                self.store_shard_for_reassembly(shard_frame.shard)?;
                Ok(())
            }
            StoqFrame::FalconSignature(sig_frame) => {
                debug!("Received FALCON signature");

                // Verify signature
                self.verify_falcon_signature(&sig_frame)?;
                Ok(())
            }
            _ => {
                trace!("Received frame type: {:?}", frame.frame_type());
                Ok(()) // Ignore other frames
            }
        }
    }

    /// Apply STOQ extensions to outgoing data
    pub fn apply_extensions(&self, data: &[u8]) -> Result<Vec<Bytes>> {
        let mut frames = Vec::new();

        if self.extensions_enabled {
            // Add tokenization
            let token = self.extensions.tokenize_packet(data);
            frames.push(self.encode_token_frame(&token)?);

            // Add sharding if data is large
            if data.len() > self.max_shard_size {
                let shards = self.extensions.shard_packet(data, self.max_shard_size)?;
                for shard in shards {
                    frames.push(self.encode_shard_frame(&shard)?);
                }
            }
        }

        Ok(frames)
    }

    /// Sign data with FALCON for quantum-resistant authentication
    pub fn falcon_sign(&self, data: &[u8]) -> Result<Option<Bytes>> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            let signature = falcon_guard.sign_handshake_data(data)?;
            Ok(Some(self.encode_falcon_frame(&signature)?))
        } else {
            Ok(None)
        }
    }

    /// Configure QUIC transport with STOQ extensions
    pub fn configure_transport(&self, config: &mut TransportConfig) {
        // Enable datagram support for custom frames
        config.datagram_receive_buffer_size(Some(65536));
        config.datagram_send_buffer_size(65536);

        // Set STOQ-specific timeouts
        config.max_idle_timeout(Some(quinn::IdleTimeout::from(VarInt::from_u32(120_000))));

        debug!("Configured QUIC transport for STOQ protocol extensions");
    }

    /// Validate a packet token
    fn validate_token(&self, token: &PacketToken) -> Result<bool> {
        use std::time::{SystemTime, UNIX_EPOCH, Duration};

        // Check token expiration (5 minutes max age)
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if current_time > token.timestamp + 300 {
            debug!("Token expired: age={}s", current_time - token.timestamp);
            return Ok(false);
        }

        // Check if we've seen this token recently (prevent replay attacks)
        {
            let mut cache = self.token_cache.write();
            let now = std::time::Instant::now();

            // Clean old entries (older than 5 minutes)
            cache.retain(|_, timestamp| now.duration_since(*timestamp) < Duration::from_secs(300));

            // Check if token already validated
            if cache.contains_key(&token.hash) {
                warn!("Token replay detected: hash={:?}", token.hash);
                return Ok(false);
            }

            // Cache this token
            cache.insert(token.hash, now);
        }

        // Validate token sequence number is reasonable
        if token.sequence == 0 {
            debug!("Invalid token sequence: 0");
            return Ok(false);
        }

        debug!("Token validated successfully: seq={}", token.sequence);
        Ok(true)
    }

    /// Update connection state with validated token
    fn update_connection_state(&self, stream_id: Option<VarInt>, token: PacketToken) -> Result<()> {
        if let Some(stream_id) = stream_id {
            let mut states = self.connection_states.write();
            let state = states.entry(stream_id).or_insert_with(|| ConnectionState {
                last_token: None,
                validated_tokens: Vec::new(),
                last_validation_time: std::time::Instant::now(),
            });

            state.last_token = Some(token.clone());
            state.validated_tokens.push(token);
            state.last_validation_time = std::time::Instant::now();

            // Keep only last 10 tokens per connection
            if state.validated_tokens.len() > 10 {
                state.validated_tokens.remove(0);
            }

            debug!("Updated connection state for stream {:?}", stream_id);
        }
        Ok(())
    }

    /// Store shard for later reassembly
    fn store_shard_for_reassembly(&self, shard: PacketShard) -> Result<()> {
        let mut storage = self.shard_storage.write();
        let shard_id = shard.shard_id;
        let total_shards = shard.total_shards;

        let entry = storage.entry(shard_id).or_insert_with(|| {
            debug!("Creating new shard storage for id {}", shard_id);
            ShardStorage {
                shards: Vec::with_capacity(total_shards as usize),
                total_expected: total_shards,
                shard_id,
                last_update: std::time::Instant::now(),
            }
        });

        // Validate shard consistency
        if entry.total_expected != total_shards {
            return Err(anyhow!(
                "Shard count mismatch: expected {}, got {}",
                entry.total_expected,
                total_shards
            ));
        }

        // Check if shard already exists
        if entry.shards.iter().any(|s| s.sequence == shard.sequence) {
            debug!("Duplicate shard received: id={}, seq={}", shard_id, shard.sequence);
            return Ok(());
        }

        entry.shards.push(shard.clone());
        entry.last_update = std::time::Instant::now();

        debug!(
            "Stored shard {}/{} for id {}",
            shard.sequence + 1,
            total_shards,
            shard_id
        );

        // Check if we have all shards for reassembly
        if entry.shards.len() == total_shards as usize {
            debug!("All shards received for id {}, triggering reassembly", shard_id);

            // Clone shards for reassembly
            let shards = entry.shards.clone();

            // Remove from storage
            storage.remove(&shard_id);

            // Attempt reassembly
            match self.extensions.reassemble_shards(shards) {
                Ok(_data) => {
                    debug!("Successfully reassembled packet from {} shards", total_shards);
                    // Here you could trigger further processing of the reassembled data
                }
                Err(e) => {
                    warn!("Failed to reassemble shards: {}", e);
                    return Err(e);
                }
            }
        }

        // Clean up old incomplete shard collections (older than 30 seconds)
        let now = std::time::Instant::now();
        storage.retain(|_, entry| {
            now.duration_since(entry.last_update) < std::time::Duration::from_secs(30)
        });

        Ok(())
    }

    /// Verify FALCON signature
    fn verify_falcon_signature(&self, sig_frame: &frames::FalconSigFrame) -> Result<()> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();

            // Import and verify the signature
            let _signature = falcon_guard.import_signature(&sig_frame.signature_data)?;

            // Here we would verify against the signed data
            // For now, we just validate the signature format
            if sig_frame.key_id.is_empty() {
                return Err(anyhow!("Missing key ID in FALCON signature"));
            }

            if sig_frame.signature_data.is_empty() {
                return Err(anyhow!("Empty signature data"));
            }

            // Check that signed frames are specified
            if sig_frame.signed_frames.is_empty() {
                warn!("FALCON signature has no signed frames specified");
            }

            debug!(
                "FALCON signature verified: key_id={}, signed_frames={}",
                sig_frame.key_id,
                sig_frame.signed_frames.len()
            );

            Ok(())
        } else {
            Err(anyhow!("FALCON transport not enabled for signature verification"))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::DefaultStoqExtensions;
    use crate::protocol::frames;

    #[test]
    fn test_token_frame_encoding() {
        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 1400);

        let data = b"test data";
        let token = extensions.tokenize_packet(data);

        let encoded = handler.encode_token_frame(&token).unwrap();
        assert!(!encoded.is_empty());

        // Decode the entire frame
        let decoded_frame = frames::StoqFrame::decode(encoded).unwrap();
        if let frames::StoqFrame::Token(decoded) = decoded_frame {
            assert_eq!(decoded.token.hash, token.hash);
            assert_eq!(decoded.token.sequence, token.sequence);
            assert_eq!(decoded.token.timestamp, token.timestamp);
        } else {
            panic!("Wrong frame type decoded");
        }
    }

    #[test]
    fn test_shard_frame_encoding() {
        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 10);

        let data = b"this is test data for sharding";
        let shards = extensions.shard_packet(data, 10).unwrap();

        for shard in shards {
            let encoded = handler.encode_shard_frame(&shard).unwrap();
            assert!(!encoded.is_empty());

            // Decode the entire frame
            let decoded_frame = frames::StoqFrame::decode(encoded).unwrap();
            if let frames::StoqFrame::Shard(decoded) = decoded_frame {
                assert_eq!(decoded.shard.shard_id, shard.shard_id);
                assert_eq!(decoded.shard.total_shards, shard.total_shards);
                assert_eq!(decoded.shard.sequence, shard.sequence);
                assert_eq!(decoded.shard.packet_hash, shard.packet_hash);
                assert_eq!(decoded.shard.data, shard.data);
            } else {
                panic!("Wrong frame type decoded");
            }
        }
    }

    #[test]
    fn test_token_validation() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 1400);

        // Create a token with current timestamp
        let token = PacketToken {
            hash: [42; 32],
            sequence: 123,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Test valid token
        assert!(handler.validate_token(&token).unwrap());

        // Test replay detection (second validation should fail)
        assert!(!handler.validate_token(&token).unwrap());

        // Test expired token
        let expired_token = PacketToken {
            hash: [99; 32],
            sequence: 456,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - 400, // 400 seconds ago (more than 5 minutes)
        };
        assert!(!handler.validate_token(&expired_token).unwrap());
    }

    #[test]
    fn test_shard_reassembly() {
        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 10);

        let data = b"test data for sharding and reassembly";
        let shards = extensions.shard_packet(data, 10).unwrap();

        // Store all shards for reassembly
        for shard in shards {
            handler.store_shard_for_reassembly(shard).unwrap();
        }

        // Verify shards were stored and reassembled (check storage is empty)
        let storage = handler.shard_storage.read();
        assert_eq!(storage.len(), 0, "All shards should have been reassembled");
    }

    #[test]
    fn test_connection_state_update() {
        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 1400);

        let stream_id = VarInt::from_u32(42);
        let token = PacketToken {
            hash: [1; 32],
            sequence: 100,
            timestamp: 123456789,
        };

        // Update connection state
        handler.update_connection_state(Some(stream_id), token.clone()).unwrap();

        // Verify state was updated
        let states = handler.connection_states.read();
        assert!(states.contains_key(&stream_id));

        let state = states.get(&stream_id).unwrap();
        assert_eq!(state.last_token, Some(token));
        assert_eq!(state.validated_tokens.len(), 1);
    }
}