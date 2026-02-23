// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Custom QUIC Frames
//!
//! This module defines custom QUIC frame types for STOQ protocol extensions
//! including tokenization, sharding, and FALCON signatures.

use bytes::{Bytes, BytesMut};
use quinn::VarInt;
use anyhow::{Result, anyhow};
use tracing::trace;

use crate::extensions::{PacketToken, PacketShard, HopInfo, SeedInfo};

// Re-export varint utilities (public API)
pub use super::frame_codec::{encode_varint, decode_varint};

use super::frame_codec::{
    encode_token_frame, encode_shard_frame, encode_hop_frame,
    encode_seed_frame, encode_falcon_sig_frame, encode_falcon_key_frame,
    decode_token_frame, decode_shard_frame, decode_hop_frame,
    decode_seed_frame, decode_falcon_sig_frame, decode_falcon_key_frame,
};

/// STOQ frame type enum
#[derive(Debug, Clone)]
pub enum StoqFrame {
    /// Token frame for packet validation
    Token(TokenFrame),

    /// Shard frame for packet fragmentation
    Shard(ShardFrame),

    /// Hop frame for routing information
    Hop(HopFrame),

    /// Seed frame for distribution info
    Seed(SeedFrame),

    /// FALCON signature frame
    FalconSignature(FalconSigFrame),

    /// FALCON public key frame
    FalconKey(FalconKeyFrame),

    /// Unknown frame type (for forward compatibility)
    Unknown { frame_type: VarInt, data: Bytes },
}

/// Token frame structure
#[derive(Debug, Clone)]
pub struct TokenFrame {
    pub token: PacketToken,
    pub stream_id: Option<VarInt>,
}

/// Shard frame structure
#[derive(Debug, Clone)]
pub struct ShardFrame {
    pub shard: PacketShard,
    pub stream_id: Option<VarInt>,
}

/// Hop frame structure
#[derive(Debug, Clone)]
pub struct HopFrame {
    pub hop: HopInfo,
    pub hop_count: u8,
    pub max_hops: u8,
}

/// Seed frame structure
#[derive(Debug, Clone)]
pub struct SeedFrame {
    pub seed_info: SeedInfo,
    pub packet_id: [u8; 32],
}

/// FALCON signature frame
#[derive(Debug, Clone)]
pub struct FalconSigFrame {
    pub signature_data: Vec<u8>,
    pub key_id: String,
    pub signed_frames: Vec<VarInt>, // Frame types that were signed
}

/// FALCON public key frame
#[derive(Debug, Clone)]
pub struct FalconKeyFrame {
    pub key_data: Vec<u8>,
    pub key_id: String,
    pub variant: u8, // 0 = Falcon512, 1 = Falcon1024
}

impl StoqFrame {
    /// Get the frame type identifier
    pub fn frame_type(&self) -> VarInt {
        match self {
            Self::Token(_) => super::frame_types::STOQ_TOKEN,
            Self::Shard(_) => super::frame_types::STOQ_SHARD,
            Self::Hop(_) => super::frame_types::STOQ_HOP,
            Self::Seed(_) => super::frame_types::STOQ_SEED,
            Self::FalconSignature(_) => super::frame_types::FALCON_SIG,
            Self::FalconKey(_) => super::frame_types::FALCON_KEY,
            Self::Unknown { frame_type, .. } => *frame_type,
        }
    }

    /// Encode frame to bytes
    pub fn encode(&self) -> Result<Bytes> {
        let mut buf = BytesMut::new();

        // Encode frame type
        encode_varint(&mut buf, self.frame_type());

        // Encode frame-specific data
        match self {
            Self::Token(frame) => encode_token_frame(&mut buf, frame)?,
            Self::Shard(frame) => encode_shard_frame(&mut buf, frame)?,
            Self::Hop(frame) => encode_hop_frame(&mut buf, frame)?,
            Self::Seed(frame) => encode_seed_frame(&mut buf, frame)?,
            Self::FalconSignature(frame) => encode_falcon_sig_frame(&mut buf, frame)?,
            Self::FalconKey(frame) => encode_falcon_key_frame(&mut buf, frame)?,
            Self::Unknown { data, .. } => {
                use bytes::BufMut;
                buf.put_slice(data);
            }
        }

        trace!("Encoded STOQ frame: type={:?}, size={}", self.frame_type(), buf.len());
        Ok(buf.freeze())
    }

    /// Decode frame from bytes
    pub fn decode(mut data: Bytes) -> Result<Self> {
        if data.is_empty() {
            return Err(anyhow!("Empty frame data"));
        }

        let frame_type = decode_varint(&mut data)
            .ok_or_else(|| anyhow!("Failed to decode frame type"))?;

        match frame_type {
            super::frame_types::STOQ_TOKEN => {
                Ok(Self::Token(decode_token_frame(&mut data)?))
            }
            super::frame_types::STOQ_SHARD => {
                Ok(Self::Shard(decode_shard_frame(&mut data)?))
            }
            super::frame_types::STOQ_HOP => {
                Ok(Self::Hop(decode_hop_frame(&mut data)?))
            }
            super::frame_types::STOQ_SEED => {
                Ok(Self::Seed(decode_seed_frame(&mut data)?))
            }
            super::frame_types::FALCON_SIG => {
                Ok(Self::FalconSignature(decode_falcon_sig_frame(&mut data)?))
            }
            super::frame_types::FALCON_KEY => {
                Ok(Self::FalconKey(decode_falcon_key_frame(&mut data)?))
            }
            _ => {
                trace!("Unknown frame type: {:?}", frame_type);
                Ok(Self::Unknown { frame_type, data })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_frame() {
        let frame = TokenFrame {
            token: PacketToken {
                hash: [1; 32],
                sequence: 12345,
                timestamp: 67890,
            },
            stream_id: Some(VarInt::from_u32(42)),
        };

        let stoq_frame = StoqFrame::Token(frame.clone());
        let encoded = stoq_frame.encode().unwrap();
        let decoded = StoqFrame::decode(encoded).unwrap();

        if let StoqFrame::Token(decoded_frame) = decoded {
            assert_eq!(decoded_frame.token.hash, frame.token.hash);
            assert_eq!(decoded_frame.token.sequence, frame.token.sequence);
            assert_eq!(decoded_frame.token.timestamp, frame.token.timestamp);
            assert_eq!(decoded_frame.stream_id, frame.stream_id);
        } else {
            panic!("Wrong frame type decoded");
        }
    }

    #[test]
    fn test_shard_frame() {
        let frame = ShardFrame {
            shard: PacketShard {
                shard_id: 123,
                total_shards: 10,
                sequence: 3,
                data: Bytes::from_static(b"test shard data"),
                packet_hash: [2; 32],
            },
            stream_id: None,
        };

        let stoq_frame = StoqFrame::Shard(frame.clone());
        let encoded = stoq_frame.encode().unwrap();
        let decoded = StoqFrame::decode(encoded).unwrap();

        if let StoqFrame::Shard(decoded_frame) = decoded {
            assert_eq!(decoded_frame.shard.shard_id, frame.shard.shard_id);
            assert_eq!(decoded_frame.shard.total_shards, frame.shard.total_shards);
            assert_eq!(decoded_frame.shard.sequence, frame.shard.sequence);
            assert_eq!(decoded_frame.shard.data, frame.shard.data);
            assert_eq!(decoded_frame.shard.packet_hash, frame.shard.packet_hash);
            assert_eq!(decoded_frame.stream_id, frame.stream_id);
        } else {
            panic!("Wrong frame type decoded");
        }
    }
}
