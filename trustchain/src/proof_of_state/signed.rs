// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Signed proof types for bilateral authentication.
//!
//! Two representations exist:
//!
//! - [`WireSignedProof`] — **serialization format** used on the wire.
//!   Contains raw bytes (`proof_bytes`) so it can be serialized/deserialized
//!   without knowing the inner proof structure.
//!
//! - [`SignedStateProof`] — **in-memory deserialized** form.
//!   Contains a fully parsed [`StateProof`] plus its signature metadata,
//!   making it convenient for programmatic inspection.
//!
//! Use `From`/`TryFrom` conversions to move between the two.

use anyhow::{anyhow, Result};
use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};

use super::StateProof;

/// Wire format for FALCON-signed state proofs.
///
/// This envelope wraps a serialized `StateProof` with a FALCON-1024 detached
/// signature, the signer's public key, and a replay-prevention nonce. It is
/// the on-the-wire format so that every proof exchanged during bilateral
/// handshakes is cryptographically bound to the signing node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireSignedProof {
    /// JSON-serialized `StateProof`
    pub proof_bytes: Vec<u8>,
    /// FALCON-1024 detached signature over `BLAKE3(proof_bytes || nonce)`
    pub signature: Vec<u8>,
    /// Signer's full FALCON-1024 public key
    pub signer_pubkey: Vec<u8>,
    /// Random nonce to prevent replay attacks
    pub nonce: [u8; 32],
}

/// A state proof signed with FALCON-1024 for bilateral authentication.
///
/// In-memory deserialized form: wraps a fully parsed [`StateProof`] with its
/// signature metadata. Use this when you need to inspect proof fields directly.
/// Convert to [`WireSignedProof`] for network transmission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedStateProof {
    /// The underlying four-proof state proof
    pub proof: StateProof,
    /// FALCON-1024 detached signature over BLAKE3(proof_bytes || nonce)
    pub signature: Vec<u8>,
    /// Signer's FALCON-1024 public key
    pub signer_pubkey: Vec<u8>,
    /// Random nonce to prevent replay attacks
    pub nonce: [u8; 32],
}

// ---------------------------------------------------------------------------
// From / TryFrom conversions
// ---------------------------------------------------------------------------

impl From<SignedStateProof> for WireSignedProof {
    /// Convert an in-memory `SignedStateProof` to wire format.
    ///
    /// The inner `StateProof` is serialized to bincode bytes for the
    /// `proof_bytes` field. This matches the signing input format.
    fn from(signed: SignedStateProof) -> Self {
        // Use bincode to match the signing format used in SignedStateProof::sign()
        let proof_bytes = signed
            .proof
            .to_bytes()
            .unwrap_or_default();

        Self {
            proof_bytes,
            signature: signed.signature,
            signer_pubkey: signed.signer_pubkey,
            nonce: signed.nonce,
        }
    }
}

impl TryFrom<WireSignedProof> for SignedStateProof {
    type Error = anyhow::Error;

    /// Convert a wire-format proof to the in-memory deserialized form.
    ///
    /// Fails if the inner `proof_bytes` cannot be deserialized as a
    /// `StateProof` (tries bincode first, then JSON for forward compat).
    fn try_from(wire: WireSignedProof) -> Result<Self> {
        // Try bincode first (canonical format)
        let proof = StateProof::from_bytes(&wire.proof_bytes)
            .or_else(|_| {
                serde_json::from_slice::<StateProof>(&wire.proof_bytes)
                    .map_err(|e| anyhow!("Failed to deserialize proof_bytes: {e}"))
            })?;

        Ok(Self {
            proof,
            signature: wire.signature,
            signer_pubkey: wire.signer_pubkey,
            nonce: wire.nonce,
        })
    }
}

// ---------------------------------------------------------------------------
// SignedStateProof methods
// ---------------------------------------------------------------------------

impl SignedStateProof {
    /// Create a signed state proof using FALCON-1024 keys.
    pub fn sign(proof: StateProof, secret_key: &[u8], public_key: &[u8]) -> Result<Self> {
        use rand::RngCore;
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);

        let proof_bytes = proof.to_bytes()?;
        let mut message_input = Vec::with_capacity(proof_bytes.len() + 32);
        message_input.extend_from_slice(&proof_bytes);
        message_input.extend_from_slice(&nonce);
        let message_hash = blake3::hash(&message_input);

        let sk = falcon1024::SecretKey::from_bytes(secret_key)
            .map_err(|e| anyhow!("Invalid FALCON-1024 secret key: {e}"))?;
        let sig = falcon1024::detached_sign(message_hash.as_bytes(), &sk);

        Ok(Self {
            proof,
            signature: sig.as_bytes().to_vec(),
            signer_pubkey: public_key.to_vec(),
            nonce,
        })
    }

    /// Verify the FALCON-1024 signature on this proof.
    pub fn verify(&self) -> Result<bool> {
        let proof_bytes = self.proof.to_bytes()?;
        let mut message_input = Vec::with_capacity(proof_bytes.len() + 32);
        message_input.extend_from_slice(&proof_bytes);
        message_input.extend_from_slice(&self.nonce);
        let message_hash = blake3::hash(&message_input);

        let pk = falcon1024::PublicKey::from_bytes(&self.signer_pubkey)
            .map_err(|e| anyhow!("Invalid FALCON-1024 public key: {e}"))?;
        let sig = falcon1024::DetachedSignature::from_bytes(&self.signature)
            .map_err(|e| anyhow!("Invalid FALCON-1024 signature: {e}"))?;

        Ok(falcon1024::verify_detached_signature(&sig, message_hash.as_bytes(), &pk).is_ok())
    }

    /// Derive the signer's node ID (BLAKE3 hex of public key).
    pub fn signer_node_id(&self) -> String {
        blake3::hash(&self.signer_pubkey).to_hex().to_string()
    }

    /// Serialize for network transmission (JSON).
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| anyhow!("Failed to serialize SignedStateProof: {e}"))
    }

    /// Deserialize from network transmission (JSON).
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data)
            .map_err(|e| anyhow!("Failed to deserialize SignedStateProof: {e}"))
    }
}
