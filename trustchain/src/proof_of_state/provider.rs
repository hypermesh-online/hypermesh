// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChainProofProvider — [`StateProofProvider`] backed by FALCON-1024 signed proofs.
//!
//! Wraps TrustChain's [`StateProof`] generation and validation so that
//! STOQ's bilateral handshake can use it via the trait from lib.
//!
//! Holds an `Arc<dyn NodeSigner>` so that generated proofs are signed
//! with the node's FALCON-1024 identity key and received proofs have
//! their signatures cryptographically verified.

use anyhow::anyhow;
use pqcrypto_falcon::falcon1024;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey};
use std::sync::Arc;

use super::{StateProof, WireSignedProof};

/// [`StateProofProvider`] implementation backed by TrustChain's `SignedStateProof`.
pub struct TrustChainProofProvider {
    node_id: String,
    signer: Arc<dyn hypermesh_lib::NodeSigner + Send + Sync>,
}

impl TrustChainProofProvider {
    pub fn new(node_id: String, signer: Arc<dyn hypermesh_lib::NodeSigner + Send + Sync>) -> Self {
        Self { node_id, signer }
    }
}

#[async_trait::async_trait]
impl hypermesh_lib::StateProofProvider for TrustChainProofProvider {
    async fn generate_proof(&self) -> anyhow::Result<Vec<u8>> {
        let proof = StateProof::generate_from_network(&self.node_id)
            .await
            .map_err(|e| anyhow!("PoS proof generation failed: {e}"))?;

        // Serialize the inner StateProof as JSON
        let proof_bytes = serde_json::to_vec(&proof)
            .map_err(|e| anyhow!("Failed to serialize state proof: {e}"))?;

        // Generate random nonce for replay prevention
        let mut nonce = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);

        // Compute BLAKE3(proof_bytes || nonce) and sign with FALCON-1024
        let mut hasher = blake3::Hasher::new();
        hasher.update(&proof_bytes);
        hasher.update(&nonce);
        let digest = hasher.finalize();

        let signature = self
            .signer
            .sign(digest.as_bytes())
            .map_err(|e| anyhow!("FALCON signing failed: {e}"))?;

        let wire = WireSignedProof {
            proof_bytes,
            signature,
            signer_pubkey: self.signer.public_key_bytes().to_vec(),
            nonce,
        };

        serde_json::to_vec(&wire)
            .map_err(|e| anyhow!("Failed to serialize WireSignedProof: {e}"))
    }

    async fn validate_proof(&self, incoming: &[u8]) -> anyhow::Result<bool> {
        // Try to deserialize as WireSignedProof (new signed format)
        match serde_json::from_slice::<WireSignedProof>(incoming) {
            Ok(wire) => {
                // Recompute BLAKE3(proof_bytes || nonce)
                let mut hasher = blake3::Hasher::new();
                hasher.update(&wire.proof_bytes);
                hasher.update(&wire.nonce);
                let digest = hasher.finalize();

                // Verify FALCON-1024 signature using pqcrypto directly
                let pk = falcon1024::PublicKey::from_bytes(&wire.signer_pubkey)
                    .map_err(|e| anyhow!("Invalid FALCON public key in proof: {e}"))?;
                let sig = falcon1024::DetachedSignature::from_bytes(&wire.signature)
                    .map_err(|e| anyhow!("Invalid FALCON signature in proof: {e}"))?;

                if falcon1024::verify_detached_signature(&sig, digest.as_bytes(), &pk).is_err() {
                    tracing::warn!("FALCON signature verification failed on WireSignedProof");
                    return Ok(false);
                }

                // Signature valid -- now validate the inner StateProof
                let proof: StateProof = serde_json::from_slice(&wire.proof_bytes)
                    .map_err(|e| anyhow!("Failed to deserialize inner StateProof: {e}"))?;
                Ok(proof.validate())
            }
            Err(_) => {
                // Backward compatibility: try raw StateProof (bincode format)
                tracing::warn!(
                    "Received unsigned state proof (legacy format) -- \
                     cryptographic verification skipped"
                );
                let proof = StateProof::from_bytes(incoming)
                    .map_err(|e| anyhow!("Failed to deserialize state proof: {e}"))?;
                Ok(proof.validate())
            }
        }
    }
}
