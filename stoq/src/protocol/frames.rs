//! STOQ Custom QUIC Frames
//!
//! This module defines custom QUIC frame types for STOQ protocol extensions
//! including tokenization, sharding, and FALCON signatures.

use bytes::{Bytes, BytesMut, BufMut, Buf};
use quinn::VarInt;
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use tracing::trace;

use crate::extensions::{PacketToken, PacketShard, HopInfo, SeedInfo, SeedNode, SeedPriority};

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
            Self::Unknown { data, .. } => buf.put_slice(data),
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

// Frame encoding functions

fn encode_token_frame(buf: &mut BytesMut, frame: &TokenFrame) -> Result<()> {
    // Encode token
    buf.put_slice(&frame.token.hash);
    buf.put_u64(frame.token.sequence);
    buf.put_u64(frame.token.timestamp);

    // Encode optional stream ID
    if let Some(stream_id) = frame.stream_id {
        buf.put_u8(1); // Has stream ID
        encode_varint(buf, stream_id);
    } else {
        buf.put_u8(0); // No stream ID
    }

    Ok(())
}

fn encode_shard_frame(buf: &mut BytesMut, frame: &ShardFrame) -> Result<()> {
    // Encode shard metadata
    buf.put_u32(frame.shard.shard_id);
    buf.put_u32(frame.shard.total_shards);
    buf.put_u32(frame.shard.sequence);
    buf.put_slice(&frame.shard.packet_hash);

    // Encode shard data
    buf.put_u32(frame.shard.data.len() as u32);
    buf.put_slice(&frame.shard.data);

    // Encode optional stream ID
    if let Some(stream_id) = frame.stream_id {
        buf.put_u8(1); // Has stream ID
        encode_varint(buf, stream_id);
    } else {
        buf.put_u8(0); // No stream ID
    }

    Ok(())
}

fn encode_hop_frame(buf: &mut BytesMut, frame: &HopFrame) -> Result<()> {
    // Encode hop info
    buf.put_slice(&frame.hop.address.octets());
    buf.put_u16(frame.hop.port);
    buf.put_u64(frame.hop.timestamp);

    // Encode hop metadata
    buf.put_u32(frame.hop.metadata.len() as u32);
    for (key, value) in &frame.hop.metadata {
        buf.put_u32(key.len() as u32);
        buf.put_slice(key.as_bytes());
        buf.put_u32(value.len() as u32);
        buf.put_slice(value.as_bytes());
    }

    // Encode hop count info
    buf.put_u8(frame.hop_count);
    buf.put_u8(frame.max_hops);

    Ok(())
}

fn encode_seed_frame(buf: &mut BytesMut, frame: &SeedFrame) -> Result<()> {
    // Encode packet ID
    buf.put_slice(&frame.packet_id);

    // Encode seed nodes
    buf.put_u32(frame.seed_info.seed_nodes.len() as u32);
    for node in &frame.seed_info.seed_nodes {
        buf.put_slice(&node.address.octets());
        buf.put_u16(node.port);
        buf.put_u8(node.reliability);
    }

    // Encode replication factor
    buf.put_u32(frame.seed_info.replication_factor);

    // Encode priority
    buf.put_u8(match frame.seed_info.priority {
        SeedPriority::Low => 0,
        SeedPriority::Normal => 1,
        SeedPriority::High => 2,
        SeedPriority::Critical => 3,
    });

    Ok(())
}

fn encode_falcon_sig_frame(buf: &mut BytesMut, frame: &FalconSigFrame) -> Result<()> {
    // Encode key ID
    buf.put_u32(frame.key_id.len() as u32);
    buf.put_slice(frame.key_id.as_bytes());

    // Encode signature data
    buf.put_u32(frame.signature_data.len() as u32);
    buf.put_slice(&frame.signature_data);

    // Encode signed frame types
    buf.put_u32(frame.signed_frames.len() as u32);
    for frame_type in &frame.signed_frames {
        encode_varint(buf, *frame_type);
    }

    Ok(())
}

fn encode_falcon_key_frame(buf: &mut BytesMut, frame: &FalconKeyFrame) -> Result<()> {
    // Encode key ID
    buf.put_u32(frame.key_id.len() as u32);
    buf.put_slice(frame.key_id.as_bytes());

    // Encode variant
    buf.put_u8(frame.variant);

    // Encode key data
    buf.put_u32(frame.key_data.len() as u32);
    buf.put_slice(&frame.key_data);

    Ok(())
}

// Frame decoding functions

fn decode_token_frame(data: &mut Bytes) -> Result<TokenFrame> {
    if data.len() < 48 + 1 { // Hash + sequence + timestamp + stream flag
        return Err(anyhow!("Token frame too short"));
    }

    let mut hash = [0u8; 32];
    data.copy_to_slice(&mut hash);
    let sequence = data.get_u64();
    let timestamp = data.get_u64();

    let stream_id = if data.get_u8() == 1 {
        Some(decode_varint(data).ok_or_else(|| anyhow!("Failed to decode stream ID"))?)
    } else {
        None
    };

    Ok(TokenFrame {
        token: PacketToken {
            hash,
            sequence,
            timestamp,
        },
        stream_id,
    })
}

fn decode_shard_frame(data: &mut Bytes) -> Result<ShardFrame> {
    if data.len() < 44 + 4 + 1 { // Metadata + length + stream flag
        return Err(anyhow!("Shard frame too short"));
    }

    let shard_id = data.get_u32();
    let total_shards = data.get_u32();
    let sequence = data.get_u32();

    let mut packet_hash = [0u8; 32];
    data.copy_to_slice(&mut packet_hash);

    let data_len = data.get_u32() as usize;
    if data.len() < data_len + 1 {
        return Err(anyhow!("Shard data truncated"));
    }

    let shard_data = data.split_to(data_len);

    let stream_id = if data.get_u8() == 1 {
        Some(decode_varint(data).ok_or_else(|| anyhow!("Failed to decode stream ID"))?)
    } else {
        None
    };

    Ok(ShardFrame {
        shard: PacketShard {
            shard_id,
            total_shards,
            sequence,
            data: shard_data,
            packet_hash,
        },
        stream_id,
    })
}

fn decode_hop_frame(data: &mut Bytes) -> Result<HopFrame> {
    if data.len() < 16 + 2 + 8 + 4 + 2 { // Address + port + timestamp + metadata len + hop counts
        return Err(anyhow!("Hop frame too short"));
    }

    let mut addr_bytes = [0u8; 16];
    data.copy_to_slice(&mut addr_bytes);
    let address = std::net::Ipv6Addr::from(addr_bytes);
    let port = data.get_u16();
    let timestamp = data.get_u64();

    let metadata_len = data.get_u32() as usize;
    let mut metadata = HashMap::new();

    for _ in 0..metadata_len {
        if data.len() < 8 { // Two length fields
            return Err(anyhow!("Hop metadata truncated"));
        }

        let key_len = data.get_u32() as usize;
        if data.len() < key_len {
            return Err(anyhow!("Hop metadata key truncated"));
        }
        let key = String::from_utf8_lossy(&data.split_to(key_len)).to_string();

        let value_len = data.get_u32() as usize;
        if data.len() < value_len {
            return Err(anyhow!("Hop metadata value truncated"));
        }
        let value = String::from_utf8_lossy(&data.split_to(value_len)).to_string();

        metadata.insert(key, value);
    }

    if data.len() < 2 {
        return Err(anyhow!("Hop counts missing"));
    }

    let hop_count = data.get_u8();
    let max_hops = data.get_u8();

    Ok(HopFrame {
        hop: HopInfo {
            address,
            port,
            timestamp,
            metadata,
        },
        hop_count,
        max_hops,
    })
}

fn decode_seed_frame(data: &mut Bytes) -> Result<SeedFrame> {
    if data.len() < 32 + 4 { // Packet ID + nodes length
        return Err(anyhow!("Seed frame too short"));
    }

    let mut packet_id = [0u8; 32];
    data.copy_to_slice(&mut packet_id);

    let nodes_len = data.get_u32() as usize;
    let mut seed_nodes = Vec::with_capacity(nodes_len);

    for _ in 0..nodes_len {
        if data.len() < 16 + 2 + 1 { // Address + port + reliability
            return Err(anyhow!("Seed node data truncated"));
        }

        let mut addr_bytes = [0u8; 16];
        data.copy_to_slice(&mut addr_bytes);
        let address = std::net::Ipv6Addr::from(addr_bytes);
        let port = data.get_u16();
        let reliability = data.get_u8();

        seed_nodes.push(SeedNode {
            address,
            port,
            reliability,
        });
    }

    if data.len() < 4 + 1 { // Replication factor + priority
        return Err(anyhow!("Seed info truncated"));
    }

    let replication_factor = data.get_u32();
    let priority = match data.get_u8() {
        0 => SeedPriority::Low,
        1 => SeedPriority::Normal,
        2 => SeedPriority::High,
        3 => SeedPriority::Critical,
        _ => SeedPriority::Normal,
    };

    Ok(SeedFrame {
        seed_info: SeedInfo {
            seed_nodes,
            replication_factor,
            priority,
        },
        packet_id,
    })
}

fn decode_falcon_sig_frame(data: &mut Bytes) -> Result<FalconSigFrame> {
    if data.len() < 4 {
        return Err(anyhow!("FALCON signature frame too short"));
    }

    let key_id_len = data.get_u32() as usize;
    if data.len() < key_id_len + 4 {
        return Err(anyhow!("Key ID truncated"));
    }
    let key_id = String::from_utf8_lossy(&data.split_to(key_id_len)).to_string();

    let sig_len = data.get_u32() as usize;
    if data.len() < sig_len + 4 {
        return Err(anyhow!("Signature data truncated"));
    }
    let signature_data = data.split_to(sig_len).to_vec();

    let frames_len = data.get_u32() as usize;
    let mut signed_frames = Vec::with_capacity(frames_len);

    for _ in 0..frames_len {
        signed_frames.push(
            decode_varint(data).ok_or_else(|| anyhow!("Failed to decode signed frame type"))?
        );
    }

    Ok(FalconSigFrame {
        key_id,
        signature_data,
        signed_frames,
    })
}

fn decode_falcon_key_frame(data: &mut Bytes) -> Result<FalconKeyFrame> {
    if data.len() < 4 {
        return Err(anyhow!("FALCON key frame too short"));
    }

    let key_id_len = data.get_u32() as usize;
    if data.len() < key_id_len + 1 + 4 {
        return Err(anyhow!("Key ID truncated"));
    }
    let key_id = String::from_utf8_lossy(&data.split_to(key_id_len)).to_string();

    let variant = data.get_u8();

    let key_len = data.get_u32() as usize;
    if data.len() < key_len {
        return Err(anyhow!("Key data truncated"));
    }
    let key_data = data.split_to(key_len).to_vec();

    Ok(FalconKeyFrame {
        key_id,
        variant,
        key_data,
    })
}

// Varint encoding/decoding utilities

/// Encode a variable-length integer (QUIC varints)
pub fn encode_varint(buf: &mut BytesMut, val: VarInt) {
    let val = val.into_inner();
    if val < 0x40 {
        buf.put_u8(val as u8);
    } else if val < 0x4000 {
        buf.put_u16(0x4000 | val as u16);
    } else if val < 0x40000000 {
        buf.put_u32(0x80000000 | val as u32);
    } else {
        buf.put_u64(0xc000000000000000 | val);
    }
}

/// Decode a variable-length integer
pub fn decode_varint(buf: &mut impl Buf) -> Option<VarInt> {
    if !buf.has_remaining() {
        return None;
    }

    let first = buf.get_u8();
    let val = match first >> 6 {
        0b00 => u64::from(first & 0x3f),
        0b01 => {
            if !buf.has_remaining() {
                return None;
            }
            u64::from(first & 0x3f) << 8 | u64::from(buf.get_u8())
        }
        0b10 => {
            if buf.remaining() < 3 {
                return None;
            }
            u64::from(first & 0x3f) << 24
                | u64::from(buf.get_u8()) << 16
                | u64::from(buf.get_u8()) << 8
                | u64::from(buf.get_u8())
        }
        0b11 => {
            if buf.remaining() < 7 {
                return None;
            }
            u64::from(first & 0x3f) << 56
                | u64::from(buf.get_u8()) << 48
                | u64::from(buf.get_u8()) << 40
                | u64::from(buf.get_u8()) << 32
                | u64::from(buf.get_u8()) << 24
                | u64::from(buf.get_u8()) << 16
                | u64::from(buf.get_u8()) << 8
                | u64::from(buf.get_u8())
        }
        _ => unreachable!(),
    };

    VarInt::from_u64(val).ok()
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