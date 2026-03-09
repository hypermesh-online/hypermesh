// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cryptographic helpers for genesis authentication
//!
//! Provides TOTP computation/verification, ChaCha20-Poly1305 encrypt/decrypt,
//! Argon2id key derivation, base32 encoding, and recovery code generation.
//! Extracted from `genesis_auth` to keep production code under the 500-line gate.

use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use blake3::Hasher;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::Rng;

/// TOTP configuration
pub(crate) const TOTP_PERIOD: u64 = 30;
pub(crate) const TOTP_DIGITS: usize = 6;
pub(crate) const RECOVERY_CODE_COUNT: usize = 10;

/// Generate TOTP secret (32 random bytes)
pub(crate) fn generate_totp_secret() -> Vec<u8> {
    let mut secret = vec![0u8; 32];
    rand::thread_rng().fill(&mut secret[..]);
    secret
}

/// Generate recovery codes
pub(crate) fn generate_recovery_codes(count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            let code: String = (0..8)
                .map(|_| {
                    let chars: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
                    let idx = rand::thread_rng().gen_range(0..chars.len());
                    chars[idx] as char
                })
                .collect();
            code
        })
        .collect()
}

/// Hash recovery code with Blake3
pub(crate) fn hash_recovery_code(code: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(code.as_bytes());
    hasher.update(b"recovery_code_salt");
    format!("{}", hasher.finalize())
}

/// Derive key from passphrase using Argon2id
pub(crate) fn derive_password_key(passphrase: &str) -> Result<[u8; 32]> {
    let salt = SaltString::encode_b64(b"genesis_auth_salt_fixed_for_derivation")
        .map_err(|e| anyhow!("Salt encoding failed: {e}"))?;
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(passphrase.as_bytes(), &salt)
        .map_err(|e| anyhow!("Key derivation failed: {e}"))?;

    let hash_bytes = hash.hash.ok_or_else(|| anyhow!("No hash output"))?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash_bytes.as_bytes()[..32]);
    Ok(key)
}

/// Derive authentication key from passphrase + TOTP secret
pub(crate) fn derive_auth_key(passphrase: &str, totp_secret: &str) -> Result<[u8; 32]> {
    let combined = format!("{passphrase}{totp_secret}");
    let salt = SaltString::encode_b64(b"auth_key_salt_fixed_for_derivation")
        .map_err(|e| anyhow!("Salt encoding failed: {e}"))?;
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(combined.as_bytes(), &salt)
        .map_err(|e| anyhow!("Auth key derivation failed: {e}"))?;

    let hash_bytes = hash.hash.ok_or_else(|| anyhow!("No hash output"))?;
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash_bytes.as_bytes()[..32]);
    Ok(key)
}

/// Encrypt data with ChaCha20-Poly1305
pub(crate) fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce_bytes = rand::thread_rng().gen::<[u8; 12]>();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| anyhow!("Encryption failed: {e}"))?;

    // Prepend nonce to ciphertext
    let mut result = nonce_bytes.to_vec();
    result.append(&mut ciphertext);
    Ok(result)
}

/// Decrypt data with ChaCha20-Poly1305
pub(crate) fn decrypt_data(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if encrypted.len() < 12 {
        return Err(anyhow!("Invalid encrypted data: too short"));
    }

    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = Nonce::from_slice(&encrypted[..12]);
    let ciphertext = &encrypted[12..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {e}"))
}

/// Verify TOTP code against secret, allowing +-1 time step for clock skew
pub(crate) fn verify_totp(secret_base32: &str, code: &str) -> Result<bool> {
    if code.len() != TOTP_DIGITS {
        return Ok(false);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow!("System time error: {e}"))?
        .as_secs();

    let time_step = now / TOTP_PERIOD;
    let secret = decode_base32(secret_base32)?;

    for offset in [-1i64, 0, 1] {
        let step = (time_step as i64 + offset) as u64;
        let computed_code = compute_totp(&secret, step)?;
        if computed_code == code {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Compute TOTP code for given time step
pub(crate) fn compute_totp(secret: &[u8], time_step: u64) -> Result<String> {
    use hmac::{Hmac, Mac};
    use sha1::Sha1;

    type HmacSha1 = Hmac<Sha1>;

    let time_bytes = time_step.to_be_bytes();

    let mut mac = <HmacSha1 as hmac::Mac>::new_from_slice(secret)
        .map_err(|e| anyhow!("HMAC initialization failed: {e}"))?;
    mac.update(&time_bytes);
    let result = mac.finalize();
    let hash = result.into_bytes();

    // Dynamic truncation (RFC 6238)
    let offset = (hash[19] & 0x0f) as usize;
    let binary = u32::from_be_bytes([
        hash[offset] & 0x7f,
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ]);

    let code = binary % 1_000_000;
    Ok(format!("{code:06}"))
}

/// Generate keypair (placeholder - would be FALCON-1024 in production)
pub(crate) fn generate_keypair() -> (Vec<u8>, Vec<u8>) {
    let mut private_key = vec![0u8; 64];
    let mut public_key = vec![0u8; 32];

    rand::thread_rng().fill(&mut private_key[..]);
    rand::thread_rng().fill(&mut public_key[..]);

    (private_key, public_key)
}

/// Encode bytes to base32
pub(crate) fn encode_base32(data: &[u8]) -> String {
    base32::encode(base32::Alphabet::Rfc4648 { padding: false }, data)
}

/// Decode base32 to bytes
pub(crate) fn decode_base32(data: &str) -> Result<Vec<u8>> {
    base32::decode(base32::Alphabet::Rfc4648 { padding: false }, data)
        .ok_or_else(|| anyhow!("Invalid base32 encoding"))
}
