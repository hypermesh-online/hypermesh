//! STOQ Protocol Integration - Custom QUIC frames and transport parameters
//!
//! This module integrates STOQ protocol extensions directly into QUIC packet flow
//! using Quinn's extension points for custom frames and transport parameters.

use bytes::{Bytes, Buf};
use quinn::{VarInt, TransportConfig};
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tracing::{debug, trace};

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
                // TODO: Validate token and update connection state
                Ok(())
            }
            StoqFrame::Shard(shard_frame) => {
                debug!("Received STOQ shard: {}/{}",
                       shard_frame.shard.sequence + 1,
                       shard_frame.shard.total_shards);
                // TODO: Store shard for reassembly
                Ok(())
            }
            StoqFrame::FalconSignature(_sig_frame) => {
                debug!("Received FALCON signature");
                // TODO: Verify signature
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
}