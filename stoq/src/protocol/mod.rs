// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Protocol Integration - Custom QUIC frames and transport parameters
//!
//! This module integrates STOQ protocol extensions directly into QUIC packet flow
//! using Quinn's extension points for custom frames and transport parameters.

use anyhow::{anyhow, Result};
use bytes::{Buf, Bytes};
use quinn::{TransportConfig, VarInt};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace};

pub mod bilateral;
pub mod falcon_trustchain;
pub mod frame_codec;
pub mod frames;
mod handler_ops;
pub mod handshake;
pub mod parameters;
pub mod pos_fast_validator;
pub mod pos_integration;
pub mod pos_validator;

#[cfg(feature = "engauge")]
pub mod metrics_bridge;
#[cfg(feature = "engauge")]
pub mod metrics_frame;

// Re-exports for backward compatibility
pub use pos_validator::{
    PosToken, PosTokenValidator, ProofData, ProofOfSpace, ProofOfStake, ProofOfTime, ProofOfWork,
    TrustChainClient, ValidationResult,
};

// Re-export integration types
pub use pos_integration::{
    AssetVerification, ConnectionStats, IntegrationStats, MatrixPosition, MatrixPositionExt,
    ShardAddress, StoqPosIntegration,
};

use crate::extensions::{PacketShard, PacketToken, StoqProtocolExtension};
use crate::transport::falcon::FalconSignature;
use hypermesh_ebpf::{encode_pos_extension, WirePosHeader};

/// STOQ protocol version for QUIC ALPN
pub const STOQ_ALPN: &[u8] = b"stoq/1.0";

/// Default PoW difficulty (leading zero bits) carried in the HyperMesh
/// on-wire PoS header. Matches the kernel `KernelPosConfig` default so the
/// XDP program's `min_difficulty` structural check and the emitted header
/// agree. The field is carried on the wire but the kernel derives difficulty
/// from the work hash's leading zeros, not this field.
pub const KERNEL_POS_DIFFICULTY: u32 = 8;

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

    /// Engauge METRICS frame type for streaming metrics payloads
    #[cfg(feature = "engauge")]
    pub const STOQ_METRICS: VarInt = VarInt::from_u32(0xfe000007);
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

use handler_ops::{ConnectionState, ShardStorage};

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

    /// FALCON key identifier used in signature frames (defaults to "local")
    falcon_key_id: String,

    /// Shard storage for reassembly (shard_id -> ShardStorage)
    shard_storage: Arc<parking_lot::RwLock<HashMap<u32, ShardStorage>>>,

    /// Connection state for token validation (stream_id -> ConnectionState)
    connection_states: Arc<parking_lot::RwLock<HashMap<VarInt, ConnectionState>>>,

    /// Token cache for validation history (token hash -> validation time)
    token_cache: Arc<parking_lot::RwLock<HashMap<[u8; 32], std::time::Instant>>>,
}

impl StoqProtocolHandler {
    /// Create a new protocol handler.
    ///
    /// The FALCON `key_id` defaults to `"local"`.  Call [`set_falcon_key_id`]
    /// to change it after construction.
    pub fn new(
        extensions: Arc<dyn StoqProtocolExtension + Send + Sync>,
        falcon_transport: Option<
            Arc<parking_lot::RwLock<crate::transport::falcon::FalconTransport>>,
        >,
        max_shard_size: usize,
    ) -> Self {
        Self {
            extensions,
            falcon_transport,
            max_shard_size,
            extensions_enabled: true,
            falcon_key_id: "local".to_string(),
            shard_storage: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            connection_states: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            token_cache: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    /// Set the FALCON key identifier used in outgoing signature frames.
    pub fn set_falcon_key_id(&mut self, key_id: String) {
        self.falcon_key_id = key_id;
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
        if data.len() < 48 {
            // 32 (hash) + 8 (seq) + 8 (timestamp)
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
        use crate::protocol::frames::{ShardFrame, StoqFrame};

        let frame = StoqFrame::Shard(ShardFrame {
            shard: shard.clone(),
            stream_id: None,
        });

        frame.encode()
    }

    /// Decode a STOQ shard metadata frame
    pub fn decode_shard_frame(&self, mut data: Bytes) -> Result<PacketShard> {
        if data.len() < 48 {
            // Minimum metadata size
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
        use crate::protocol::frames::{FalconSigFrame, StoqFrame};

        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            let exported = falcon_guard.export_signature(signature);

            let frame = StoqFrame::FalconSignature(FalconSigFrame {
                signature_data: exported,
                key_id: self.falcon_key_id.clone(),
                signed_frames: vec![frame_types::STOQ_TOKEN],
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
                debug!(
                    "Received STOQ shard: {}/{}",
                    shard_frame.shard.sequence + 1,
                    shard_frame.shard.total_shards
                );

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

    /// Build the HyperMesh Proof-of-State extension header as a CLEARTEXT
    /// prefix for `data` (papers/HYPERMESH.md §5.1-5.7).
    ///
    /// Produces the exact 44-byte on-wire sequence the kernel XDP program
    /// parses: a 4-byte common header `[magic 0x484D BE][type=0x01][len=40]`
    /// followed by the 40-byte [`WirePosHeader`] (algorithm@0, difficulty@4,
    /// hash@8). The `hash` is the packet's BLAKE3 work hash — the PoS token
    /// the node already computes on the send path — and the algorithm is
    /// FALCON-1024 (the HyperMesh default). This byte sequence is
    /// byte-identical to the C `struct hmesh_header` + `struct hmesh_pos_header`.
    ///
    /// This is the prefix that must lead the UDP payload so XDP sees
    /// `magic == 0x484D` at wire speed. On the AF_XDP raw-send path it is
    /// physically prepended ahead of the encrypted body (see
    /// `StoqTransport::send`); it is also emitted as the leading extension
    /// blob by [`apply_extensions`].
    pub fn hypermesh_pos_prefix(&self, data: &[u8]) -> Vec<u8> {
        // The per-packet BLAKE3 token is the node's work hash for this packet.
        let token = self.extensions.tokenize_packet(data);
        let pos = WirePosHeader::from_pos_hash(token.hash, KERNEL_POS_DIFFICULTY);
        encode_pos_extension(&pos)
    }

    /// Apply STOQ extensions to outgoing data.
    ///
    /// The returned frames lead with the HyperMesh cleartext extension header
    /// (§5.1) so it is emitted ahead of STOQ's own token/shard frames, then
    /// the STOQ token frame, then any shard frames.
    pub fn apply_extensions(&self, data: &[u8]) -> Result<Vec<Bytes>> {
        let mut frames = Vec::new();

        if self.extensions_enabled {
            // HyperMesh cleartext extension header FIRST (§5.1). On the AF_XDP
            // raw path this rides as the plaintext prefix ahead of the payload.
            frames.push(Bytes::from(self.hypermesh_pos_prefix(data)));

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

    // validate_token, update_connection_state, store_shard_for_reassembly,
    // verify_falcon_signature are implemented in handler_ops module.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::DefaultStoqExtensions;
    use crate::protocol::frames;
    use hypermesh_ebpf::{WireExtHeader, WIRE_HDR_MAGIC, WIRE_HDR_POS, WIRE_HDR_POS_ALG_FALCON};

    #[test]
    fn test_hypermesh_pos_prefix_matches_c_wire_layout() {
        // The emit side (apply_extensions / hypermesh_pos_prefix) must produce
        // the EXACT 44-byte on-wire sequence the C XDP program parses:
        //   [magic 0x484D BE][type 0x01][len 40] + hmesh_pos_header(40).
        // This is the load-bearing byte-identity contract.
        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 1400);

        let data = b"hypermesh packet payload";
        let prefix = handler.hypermesh_pos_prefix(data);
        assert_eq!(prefix.len(), 44, "4-byte common header + 40-byte PoS payload");

        // Common header at offset 0: magic in network byte order (0x48 0x4D).
        assert_eq!(prefix[0], 0x48);
        assert_eq!(prefix[1], 0x4D);
        assert_eq!(u16::from_be_bytes([prefix[0], prefix[1]]), WIRE_HDR_MAGIC);
        assert_eq!(prefix[2], WIRE_HDR_POS);
        assert_eq!(prefix[3], 40);

        // PoS payload at offset 4: algorithm @0 (=FALCON), hash @8..40.
        assert_eq!(prefix[4], WIRE_HDR_POS_ALG_FALCON);
        // The carried work hash equals the packet's own BLAKE3 token hash.
        let token = extensions.tokenize_packet(data);
        assert_eq!(&prefix[12..44], &token.hash);

        // The common header parses back through the eBPF crate's C-offset reader.
        let common = WireExtHeader::from_bytes(&prefix[0..4]).expect("test: common header");
        assert_eq!(common.ext_type, WIRE_HDR_POS);
        assert_eq!(common.length, 40);
    }

    #[test]
    fn test_apply_extensions_leads_with_hypermesh_header() {
        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 1400);

        let data = b"lead frame is the hypermesh header";
        let frames = handler.apply_extensions(data).expect("test: apply_extensions");
        assert!(!frames.is_empty());
        // The FIRST frame is the HyperMesh cleartext extension header.
        let lead = &frames[0];
        assert_eq!(u16::from_be_bytes([lead[0], lead[1]]), WIRE_HDR_MAGIC);
        assert_eq!(lead[2], WIRE_HDR_POS);
    }

    #[test]
    fn test_token_frame_encoding() {
        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 1400);

        let data = b"test data";
        let token = extensions.tokenize_packet(data);

        let encoded = handler.encode_token_frame(&token).expect("test: expected success");
        assert!(!encoded.is_empty());

        // Decode the entire frame
        let decoded_frame = frames::StoqFrame::decode(encoded).expect("test: expected success");
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
        let shards = extensions.shard_packet(data, 10).expect("test: shard operation");

        for shard in shards {
            let encoded = handler.encode_shard_frame(&shard).expect("test: shard operation");
            assert!(!encoded.is_empty());

            // Decode the entire frame
            let decoded_frame = frames::StoqFrame::decode(encoded).expect("test: expected success");
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
