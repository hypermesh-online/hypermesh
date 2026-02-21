// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Threshold Cryptography — Shamir's Secret Sharing over GF(256)
//!
//! Distributes FALCON-1024 CA signing authority across multiple nodes so that
//! no single node holds the complete private key. Requires a configurable
//! threshold (t-of-n) of shares to reconstruct the key and produce a signature.

use std::time::SystemTime;

use anyhow::{anyhow, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::debug;

use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{DetachedSignature, SecretKey};

// ---------------------------------------------------------------------------
// GF(256) arithmetic (internal)
// ---------------------------------------------------------------------------

/// Irreducible polynomial for GF(256): x^8 + x^4 + x^3 + x + 1
const POLYNOMIAL: u16 = 0x11B;

/// Primitive element (generator) for GF(256) with polynomial 0x11B.
/// 3 is a primitive root generating all 255 nonzero elements.
const GENERATOR: u16 = 3;

/// Pre-computed exponent table: EXP_TABLE[i] = g^i in GF(256).
const EXP_TABLE: [u8; 256] = build_exp_table();
/// Pre-computed logarithm table: LOG_TABLE[a] = i where g^i = a.
/// LOG_TABLE[0] is unused (log of zero is undefined).
const LOG_TABLE: [u8; 256] = build_log_table();

/// Build the exponent table using generator 3 in GF(256) mod 0x11B.
const fn build_exp_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut v: u16 = 1; // g^0 = 1
    let mut i = 0usize;
    while i < 255 {
        table[i] = v as u8;
        // Multiply by generator in GF(256): v = v * GENERATOR
        v = gf_mul_const(v, GENERATOR);
        i += 1;
    }
    table[255] = table[0]; // wrap-around so that exp[255] = 1
    table
}

/// Constant-context GF(256) multiplication without tables (used only for
/// table generation). Russian-peasant / shift-and-add over GF(2)[x] mod poly.
const fn gf_mul_const(mut a: u16, mut b: u16) -> u16 {
    let mut result: u16 = 0;
    while b > 0 {
        if b & 1 != 0 {
            result ^= a;
        }
        a <<= 1;
        if a & 0x100 != 0 {
            a ^= POLYNOMIAL;
        }
        b >>= 1;
    }
    result & 0xFF
}

/// Build the logarithm table as the inverse of the exponent table.
const fn build_log_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    let exp = build_exp_table();
    let mut i = 0usize;
    while i < 255 {
        table[exp[i] as usize] = i as u8;
        i += 1;
    }
    // log[0] is undefined; we leave it as 0 and guard against it in gf_mul.
    table
}

/// GF(256) addition (XOR).
#[inline]
fn gf_add(a: u8, b: u8) -> u8 {
    a ^ b
}

/// GF(256) multiplication via log/exp tables.
#[inline]
fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let log_sum = (LOG_TABLE[a as usize] as u16 + LOG_TABLE[b as usize] as u16) % 255;
    EXP_TABLE[log_sum as usize]
}

/// GF(256) multiplicative inverse.
#[inline]
fn gf_inv(a: u8) -> u8 {
    assert!(a != 0, "cannot invert zero in GF(256)");
    let log_a = LOG_TABLE[a as usize] as u16;
    EXP_TABLE[(255 - log_a) as usize]
}

/// GF(256) division: a / b = a * inv(b).
#[inline]
fn gf_div(a: u8, b: u8) -> u8 {
    gf_mul(a, gf_inv(b))
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Configuration for a Shamir threshold scheme.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThresholdConfig {
    /// Minimum shares required to reconstruct the secret (t).
    pub threshold: u8,
    /// Total number of shares produced (n).
    pub total_shares: u8,
}

/// A single share of a secret, produced by Shamir's Secret Sharing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretShare {
    /// Share index (1-indexed, never 0).
    pub index: u8,
    /// Share data — same length as the original secret.
    pub data: Vec<u8>,
}

/// A share of a FALCON-1024 signing key together with metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyShare {
    /// The underlying secret share.
    pub share: SecretShare,
    /// SHA-256 fingerprint of the original *public* key (for binding).
    pub key_fingerprint: [u8; 32],
    /// Timestamp when this share was created.
    pub created_at: SystemTime,
}

// ---------------------------------------------------------------------------
// ThresholdScheme — generic Shamir SSS
// ---------------------------------------------------------------------------

/// Shamir's Secret Sharing over GF(256).
#[derive(Clone, Debug)]
pub struct ThresholdScheme {
    config: ThresholdConfig,
}

impl ThresholdScheme {
    /// Create a new threshold scheme after validating parameters.
    pub fn new(config: ThresholdConfig) -> Result<Self> {
        if config.threshold == 0 {
            return Err(anyhow!("threshold must be > 0"));
        }
        if config.total_shares == 0 {
            return Err(anyhow!("total_shares must be > 0"));
        }
        if config.threshold > config.total_shares {
            return Err(anyhow!(
                "threshold ({}) must be <= total_shares ({})",
                config.threshold,
                config.total_shares
            ));
        }
        Ok(Self { config })
    }

    /// Split `secret` into `n` shares such that any `t` shares can reconstruct it.
    ///
    /// Each byte of the secret is treated as a separate GF(256) element and
    /// protected by an independent random polynomial of degree `t - 1`.
    pub fn split_secret(&self, secret: &[u8]) -> Result<Vec<SecretShare>> {
        if secret.is_empty() {
            return Err(anyhow!("secret must not be empty"));
        }

        let t = self.config.threshold as usize;
        let n = self.config.total_shares as usize;

        // Pre-allocate share data.
        let mut shares: Vec<SecretShare> = (1..=n as u8)
            .map(|idx| SecretShare {
                index: idx,
                data: Vec::with_capacity(secret.len()),
            })
            .collect();

        // Random coefficients buffer (t-1 per byte of secret).
        let coeff_count = t - 1;
        let mut rng = rand::thread_rng();
        let mut coeffs = vec![0u8; coeff_count];

        for &secret_byte in secret {
            // Generate random coefficients for this byte's polynomial.
            rng.fill_bytes(&mut coeffs);

            // Evaluate polynomial at each share index x = 1..n.
            for share in &mut shares {
                let x = share.index;
                // P(x) = secret_byte + c1*x + c2*x^2 + ... + c_{t-1}*x^{t-1}
                let mut value = secret_byte;
                let mut x_pow = x; // x^1
                for &c in &coeffs {
                    value = gf_add(value, gf_mul(c, x_pow));
                    x_pow = gf_mul(x_pow, x);
                }
                share.data.push(value);
            }
        }

        debug!(
            "split secret ({} bytes) into {} shares with threshold {}",
            secret.len(),
            n,
            t
        );
        Ok(shares)
    }

    /// Reconstruct the original secret from at least `t` shares using
    /// Lagrange interpolation over GF(256).
    pub fn reconstruct_secret(&self, shares: &[SecretShare]) -> Result<Vec<u8>> {
        let t = self.config.threshold as usize;
        if shares.len() < t {
            return Err(anyhow!(
                "insufficient shares: need {} but got {}",
                t,
                shares.len()
            ));
        }

        // Use exactly the first `t` shares (any subset of size t works).
        let subset = &shares[..t];
        let secret_len = subset[0].data.len();

        // Validate all shares have the same length.
        for s in subset {
            if s.data.len() != secret_len {
                return Err(anyhow!(
                    "share {} has length {} but expected {}",
                    s.index,
                    s.data.len(),
                    secret_len
                ));
            }
        }

        // Pre-compute Lagrange basis coefficients (evaluated at x = 0).
        let lagrange: Vec<u8> = subset
            .iter()
            .enumerate()
            .map(|(i, si)| {
                let xi = si.index;
                let mut basis = 1u8;
                for (j, sj) in subset.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    let xj = sj.index;
                    // basis *= (0 - xj) / (xi - xj)  in GF(256)
                    // 0 - xj = xj (additive inverse is identity in GF(2^k))
                    basis = gf_mul(basis, gf_div(xj, gf_add(xi, xj)));
                }
                basis
            })
            .collect();

        // Interpolate each byte position.
        let mut secret = Vec::with_capacity(secret_len);
        for byte_idx in 0..secret_len {
            let mut value = 0u8;
            for (i, share) in subset.iter().enumerate() {
                value = gf_add(value, gf_mul(lagrange[i], share.data[byte_idx]));
            }
            secret.push(value);
        }

        debug!("reconstructed secret ({} bytes) from {} shares", secret_len, t);
        Ok(secret)
    }
}

// ---------------------------------------------------------------------------
// ThresholdSigner — FALCON-1024 key splitting / threshold signing
// ---------------------------------------------------------------------------

/// Wraps [`ThresholdScheme`] to split and reconstruct FALCON-1024 private keys,
/// then produce signatures without any single party holding the full key.
#[derive(Clone, Debug)]
pub struct ThresholdSigner {
    scheme: ThresholdScheme,
}

impl ThresholdSigner {
    /// Create a new threshold signer with the given configuration.
    pub fn new(config: ThresholdConfig) -> Result<Self> {
        let scheme = ThresholdScheme::new(config)?;
        Ok(Self { scheme })
    }

    /// Split a FALCON-1024 private key into [`KeyShare`]s.
    ///
    /// `private_key_bytes` must be a valid FALCON-1024 secret key.
    /// `public_key_fingerprint` binds shares to the matching public key.
    pub fn split_signing_key(
        &self,
        private_key_bytes: &[u8],
        public_key_fingerprint: [u8; 32],
    ) -> Result<Vec<KeyShare>> {
        if private_key_bytes.is_empty() {
            return Err(anyhow!("private key bytes must not be empty"));
        }

        let shares = self.scheme.split_secret(private_key_bytes)?;
        let now = SystemTime::now();

        let key_shares: Vec<KeyShare> = shares
            .into_iter()
            .map(|share| KeyShare {
                share,
                key_fingerprint: public_key_fingerprint,
                created_at: now,
            })
            .collect();

        debug!(
            "split FALCON-1024 key ({} bytes) into {} key shares",
            private_key_bytes.len(),
            key_shares.len()
        );
        Ok(key_shares)
    }

    /// Reconstruct a FALCON-1024 private key from key shares and immediately
    /// sign `message`, returning the raw FALCON-1024 detached signature bytes.
    ///
    /// The reconstructed key is never persisted — it exists only for the
    /// duration of this call.
    pub fn reconstruct_and_sign(
        &self,
        shares: &[KeyShare],
        message: &[u8],
    ) -> Result<Vec<u8>> {
        if shares.is_empty() {
            return Err(anyhow!("no key shares provided"));
        }

        // Validate that all shares reference the same key fingerprint.
        let expected_fp = shares[0].key_fingerprint;
        for ks in shares.iter().skip(1) {
            if ks.key_fingerprint != expected_fp {
                return Err(anyhow!("key share fingerprint mismatch: shares belong to different keys"));
            }
        }

        // Extract inner SecretShares for reconstruction.
        let inner_shares: Vec<SecretShare> = shares.iter().map(|ks| ks.share.clone()).collect();
        let reconstructed_key_bytes = self.scheme.reconstruct_secret(&inner_shares)?;

        // Re-hydrate the FALCON-1024 secret key.
        let secret_key = falcon1024::SecretKey::from_bytes(&reconstructed_key_bytes)
            .map_err(|e| anyhow!("failed to reconstruct FALCON-1024 secret key: {}", e))?;

        // Hash the message (matching FalconCrypto::sign convention).
        let message_hash = sha256_hash(message);

        // Sign.
        let signature = falcon1024::detached_sign(&message_hash, &secret_key);
        let sig_bytes = signature.as_bytes().to_vec();

        debug!(
            "threshold-signed message ({} bytes) producing {} byte signature",
            message.len(),
            sig_bytes.len()
        );
        Ok(sig_bytes)
    }
}

/// SHA-256 helper consistent with `FalconCrypto::hash_message`.
fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ThresholdConfig validation -----------------------------------------

    #[test]
    fn test_config_validation() {
        // threshold > total_shares must fail
        let result = ThresholdScheme::new(ThresholdConfig {
            threshold: 4,
            total_shares: 3,
        });
        assert!(result.is_err(), "threshold > total_shares should fail");

        // threshold == 0 must fail
        let result = ThresholdScheme::new(ThresholdConfig {
            threshold: 0,
            total_shares: 5,
        });
        assert!(result.is_err(), "threshold == 0 should fail");

        // total_shares == 0 must fail
        let result = ThresholdScheme::new(ThresholdConfig {
            threshold: 0,
            total_shares: 0,
        });
        assert!(result.is_err(), "total_shares == 0 should fail");

        // valid config succeeds
        let result = ThresholdScheme::new(ThresholdConfig {
            threshold: 3,
            total_shares: 5,
        });
        assert!(result.is_ok(), "valid 3-of-5 config should succeed");
    }

    // -- split + reconstruct: exact threshold -------------------------------

    #[test]
    fn test_split_and_reconstruct_exact_threshold() {
        let scheme = ThresholdScheme::new(ThresholdConfig {
            threshold: 3,
            total_shares: 5,
        })
        .expect("test: valid config");

        let secret = b"threshold cryptography secret data for testing";
        let shares = scheme.split_secret(secret).expect("test: split");
        assert_eq!(shares.len(), 5);

        // Use exactly 3 shares (first 3).
        let reconstructed = scheme
            .reconstruct_secret(&shares[..3])
            .expect("test: reconstruct");
        assert_eq!(reconstructed, secret);
    }

    // -- split + reconstruct: all shares ------------------------------------

    #[test]
    fn test_split_and_reconstruct_all_shares() {
        let scheme = ThresholdScheme::new(ThresholdConfig {
            threshold: 3,
            total_shares: 5,
        })
        .expect("test: valid config");

        let secret = b"all-shares reconstruction test";
        let shares = scheme.split_secret(secret).expect("test: split");

        let reconstructed = scheme
            .reconstruct_secret(&shares)
            .expect("test: reconstruct with all shares");
        assert_eq!(reconstructed, secret);
    }

    // -- insufficient shares must fail --------------------------------------

    #[test]
    fn test_insufficient_shares_fails() {
        let scheme = ThresholdScheme::new(ThresholdConfig {
            threshold: 3,
            total_shares: 5,
        })
        .expect("test: valid config");

        let secret = b"need at least 3 shares";
        let shares = scheme.split_secret(secret).expect("test: split");

        let result = scheme.reconstruct_secret(&shares[..2]);
        assert!(result.is_err(), "2 of 3-of-5 should fail");
    }

    // -- different share combinations all produce the same secret -----------

    #[test]
    fn test_different_share_combinations() {
        let scheme = ThresholdScheme::new(ThresholdConfig {
            threshold: 3,
            total_shares: 5,
        })
        .expect("test: valid config");

        let secret = b"any three of five should work";
        let shares = scheme.split_secret(secret).expect("test: split");

        // Try several distinct 3-element subsets.
        let combos: Vec<Vec<usize>> = vec![
            vec![0, 1, 2],
            vec![0, 1, 4],
            vec![0, 3, 4],
            vec![1, 2, 3],
            vec![2, 3, 4],
            vec![0, 2, 4],
        ];

        for combo in &combos {
            let subset: Vec<SecretShare> = combo.iter().map(|&i| shares[i].clone()).collect();
            let reconstructed = scheme
                .reconstruct_secret(&subset)
                .expect("test: reconstruct combo");
            assert_eq!(
                reconstructed, secret,
                "failed for combination {:?}",
                combo
            );
        }
    }

    // -- empty secret must fail ---------------------------------------------

    #[test]
    fn test_empty_secret() {
        let scheme = ThresholdScheme::new(ThresholdConfig {
            threshold: 2,
            total_shares: 3,
        })
        .expect("test: valid config");

        let result = scheme.split_secret(b"");
        assert!(result.is_err(), "empty secret should fail");
    }

    // -- large secrets (FALCON-1024 key sized, 1281 bytes) ------------------

    #[test]
    fn test_large_secret() {
        let scheme = ThresholdScheme::new(ThresholdConfig {
            threshold: 3,
            total_shares: 5,
        })
        .expect("test: valid config");

        // Simulate a FALCON-1024 private key (1281 bytes) plus extra.
        let mut secret = vec![0u8; 2048];
        rand::thread_rng().fill_bytes(&mut secret);

        let shares = scheme.split_secret(&secret).expect("test: split large");
        assert_eq!(shares.len(), 5);
        for s in &shares {
            assert_eq!(s.data.len(), 2048);
        }

        let reconstructed = scheme
            .reconstruct_secret(&shares[1..4])
            .expect("test: reconstruct large");
        assert_eq!(reconstructed, secret);
    }

    // -- ThresholdSigner: split key, reconstruct, verify signature ----------

    #[tokio::test]
    async fn test_threshold_signer_split_and_sign() {
        use pqcrypto_traits::sign::PublicKey as PkTrait;

        // Generate a real FALCON-1024 keypair.
        let (pk_native, sk_native) = falcon1024::keypair();
        let pk_bytes = PkTrait::as_bytes(&pk_native).to_vec();
        let sk_bytes = SecretKey::as_bytes(&sk_native).to_vec();

        let fingerprint: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(b"FALCON-1024-KEY:");
            h.update(&pk_bytes);
            h.finalize().into()
        };

        let signer = ThresholdSigner::new(ThresholdConfig {
            threshold: 3,
            total_shares: 5,
        })
        .expect("test: valid config");

        // Split.
        let key_shares = signer
            .split_signing_key(&sk_bytes, fingerprint)
            .expect("test: split key");
        assert_eq!(key_shares.len(), 5);
        for ks in &key_shares {
            assert_eq!(ks.key_fingerprint, fingerprint);
        }

        // Sign with 3 shares.
        let message = b"threshold signed message for TrustChain";
        let signature_bytes = signer
            .reconstruct_and_sign(&key_shares[0..3], message)
            .expect("test: threshold sign");

        // Verify with the original public key using pqcrypto directly.
        let message_hash = sha256_hash(message);
        let sig = falcon1024::DetachedSignature::from_bytes(&signature_bytes)
            .expect("test: parse signature");
        let verify_result = falcon1024::verify_detached_signature(&sig, &message_hash, &pk_native);
        assert!(verify_result.is_ok(), "threshold signature should verify");
    }

    // -- GF(256) arithmetic smoke tests ------------------------------------

    #[test]
    fn test_gf256_basic_arithmetic() {
        // Addition is XOR.
        assert_eq!(gf_add(0x53, 0xCA), 0x53 ^ 0xCA);
        assert_eq!(gf_add(0xFF, 0xFF), 0);

        // Multiplication identity.
        assert_eq!(gf_mul(0x42, 1), 0x42);
        assert_eq!(gf_mul(1, 0x42), 0x42);

        // Multiplication by zero.
        assert_eq!(gf_mul(0x42, 0), 0);
        assert_eq!(gf_mul(0, 0x42), 0);

        // Division: a / a == 1 for all nonzero a.
        for a in 1..=255u8 {
            assert_eq!(gf_div(a, a), 1, "a/a should be 1 for a={}", a);
        }

        // Inverse round-trip: inv(inv(a)) == a.
        for a in 1..=255u8 {
            assert_eq!(gf_inv(gf_inv(a)), a, "double inverse of {} should be identity", a);
        }

        // mul(a, inv(a)) == 1.
        for a in 1..=255u8 {
            assert_eq!(gf_mul(a, gf_inv(a)), 1, "a * inv(a) should be 1 for a={}", a);
        }
    }

    // -- fingerprint mismatch in KeyShares ----------------------------------

    #[test]
    fn test_key_share_fingerprint_mismatch() {
        let signer = ThresholdSigner::new(ThresholdConfig {
            threshold: 2,
            total_shares: 3,
        })
        .expect("test: valid config");

        let mut secret = vec![0u8; 64];
        rand::thread_rng().fill_bytes(&mut secret);

        let mut shares = signer
            .split_signing_key(&secret, [0xAA; 32])
            .expect("test: split");

        // Tamper with the fingerprint on one share.
        shares[1].key_fingerprint = [0xBB; 32];

        let result = signer.reconstruct_and_sign(&shares[0..2], b"msg");
        assert!(result.is_err(), "mismatched fingerprints should fail");
    }
}
