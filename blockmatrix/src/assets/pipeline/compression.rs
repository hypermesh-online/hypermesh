//! Compression Stage - Brotli streaming compression
//!
//! Provides configurable Brotli compression with streaming support for large assets.

use crate::assets::pipeline::{PipelineError, PipelineResult};
use serde::{Serialize, Deserialize};
use std::io::{Read, Write};

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// Brotli compression (default) - excellent compression ratio
    Brotli,
    /// No compression
    None,
}

impl Default for CompressionAlgorithm {
    fn default() -> Self {
        Self::Brotli
    }
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,
    /// Compression level (1-11 for Brotli, 1=fastest, 11=best compression)
    pub level: u32,
    /// Chunk size for streaming (bytes)
    pub chunk_size: usize,
    /// Enable streaming mode for large files
    pub streaming: bool,
    /// Window size (16-24 for Brotli)
    pub window_size: u32,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Brotli,
            level: 4, // Balance speed/ratio (Brotli range 1-11)
            chunk_size: 64 * 1024, // 64KB chunks
            streaming: true,
            window_size: 22, // Default Brotli window size
        }
    }
}

/// Compression statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompressionStats {
    /// Original size in bytes
    pub original_size: usize,
    /// Compressed size in bytes
    pub compressed_size: usize,
    /// Compression ratio (compressed/original)
    pub ratio: f64,
    /// Compression time in milliseconds
    pub duration_ms: u64,
    /// Throughput in MB/s
    pub throughput_mbps: f64,
}

impl CompressionStats {
    fn calculate(original_size: usize, compressed_size: usize, duration_ms: u64) -> Self {
        let ratio = if original_size > 0 {
            compressed_size as f64 / original_size as f64
        } else {
            0.0
        };

        let throughput_mbps = if duration_ms > 0 {
            (original_size as f64 / (1024.0 * 1024.0)) / (duration_ms as f64 / 1000.0)
        } else {
            0.0
        };

        Self {
            original_size,
            compressed_size,
            ratio,
            duration_ms,
            throughput_mbps,
        }
    }
}

/// Compressor for asset data
pub struct Compressor {
    config: CompressionConfig,
}

impl Compressor {
    /// Create new compressor with configuration
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Create compressor with default configuration
    pub fn default() -> Self {
        Self::new(CompressionConfig::default())
    }

    /// Compress data
    pub fn compress(&self, data: &[u8]) -> PipelineResult<(Vec<u8>, CompressionStats)> {
        let start = std::time::Instant::now();

        let compressed = match self.config.algorithm {
            CompressionAlgorithm::Brotli => self.compress_brotli(data)?,
            CompressionAlgorithm::None => data.to_vec(),
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        let stats = CompressionStats::calculate(data.len(), compressed.len(), duration_ms);

        Ok((compressed, stats))
    }

    /// Decompress data
    pub fn decompress(&self, data: &[u8]) -> PipelineResult<Vec<u8>> {
        match self.config.algorithm {
            CompressionAlgorithm::Brotli => self.decompress_brotli(data),
            CompressionAlgorithm::None => Ok(data.to_vec()),
        }
    }

    /// Compress using Brotli with streaming support
    fn compress_brotli(&self, data: &[u8]) -> PipelineResult<Vec<u8>> {
        use brotli::{BrotliCompress, enc::BrotliEncoderParams};

        let mut params = BrotliEncoderParams::default();
        params.quality = self.config.level as i32;
        params.lgwin = self.config.window_size as i32;

        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut output);

        BrotliCompress(
            &mut std::io::Cursor::new(data),
            &mut cursor,
            &params
        ).map_err(|e| PipelineError::CompressionFailed(format!("Brotli compression failed: {}", e)))?;

        Ok(output)
    }

    /// Decompress using Brotli
    fn decompress_brotli(&self, data: &[u8]) -> PipelineResult<Vec<u8>> {
        use brotli::BrotliDecompress;

        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut output);

        BrotliDecompress(
            &mut std::io::Cursor::new(data),
            &mut cursor
        ).map_err(|e| PipelineError::CompressionFailed(format!("Brotli decompression failed: {}", e)))?;

        Ok(output)
    }

    /// Compress with streaming for large files
    pub fn compress_stream<R: Read, W: Write>(
        &self,
        mut reader: R,
        mut writer: W,
    ) -> PipelineResult<CompressionStats> {
        let start = std::time::Instant::now();
        let mut total_read = 0usize;
        let mut total_written = 0usize;

        match self.config.algorithm {
            CompressionAlgorithm::Brotli => {
                use brotli::enc::BrotliEncoderParams;
                use brotli::enc::writer::CompressorWriter;

                let mut params = BrotliEncoderParams::default();
                params.quality = self.config.level as i32;
                params.lgwin = self.config.window_size as i32;

                let mut encoder = CompressorWriter::with_params(writer, 4096, &params);
                let mut buffer = vec![0u8; self.config.chunk_size];

                loop {
                    let n = reader.read(&mut buffer)
                        .map_err(|e| PipelineError::CompressionFailed(format!("Stream read failed: {}", e)))?;
                    if n == 0 { break; }

                    total_read += n;
                    let written = encoder.write(&buffer[..n])
                        .map_err(|e| PipelineError::CompressionFailed(format!("Stream write failed: {}", e)))?;
                    total_written += written;
                }

                encoder.flush()
                    .map_err(|e| PipelineError::CompressionFailed(format!("Stream flush failed: {}", e)))?;

                // Finalize the encoder
                drop(encoder);
            }
            CompressionAlgorithm::None => {
                total_written = std::io::copy(&mut reader, &mut writer)
                    .map_err(|e| PipelineError::CompressionFailed(format!("Direct copy failed: {}", e)))? as usize;
                total_read = total_written;
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        Ok(CompressionStats::calculate(total_read, total_written, duration_ms))
    }

    /// Get compression configuration
    pub fn config(&self) -> &CompressionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brotli_compression() {
        let compressor = Compressor::default();
        let data = vec![0u8; 10000]; // Highly compressible data

        let (compressed, stats) = compressor.compress(&data).unwrap();
        assert!(compressed.len() < data.len());
        assert!(stats.ratio < 1.0);

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_brotli_text_compression() {
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Brotli,
            level: 4,
            ..Default::default()
        };
        let compressor = Compressor::new(config);
        let data = b"Hello, World! ".repeat(1000);

        let (compressed, stats) = compressor.compress(&data).unwrap();
        assert!(compressed.len() < data.len());
        assert!(stats.ratio < 0.1); // Brotli excels at text compression

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_no_compression() {
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::None,
            ..Default::default()
        };
        let compressor = Compressor::new(config);
        let data = vec![1, 2, 3, 4, 5];

        let (compressed, stats) = compressor.compress(&data).unwrap();
        assert_eq!(compressed, data);
        assert_eq!(stats.ratio, 1.0);
    }

    #[test]
    fn test_compression_stats() {
        let compressor = Compressor::default();
        let data = vec![0u8; 100000];

        let (_, stats) = compressor.compress(&data).unwrap();
        assert_eq!(stats.original_size, 100000);
        assert!(stats.compressed_size < stats.original_size);
        assert!(stats.ratio < 1.0);
        assert!(stats.throughput_mbps > 0.0);
    }

    #[test]
    fn test_different_brotli_levels() {
        let data = b"The quick brown fox jumps over the lazy dog. ".repeat(1000);

        for level in [1, 4, 7, 11] {
            let config = CompressionConfig {
                algorithm: CompressionAlgorithm::Brotli,
                level,
                ..Default::default()
            };
            let compressor = Compressor::new(config);

            let (compressed, stats) = compressor.compress(&data).unwrap();
            let decompressed = compressor.decompress(&compressed).unwrap();
            assert_eq!(decompressed, data);
            assert!(stats.ratio < 1.0);

            // Higher levels should generally give better compression
            if level > 1 {
                println!("Level {}: ratio = {:.3}, size = {}", level, stats.ratio, compressed.len());
            }
        }
    }

    #[test]
    fn test_streaming_compression() {
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Brotli,
            level: 4,
            streaming: true,
            ..Default::default()
        };
        let compressor = Compressor::new(config);

        // Create large test data
        let data = b"Stream test data. ".repeat(10000);
        let mut reader = std::io::Cursor::new(&data);
        let mut output = Vec::new();

        let stats = compressor.compress_stream(&mut reader, &mut output).unwrap();

        assert!(output.len() < data.len());
        assert!(stats.ratio < 1.0);

        // Verify we can decompress
        let decompressed = compressor.decompress(&output).unwrap();
        assert_eq!(decompressed, data);
    }
}
