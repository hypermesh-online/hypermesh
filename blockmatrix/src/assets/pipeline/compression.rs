// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Compression Stage - Brotli and Zstd streaming compression
//!
//! Provides configurable compression with streaming support for large assets.
//! Supports Brotli (excellent ratio for text), Zstd (fast for large/binary data),
//! and Auto mode (content-type-based algorithm selection).

use crate::assets::pipeline::{PipelineError, PipelineResult};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// Brotli compression (default) - excellent compression ratio
    Brotli,
    /// No compression
    None,
    /// Zstd compression - fast with good ratio for large/binary data
    Zstd,
    /// Auto-select algorithm based on content type and size
    Auto,
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
            level: 4,              // Balance speed/ratio (Brotli range 1-11)
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
        } else if original_size > 0 {
            // If duration is too small to measure, use a minimum of 0.001ms (1 microsecond)
            (original_size as f64 / (1024.0 * 1024.0)) / 0.001
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
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(CompressionConfig::default())
    }

    /// Compress data
    pub fn compress(&self, data: &[u8]) -> PipelineResult<(Vec<u8>, CompressionStats)> {
        let start = std::time::Instant::now();

        let compressed = match self.config.algorithm {
            CompressionAlgorithm::Brotli => self.compress_brotli(data)?,
            CompressionAlgorithm::None => data.to_vec(),
            CompressionAlgorithm::Zstd => self.compress_zstd_raw(data)?,
            CompressionAlgorithm::Auto => {
                let algo = self.select_algorithm("application/octet-stream", data.len());
                let mut temp_config = self.config.clone();
                temp_config.algorithm = algo;
                let temp = Compressor::new(temp_config);
                return temp.compress(data);
            }
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
            CompressionAlgorithm::Zstd => self.decompress_zstd(data),
            CompressionAlgorithm::Auto => {
                // Auto-detect: zstd magic bytes (0x28 0xB5 0x2F 0xFD), else try brotli
                if data.len() >= 4
                    && data[0] == 0x28
                    && data[1] == 0xB5
                    && data[2] == 0x2F
                    && data[3] == 0xFD
                {
                    self.decompress_zstd(data)
                } else {
                    self.decompress_brotli(data)
                }
            }
        }
    }

    /// Compress using Brotli with streaming support
    fn compress_brotli(&self, data: &[u8]) -> PipelineResult<Vec<u8>> {
        use brotli::{enc::BrotliEncoderParams, BrotliCompress};

        let params = BrotliEncoderParams {
            quality: self.config.level as i32,
            lgwin: self.config.window_size as i32,
            ..BrotliEncoderParams::default()
        };

        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut output);

        BrotliCompress(&mut std::io::Cursor::new(data), &mut cursor, &params).map_err(|e| {
            PipelineError::CompressionFailed(format!("Brotli compression failed: {e}"))
        })?;

        Ok(output)
    }

    /// Decompress using Brotli
    fn decompress_brotli(&self, data: &[u8]) -> PipelineResult<Vec<u8>> {
        use brotli::BrotliDecompress;

        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut output);

        BrotliDecompress(&mut std::io::Cursor::new(data), &mut cursor).map_err(|e| {
            PipelineError::CompressionFailed(format!("Brotli decompression failed: {e}"))
        })?;

        Ok(output)
    }

    /// Compress using Zstd (raw bytes, stats calculated by caller)
    fn compress_zstd_raw(&self, data: &[u8]) -> PipelineResult<Vec<u8>> {
        let level = self.config.level.min(22) as i32; // zstd max level is 22
        zstd::encode_all(std::io::Cursor::new(data), level)
            .map_err(|e| PipelineError::CompressionFailed(format!("zstd compression failed: {e}")))
    }

    /// Decompress using Zstd
    fn decompress_zstd(&self, data: &[u8]) -> PipelineResult<Vec<u8>> {
        zstd::decode_all(std::io::Cursor::new(data))
            .map_err(|e| PipelineError::CompressionFailed(format!("zstd decompression failed: {e}")))
    }

    /// Select the best compression algorithm based on content type and size.
    ///
    /// - Already-compressed formats (video, audio, jpeg, zip, etc.) → None
    /// - Small text content (<10 MB) → Brotli (best ratio for text)
    /// - Everything else → Zstd (fast, good for large/heterogeneous data)
    pub fn select_algorithm(&self, content_type: &str, size: usize) -> CompressionAlgorithm {
        let ct = content_type.to_lowercase();

        // Already-compressed formats: skip compression
        if ct.starts_with("video/")
            || ct.starts_with("audio/")
            || ct == "image/jpeg"
            || ct == "image/png"
            || ct == "image/webp"
            || ct == "application/zip"
            || ct == "application/gzip"
            || ct == "application/x-xz"
            || ct == "application/zstd"
            || ct == "application/x-bzip2"
        {
            return CompressionAlgorithm::None;
        }

        // Small text content: Brotli (best ratio for text)
        if size < 10 * 1024 * 1024
            && (ct.starts_with("text/")
                || ct == "application/json"
                || ct == "application/javascript"
                || ct == "application/xml"
                || ct == "application/xhtml+xml")
        {
            return CompressionAlgorithm::Brotli;
        }

        // Everything else: zstd (fast, good for large heterogeneous data)
        CompressionAlgorithm::Zstd
    }

    /// Compress with streaming for large files
    pub fn compress_stream<R: Read, W: Write>(
        &self,
        mut reader: R,
        mut writer: W,
    ) -> PipelineResult<CompressionStats> {
        let start = std::time::Instant::now();
        let mut total_read = 0usize;
        #[allow(unused_assignments)]
        let mut total_written = 0usize;

        match self.config.algorithm {
            CompressionAlgorithm::Brotli => {
                use brotli::enc::writer::CompressorWriter;
                use brotli::enc::BrotliEncoderParams;
                use std::io::Write;

                let params = BrotliEncoderParams {
                    quality: self.config.level as i32,
                    lgwin: self.config.window_size as i32,
                    ..BrotliEncoderParams::default()
                };

                // Use a buffer to capture output for size tracking
                let mut output_buffer = Vec::new();
                {
                    let mut encoder =
                        CompressorWriter::with_params(&mut output_buffer, 4096, &params);
                    let mut buffer = vec![0u8; self.config.chunk_size];

                    loop {
                        let n = reader.read(&mut buffer).map_err(|e| {
                            PipelineError::CompressionFailed(format!("Stream read failed: {e}"))
                        })?;
                        if n == 0 {
                            break;
                        }

                        total_read += n;
                        encoder.write_all(&buffer[..n]).map_err(|e| {
                            PipelineError::CompressionFailed(format!("Stream write failed: {e}"))
                        })?;
                    }

                    encoder.flush().map_err(|e| {
                        PipelineError::CompressionFailed(format!("Stream flush failed: {e}"))
                    })?;
                } // encoder dropped here, finalizing compression

                // Now write the compressed data and track its size
                total_written = output_buffer.len();
                writer.write_all(&output_buffer).map_err(|e| {
                    PipelineError::CompressionFailed(format!("Final write failed: {e}"))
                })?;
            }
            CompressionAlgorithm::None => {
                total_written = std::io::copy(&mut reader, &mut writer).map_err(|e| {
                    PipelineError::CompressionFailed(format!("Direct copy failed: {e}"))
                })? as usize;
                total_read = total_written;
            }
            CompressionAlgorithm::Zstd => {
                let level = self.config.level.min(22) as i32;
                let mut input_buffer = Vec::new();
                reader.read_to_end(&mut input_buffer).map_err(|e| {
                    PipelineError::CompressionFailed(format!("Stream read failed: {e}"))
                })?;
                total_read = input_buffer.len();

                let compressed =
                    zstd::encode_all(std::io::Cursor::new(&input_buffer), level).map_err(|e| {
                        PipelineError::CompressionFailed(format!(
                            "zstd stream compression failed: {e}"
                        ))
                    })?;

                total_written = compressed.len();
                writer.write_all(&compressed).map_err(|e| {
                    PipelineError::CompressionFailed(format!("Final write failed: {e}"))
                })?;
            }
            CompressionAlgorithm::Auto => {
                // For streaming, read all data first to detect, then compress in-memory
                let mut input_buffer = Vec::new();
                reader.read_to_end(&mut input_buffer).map_err(|e| {
                    PipelineError::CompressionFailed(format!("Stream read failed: {e}"))
                })?;
                total_read = input_buffer.len();

                let algo =
                    self.select_algorithm("application/octet-stream", input_buffer.len());
                let mut temp_config = self.config.clone();
                temp_config.algorithm = algo;
                let temp = Compressor::new(temp_config);

                let (compressed, _) = temp.compress(&input_buffer)?;
                total_written = compressed.len();
                writer.write_all(&compressed).map_err(|e| {
                    PipelineError::CompressionFailed(format!("Final write failed: {e}"))
                })?;
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        Ok(CompressionStats::calculate(
            total_read,
            total_written,
            duration_ms,
        ))
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

        let (compressed, stats) = compressor.compress(&data).expect("test: compression operation");
        assert!(compressed.len() < data.len());
        assert!(stats.ratio < 1.0);

        let decompressed = compressor.decompress(&compressed).expect("test: compression operation");
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

        let (compressed, stats) = compressor.compress(&data).expect("test: compression operation");
        assert!(compressed.len() < data.len());
        assert!(stats.ratio < 0.1); // Brotli excels at text compression

        let decompressed = compressor.decompress(&compressed).expect("test: compression operation");
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

        let (compressed, stats) = compressor.compress(&data).expect("test: compression operation");
        assert_eq!(compressed, data);
        assert_eq!(stats.ratio, 1.0);
    }

    #[test]
    fn test_compression_stats() {
        let compressor = Compressor::default();
        let data = vec![0u8; 100000];

        let (_, stats) = compressor.compress(&data).expect("test: compression operation");
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

            let (compressed, stats) = compressor.compress(&data).expect("test: compression operation");
            let decompressed = compressor.decompress(&compressed).expect("test: compression operation");
            assert_eq!(decompressed, data);
            assert!(stats.ratio < 1.0);

            // Higher levels should generally give better compression
            if level > 1 {
                println!(
                    "Level {}: ratio = {:.3}, size = {}",
                    level,
                    stats.ratio,
                    compressed.len()
                );
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

        let stats = compressor
            .compress_stream(&mut reader, &mut output)
            .expect("test: expected success");

        assert!(output.len() < data.len());
        assert!(stats.ratio < 1.0);

        // Verify we can decompress
        let decompressed = compressor.decompress(&output).expect("test: compression operation");
        assert_eq!(decompressed, data);
    }

    // --- Zstd and Auto tests ---

    #[test]
    fn test_zstd_compression_roundtrip() {
        let data = b"Hello, zstd compression! This is a test of the zstd algorithm.".repeat(100);
        let compressor = Compressor::new(CompressionConfig {
            algorithm: CompressionAlgorithm::Zstd,
            level: 3,
            ..CompressionConfig::default()
        });
        let (compressed, stats) = compressor.compress(&data).expect("test: compress");
        assert!(compressed.len() < data.len(), "test: zstd should compress");
        assert!(stats.ratio < 1.0);

        let decompressed = compressor.decompress(&compressed).expect("test: decompress");
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_zstd_large_data() {
        let data: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();
        let compressor = Compressor::new(CompressionConfig {
            algorithm: CompressionAlgorithm::Zstd,
            level: 3,
            ..CompressionConfig::default()
        });
        let (compressed, _) = compressor.compress(&data).expect("test: compress 1MB");
        let decompressed = compressor.decompress(&compressed).expect("test: decompress 1MB");
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_auto_detect_video_skips_compression() {
        let compressor = Compressor::default();
        assert!(matches!(
            compressor.select_algorithm("video/mp4", 1000),
            CompressionAlgorithm::None
        ));
        assert!(matches!(
            compressor.select_algorithm("audio/mpeg", 1000),
            CompressionAlgorithm::None
        ));
        assert!(matches!(
            compressor.select_algorithm("image/jpeg", 1000),
            CompressionAlgorithm::None
        ));
        assert!(matches!(
            compressor.select_algorithm("application/zip", 1000),
            CompressionAlgorithm::None
        ));
    }

    #[test]
    fn test_auto_detect_text_uses_brotli() {
        let compressor = Compressor::default();
        assert!(matches!(
            compressor.select_algorithm("text/html", 5000),
            CompressionAlgorithm::Brotli
        ));
        assert!(matches!(
            compressor.select_algorithm("application/json", 1_000_000),
            CompressionAlgorithm::Brotli
        ));
    }

    #[test]
    fn test_auto_detect_large_text_uses_zstd() {
        let compressor = Compressor::default();
        // Text > 10MB uses zstd
        assert!(matches!(
            compressor.select_algorithm("text/plain", 20_000_000),
            CompressionAlgorithm::Zstd
        ));
    }

    #[test]
    fn test_auto_detect_binary_uses_zstd() {
        let compressor = Compressor::default();
        assert!(matches!(
            compressor.select_algorithm("application/octet-stream", 1000),
            CompressionAlgorithm::Zstd
        ));
        assert!(matches!(
            compressor.select_algorithm("application/pdf", 1000),
            CompressionAlgorithm::Zstd
        ));
    }
}
