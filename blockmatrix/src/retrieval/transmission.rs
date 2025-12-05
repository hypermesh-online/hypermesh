//! Instruction Transmission
//!
//! Efficiently serialize and transmit retrieval instructions with minimal overhead.

use anyhow::Result;
use serde::{Serialize, Deserialize};

use super::RetrievalPlan;

/// Compression format for instruction transmission
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionFormat {
    /// No compression (fastest)
    None,

    /// Brotli compression (best ratio)
    Brotli,

    /// Zstd compression (balanced)
    Zstd,

    /// MessagePack binary format
    MessagePack,
}

impl CompressionFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &str {
        match self {
            CompressionFormat::None => "json",
            CompressionFormat::Brotli => "br",
            CompressionFormat::Zstd => "zst",
            CompressionFormat::MessagePack => "msgpack",
        }
    }

    /// Get MIME type for this format
    pub fn mime_type(&self) -> &str {
        match self {
            CompressionFormat::None => "application/json",
            CompressionFormat::Brotli => "application/x-brotli",
            CompressionFormat::Zstd => "application/zstd",
            CompressionFormat::MessagePack => "application/msgpack",
        }
    }
}

/// Statistics for transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransmissionStats {
    /// Original size before compression
    pub original_size: usize,

    /// Compressed size
    pub compressed_size: usize,

    /// Compression ratio (compressed / original)
    pub compression_ratio: f64,

    /// Time taken to encode (microseconds)
    pub encode_time_us: u64,

    /// Format used
    pub format: CompressionFormat,
}

impl TransmissionStats {
    /// Calculate space saved
    pub fn space_saved(&self) -> usize {
        self.original_size.saturating_sub(self.compressed_size)
    }

    /// Calculate percentage saved
    pub fn percentage_saved(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        (self.space_saved() as f64 / self.original_size as f64) * 100.0
    }
}

/// Instruction transmitter for efficient encoding
pub struct InstructionTransmitter {
    /// Default compression format
    format: CompressionFormat,
}

impl InstructionTransmitter {
    /// Create a new transmitter with default format
    pub fn new(format: CompressionFormat) -> Self {
        Self { format }
    }

    /// Encode retrieval plan to bytes
    pub fn encode(&self, plan: &RetrievalPlan) -> Result<Vec<u8>> {
        self.encode_with_format(plan, self.format)
    }

    /// Encode with specific format
    pub fn encode_with_format(
        &self,
        plan: &RetrievalPlan,
        format: CompressionFormat,
    ) -> Result<Vec<u8>> {
        let start = std::time::Instant::now();

        let bytes = match format {
            CompressionFormat::None => {
                serde_json::to_vec(plan)?
            }
            CompressionFormat::Brotli => {
                let json = serde_json::to_vec(plan)?;
                self.compress_brotli(&json)?
            }
            CompressionFormat::Zstd => {
                let json = serde_json::to_vec(plan)?;
                self.compress_zstd(&json)?
            }
            CompressionFormat::MessagePack => {
                rmp_serde::to_vec(plan)?
            }
        };

        Ok(bytes)
    }

    /// Encode with statistics tracking
    pub fn encode_with_stats(
        &self,
        plan: &RetrievalPlan,
    ) -> Result<(Vec<u8>, TransmissionStats)> {
        let start = std::time::Instant::now();

        // Get uncompressed size
        let uncompressed = serde_json::to_vec(plan)?;
        let original_size = uncompressed.len();

        // Encode with format
        let compressed = self.encode(plan)?;
        let compressed_size = compressed.len();

        let elapsed = start.elapsed().as_micros() as u64;

        let stats = TransmissionStats {
            original_size,
            compressed_size,
            compression_ratio: compressed_size as f64 / original_size as f64,
            encode_time_us: elapsed,
            format: self.format,
        };

        Ok((compressed, stats))
    }

    /// Decode bytes back to retrieval plan
    pub fn decode(&self, bytes: &[u8]) -> Result<RetrievalPlan> {
        self.decode_with_format(bytes, self.format)
    }

    /// Decode with specific format
    pub fn decode_with_format(
        &self,
        bytes: &[u8],
        format: CompressionFormat,
    ) -> Result<RetrievalPlan> {
        match format {
            CompressionFormat::None => {
                Ok(serde_json::from_slice(bytes)?)
            }
            CompressionFormat::Brotli => {
                let decompressed = self.decompress_brotli(bytes)?;
                Ok(serde_json::from_slice(&decompressed)?)
            }
            CompressionFormat::Zstd => {
                let decompressed = self.decompress_zstd(bytes)?;
                Ok(serde_json::from_slice(&decompressed)?)
            }
            CompressionFormat::MessagePack => {
                Ok(rmp_serde::from_slice(bytes)?)
            }
        }
    }

    /// Compress using Brotli
    fn compress_brotli(&self, data: &[u8]) -> Result<Vec<u8>> {
        use brotli::enc::BrotliEncoderParams;

        let mut output = Vec::new();
        let params = BrotliEncoderParams {
            quality: 6, // Balanced quality
            ..Default::default()
        };

        brotli::BrotliCompress(
            &mut std::io::Cursor::new(data),
            &mut output,
            &params,
        )?;

        Ok(output)
    }

    /// Decompress Brotli
    fn decompress_brotli(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        brotli::BrotliDecompress(
            &mut std::io::Cursor::new(data),
            &mut output,
        )?;
        Ok(output)
    }

    /// Compress using Zstd
    fn compress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(zstd::encode_all(data, 3)?) // Level 3 for speed
    }

    /// Decompress Zstd
    fn decompress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(zstd::decode_all(data)?)
    }

    /// Benchmark all formats and return best
    pub fn benchmark_formats(&self, plan: &RetrievalPlan) -> Result<CompressionFormat> {
        let formats = vec![
            CompressionFormat::None,
            CompressionFormat::Brotli,
            CompressionFormat::Zstd,
            CompressionFormat::MessagePack,
        ];

        let mut best_format = CompressionFormat::None;
        let mut best_size = usize::MAX;

        for format in formats {
            if let Ok(bytes) = self.encode_with_format(plan, format) {
                if bytes.len() < best_size {
                    best_size = bytes.len();
                    best_format = format;
                }
            }
        }

        Ok(best_format)
    }
}

impl Default for InstructionTransmitter {
    fn default() -> Self {
        Self::new(CompressionFormat::Brotli)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retrieval::{CompleteShardMap, RetrievalMetadata, ShardMapEntry, ShardLocation};
    use crate::matrix::MatrixCoordinate;

    fn create_test_plan() -> RetrievalPlan {
        let content_hash = [1u8; 32];
        let mut shard_map = CompleteShardMap::new();

        // Create 14 shards (Reed-Solomon 10+4)
        for i in 0..14 {
            let shard_hash = [i as u8; 32];
            let locations = vec![
                ShardLocation::new(MatrixCoordinate::new(i as i64, 0, 0).unwrap(), 0.9),
                ShardLocation::new(MatrixCoordinate::new(i as i64, 1, 0).unwrap(), 0.85),
            ];
            let entry = ShardMapEntry::new(shard_hash, locations);
            shard_map.add_entry(entry);
        }

        let metadata = RetrievalMetadata {
            erasure_coding: (10, 4),
            compression: "brotli".to_string(),
            encryption: "aes-256-gcm".to_string(),
            content_type: "application/octet-stream".to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };

        RetrievalPlan::new(content_hash, shard_map, metadata)
    }

    #[test]
    fn test_transmitter_creation() {
        let transmitter = InstructionTransmitter::new(CompressionFormat::Brotli);
        assert_eq!(transmitter.format, CompressionFormat::Brotli);
    }

    #[test]
    fn test_encode_decode_none() {
        let plan = create_test_plan();
        let transmitter = InstructionTransmitter::new(CompressionFormat::None);

        let encoded = transmitter.encode(&plan).unwrap();
        let decoded = transmitter.decode(&encoded).unwrap();

        assert_eq!(plan.content_hash, decoded.content_hash);
        assert_eq!(plan.shard_map.entries.len(), decoded.shard_map.entries.len());
    }

    #[test]
    fn test_encode_decode_brotli() {
        let plan = create_test_plan();
        let transmitter = InstructionTransmitter::new(CompressionFormat::Brotli);

        let encoded = transmitter.encode(&plan).unwrap();
        let decoded = transmitter.decode(&encoded).unwrap();

        assert_eq!(plan.content_hash, decoded.content_hash);
        assert_eq!(plan.shard_map.entries.len(), decoded.shard_map.entries.len());
    }

    #[test]
    fn test_encode_decode_zstd() {
        let plan = create_test_plan();
        let transmitter = InstructionTransmitter::new(CompressionFormat::Zstd);

        let encoded = transmitter.encode(&plan).unwrap();
        let decoded = transmitter.decode(&encoded).unwrap();

        assert_eq!(plan.content_hash, decoded.content_hash);
    }

    #[test]
    fn test_encode_decode_messagepack() {
        let plan = create_test_plan();
        let transmitter = InstructionTransmitter::new(CompressionFormat::MessagePack);

        let encoded = transmitter.encode(&plan).unwrap();
        let decoded = transmitter.decode(&encoded).unwrap();

        assert_eq!(plan.content_hash, decoded.content_hash);
    }

    #[test]
    fn test_compression_effectiveness() {
        let plan = create_test_plan();
        let transmitter = InstructionTransmitter::new(CompressionFormat::Brotli);

        let (compressed, stats) = transmitter.encode_with_stats(&plan).unwrap();

        println!("Original size: {} bytes", stats.original_size);
        println!("Compressed size: {} bytes", stats.compressed_size);
        println!("Compression ratio: {:.2}", stats.compression_ratio);
        println!("Space saved: {:.2}%", stats.percentage_saved());

        // Brotli should achieve good compression
        assert!(stats.compressed_size < stats.original_size);
        assert!(stats.compression_ratio < 1.0);
    }

    #[test]
    fn test_instruction_size_target() {
        let plan = create_test_plan();
        let transmitter = InstructionTransmitter::new(CompressionFormat::Brotli);

        let (compressed, stats) = transmitter.encode_with_stats(&plan).unwrap();

        println!("Compressed instruction size: {} bytes", compressed.len());

        // Target: <1KB for typical retrieval plan
        assert!(compressed.len() < 1024,
            "Instruction size {} exceeds 1KB target", compressed.len());
    }

    #[test]
    fn test_benchmark_formats() {
        let plan = create_test_plan();
        let transmitter = InstructionTransmitter::default();

        let best_format = transmitter.benchmark_formats(&plan).unwrap();
        println!("Best format: {:?}", best_format);

        // Should find a format
        assert!(matches!(best_format,
            CompressionFormat::Brotli |
            CompressionFormat::Zstd |
            CompressionFormat::MessagePack |
            CompressionFormat::None
        ));
    }

    #[test]
    fn test_format_extensions() {
        assert_eq!(CompressionFormat::None.extension(), "json");
        assert_eq!(CompressionFormat::Brotli.extension(), "br");
        assert_eq!(CompressionFormat::Zstd.extension(), "zst");
        assert_eq!(CompressionFormat::MessagePack.extension(), "msgpack");
    }

    #[test]
    fn test_format_mime_types() {
        assert_eq!(CompressionFormat::None.mime_type(), "application/json");
        assert_eq!(CompressionFormat::Brotli.mime_type(), "application/x-brotli");
        assert_eq!(CompressionFormat::Zstd.mime_type(), "application/zstd");
        assert_eq!(CompressionFormat::MessagePack.mime_type(), "application/msgpack");
    }
}
