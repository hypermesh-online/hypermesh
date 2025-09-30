//! Content Addressing System for P2P Distribution
//!
//! Provides content-addressed storage with Merkle trees for integrity verification

use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

use crate::assets::{AssetPackage, AssetPackageId};

/// Content address (SHA-256 hash)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentAddress {
    hash: [u8; 32],
}

impl ContentAddress {
    /// Create content address from data
    pub fn from_data(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result[..32]);
        Self { hash }
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.hash)
    }

    /// Create from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)
            .context("Invalid hex string")?;

        if bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid hash length"));
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(Self { hash })
    }

    /// Get the underlying bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }
}

impl std::fmt::Display for ContentAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.to_hex()[..8])
    }
}

/// Merkle tree for package integrity verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Root hash of the tree
    pub root: ContentAddress,
    /// Tree nodes (level -> nodes at that level)
    nodes: Vec<Vec<ContentAddress>>,
    /// Leaf nodes (chunk hashes)
    leaves: Vec<ContentAddress>,
}

impl MerkleTree {
    /// Create a Merkle tree from package chunks
    pub fn from_chunks(chunks: &[Vec<u8>]) -> Result<Self> {
        if chunks.is_empty() {
            return Err(anyhow::anyhow!("Cannot create Merkle tree from empty chunks"));
        }

        // Create leaf nodes from chunk hashes
        let mut leaves = Vec::new();
        for chunk in chunks {
            leaves.push(ContentAddress::from_data(chunk));
        }

        // Build tree bottom-up
        let mut nodes = vec![leaves.clone()];
        let mut current_level = leaves.clone();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            // Pair up nodes and hash them
            for i in (0..current_level.len()).step_by(2) {
                let left = &current_level[i];

                let hash = if i + 1 < current_level.len() {
                    // Combine with right node
                    let right = &current_level[i + 1];
                    Self::hash_pair(left, right)
                } else {
                    // Odd number of nodes, duplicate the last one
                    Self::hash_pair(left, left)
                };

                next_level.push(hash);
            }

            nodes.push(next_level.clone());
            current_level = next_level;
        }

        Ok(Self {
            root: current_level[0].clone(),
            nodes,
            leaves: leaves.clone(),
        })
    }

    /// Create a Merkle tree from an asset package
    pub fn from_package(package: &AssetPackage) -> Result<Self> {
        let chunks = Self::package_to_chunks(package)?;
        Self::from_chunks(&chunks)
    }

    /// Hash two nodes together
    fn hash_pair(left: &ContentAddress, right: &ContentAddress) -> ContentAddress {
        let mut hasher = Sha256::new();
        hasher.update(left.as_bytes());
        hasher.update(right.as_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result[..32]);
        ContentAddress { hash }
    }

    /// Convert package to chunks
    fn package_to_chunks(package: &AssetPackage) -> Result<Vec<Vec<u8>>> {
        let mut chunks = Vec::new();
        const CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks

        // Serialize package
        let package_data = bincode::serialize(package)?;

        // Split into chunks
        for chunk in package_data.chunks(CHUNK_SIZE) {
            chunks.push(chunk.to_vec());
        }

        Ok(chunks)
    }

    /// Verify a chunk against the tree
    pub fn verify_chunk(&self, chunk_index: usize, chunk_data: &[u8]) -> Result<bool> {
        if chunk_index >= self.leaves.len() {
            return Err(anyhow::anyhow!("Chunk index out of bounds"));
        }

        let chunk_hash = ContentAddress::from_data(chunk_data);
        Ok(self.leaves[chunk_index] == chunk_hash)
    }

    /// Get proof for a chunk
    pub fn get_proof(&self, chunk_index: usize) -> Result<MerkleProof> {
        if chunk_index >= self.leaves.len() {
            return Err(anyhow::anyhow!("Chunk index out of bounds"));
        }

        let mut proof = Vec::new();
        let mut index = chunk_index;

        // Traverse up the tree
        for level in 0..self.nodes.len() - 1 {
            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };

            if sibling_index < self.nodes[level].len() {
                proof.push(ProofNode {
                    hash: self.nodes[level][sibling_index].clone(),
                    is_left: index % 2 == 1,
                });
            }

            index /= 2;
        }

        Ok(MerkleProof {
            chunk_index,
            chunk_hash: self.leaves[chunk_index].clone(),
            siblings: proof,
            root: self.root.clone(),
        })
    }

    /// Verify a Merkle proof
    pub fn verify_proof(proof: &MerkleProof, chunk_data: &[u8]) -> bool {
        let chunk_hash = ContentAddress::from_data(chunk_data);

        if chunk_hash != proof.chunk_hash {
            return false;
        }

        let mut current_hash = chunk_hash;

        for sibling in &proof.siblings {
            current_hash = if sibling.is_left {
                Self::hash_pair(&sibling.hash, &current_hash)
            } else {
                Self::hash_pair(&current_hash, &sibling.hash)
            };
        }

        current_hash == proof.root
    }

    /// Verify entire package integrity
    pub fn verify_package(&self, package: &AssetPackage) -> Result<()> {
        let chunks = Self::package_to_chunks(package)?;

        if chunks.len() != self.leaves.len() {
            return Err(anyhow::anyhow!(
                "Chunk count mismatch: expected {}, got {}",
                self.leaves.len(),
                chunks.len()
            ));
        }

        for (i, chunk) in chunks.iter().enumerate() {
            if !self.verify_chunk(i, chunk)? {
                return Err(anyhow::anyhow!("Chunk {} verification failed", i));
            }
        }

        Ok(())
    }

    /// Get the root hash
    pub fn root_hash(&self) -> &ContentAddress {
        &self.root
    }

    /// Get number of chunks
    pub fn chunk_count(&self) -> usize {
        self.leaves.len()
    }

    /// Get chunk hashes
    pub fn chunk_hashes(&self) -> &[ContentAddress] {
        &self.leaves
    }
}

/// Merkle proof for a specific chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Chunk index
    pub chunk_index: usize,
    /// Chunk hash
    pub chunk_hash: ContentAddress,
    /// Sibling nodes in the path to root
    pub siblings: Vec<ProofNode>,
    /// Root hash
    pub root: ContentAddress,
}

/// Node in a Merkle proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofNode {
    /// Hash of the sibling node
    pub hash: ContentAddress,
    /// Whether this node is on the left
    pub is_left: bool,
}

/// Content chunker for efficient distribution
pub struct ContentChunker {
    /// Chunk size in bytes
    chunk_size: usize,
    /// Compression type
    compression: CompressionType,
}

/// Compression type for chunks
#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
}

impl ContentChunker {
    /// Create a new content chunker
    pub fn new(chunk_size: usize, compression: CompressionType) -> Self {
        Self {
            chunk_size,
            compression,
        }
    }

    /// Split data into chunks
    pub fn chunk_data(&self, data: &[u8]) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();

        for (i, chunk_data) in data.chunks(self.chunk_size).enumerate() {
            let compressed_data = self.compress(chunk_data)?;

            let chunk = Chunk {
                index: i,
                data: compressed_data,
                hash: ContentAddress::from_data(chunk_data),
                size: chunk_data.len(),
                compressed_size: compressed_data.len(),
                compression: self.compression,
            };

            chunks.push(chunk);
        }

        Ok(chunks)
    }

    /// Reassemble chunks into original data
    pub fn reassemble(&self, chunks: &[Chunk]) -> Result<Vec<u8>> {
        // Sort chunks by index
        let mut sorted_chunks = chunks.to_vec();
        sorted_chunks.sort_by_key(|c| c.index);

        // Verify no missing chunks
        for (i, chunk) in sorted_chunks.iter().enumerate() {
            if chunk.index != i {
                return Err(anyhow::anyhow!("Missing chunk at index {}", i));
            }
        }

        // Reassemble data
        let mut data = Vec::new();
        for chunk in sorted_chunks {
            let decompressed = self.decompress(&chunk.data, chunk.compression)?;
            data.extend(decompressed);
        }

        Ok(data)
    }

    /// Compress data
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;

                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)?;
                Ok(encoder.finish()?)
            }
            CompressionType::Zstd => {
                Ok(zstd::encode_all(data, 3)?)
            }
            CompressionType::Lz4 => {
                Ok(lz4::block::compress(data, None, true)?)
            }
        }
    }

    /// Decompress data
    fn decompress(&self, data: &[u8], compression: CompressionType) -> Result<Vec<u8>> {
        match compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                use flate2::read::GzDecoder;
                use std::io::Read;

                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                Ok(decompressed)
            }
            CompressionType::Zstd => {
                Ok(zstd::decode_all(data)?)
            }
            CompressionType::Lz4 => {
                Ok(lz4::block::decompress(data, None)?)
            }
        }
    }
}

/// Individual chunk with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Chunk index
    pub index: usize,
    /// Chunk data (possibly compressed)
    pub data: Vec<u8>,
    /// Hash of uncompressed data
    pub hash: ContentAddress,
    /// Original size
    pub size: usize,
    /// Compressed size
    pub compressed_size: usize,
    /// Compression type used
    pub compression: CompressionType,
}

// Implement Serialize/Deserialize for CompressionType
impl Serialize for CompressionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value = match self {
            CompressionType::None => 0,
            CompressionType::Gzip => 1,
            CompressionType::Zstd => 2,
            CompressionType::Lz4 => 3,
        };
        serializer.serialize_u8(value)
    }
}

impl<'de> Deserialize<'de> for CompressionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        match value {
            0 => Ok(CompressionType::None),
            1 => Ok(CompressionType::Gzip),
            2 => Ok(CompressionType::Zstd),
            3 => Ok(CompressionType::Lz4),
            _ => Err(serde::de::Error::custom("Invalid compression type")),
        }
    }
}

/// Binary diff for incremental updates
pub struct BinaryDiff;

impl BinaryDiff {
    /// Create a diff between two versions
    pub fn create_diff(old_data: &[u8], new_data: &[u8]) -> Result<Vec<u8>> {
        // Simple implementation - can be replaced with more sophisticated algorithms
        // like xdelta or bsdiff

        // For now, just store the new data if it's significantly different
        let similarity = Self::calculate_similarity(old_data, new_data);

        if similarity > 0.8 {
            // High similarity, create a simple diff
            // Format: [operation, offset, length, data]
            let mut diff = Vec::new();

            // Find common prefix
            let common_prefix_len = old_data.iter()
                .zip(new_data.iter())
                .take_while(|(a, b)| a == b)
                .count();

            if common_prefix_len > 0 {
                // Operation: COPY
                diff.push(0u8);
                diff.extend(&(common_prefix_len as u32).to_le_bytes());
            }

            // Add remaining new data
            if common_prefix_len < new_data.len() {
                // Operation: ADD
                diff.push(1u8);
                let remaining = &new_data[common_prefix_len..];
                diff.extend(&(remaining.len() as u32).to_le_bytes());
                diff.extend(remaining);
            }

            Ok(diff)
        } else {
            // Low similarity, just store the full new data
            let mut diff = Vec::new();
            diff.push(2u8); // Operation: REPLACE
            diff.extend(&(new_data.len() as u32).to_le_bytes());
            diff.extend(new_data);
            Ok(diff)
        }
    }

    /// Apply a diff to old data to get new data
    pub fn apply_diff(old_data: &[u8], diff: &[u8]) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut offset = 0;

        while offset < diff.len() {
            let operation = diff[offset];
            offset += 1;

            match operation {
                0 => {
                    // COPY operation
                    let length = u32::from_le_bytes([
                        diff[offset],
                        diff[offset + 1],
                        diff[offset + 2],
                        diff[offset + 3],
                    ]) as usize;
                    offset += 4;

                    result.extend(&old_data[..length.min(old_data.len())]);
                }
                1 => {
                    // ADD operation
                    let length = u32::from_le_bytes([
                        diff[offset],
                        diff[offset + 1],
                        diff[offset + 2],
                        diff[offset + 3],
                    ]) as usize;
                    offset += 4;

                    result.extend(&diff[offset..offset + length]);
                    offset += length;
                }
                2 => {
                    // REPLACE operation
                    let length = u32::from_le_bytes([
                        diff[offset],
                        diff[offset + 1],
                        diff[offset + 2],
                        diff[offset + 3],
                    ]) as usize;
                    offset += 4;

                    result = diff[offset..offset + length].to_vec();
                    offset += length;
                }
                _ => {
                    return Err(anyhow::anyhow!("Invalid diff operation: {}", operation));
                }
            }
        }

        Ok(result)
    }

    /// Calculate similarity between two byte arrays (0.0 - 1.0)
    fn calculate_similarity(a: &[u8], b: &[u8]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let common_bytes = a.iter()
            .zip(b.iter())
            .filter(|(x, y)| x == y)
            .count();

        let max_len = a.len().max(b.len());
        common_bytes as f64 / max_len as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_address() {
        let data = b"Hello, World!";
        let addr1 = ContentAddress::from_data(data);
        let addr2 = ContentAddress::from_data(data);

        assert_eq!(addr1, addr2);

        let hex = addr1.to_hex();
        let addr3 = ContentAddress::from_hex(&hex).unwrap();

        assert_eq!(addr1, addr3);
    }

    #[test]
    fn test_merkle_tree() {
        let chunks = vec![
            b"chunk1".to_vec(),
            b"chunk2".to_vec(),
            b"chunk3".to_vec(),
            b"chunk4".to_vec(),
        ];

        let tree = MerkleTree::from_chunks(&chunks).unwrap();

        // Verify chunks
        for (i, chunk) in chunks.iter().enumerate() {
            assert!(tree.verify_chunk(i, chunk).unwrap());
        }

        // Test proof generation and verification
        let proof = tree.get_proof(0).unwrap();
        assert!(MerkleTree::verify_proof(&proof, &chunks[0]));
    }

    #[test]
    fn test_content_chunker() {
        let data = vec![0u8; 5000];
        let chunker = ContentChunker::new(1024, CompressionType::None);

        let chunks = chunker.chunk_data(&data).unwrap();
        assert_eq!(chunks.len(), 5); // 5000 / 1024 = 4.88 -> 5 chunks

        let reassembled = chunker.reassemble(&chunks).unwrap();
        assert_eq!(reassembled, data);
    }

    #[test]
    fn test_binary_diff() {
        let old_data = b"Hello, World!";
        let new_data = b"Hello, Rust!";

        let diff = BinaryDiff::create_diff(old_data, new_data).unwrap();
        let result = BinaryDiff::apply_diff(old_data, &diff).unwrap();

        assert_eq!(result, new_data);
    }
}