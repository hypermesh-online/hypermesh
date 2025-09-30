//! Phoenix Compression Engine
//!
//! Automatic compression selection based on performance tier and data characteristics.

use crate::{
    config::PerformanceTier,
    errors::{PhoenixError, Result},
};

/// Compression engine for Phoenix SDK
pub struct CompressionEngine {
    performance_tier: PerformanceTier,
    enabled: bool,
}

impl CompressionEngine {
    /// Create new compression engine based on performance tier
    pub fn new(performance_tier: PerformanceTier) -> Self {
        // Disable compression for high-throughput scenarios
        let enabled = match &performance_tier {
            PerformanceTier::HighThroughput => false,
            PerformanceTier::Custom(gbps) if *gbps > 10 => false,
            _ => cfg!(any(feature = "flate2", feature = "zstd", feature = "lz4")),
        };

        Self {
            performance_tier,
            enabled,
        }
    }

    /// Compress data if compression is available and enabled
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if !self.enabled || data.len() < 1024 {
            return Ok(data.to_vec());
        }

        #[cfg(feature = "flate2")]
        {
            return self.compress_gzip(data);
        }

        #[cfg(all(not(feature = "flate2"), feature = "zstd"))]
        {
            return self.compress_zstd(data);
        }

        #[cfg(all(not(feature = "flate2"), not(feature = "zstd"), feature = "lz4"))]
        {
            return self.compress_lz4(data);
        }

        #[cfg(not(any(feature = "flate2", feature = "zstd", feature = "lz4")))]
        {
            Ok(data.to_vec())
        }
    }

    /// Decompress data if it appears to be compressed
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if !self.enabled || data.is_empty() {
            return Ok(data.to_vec());
        }

        // Check magic bytes and decompress accordingly
        #[cfg(feature = "flate2")]
        {
            if self.is_gzip(data) {
                return self.decompress_gzip(data);
            }
        }

        #[cfg(feature = "zstd")]
        {
            if self.is_zstd(data) {
                return self.decompress_zstd(data);
            }
        }

        #[cfg(feature = "lz4")]
        {
            if self.is_lz4(data) {
                return self.decompress_lz4(data);
            }
        }

        Ok(data.to_vec())
    }

    /// Check if data appears to be compressed
    pub fn is_compressed(&self, data: &[u8]) -> bool {
        if data.len() < 2 {
            return false;
        }

        #[cfg(feature = "flate2")]
        {
            if self.is_gzip(data) {
                return true;
            }
        }

        #[cfg(feature = "zstd")]
        {
            if self.is_zstd(data) {
                return true;
            }
        }

        #[cfg(feature = "lz4")]
        {
            if self.is_lz4(data) {
                return true;
            }
        }

        false
    }

    // Compression implementations

    #[cfg(feature = "flate2")]
    fn compress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(data)
            .map_err(|e| PhoenixError::CompressionError(e.to_string()))?;

        encoder.finish()
            .map_err(|e| PhoenixError::CompressionError(e.to_string()))
    }

    #[cfg(feature = "zstd")]
    fn compress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        zstd::encode_all(data, 3)
            .map_err(|e| PhoenixError::CompressionError(e.to_string()))
    }

    #[cfg(feature = "lz4")]
    fn compress_lz4(&self, data: &[u8]) -> Result<Vec<u8>> {
        lz4::block::compress(data, None, true)
            .map_err(|e| PhoenixError::CompressionError(e.to_string()))
    }

    // Decompression implementations

    #[cfg(feature = "flate2")]
    fn decompress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| PhoenixError::CompressionError(e.to_string()))?;
        Ok(decompressed)
    }

    #[cfg(feature = "zstd")]
    fn decompress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        zstd::decode_all(data)
            .map_err(|e| PhoenixError::CompressionError(e.to_string()))
    }

    #[cfg(feature = "lz4")]
    fn decompress_lz4(&self, data: &[u8]) -> Result<Vec<u8>> {
        lz4::block::decompress(data, None)
            .map_err(|e| PhoenixError::CompressionError(e.to_string()))
    }

    // Format detection helpers

    #[cfg(feature = "flate2")]
    fn is_gzip(&self, data: &[u8]) -> bool {
        data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
    }

    #[cfg(feature = "zstd")]
    fn is_zstd(&self, data: &[u8]) -> bool {
        data.len() >= 4 &&
        data[0] == 0x28 && data[1] == 0xb5 && data[2] == 0x2f && data[3] == 0xfd
    }

    #[cfg(feature = "lz4")]
    fn is_lz4(&self, data: &[u8]) -> bool {
        // LZ4 magic number: 0x184D2204
        data.len() >= 4 &&
        data[0] == 0x04 && data[1] == 0x22 && data[2] == 0x4d && data[3] == 0x18
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_engine() {
        let engine = CompressionEngine::new(PerformanceTier::Production);

        let data = b"Hello, Phoenix SDK! This is a test message for compression.";

        // Compression is only available if features are enabled
        let compressed = engine.compress(data).unwrap();
        let decompressed = engine.decompress(&compressed).unwrap();

        // If no compression features are enabled, data should be unchanged
        #[cfg(not(any(feature = "flate2", feature = "zstd", feature = "lz4")))]
        {
            assert_eq!(compressed, data);
        }

        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_small_data_not_compressed() {
        let engine = CompressionEngine::new(PerformanceTier::Production);

        let small_data = b"Small";
        let result = engine.compress(small_data).unwrap();

        // Small data should not be compressed
        assert_eq!(result, small_data);
    }
}