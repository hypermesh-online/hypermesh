// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Encryption Stage - Kyber-1024 KEM + AES-256-GCM whole-blob encryption
//!
//! Encrypts the entire compressed blob before sharding using quantum-resistant
//! Kyber-1024 key encapsulation mechanism (KEM) to establish a shared secret,
//! then AES-256-GCM for symmetric encryption of the data.
//!
//! Pipeline order: Compress -> **Encrypt (whole blob)** -> Shard -> Distribute

use crate::assets::pipeline::key_derivation::derive_segment_key;
use crate::assets::pipeline::{PipelineError, PipelineResult};
use serde::{Deserialize, Serialize};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use blake3;
use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{Ciphertext, PublicKey, SecretKey, SharedSecret};
use rand::RngCore;

// ── Configuration ────────────────────────────────────────────────────────────

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable quantum-resistant key exchange (Kyber-1024 KEM + AES-256-GCM).
    /// When false, falls back to plain AES-256-GCM with a random key.
    pub quantum_resistant: bool,
    /// Nonce size in bytes (12 for AES-GCM)
    pub nonce_size: usize,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            quantum_resistant: true,
            nonce_size: 12,
        }
    }
}

// ── Statistics ────────────────────────────────────────────────────────────────

/// Encryption statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EncryptionStats {
    /// Original size in bytes
    pub original_size: usize,
    /// Encrypted size in bytes (AES ciphertext including auth tag)
    pub encrypted_size: usize,
    /// Encryption time in milliseconds
    pub duration_ms: u64,
    /// Throughput in MB/s
    pub throughput_mbps: f64,
    /// Whether Kyber-1024 KEM was used
    pub quantum_resistant: bool,
}

impl EncryptionStats {
    fn calculate(original_size: usize, encrypted_size: usize, duration_ms: u64, qr: bool) -> Self {
        let throughput_mbps = if duration_ms > 0 {
            (original_size as f64 / (1024.0 * 1024.0)) / (duration_ms as f64 / 1000.0)
        } else if original_size > 0 {
            (original_size as f64 / (1024.0 * 1024.0)) / 0.001
        } else {
            0.0
        };

        Self {
            original_size,
            encrypted_size,
            duration_ms,
            throughput_mbps,
            quantum_resistant: qr,
        }
    }
}

// ── Key types ────────────────────────────────────────────────────────────────

/// Kyber-1024 key pair (public + secret) for KEM operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KyberKeyPair {
    /// Kyber-1024 public key bytes
    #[serde(with = "serde_bytes")]
    pub public_key: Vec<u8>,
    /// Kyber-1024 secret key bytes
    #[serde(with = "serde_bytes")]
    pub secret_key: Vec<u8>,
}

/// Result of Kyber-1024 KEM + AES-256-GCM encryption on the whole blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KyberEncryptionResult {
    /// Kyber KEM ciphertext (for key agreement / decapsulation)
    #[serde(with = "serde_bytes")]
    pub ciphertext_kem: Vec<u8>,
    /// AES-256-GCM encrypted data (includes 16-byte auth tag appended by aes-gcm)
    #[serde(with = "serde_bytes")]
    pub encrypted_data: Vec<u8>,
    /// 12-byte GCM nonce
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
    /// Pre-encryption size in bytes
    pub original_size: usize,
}

/// Legacy whole-blob encrypted data (plain AES-256-GCM, no Kyber).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// AES-256-GCM ciphertext (includes auth tag)
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
    /// 12-byte nonce
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
    /// Original data size
    pub original_size: usize,
}

/// Plain AES-256-GCM key (used when `quantum_resistant` is false).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AesKey {
    /// AES-256 key (32 bytes)
    #[serde(with = "serde_bytes")]
    pub key: Vec<u8>,
    /// Nonce (12 bytes)
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
}

// ── Encryptor ────────────────────────────────────────────────────────────────

/// Whole-blob encryptor using Kyber-1024 KEM + AES-256-GCM.
pub struct Encryptor {
    config: EncryptionConfig,
}

impl Encryptor {
    /// Create new encryptor with configuration
    pub fn new(config: EncryptionConfig) -> Self {
        Self { config }
    }

    /// Create encryptor with default (quantum-resistant) configuration
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(EncryptionConfig::default())
    }

    // ── Kyber keypair ────────────────────────────────────────────────────

    /// Generate a Kyber-1024 key pair for KEM-based encryption.
    pub fn generate_keypair(&self) -> PipelineResult<KyberKeyPair> {
        let (pk, sk) = kyber1024::keypair();

        let public_key = pk.as_bytes().to_vec();
        let secret_key = sk.as_bytes().to_vec();

        if public_key.len() != kyber1024::public_key_bytes() {
            return Err(PipelineError::EncryptionFailed(format!(
                "Kyber-1024 public key size mismatch: expected {}, got {}",
                kyber1024::public_key_bytes(),
                public_key.len()
            )));
        }
        if secret_key.len() != kyber1024::secret_key_bytes() {
            return Err(PipelineError::EncryptionFailed(format!(
                "Kyber-1024 secret key size mismatch: expected {}, got {}",
                kyber1024::secret_key_bytes(),
                secret_key.len()
            )));
        }

        Ok(KyberKeyPair {
            public_key,
            secret_key,
        })
    }

    // ── Encrypt (whole blob) ─────────────────────────────────────────────

    /// Encrypt entire data blob with Kyber-1024 KEM + AES-256-GCM.
    ///
    /// 1. `kyber1024::encapsulate(public_key)` produces `(shared_secret, ciphertext_kem)`
    /// 2. Derive AES-256 key from `shared_secret` via SHA-256
    /// 3. Generate random 12-byte nonce
    /// 4. AES-256-GCM encrypt the full data blob
    pub fn encrypt(
        &self,
        data: &[u8],
        public_key: &[u8],
    ) -> PipelineResult<(KyberEncryptionResult, EncryptionStats)> {
        let start = std::time::Instant::now();

        // Reconstruct Kyber public key
        let pk = kyber1024::PublicKey::from_bytes(public_key).map_err(|e| {
            PipelineError::EncryptionFailed(format!("Invalid Kyber-1024 public key: {e}"))
        })?;

        // KEM encapsulation
        let (shared_secret, kem_ct) = kyber1024::encapsulate(&pk);

        // Derive AES-256 key from shared secret
        let aes_key = derive_aes_key(shared_secret.as_bytes());

        // Generate random 12-byte nonce
        let mut nonce_bytes = vec![0u8; self.config.nonce_size];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);

        // AES-256-GCM encrypt entire blob
        let cipher = Aes256Gcm::new_from_slice(&aes_key)
            .map_err(|e| PipelineError::EncryptionFailed(format!("AES key init failed: {e}")))?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, data).map_err(|e| {
            PipelineError::EncryptionFailed(format!("AES-GCM encryption failed: {e}"))
        })?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stats = EncryptionStats::calculate(data.len(), ciphertext.len(), duration_ms, true);

        let result = KyberEncryptionResult {
            ciphertext_kem: kem_ct.as_bytes().to_vec(),
            encrypted_data: ciphertext,
            nonce: nonce_bytes,
            original_size: data.len(),
        };

        Ok((result, stats))
    }

    // ── Decrypt (whole blob) ─────────────────────────────────────────────

    /// Decrypt entire data blob with Kyber-1024 KEM + AES-256-GCM.
    ///
    /// 1. `kyber1024::decapsulate(ciphertext_kem, secret_key)` recovers `shared_secret`
    /// 2. Derive AES-256 key from `shared_secret` via SHA-256
    /// 3. AES-256-GCM decrypt using the stored nonce
    pub fn decrypt(
        &self,
        encrypted: &KyberEncryptionResult,
        secret_key: &[u8],
    ) -> PipelineResult<Vec<u8>> {
        // Reconstruct Kyber secret key
        let sk = kyber1024::SecretKey::from_bytes(secret_key).map_err(|e| {
            PipelineError::EncryptionFailed(format!("Invalid Kyber-1024 secret key: {e}"))
        })?;

        // Reconstruct KEM ciphertext
        let kem_ct = kyber1024::Ciphertext::from_bytes(&encrypted.ciphertext_kem).map_err(|e| {
            PipelineError::EncryptionFailed(format!("Invalid Kyber-1024 KEM ciphertext: {e}"))
        })?;

        // KEM decapsulation
        let shared_secret = kyber1024::decapsulate(&kem_ct, &sk);

        // Derive same AES-256 key
        let aes_key = derive_aes_key(shared_secret.as_bytes());

        // AES-256-GCM decrypt
        let cipher = Aes256Gcm::new_from_slice(&aes_key)
            .map_err(|e| PipelineError::EncryptionFailed(format!("AES key init failed: {e}")))?;
        let nonce = Nonce::from_slice(&encrypted.nonce);
        let plaintext = cipher
            .decrypt(nonce, encrypted.encrypted_data.as_ref())
            .map_err(|e| {
                PipelineError::EncryptionFailed(format!("AES-GCM decryption failed: {e}"))
            })?;

        Ok(plaintext)
    }

    // ── Plain AES-256-GCM helpers (non-quantum fallback) ─────────────────

    /// Generate a random AES-256 key + nonce (non-quantum fallback).
    pub fn generate_aes_key(&self) -> PipelineResult<AesKey> {
        let mut key = vec![0u8; 32];
        let mut nonce = vec![0u8; self.config.nonce_size];
        rand::thread_rng().fill_bytes(&mut key);
        rand::thread_rng().fill_bytes(&mut nonce);
        Ok(AesKey { key, nonce })
    }

    /// Encrypt data with plain AES-256-GCM (non-quantum fallback).
    pub fn encrypt_aes(
        &self,
        data: &[u8],
        key: &AesKey,
    ) -> PipelineResult<(EncryptedData, EncryptionStats)> {
        let start = std::time::Instant::now();

        let cipher = Aes256Gcm::new_from_slice(&key.key)
            .map_err(|e| PipelineError::EncryptionFailed(format!("Invalid AES key: {e}")))?;
        let nonce = Nonce::from_slice(&key.nonce);
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| PipelineError::EncryptionFailed(format!("AES encryption failed: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stats = EncryptionStats::calculate(data.len(), ciphertext.len(), duration_ms, false);

        let encrypted = EncryptedData {
            ciphertext,
            nonce: key.nonce.clone(),
            original_size: data.len(),
        };

        Ok((encrypted, stats))
    }

    /// Decrypt data with plain AES-256-GCM (non-quantum fallback).
    pub fn decrypt_aes(&self, encrypted: &EncryptedData, key: &AesKey) -> PipelineResult<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&key.key)
            .map_err(|e| PipelineError::EncryptionFailed(format!("Invalid AES key: {e}")))?;
        let nonce = Nonce::from_slice(&encrypted.nonce);
        let plaintext = cipher
            .decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| PipelineError::EncryptionFailed(format!("AES decryption failed: {e}")))?;
        Ok(plaintext)
    }

    // ── Per-segment encryption (streaming pipeline) ─────────────────────

    /// Encrypt a single segment with a derived key from the master key + segment index.
    /// Uses AES-256-GCM with deterministic key+nonce derived via BLAKE3 HKDF.
    pub fn encrypt_segment(
        &self,
        segment_data: &[u8],
        master_key: &[u8; 32],
        segment_index: u32,
    ) -> PipelineResult<(Vec<u8>, EncryptionStats)> {
        let start = std::time::Instant::now();
        let (key_bytes, nonce_bytes) = derive_segment_key(master_key, segment_index);

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, segment_data.as_ref())
            .map_err(|_| PipelineError::EncryptionFailed(
                format!("AES-GCM segment encryption failed for segment {}", segment_index)
            ))?;

        let encrypted_size = ciphertext.len();
        let duration = start.elapsed();
        let duration_ms = duration.as_millis() as u64;
        let throughput = if duration_ms > 0 {
            (segment_data.len() as f64 / (1024.0 * 1024.0)) / (duration_ms as f64 / 1000.0)
        } else if segment_data.is_empty() {
            0.0
        } else {
            (segment_data.len() as f64 / (1024.0 * 1024.0)) / 0.001
        };

        Ok((ciphertext, EncryptionStats {
            original_size: segment_data.len(),
            encrypted_size,
            duration_ms,
            throughput_mbps: throughput,
            quantum_resistant: true, // Key derived from Kyber KEM shared secret
        }))
    }

    /// Decrypt a single segment with a derived key from the master key + segment index.
    pub fn decrypt_segment(
        &self,
        encrypted_segment: &[u8],
        master_key: &[u8; 32],
        segment_index: u32,
    ) -> PipelineResult<Vec<u8>> {
        let (key_bytes, nonce_bytes) = derive_segment_key(master_key, segment_index);

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

        cipher
            .decrypt(nonce, encrypted_segment.as_ref())
            .map_err(|_| PipelineError::EncryptionFailed(
                format!("AES-GCM segment decryption failed for segment {}", segment_index)
            ))
    }

    /// Get encryption configuration
    pub fn config(&self) -> &EncryptionConfig {
        &self.config
    }
}

// ── Shared utility ───────────────────────────────────────────────────────────

/// Derive AES-256 key from Kyber shared secret via BLAKE3.
/// Domain-separated with "KYBER-1024-AES-KEY:" prefix (same as trustchain).
fn derive_aes_key(shared_secret: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"KYBER-1024-AES-KEY:");
    hasher.update(shared_secret);
    *hasher.finalize().as_bytes()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kyber_keypair_generation() {
        let encryptor = Encryptor::default();
        let kp = encryptor
            .generate_keypair()
            .expect("test: keypair generation");

        assert_eq!(
            kp.public_key.len(),
            kyber1024::public_key_bytes(),
            "public key size"
        );
        assert_eq!(
            kp.secret_key.len(),
            kyber1024::secret_key_bytes(),
            "secret key size"
        );
    }

    #[test]
    fn test_kyber_encrypt_decrypt_roundtrip() {
        let encryptor = Encryptor::default();
        let kp = encryptor
            .generate_keypair()
            .expect("test: keypair generation");
        let data = b"Hello, World! This is a quantum-resistant test message.";

        let (encrypted, stats) = encryptor
            .encrypt(data, &kp.public_key)
            .expect("test: encryption");

        assert!(stats.quantum_resistant);
        assert_eq!(stats.original_size, data.len());
        assert!(
            stats.encrypted_size > data.len(),
            "ciphertext includes auth tag"
        );
        assert!(stats.throughput_mbps > 0.0);
        assert_eq!(encrypted.original_size, data.len());
        assert!(!encrypted.ciphertext_kem.is_empty());
        assert_eq!(encrypted.nonce.len(), 12);

        let decrypted = encryptor
            .decrypt(&encrypted, &kp.secret_key)
            .expect("test: decryption");
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_wrong_secret_key_fails() {
        let encryptor = Encryptor::default();
        let kp1 = encryptor.generate_keypair().expect("test: keypair 1");
        let kp2 = encryptor.generate_keypair().expect("test: keypair 2");
        let data = b"Secret message";

        let (encrypted, _) = encryptor
            .encrypt(data, &kp1.public_key)
            .expect("test: encryption");

        let result = encryptor.decrypt(&encrypted, &kp2.secret_key);
        assert!(result.is_err(), "decryption with wrong key must fail");
    }

    #[test]
    fn test_large_blob_encrypt_decrypt() {
        let encryptor = Encryptor::default();
        let kp = encryptor
            .generate_keypair()
            .expect("test: keypair generation");

        // 1 MB blob
        let data = vec![0xABu8; 1024 * 1024];

        let (encrypted, stats) = encryptor
            .encrypt(&data, &kp.public_key)
            .expect("test: large encryption");

        assert_eq!(stats.original_size, 1024 * 1024);
        assert!(stats.encrypted_size > 1024 * 1024);

        let decrypted = encryptor
            .decrypt(&encrypted, &kp.secret_key)
            .expect("test: large decryption");
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_aes_fallback_encrypt_decrypt() {
        let config = EncryptionConfig {
            quantum_resistant: false,
            ..Default::default()
        };
        let encryptor = Encryptor::new(config);
        let key = encryptor
            .generate_aes_key()
            .expect("test: AES key generation");

        assert_eq!(key.key.len(), 32);
        assert_eq!(key.nonce.len(), 12);

        let data = b"Fallback AES-256-GCM test data";
        let (encrypted, stats) = encryptor
            .encrypt_aes(data, &key)
            .expect("test: AES encryption");

        assert!(!stats.quantum_resistant);
        assert!(encrypted.ciphertext.len() > data.len());

        let decrypted = encryptor
            .decrypt_aes(&encrypted, &key)
            .expect("test: AES decryption");
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encryption_stats_accuracy() {
        let encryptor = Encryptor::default();
        let kp = encryptor
            .generate_keypair()
            .expect("test: keypair generation");

        let data = vec![0u8; 100_000];
        let (_, stats) = encryptor
            .encrypt(&data, &kp.public_key)
            .expect("test: encryption");

        assert_eq!(stats.original_size, 100_000);
        assert!(stats.encrypted_size > 100_000, "auth tag adds bytes");
        assert!(stats.throughput_mbps > 0.0);
        assert!(stats.quantum_resistant);
    }

    // ── Per-segment encryption tests ─────────────────────────────────────

    #[test]
    fn test_segment_encrypt_decrypt_roundtrip() {
        let encryptor = Encryptor::default();
        let master_key = [42u8; 32];
        let data = b"Hello, segment encryption! This tests per-segment AES-GCM.";

        let (encrypted, stats) = encryptor
            .encrypt_segment(data, &master_key, 0)
            .expect("test: encrypt segment");
        assert_ne!(encrypted, data);
        assert!(stats.quantum_resistant);

        let decrypted = encryptor
            .decrypt_segment(&encrypted, &master_key, 0)
            .expect("test: decrypt segment");
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_segment_encrypt_different_indices_produce_different_ciphertext() {
        let encryptor = Encryptor::default();
        let master_key = [42u8; 32];
        let data = b"Same data, different segment indices";

        let (encrypted_0, _) = encryptor
            .encrypt_segment(data, &master_key, 0)
            .expect("test: encrypt seg 0");
        let (encrypted_1, _) = encryptor
            .encrypt_segment(data, &master_key, 1)
            .expect("test: encrypt seg 1");

        assert_ne!(encrypted_0, encrypted_1, "Different indices must produce different ciphertext");
    }

    #[test]
    fn test_segment_decrypt_wrong_index_fails() {
        let encryptor = Encryptor::default();
        let master_key = [42u8; 32];
        let data = b"Encrypted with index 0, decrypted with index 1";

        let (encrypted, _) = encryptor
            .encrypt_segment(data, &master_key, 0)
            .expect("test: encrypt");

        let result = encryptor.decrypt_segment(&encrypted, &master_key, 1);
        assert!(result.is_err(), "Wrong index must fail decryption");
    }

    #[test]
    fn test_segment_encrypt_deterministic() {
        let encryptor = Encryptor::default();
        let master_key = [42u8; 32];
        let data = b"Deterministic encryption test";

        let (encrypted_1, _) = encryptor
            .encrypt_segment(data, &master_key, 5)
            .expect("test: encrypt 1");
        let (encrypted_2, _) = encryptor
            .encrypt_segment(data, &master_key, 5)
            .expect("test: encrypt 2");

        assert_eq!(encrypted_1, encrypted_2, "Same input+key+index must produce same ciphertext");
    }

    #[test]
    fn test_kyber_kem_to_segment_encryption_flow() {
        use crate::assets::pipeline::key_derivation::derive_master_key;

        let encryptor = Encryptor::default();

        // Generate Kyber keypair
        let _keypair = encryptor.generate_keypair().expect("test: keypair");

        // Simulate KEM: use a fixed shared secret for deterministic test
        let fake_shared_secret = [0xABu8; 32];
        let master_key = derive_master_key(&fake_shared_secret);

        // Encrypt 3 segments
        let data_0 = b"Segment zero data content here";
        let data_1 = b"Segment one has different data";
        let data_2 = b"Final segment with more content";

        let (enc_0, _) = encryptor.encrypt_segment(data_0, &master_key, 0).expect("test: seg 0");
        let (enc_1, _) = encryptor.encrypt_segment(data_1, &master_key, 1).expect("test: seg 1");
        let (enc_2, _) = encryptor.encrypt_segment(data_2, &master_key, 2).expect("test: seg 2");

        // Decrypt in any order (random access)
        let dec_2 = encryptor.decrypt_segment(&enc_2, &master_key, 2).expect("test: dec 2");
        let dec_0 = encryptor.decrypt_segment(&enc_0, &master_key, 0).expect("test: dec 0");
        let dec_1 = encryptor.decrypt_segment(&enc_1, &master_key, 1).expect("test: dec 1");

        assert_eq!(dec_0, data_0);
        assert_eq!(dec_1, data_1);
        assert_eq!(dec_2, data_2);
    }
}
