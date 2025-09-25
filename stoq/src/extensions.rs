//! STOQ Protocol Extensions - Pure protocol-level features
//!
//! This module provides the core protocol extensions that make STOQ more than just
//! a QUIC wrapper. These are protocol-level features, not application features.
//!
//! Extensions included:
//! - Packet tokenization and validation
//! - Packet sharding support
//! - Multi-hop routing protocol
//! - Seeding and mirroring protocol

use bytes::{Bytes, BytesMut, BufMut};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use anyhow::{Result, anyhow};

/// STOQ protocol extension trait - defines the core protocol enhancements
pub trait StoqProtocolExtension {
    /// Generate a cryptographic token for a packet
    fn tokenize_packet(&self, data: &[u8]) -> PacketToken;

    /// Validate a packet token
    fn validate_token(&self, data: &[u8], token: &PacketToken) -> bool;

    /// Shard a packet into multiple fragments for transport
    fn shard_packet(&self, data: &[u8], max_shard_size: usize) -> Result<Vec<PacketShard>>;

    /// Reassemble packet shards back into original data
    fn reassemble_shards(&self, shards: Vec<PacketShard>) -> Result<Bytes>;

    /// Add hop information to packet for routing
    fn add_hop_info(&self, packet: &mut StoqPacket, hop: HopInfo) -> Result<()>;

    /// Get seeding information for packet distribution
    fn get_seed_info(&self, packet: &StoqPacket) -> Option<SeedInfo>;
}

/// Cryptographic token for packet validation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PacketToken {
    /// SHA-256 hash of packet data
    pub hash: [u8; 32],
    /// Packet sequence number
    pub sequence: u64,
    /// Token generation timestamp (Unix epoch)
    pub timestamp: u64,
}

impl PacketToken {
    /// Create a new packet token
    pub fn new(data: &[u8], sequence: u64) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize().into();

        Self {
            hash,
            sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Validate token against data
    pub fn validate(&self, data: &[u8]) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let computed_hash: [u8; 32] = hasher.finalize().into();
        computed_hash == self.hash
    }
}

/// Packet shard for fragmented transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketShard {
    /// Unique shard identifier
    pub shard_id: u32,
    /// Total number of shards for this packet
    pub total_shards: u32,
    /// Shard sequence number (0-based)
    pub sequence: u32,
    /// Actual shard data
    pub data: Bytes,
    /// Hash of the original complete packet
    pub packet_hash: [u8; 32],
}

/// Hop information for multi-hop routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopInfo {
    /// IPv6 address of the hop
    pub address: std::net::Ipv6Addr,
    /// Port number
    pub port: u16,
    /// Hop timestamp
    pub timestamp: u64,
    /// Optional hop metadata
    pub metadata: HashMap<String, String>,
}

/// Seeding information for packet distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedInfo {
    /// Seed nodes for packet distribution
    pub seed_nodes: Vec<SeedNode>,
    /// Replication factor
    pub replication_factor: u32,
    /// Packet priority for seeding
    pub priority: SeedPriority,
}

/// Seed node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedNode {
    /// IPv6 address of seed node
    pub address: std::net::Ipv6Addr,
    /// Port number
    pub port: u16,
    /// Node reliability score (0-100)
    pub reliability: u8,
}

/// Seeding priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeedPriority {
    /// Low priority - best effort seeding
    Low,
    /// Normal priority - standard seeding
    Normal,
    /// High priority - prioritized seeding
    High,
    /// Critical priority - immediate seeding required
    Critical,
}

/// Enhanced STOQ packet with protocol extension support
#[derive(Debug, Clone)]
pub struct StoqPacket {
    /// Original packet data
    pub data: Bytes,
    /// Packet token for validation
    pub token: Option<PacketToken>,
    /// Hop chain for routing
    pub hops: Vec<HopInfo>,
    /// Seeding information
    pub seed_info: Option<SeedInfo>,
    /// Protocol extension metadata
    pub metadata: HashMap<String, String>,
}

impl StoqPacket {
    /// Create a new STOQ packet
    pub fn new(data: Bytes) -> Self {
        Self {
            data,
            token: None,
            hops: Vec::new(),
            seed_info: None,
            metadata: HashMap::new(),
        }
    }

    /// Serialize packet for transmission
    pub fn serialize(&self) -> Result<Bytes> {
        let mut buffer = BytesMut::new();

        // Write packet header
        buffer.put_u32(self.data.len() as u32);

        // Write token if present
        if let Some(token) = &self.token {
            buffer.put_u8(1); // Token present flag
            let token_bytes = bincode::serialize(token)?;
            buffer.put_u32(token_bytes.len() as u32);
            buffer.put_slice(&token_bytes);
        } else {
            buffer.put_u8(0); // No token flag
        }

        // Write hop count and hops
        buffer.put_u32(self.hops.len() as u32);
        for hop in &self.hops {
            let hop_bytes = bincode::serialize(hop)?;
            buffer.put_u32(hop_bytes.len() as u32);
            buffer.put_slice(&hop_bytes);
        }

        // Write seed info if present
        if let Some(seed_info) = &self.seed_info {
            buffer.put_u8(1); // Seed info present flag
            let seed_bytes = bincode::serialize(seed_info)?;
            buffer.put_u32(seed_bytes.len() as u32);
            buffer.put_slice(&seed_bytes);
        } else {
            buffer.put_u8(0); // No seed info flag
        }

        // Write metadata count and metadata
        buffer.put_u32(self.metadata.len() as u32);
        for (key, value) in &self.metadata {
            buffer.put_u32(key.len() as u32);
            buffer.put_slice(key.as_bytes());
            buffer.put_u32(value.len() as u32);
            buffer.put_slice(value.as_bytes());
        }

        // Write actual packet data
        buffer.put_slice(&self.data);

        Ok(buffer.freeze())
    }
}

/// Default implementation of STOQ protocol extensions
pub struct DefaultStoqExtensions {
    /// Packet sequence counter
    sequence_counter: std::sync::atomic::AtomicU64,
}

impl DefaultStoqExtensions {
    /// Create a new default extensions instance
    pub fn new() -> Self {
        Self {
            sequence_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

impl Default for DefaultStoqExtensions {
    fn default() -> Self {
        Self::new()
    }
}

impl StoqProtocolExtension for DefaultStoqExtensions {
    fn tokenize_packet(&self, data: &[u8]) -> PacketToken {
        let sequence = self.sequence_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        PacketToken::new(data, sequence)
    }

    fn validate_token(&self, data: &[u8], token: &PacketToken) -> bool {
        token.validate(data)
    }

    fn shard_packet(&self, data: &[u8], max_shard_size: usize) -> Result<Vec<PacketShard>> {
        if max_shard_size == 0 {
            return Err(anyhow!("Maximum shard size must be greater than 0"));
        }

        let mut hasher = Sha256::new();
        hasher.update(data);
        let packet_hash = hasher.finalize().into();

        let total_shards = (data.len() + max_shard_size - 1) / max_shard_size;
        let shard_id = rand::random::<u32>();

        let mut shards = Vec::new();
        for (i, chunk) in data.chunks(max_shard_size).enumerate() {
            shards.push(PacketShard {
                shard_id,
                total_shards: total_shards as u32,
                sequence: i as u32,
                data: Bytes::copy_from_slice(chunk),
                packet_hash,
            });
        }

        Ok(shards)
    }

    fn reassemble_shards(&self, mut shards: Vec<PacketShard>) -> Result<Bytes> {
        if shards.is_empty() {
            return Err(anyhow!("No shards to reassemble"));
        }

        // Validate all shards have same shard_id and packet_hash
        let shard_id = shards[0].shard_id;
        let packet_hash = shards[0].packet_hash;
        let total_shards = shards[0].total_shards;

        for shard in &shards {
            if shard.shard_id != shard_id {
                return Err(anyhow!("Mismatched shard IDs"));
            }
            if shard.packet_hash != packet_hash {
                return Err(anyhow!("Mismatched packet hashes"));
            }
            if shard.total_shards != total_shards {
                return Err(anyhow!("Mismatched total shard counts"));
            }
        }

        // Sort shards by sequence number
        shards.sort_by_key(|s| s.sequence);

        // Verify we have all shards
        if shards.len() != total_shards as usize {
            return Err(anyhow!("Missing shards: expected {}, got {}", total_shards, shards.len()));
        }

        for (i, shard) in shards.iter().enumerate() {
            if shard.sequence != i as u32 {
                return Err(anyhow!("Missing shard sequence {}", i));
            }
        }

        // Reassemble data
        let mut reassembled = BytesMut::new();
        for shard in shards {
            reassembled.put_slice(&shard.data);
        }

        let result = reassembled.freeze();

        // Validate reassembled data
        let mut hasher = Sha256::new();
        hasher.update(&result);
        let computed_hash: [u8; 32] = hasher.finalize().into();

        if computed_hash != packet_hash {
            return Err(anyhow!("Reassembled data hash mismatch"));
        }

        Ok(result)
    }

    fn add_hop_info(&self, packet: &mut StoqPacket, hop: HopInfo) -> Result<()> {
        packet.hops.push(hop);
        Ok(())
    }

    fn get_seed_info(&self, packet: &StoqPacket) -> Option<SeedInfo> {
        packet.seed_info.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_token_validation() {
        let data = b"test packet data";
        let token = PacketToken::new(data, 1);

        assert!(token.validate(data));
        assert!(!token.validate(b"different data"));
    }

    #[test]
    fn test_packet_sharding_and_reassembly() {
        let extensions = DefaultStoqExtensions::new();
        let data = b"this is a test packet that will be sharded into multiple pieces";
        let max_shard_size = 10;

        let shards = extensions.shard_packet(data, max_shard_size).unwrap();
        assert!(shards.len() > 1);

        let reassembled = extensions.reassemble_shards(shards).unwrap();
        assert_eq!(reassembled.as_ref(), data);
    }

    #[test]
    fn test_hop_info_addition() {
        let extensions = DefaultStoqExtensions::new();
        let mut packet = StoqPacket::new(Bytes::from_static(b"test"));

        let hop = HopInfo {
            address: std::net::Ipv6Addr::LOCALHOST,
            port: 9292,
            timestamp: 12345,
            metadata: HashMap::new(),
        };

        extensions.add_hop_info(&mut packet, hop).unwrap();
        assert_eq!(packet.hops.len(), 1);
    }

    #[test]
    fn test_packet_serialization() {
        let mut packet = StoqPacket::new(Bytes::from_static(b"test data"));
        packet.token = Some(PacketToken::new(b"test data", 1));
        packet.metadata.insert("key1".to_string(), "value1".to_string());

        let serialized = packet.serialize().unwrap();
        assert!(!serialized.is_empty());
    }
}