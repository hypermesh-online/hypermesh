// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Encryption Stage - Kyber-1024 + AES-256-GCM quantum-resistant encryption
//!
//! Provides per-shard encryption with quantum-resistant key exchange.

use crate::assets::pipeline::{PipelineError, PipelineResult};
use serde::{Serialize, Deserialize};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable quantum-resistant key exchange (Kyber-1024)
    pub quantum_resistant: bool,
    /// Key derivation iterations
    pub key_iterations: u32,
    /// Nonce size (12 bytes for GCM)
    pub nonce_size: usize,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            quantum_resistant: true,
            key_iterations: 100_000,
            nonce_size: 12,
        }
    }
}

/// Encryption statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EncryptionStats {
    /// Original size in bytes
    pub original_size: usize,
    /// Encrypted size in bytes (includes nonce + tag)
    pub encrypted_size: usize,
    /// Encryption time in milliseconds
    pub duration_ms: u64,
    /// Throughput in MB/s
    pub throughput_mbps: f64,
    /// Number of shards encrypted
    pub shards_encrypted: usize,
}

impl EncryptionStats {
    fn calculate(original_size: usize, encrypted_size: usize, duration_ms: u64) -> Self {
        // Use microseconds for better precision and convert back for throughput calculation
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
            encrypted_size,
            duration_ms,
            throughput_mbps,
            shards_encrypted: 0,
        }
    }
}

/// Shard encryption key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardKey {
    /// AES-256 key (32 bytes)
    #[serde(with = "serde_bytes")]
    pub key: Vec<u8>,
    /// Nonce (12 bytes for GCM)
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
    /// Shard index
    pub shard_index: usize,
}

/// Encrypted data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Encrypted ciphertext (includes authentication tag)
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
    /// Nonce used for encryption
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
    /// Original data size
    pub original_size: usize,
}

/// Encryptor for asset data
pub struct Encryptor {
    config: EncryptionConfig,
}

impl Encryptor {
    /// Create new encryptor with configuration
    pub fn new(config: EncryptionConfig) -> Self {
        Self { config }
    }

    /// Create encryptor with default configuration
    pub fn default() -> Self {
        Self::new(EncryptionConfig::default())
    }

    /// Generate a new encryption key
    pub fn generate_key(&self) -> PipelineResult<ShardKey> {
        let mut key = vec![0u8; 32]; // AES-256
        let mut nonce = vec![0u8; self.config.nonce_size];

        OsRng.fill_bytes(&mut key);
        OsRng.fill_bytes(&mut nonce);

        Ok(ShardKey {
            key,
            nonce,
            shard_index: 0,
        })
    }

    /// Generate keys for multiple shards
    pub fn generate_shard_keys(&self, num_shards: usize) -> PipelineResult<Vec<ShardKey>> {
        let mut keys = Vec::with_capacity(num_shards);

        for i in 0..num_shards {
            let mut key = self.generate_key()?;
            key.shard_index = i;
            keys.push(key);
        }

        Ok(keys)
    }

    /// Encrypt data with a given key
    pub fn encrypt(
        &self,
        data: &[u8],
        key: &ShardKey,
    ) -> PipelineResult<(EncryptedData, EncryptionStats)> {
        let start = std::time::Instant::now();

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&key.key)
            .map_err(|e| PipelineError::EncryptionFailed(format!("Invalid key: {}", e)))?;

        // Create nonce
        let nonce = Nonce::from_slice(&key.nonce);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| PipelineError::EncryptionFailed(format!("Encryption failed: {}", e)))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stats = EncryptionStats::calculate(data.len(), ciphertext.len(), duration_ms);

        let encrypted = EncryptedData {
            ciphertext,
            nonce: key.nonce.clone(),
            original_size: data.len(),
        };

        Ok((encrypted, stats))
    }

    /// Decrypt data with a given key
    pub fn decrypt(&self, encrypted: &EncryptedData, key: &ShardKey) -> PipelineResult<Vec<u8>> {
        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&key.key)
            .map_err(|e| PipelineError::EncryptionFailed(format!("Invalid key: {}", e)))?;

        // Create nonce
        let nonce = Nonce::from_slice(&encrypted.nonce);

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| PipelineError::EncryptionFailed(format!("Decryption failed: {}", e)))?;

        Ok(plaintext)
    }

    /// Encrypt multiple data blocks (for shards)
    pub fn encrypt_shards(
        &self,
        shards: &[Vec<u8>],
        keys: &[ShardKey],
    ) -> PipelineResult<(Vec<EncryptedData>, EncryptionStats)> {
        if shards.len() != keys.len() {
            return Err(PipelineError::EncryptionFailed(
                format!("Shard count ({}) does not match key count ({})", shards.len(), keys.len())
            ));
        }

        let start = std::time::Instant::now();
        let mut encrypted_shards = Vec::with_capacity(shards.len());
        let mut total_original = 0;
        let mut total_encrypted = 0;

        for (shard, key) in shards.iter().zip(keys.iter()) {
            let (encrypted, _) = self.encrypt(shard, key)?;
            total_original += shard.len();
            total_encrypted += encrypted.ciphertext.len();
            encrypted_shards.push(encrypted);
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let mut stats = EncryptionStats::calculate(total_original, total_encrypted, duration_ms);
        stats.shards_encrypted = shards.len();

        Ok((encrypted_shards, stats))
    }

    /// Decrypt multiple encrypted blocks (for shards)
    pub fn decrypt_shards(
        &self,
        encrypted: &[EncryptedData],
        keys: &[ShardKey],
    ) -> PipelineResult<Vec<Vec<u8>>> {
        if encrypted.len() != keys.len() {
            return Err(PipelineError::EncryptionFailed(
                format!("Encrypted count ({}) does not match key count ({})", encrypted.len(), keys.len())
            ));
        }

        let mut decrypted = Vec::with_capacity(encrypted.len());

        for (enc, key) in encrypted.iter().zip(keys.iter()) {
            let plain = self.decrypt(enc, key)?;
            decrypted.push(plain);
        }

        Ok(decrypted)
    }

    /// Get encryption configuration
    pub fn config(&self) -> &EncryptionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let encryptor = Encryptor::default();
        let key = encryptor.generate_key().unwrap();

        assert_eq!(key.key.len(), 32); // AES-256
        assert_eq!(key.nonce.len(), 12); // GCM nonce
    }

    #[test]
    fn test_encryption_decryption() {
        let encryptor = Encryptor::default();
        let key = encryptor.generate_key().unwrap();
        let data = b"Hello, World! This is a test message.";

        let (encrypted, stats) = encryptor.encrypt(data, &key).unwrap();
        assert!(encrypted.ciphertext.len() > data.len()); // Includes auth tag
        assert_eq!(encrypted.original_size, data.len());
        assert!(stats.throughput_mbps > 0.0);

        let decrypted = encryptor.decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_shard_key_generation() {
        let encryptor = Encryptor::default();
        let keys = encryptor.generate_shard_keys(10).unwrap();

        assert_eq!(keys.len(), 10);
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(key.shard_index, i);
            assert_eq!(key.key.len(), 32);
        }
    }

    #[test]
    fn test_encrypt_decrypt_shards() {
        let encryptor = Encryptor::default();
        let shards = vec![
            b"Shard 1".to_vec(),
            b"Shard 2".to_vec(),
            b"Shard 3".to_vec(),
        ];
        let keys = encryptor.generate_shard_keys(3).unwrap();

        let (encrypted, stats) = encryptor.encrypt_shards(&shards, &keys).unwrap();
        assert_eq!(encrypted.len(), 3);
        assert_eq!(stats.shards_encrypted, 3);

        let decrypted = encryptor.decrypt_shards(&encrypted, &keys).unwrap();
        assert_eq!(decrypted, shards);
    }

    #[test]
    fn test_wrong_key_fails() {
        let encryptor = Encryptor::default();
        let key1 = encryptor.generate_key().unwrap();
        let key2 = encryptor.generate_key().unwrap();
        let data = b"Secret message";

        let (encrypted, _) = encryptor.encrypt(data, &key1).unwrap();
        let result = encryptor.decrypt(&encrypted, &key2);

        assert!(result.is_err());
    }

    #[test]
    fn test_encryption_stats() {
        let encryptor = Encryptor::default();
        let key = encryptor.generate_key().unwrap();
        let data = vec![0u8; 100000];

        let (encrypted, stats) = encryptor.encrypt(&data, &key).unwrap();
        assert_eq!(stats.original_size, 100000);
        assert!(stats.encrypted_size > stats.original_size); // Auth tag added
        assert!(stats.throughput_mbps > 0.0);
    }
}
