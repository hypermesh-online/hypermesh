//! Compression engine

use anyhow::Result;
use bytes::Bytes;
use crate::chunking::CompressionAlgorithm;

pub struct CompressionEngine {
    algorithm: CompressionAlgorithm,
}

impl CompressionEngine {
    pub fn new(algorithm: CompressionAlgorithm) -> Result<Self> {
        Ok(Self { algorithm })
    }
    
    pub fn compress(&self, data: &[u8]) -> Result<Bytes> {
        match self.algorithm {
            CompressionAlgorithm::None => Ok(Bytes::copy_from_slice(data)),
            CompressionAlgorithm::Zstd => {
                let compressed = zstd::encode_all(data, 3)?;
                Ok(Bytes::from(compressed))
            },
            CompressionAlgorithm::Lz4 => {
                let compressed = lz4::block::compress(data, Some(lz4::block::CompressionMode::HIGHCOMPRESSION(10)), true)?;
                Ok(Bytes::from(compressed))
            },
            CompressionAlgorithm::Adaptive => {
                // Choose best algorithm based on data
                self.compress(data) // Default to zstd for now
            }
        }
    }
    
    pub fn decompress(&self, data: &[u8]) -> Result<Bytes> {
        match self.algorithm {
            CompressionAlgorithm::None => Ok(Bytes::copy_from_slice(data)),
            CompressionAlgorithm::Zstd => {
                let decompressed = zstd::decode_all(data)?;
                Ok(Bytes::from(decompressed))
            },
            CompressionAlgorithm::Lz4 => {
                let decompressed = lz4::block::decompress(data, None)?;
                Ok(Bytes::from(decompressed))
            },
            CompressionAlgorithm::Adaptive => {
                // Auto-detect and decompress
                self.decompress(data)
            }
        }
    }
}