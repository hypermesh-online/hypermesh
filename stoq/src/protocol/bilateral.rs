// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Bilateral Handshake Protocol (R11)
//!
//! 3-message challenge-response handshake performed AFTER TLS but BEFORE
//! application data. Uses [`NodeSigner`] for FALCON-1024 identity and
//! [`StateProofProvider`] for Proof of State exchange.
//!
//! Protocol flow:
//!   Msg 1 (A→B): node_id, falcon_pubkey, nonce_a, coordinate
//!   Msg 2 (B→A): node_id, falcon_pubkey, nonce_b, coordinate,
//!                 proof_bytes, signature(BLAKE3(nonce_a || proof))
//!   Msg 3 (A→B): proof_bytes, signature(BLAKE3(nonce_b || proof))
//!
//! Both sides verify:
//!   1. BLAKE3(falcon_pubkey) == declared node_id (identity binding)
//!   2. FALCON signature covers OUR nonce (prevents replay)
//!   3. StateProof validates (PoS thresholds)
//!
//! STOQ depends only on lib traits — NOT on trustchain or blockmatrix.

use anyhow::{anyhow, Result};
use hypermesh_lib::{NodeSigner, StateProofProvider};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

use crate::transport::connection::Stream;

/// Metadata returned after a successful bilateral handshake.
#[derive(Debug, Clone)]
pub struct HandshakeResult {
    /// Peer's node ID (BLAKE3 hex of FALCON pubkey)
    pub peer_node_id: String,
    /// Peer's raw FALCON-1024 public key
    pub peer_pubkey: Vec<u8>,
    /// Peer's matrix coordinate (x, y, z) — raw i64 tuples to avoid
    /// depending on blockmatrix's MatrixCoordinate type
    pub peer_coordinate: (i64, i64, i64),
    /// Peer's state proof bytes (opaque to STOQ)
    pub peer_proof: Vec<u8>,
    /// Key rotation chain from peer (empty if no rotations).
    ///
    /// Each entry is a JSON-serialized `KeyRotationEntry` from trustchain.
    /// Consumers (e.g. blockmatrix) can deserialize and verify the chain
    /// to confirm identity continuity after key rotation.
    pub peer_rotation_chain: Vec<String>,
}

/// Handshake message 1: initiator introduces itself with challenge nonce.
#[derive(Debug, Serialize, Deserialize)]
struct Msg1 {
    node_id: String,
    falcon_pubkey: String, // hex
    nonce: String,         // hex, 32 bytes
    coordinate: (i64, i64, i64),
    /// Optional key rotation chain: JSON-serialized KeyRotationEntry items.
    /// Present when the node has rotated keys since genesis.
    /// The peer verifies this chain to confirm identity continuity.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    rotation_chain: Vec<String>,
}

/// Handshake message 2: responder introduces itself, answers challenge,
/// sends its own challenge nonce.
#[derive(Debug, Serialize, Deserialize)]
struct Msg2 {
    node_id: String,
    falcon_pubkey: String,
    nonce: String,
    coordinate: (i64, i64, i64),
    proof_bytes: String,  // hex
    signature: String,    // hex — FALCON sig over BLAKE3(nonce_a || proof)
    /// Optional key rotation chain: JSON-serialized KeyRotationEntry items.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    rotation_chain: Vec<String>,
}

/// Handshake message 3: initiator answers responder's challenge.
#[derive(Debug, Serialize, Deserialize)]
struct Msg3 {
    proof_bytes: String,
    signature: String,
}

/// Execute the bilateral handshake as the INITIATOR (client side).
///
/// Opens a stream on `connection`, performs 3-message exchange, then
/// closes the stream's write half.
pub async fn initiate_handshake(
    connection: &Arc<crate::Connection>,
    signer: &dyn NodeSigner,
    proof_provider: &dyn StateProofProvider,
    local_coordinate: (i64, i64, i64),
) -> Result<HandshakeResult> {
    let mut stream = connection.open_stream().await?;
    let result = do_initiate(&mut stream, signer, proof_provider, local_coordinate).await;
    let _ = stream.finish_send();
    result
}

/// Execute the bilateral handshake as the INITIATOR on a pre-opened stream.
///
/// Use this when the caller needs to write a connection-type discriminator
/// byte before the handshake messages. The caller is responsible for
/// calling `stream.finish_send()` after this returns.
pub async fn initiate_handshake_on_stream(
    stream: &mut crate::Stream,
    signer: &dyn NodeSigner,
    proof_provider: &dyn StateProofProvider,
    local_coordinate: (i64, i64, i64),
) -> Result<HandshakeResult> {
    do_initiate(stream, signer, proof_provider, local_coordinate).await
}

async fn do_initiate(
    stream: &mut Stream,
    signer: &dyn NodeSigner,
    proof_provider: &dyn StateProofProvider,
    local_coordinate: (i64, i64, i64),
) -> Result<HandshakeResult> {
    // Generate 32-byte challenge nonce
    let nonce_a = generate_nonce();

    // --- Msg 1: send our info + challenge nonce ---
    let msg1 = Msg1 {
        node_id: signer.node_id().to_string(),
        falcon_pubkey: hex::encode(signer.public_key_bytes()),
        nonce: hex::encode(&nonce_a),
        coordinate: local_coordinate,
        rotation_chain: signer.rotation_chain(),
    };
    let msg1_bytes = serde_json::to_vec(&msg1)?;
    stream.write_msg(&msg1_bytes).await?;
    debug!("Bilateral handshake: sent Msg1 (initiator)");

    // --- Receive Msg 2 from responder ---
    let msg2_bytes = stream.read_msg().await?;
    let msg2: Msg2 = serde_json::from_slice(&msg2_bytes)?;

    // Verify peer identity binding
    let peer_pubkey = verify_identity_binding(&msg2.node_id, &msg2.falcon_pubkey)?;

    // Verify peer's challenge-response signature
    let peer_proof_bytes = hex::decode(&msg2.proof_bytes)
        .map_err(|e| anyhow!("Invalid hex in proof_bytes: {e}"))?;
    let peer_signature = hex::decode(&msg2.signature)
        .map_err(|e| anyhow!("Invalid hex in signature: {e}"))?;
    verify_challenge_response(
        &peer_pubkey, &nonce_a, &peer_proof_bytes, &peer_signature,
    )?;

    // Validate peer's state proof
    if !proof_provider.validate_proof(&peer_proof_bytes).await? {
        return Err(anyhow!("Peer state proof validation failed"));
    }

    // Extract peer's challenge nonce
    let nonce_b = hex::decode(&msg2.nonce)
        .map_err(|e| anyhow!("Invalid hex in peer nonce: {e}"))?;
    if nonce_b.len() != 32 {
        return Err(anyhow!("Invalid peer nonce length: {}", nonce_b.len()));
    }

    // --- Msg 3: answer peer's challenge ---
    let our_proof = proof_provider.generate_proof().await?;
    let our_signature = sign_challenge(signer, &nonce_b, &our_proof)?;

    let msg3 = Msg3 {
        proof_bytes: hex::encode(&our_proof),
        signature: hex::encode(&our_signature),
    };
    let msg3_bytes = serde_json::to_vec(&msg3)?;
    stream.write_msg(&msg3_bytes).await?;

    info!(
        "Bilateral handshake complete (initiator) with peer {}",
        &msg2.node_id[..8.min(msg2.node_id.len())]
    );

    Ok(HandshakeResult {
        peer_node_id: msg2.node_id,
        peer_pubkey,
        peer_coordinate: msg2.coordinate,
        peer_proof: peer_proof_bytes,
        peer_rotation_chain: msg2.rotation_chain,
    })
}

/// Execute the bilateral handshake as the RESPONDER (server side).
///
/// Accepts a stream from an incoming connection, performs 3-message exchange.
pub async fn accept_handshake(
    stream: &mut Stream,
    signer: &dyn NodeSigner,
    proof_provider: &dyn StateProofProvider,
    local_coordinate: (i64, i64, i64),
) -> Result<HandshakeResult> {
    // --- Receive Msg 1 from initiator ---
    let msg1_bytes = stream.read_msg().await?;
    let msg1: Msg1 = serde_json::from_slice(&msg1_bytes)?;

    // Verify initiator identity binding
    let peer_pubkey = verify_identity_binding(&msg1.node_id, &msg1.falcon_pubkey)?;

    // Extract initiator's challenge nonce
    let nonce_a = hex::decode(&msg1.nonce)
        .map_err(|e| anyhow!("Invalid hex in peer nonce: {e}"))?;
    if nonce_a.len() != 32 {
        return Err(anyhow!("Invalid peer nonce length: {}", nonce_a.len()));
    }

    // Generate our challenge nonce
    let nonce_b = generate_nonce();

    // Generate our state proof and sign challenge
    let our_proof = proof_provider.generate_proof().await?;
    let our_signature = sign_challenge(signer, &nonce_a, &our_proof)?;

    // --- Msg 2: send our info + answer challenge + our challenge ---
    let msg2 = Msg2 {
        node_id: signer.node_id().to_string(),
        falcon_pubkey: hex::encode(signer.public_key_bytes()),
        nonce: hex::encode(&nonce_b),
        coordinate: local_coordinate,
        proof_bytes: hex::encode(&our_proof),
        signature: hex::encode(&our_signature),
        rotation_chain: signer.rotation_chain(),
    };
    let msg2_bytes = serde_json::to_vec(&msg2)?;
    stream.write_msg(&msg2_bytes).await?;
    debug!("Bilateral handshake: sent Msg2 (responder)");

    // --- Receive Msg 3 from initiator ---
    let msg3_bytes = stream.read_msg().await?;
    let msg3: Msg3 = serde_json::from_slice(&msg3_bytes)?;

    // Verify initiator's challenge-response
    let peer_proof_bytes = hex::decode(&msg3.proof_bytes)
        .map_err(|e| anyhow!("Invalid hex in proof_bytes: {e}"))?;
    let peer_signature = hex::decode(&msg3.signature)
        .map_err(|e| anyhow!("Invalid hex in signature: {e}"))?;
    verify_challenge_response(
        &peer_pubkey, &nonce_b, &peer_proof_bytes, &peer_signature,
    )?;

    // Validate initiator's state proof
    if !proof_provider.validate_proof(&peer_proof_bytes).await? {
        return Err(anyhow!("Peer state proof validation failed"));
    }

    info!(
        "Bilateral handshake complete (responder) with peer {}",
        &msg1.node_id[..8.min(msg1.node_id.len())]
    );

    Ok(HandshakeResult {
        peer_node_id: msg1.node_id,
        peer_pubkey,
        peer_coordinate: msg1.coordinate,
        peer_proof: peer_proof_bytes,
        peer_rotation_chain: msg1.rotation_chain,
    })
}

// ── Helpers ──────────────────────────────────────────────────────────

fn generate_nonce() -> [u8; 32] {
    let mut nonce = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);
    nonce
}

/// Verify BLAKE3(falcon_pubkey) == declared node_id.
fn verify_identity_binding(declared_id: &str, pubkey_hex: &str) -> Result<Vec<u8>> {
    let pubkey = hex::decode(pubkey_hex)
        .map_err(|e| anyhow!("Invalid hex in falcon_pubkey: {e}"))?;
    let computed_id = blake3::hash(&pubkey).to_hex().to_string();
    if computed_id != declared_id {
        return Err(anyhow!(
            "Identity binding failed: BLAKE3(pubkey)={} != declared node_id={}",
            &computed_id[..16],
            &declared_id[..16.min(declared_id.len())],
        ));
    }
    Ok(pubkey)
}

/// Sign BLAKE3(nonce || proof_bytes) with the node's FALCON key.
fn sign_challenge(
    signer: &dyn NodeSigner,
    challenge_nonce: &[u8],
    proof_bytes: &[u8],
) -> Result<Vec<u8>> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(challenge_nonce);
    hasher.update(proof_bytes);
    let digest = hasher.finalize();
    signer.sign(digest.as_bytes())
}

/// Verify a challenge-response: FALCON-1024 signature over BLAKE3(nonce || proof_bytes).
///
/// Uses pqcrypto_falcon directly — STOQ already depends on it.
fn verify_challenge_response(
    pubkey: &[u8],
    our_nonce: &[u8],
    proof_bytes: &[u8],
    signature: &[u8],
) -> Result<()> {
    use pqcrypto_falcon::falcon1024;
    use pqcrypto_traits::sign::{DetachedSignature, PublicKey};

    let mut hasher = blake3::Hasher::new();
    hasher.update(our_nonce);
    hasher.update(proof_bytes);
    let digest = hasher.finalize();

    let pk = falcon1024::PublicKey::from_bytes(pubkey)
        .map_err(|e| anyhow!("Invalid FALCON public key: {e}"))?;
    let sig = falcon1024::DetachedSignature::from_bytes(signature)
        .map_err(|e| anyhow!("Invalid FALCON signature: {e}"))?;
    let valid = falcon1024::verify_detached_signature(&sig, digest.as_bytes(), &pk).is_ok();

    if !valid {
        return Err(anyhow!("FALCON challenge-response verification failed"));
    }
    debug!("FALCON challenge-response verified successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal NodeSigner for testing (uses FALCON-1024).
    struct TestSigner {
        node_id: String,
        pubkey: Vec<u8>,
        secret_key: Vec<u8>,
    }

    impl TestSigner {
        fn generate() -> Self {
            use pqcrypto_falcon::falcon1024;
            use pqcrypto_traits::sign::{PublicKey, SecretKey};
            let (pk, sk) = falcon1024::keypair();
            let pk_bytes = pk.as_bytes().to_vec();
            let node_id = blake3::hash(&pk_bytes).to_hex().to_string();
            Self {
                node_id,
                pubkey: pk_bytes,
                secret_key: sk.as_bytes().to_vec(),
            }
        }
    }

    impl NodeSigner for TestSigner {
        fn node_id(&self) -> &str {
            &self.node_id
        }
        fn public_key_bytes(&self) -> &[u8] {
            &self.pubkey
        }
        fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
            use pqcrypto_falcon::falcon1024;
            use pqcrypto_traits::sign::{DetachedSignature, SecretKey};
            let sk = falcon1024::SecretKey::from_bytes(&self.secret_key)
                .map_err(|e| anyhow!("test: {e}"))?;
            let sig = falcon1024::detached_sign(data, &sk);
            Ok(sig.as_bytes().to_vec())
        }
        fn verify_signature(pubkey: &[u8], data: &[u8], signature: &[u8]) -> Result<bool> {
            use pqcrypto_falcon::falcon1024;
            use pqcrypto_traits::sign::{DetachedSignature, PublicKey};
            let pk = falcon1024::PublicKey::from_bytes(pubkey)
                .map_err(|e| anyhow!("test: {e}"))?;
            let sig = falcon1024::DetachedSignature::from_bytes(signature)
                .map_err(|e| anyhow!("test: {e}"))?;
            Ok(falcon1024::verify_detached_signature(&sig, data, &pk).is_ok())
        }
    }

    /// Minimal StateProofProvider for testing.
    struct TestProofProvider;

    #[async_trait::async_trait]
    impl StateProofProvider for TestProofProvider {
        async fn generate_proof(&self) -> Result<Vec<u8>> {
            Ok(b"test-state-proof-data".to_vec())
        }
        async fn validate_proof(&self, proof_bytes: &[u8]) -> Result<bool> {
            Ok(!proof_bytes.is_empty())
        }
    }

    #[test]
    fn test_identity_binding_valid() {
        let signer = TestSigner::generate();
        let result = verify_identity_binding(
            &signer.node_id,
            &hex::encode(&signer.pubkey),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_identity_binding_invalid() {
        let signer = TestSigner::generate();
        let result = verify_identity_binding(
            "0000000000000000000000000000000000000000000000000000000000000000",
            &hex::encode(&signer.pubkey),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_verify_challenge() {
        let signer = TestSigner::generate();
        let nonce = generate_nonce();
        let proof = b"test-proof-data";

        let sig = sign_challenge(&signer, &nonce, proof)
            .expect("test: signing");
        verify_challenge_response(
            &signer.pubkey, &nonce, proof, &sig,
        )
        .expect("test: verification");
    }

    #[test]
    fn test_challenge_wrong_nonce_fails() {
        let signer = TestSigner::generate();
        let nonce = generate_nonce();
        let wrong_nonce = generate_nonce();
        let proof = b"test-proof-data";

        let sig = sign_challenge(&signer, &nonce, proof)
            .expect("test: signing");
        let result = verify_challenge_response(
            &signer.pubkey, &wrong_nonce, proof, &sig,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_handshake_result_has_empty_rotation_chain() {
        // Default TestSigner uses the default rotation_chain() which returns empty
        let signer = TestSigner::generate();
        let chain = signer.rotation_chain();
        assert!(chain.is_empty(), "Default rotation chain should be empty");
    }

    #[test]
    fn test_msg1_serializes_without_rotation_chain() {
        let msg = Msg1 {
            node_id: "abc".to_string(),
            falcon_pubkey: "def".to_string(),
            nonce: "012".to_string(),
            coordinate: (1, 2, 3),
            rotation_chain: Vec::new(),
        };
        let json = serde_json::to_string(&msg).expect("test: serialize Msg1");
        // skip_serializing_if = "Vec::is_empty" should omit the field
        assert!(
            !json.contains("rotation_chain"),
            "Empty rotation_chain should be omitted from JSON"
        );
    }

    #[test]
    fn test_msg1_deserializes_without_rotation_chain() {
        // Simulate a message from an older node that doesn't include rotation_chain
        let json = r#"{"node_id":"abc","falcon_pubkey":"def","nonce":"012","coordinate":[1,2,3]}"#;
        let msg: Msg1 = serde_json::from_str(json).expect("test: deserialize Msg1");
        assert!(msg.rotation_chain.is_empty(), "Missing field should default to empty");
    }

    #[test]
    fn test_msg2_roundtrips_with_rotation_chain() {
        let chain = vec!["entry1".to_string(), "entry2".to_string()];
        let msg = Msg2 {
            node_id: "n".to_string(),
            falcon_pubkey: "fp".to_string(),
            nonce: "nn".to_string(),
            coordinate: (0, 0, 0),
            proof_bytes: "pb".to_string(),
            signature: "sig".to_string(),
            rotation_chain: chain.clone(),
        };
        let json = serde_json::to_string(&msg).expect("test: serialize Msg2");
        assert!(json.contains("rotation_chain"), "Non-empty chain should be serialized");
        let back: Msg2 = serde_json::from_str(&json).expect("test: deserialize Msg2");
        assert_eq!(back.rotation_chain, chain);
    }
}
