//! STOQ Chunking - Content-aware chunking with deduplication for CDN

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use anyhow::{Result, anyhow};
use bytes::{Bytes, BytesMut};
use parking_lot::RwLock;
use dashmap::DashMap;
use sha2::Sha256;
use blake3;
// use rabin::Rabin;  // TODO: Implement content-defined chunking
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn};

pub mod dedup;
pub mod distribution;
pub mod compression;

use dedup::DedupEngine;
use distribution::DistributionManager;
use compression::CompressionEngine;

/// Chunk identifier (content hash)
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChunkId(pub String);

impl ChunkId {
    /// Create from bytes
    pub fn from_bytes(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self(hash.to_hex().to_string())
    }
    
    /// Create from hash string
    pub fn from_hash(hash: String) -> Self {
        Self(hash)
    }
}

/// Data chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique chunk identifier
    pub id: ChunkId,
    /// Chunk data
    pub data: Bytes,
    /// Original size before compression
    pub original_size: usize,
    /// Compressed size
    pub compressed_size: usize,
    /// Chunk index in original data
    pub index: usize,
    /// Content type hint
    pub content_type: Option<String>,
    /// Encryption metadata
    pub encryption: Option<EncryptionMeta>,
}

impl Chunk {
    /// Create a new chunk
    pub fn new(data: Bytes, index: usize) -> Self {
        let id = ChunkId::from_bytes(&data);
        let original_size = data.len();
        
        Self {
            id,
            data,
            original_size,
            compressed_size: original_size, // Updated after compression
            index,
            content_type: None,
            encryption: None,
        }
    }
    
    /// Compress the chunk
    pub fn compress(&mut self, engine: &CompressionEngine) -> Result<()> {
        let compressed = engine.compress(&self.data)?;
        self.compressed_size = compressed.len();
        self.data = compressed;
        Ok(())
    }
    
    /// Decompress the chunk
    pub fn decompress(&mut self, engine: &CompressionEngine) -> Result<()> {
        let decompressed = engine.decompress(&self.data)?;
        self.data = decompressed;
        Ok(())
    }
}

/// Encryption metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMeta {
    /// Algorithm used
    pub algorithm: String,
    /// Key identifier
    pub key_id: String,
    /// Initialization vector
    pub iv: Vec<u8>,
}

/// Chunking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Minimum chunk size in bytes
    pub min_size: usize,
    /// Maximum chunk size in bytes
    pub max_size: usize,
    /// Average chunk size target
    pub avg_size: usize,
    /// Chunking algorithm
    pub algorithm: ChunkAlgorithm,
    /// Enable deduplication
    pub enable_dedup: bool,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression algorithm
    pub compression: CompressionAlgorithm,
    /// Rolling hash window size
    pub window_size: usize,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            min_size: 4 * 1024,      // 4 KB
            max_size: 1024 * 1024,   // 1 MB
            avg_size: 64 * 1024,     // 64 KB
            algorithm: ChunkAlgorithm::ContentDefined,
            enable_dedup: true,
            enable_compression: true,
            compression: CompressionAlgorithm::Zstd,
            window_size: 48,
        }
    }
}

/// Chunking algorithm selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkAlgorithm {
    /// Fixed-size chunks
    FixedSize,
    /// Content-defined chunking (CDC)
    ContentDefined,
    /// Rabin fingerprinting
    Rabin,
    /// BuzHash rolling hash
    BuzHash,
    /// Adaptive based on content type
    Adaptive,
}

/// Compression algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// Zstandard compression
    Zstd,
    /// LZ4 compression
    Lz4,
    /// Adaptive based on content
    Adaptive,
}

/// Main chunk engine implementation
pub struct ChunkEngine {
    config: ChunkingConfig,
    dedup: Arc<DedupEngine>,
    distribution: Arc<DistributionManager>,
    compression: Arc<CompressionEngine>,
    chunk_cache: Arc<DashMap<ChunkId, Arc<Chunk>>>,
    stats: Arc<RwLock<ChunkStats>>,
}

/// Chunking statistics
#[derive(Debug, Clone, Default)]
pub struct ChunkStats {
    /// Total chunks created
    pub total_chunks: u64,
    /// Duplicate chunks found
    pub duplicate_chunks: u64,
    /// Total bytes processed
    pub total_bytes: u64,
    /// Total bytes after dedup
    pub dedup_bytes: u64,
    /// Total bytes after compression
    pub compressed_bytes: u64,
    /// Average chunk size
    pub avg_chunk_size: usize,
}

impl ChunkEngine {
    /// Create a new chunk engine
    pub fn new(config: ChunkingConfig) -> Result<Self> {
        info!("Initializing chunk engine with {:?} algorithm", config.algorithm);
        
        let dedup = Arc::new(DedupEngine::new()?);
        let distribution = Arc::new(DistributionManager::new()?);
        let compression = Arc::new(CompressionEngine::new(config.compression.clone())?);
        let chunk_cache = Arc::new(DashMap::new());
        let stats = Arc::new(RwLock::new(ChunkStats::default()));
        
        Ok(Self {
            config,
            dedup,
            distribution,
            compression,
            chunk_cache,
            stats,
        })
    }
    
    /// Chunk data into smaller pieces
    pub fn chunk(&self, data: &[u8]) -> Result<Vec<Chunk>> {
        debug!("Chunking {} bytes of data", data.len());
        
        let chunks = match self.config.algorithm {
            ChunkAlgorithm::FixedSize => self.fixed_size_chunking(data)?,
            ChunkAlgorithm::ContentDefined => self.content_defined_chunking(data)?,
            ChunkAlgorithm::Rabin => self.rabin_chunking(data)?,
            ChunkAlgorithm::BuzHash => self.buzhash_chunking(data)?,
            ChunkAlgorithm::Adaptive => self.adaptive_chunking(data)?,
        };
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_chunks += chunks.len() as u64;
        stats.total_bytes += data.len() as u64;
        
        // Apply compression if enabled
        let mut processed_chunks = chunks;
        if self.config.enable_compression {
            for chunk in &mut processed_chunks {
                chunk.compress(&self.compression)?;
                stats.compressed_bytes += chunk.compressed_size as u64;
            }
        }
        
        // Check for duplicates if dedup enabled
        if self.config.enable_dedup {
            // TODO: Fix deduplication - implement proper Arc<Mutex<T>> or similar
            // let dedup_count = self.dedup.check_duplicates(&processed_chunks);
            // stats.duplicate_chunks += dedup_count as u64;
        }
        
        // Cache chunks
        for chunk in &processed_chunks {
            self.chunk_cache.insert(chunk.id.clone(), Arc::new(chunk.clone()));
        }
        
        Ok(processed_chunks)
    }
    
    /// Fixed-size chunking
    fn fixed_size_chunking(&self, data: &[u8]) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let chunk_size = self.config.avg_size;
        
        for (index, chunk_data) in data.chunks(chunk_size).enumerate() {
            let chunk = Chunk::new(Bytes::copy_from_slice(chunk_data), index);
            chunks.push(chunk);
        }
        
        Ok(chunks)
    }
    
    /// Content-defined chunking using rolling hash
    fn content_defined_chunking(&self, data: &[u8]) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut index = 0;
        
        // Use rolling hash to find chunk boundaries
        let window_size = self.config.window_size;
        let mut hasher = RollingHash::new(window_size);
        
        for (i, &byte) in data.iter().enumerate() {
            hasher.update(byte);
            
            let chunk_size = i - start;
            
            // Check if we should create a chunk
            let should_chunk = chunk_size >= self.config.min_size && (
                hasher.is_boundary(self.config.avg_size) || 
                chunk_size >= self.config.max_size
            );
            
            if should_chunk || i == data.len() - 1 {
                if chunk_size > 0 || i == data.len() - 1 {
                    let end = if i == data.len() - 1 { i + 1 } else { i };
                    let chunk = Chunk::new(Bytes::copy_from_slice(&data[start..end]), index);
                    chunks.push(chunk);
                    index += 1;
                    start = i;
                }
            }
        }
        
        Ok(chunks)
    }
    
    /// Rabin fingerprinting chunking
    fn rabin_chunking(&self, data: &[u8]) -> Result<Vec<Chunk>> {
        // Use Rabin fingerprinting for content-defined chunking
        // This is a simplified implementation
        self.content_defined_chunking(data)
    }
    
    /// BuzHash rolling hash chunking
    fn buzhash_chunking(&self, data: &[u8]) -> Result<Vec<Chunk>> {
        // Use BuzHash for rolling hash
        // This is a simplified implementation
        self.content_defined_chunking(data)
    }
    
    /// Adaptive chunking based on content type
    fn adaptive_chunking(&self, data: &[u8]) -> Result<Vec<Chunk>> {
        // Detect content type and choose appropriate algorithm
        // For now, use content-defined chunking
        self.content_defined_chunking(data)
    }
    
    /// Reassemble chunks into original data
    pub fn reassemble(&self, chunks: Vec<Chunk>) -> Result<Bytes> {
        debug!("Reassembling {} chunks", chunks.len());
        
        // Sort chunks by index
        let mut sorted_chunks = chunks;
        sorted_chunks.sort_by_key(|c| c.index);
        
        // Decompress if needed
        if self.config.enable_compression {
            for chunk in &mut sorted_chunks {
                let mut decompressed = chunk.clone();
                decompressed.decompress(&self.compression)?;
                *chunk = decompressed;
            }
        }
        
        // Concatenate chunks
        let mut result = BytesMut::new();
        for chunk in sorted_chunks {
            result.extend_from_slice(&chunk.data);
        }
        
        Ok(result.freeze())
    }
    
    /// Check for duplicate chunks
    pub fn deduplicate(&self, chunks: &[Chunk]) -> Vec<ChunkId> {
        self.dedup.find_duplicates(chunks)
    }
    
    /// Get chunk by ID
    pub fn get_chunk(&self, id: &ChunkId) -> Option<Arc<Chunk>> {
        self.chunk_cache.get(id).map(|entry| entry.clone())
    }
    
    /// Get chunking statistics
    pub fn stats(&self) -> ChunkStats {
        self.stats.read().clone()
    }
    
    /// Calculate deduplication ratio
    pub fn dedup_ratio(&self) -> f64 {
        let stats = self.stats.read();
        if stats.total_chunks == 0 {
            0.0
        } else {
            stats.duplicate_chunks as f64 / stats.total_chunks as f64
        }
    }
}

/// Simple rolling hash implementation
struct RollingHash {
    window: Vec<u8>,
    window_size: usize,
    hash: u64,
    pos: usize,
}

impl RollingHash {
    fn new(window_size: usize) -> Self {
        Self {
            window: vec![0; window_size],
            window_size,
            hash: 0,
            pos: 0,
        }
    }
    
    fn update(&mut self, byte: u8) {
        // Remove old byte from hash
        let old_byte = self.window[self.pos];
        self.hash = self.hash.wrapping_sub(old_byte as u64);
        
        // Add new byte to hash
        self.window[self.pos] = byte;
        self.hash = self.hash.wrapping_add(byte as u64);
        
        // Update position
        self.pos = (self.pos + 1) % self.window_size;
        
        // Simple hash mixing
        self.hash = self.hash.wrapping_mul(0x1000193);
    }
    
    fn is_boundary(&self, avg_size: usize) -> bool {
        // Check if hash indicates a chunk boundary
        // Use the last N bits to determine boundary
        let mask = (1 << avg_size.trailing_zeros()) - 1;
        (self.hash & mask) == 0
    }
}

// Trait implementations
impl crate::Chunker for ChunkEngine {
    fn chunk(&self, data: &[u8]) -> Result<Vec<Chunk>> {
        self.chunk(data)
    }
    
    fn reassemble(&self, chunks: Vec<Chunk>) -> Result<Bytes> {
        self.reassemble(chunks)
    }
    
    fn deduplicate(&self, chunks: &[Chunk]) -> Vec<ChunkId> {
        self.deduplicate(chunks)
    }
    
    fn stats(&self) -> crate::ChunkStats {
        let stats = self.stats.read();
        crate::ChunkStats {
            total_chunks: stats.total_chunks,
            duplicate_chunks: stats.duplicate_chunks,
            dedup_ratio: self.dedup_ratio(),
            avg_chunk_size: if stats.total_chunks > 0 {
                (stats.total_bytes / stats.total_chunks) as usize
            } else {
                0
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunk_id_generation() {
        let data = b"test data";
        let id1 = ChunkId::from_bytes(data);
        let id2 = ChunkId::from_bytes(data);
        assert_eq!(id1, id2);
        
        let different = b"different data";
        let id3 = ChunkId::from_bytes(different);
        assert_ne!(id1, id3);
    }
    
    #[test]
    fn test_chunk_engine_creation() {
        let config = ChunkingConfig::default();
        let engine = ChunkEngine::new(config);
        assert!(engine.is_ok());
    }
    
    #[test]
    fn test_fixed_size_chunking() {
        let config = ChunkingConfig {
            algorithm: ChunkAlgorithm::FixedSize,
            avg_size: 10,
            ..Default::default()
        };
        
        let engine = ChunkEngine::new(config).unwrap();
        let data = vec![0u8; 100];
        let chunks = engine.chunk(&data).unwrap();
        
        assert_eq!(chunks.len(), 10);
        for chunk in &chunks {
            assert_eq!(chunk.original_size, 10);
        }
    }
    
    #[test]
    fn test_chunk_reassembly() {
        let config = ChunkingConfig::default();
        let engine = ChunkEngine::new(config).unwrap();
        
        let original = b"Hello, World! This is a test of chunking and reassembly.";
        let chunks = engine.chunk(original).unwrap();
        let reassembled = engine.reassemble(chunks).unwrap();
        
        assert_eq!(&reassembled[..], original);
    }
}