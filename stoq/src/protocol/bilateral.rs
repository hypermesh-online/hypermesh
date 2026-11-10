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

/// STOQ bilateral-handshake protocol version (semver).
///
/// P6: stamped into Msg1/Msg2 and enforced on the MAJOR component only.
/// Peers with a different major version speak an incompatible wire dialect and
/// are rejected during the handshake (before any application data). Minor/patch
/// differences are compatible — additive, backward-safe changes.
pub const STOQ_PROTOCOL_VERSION: &str = "1.0.0";

/// Parse the major-version component of a semver string.
///
/// Mirrors `blockmatrix::ipc::protocol::major_version`. Returns `None` for
/// unparseable input.
fn major_version(version: &str) -> Option<u64> {
    let core = version.split('-').next().unwrap_or(version);
    core.split('.').next()?.parse::<u64>().ok()
}

/// Whether a peer's advertised protocol version is compatible with ours.
///
/// Backward-compat rule (P6): an EMPTY peer version (a pre-P6 node that omits
/// the `#[serde(default)]` field) is always accepted — old peers keep working.
/// Otherwise the MAJOR component must match [`STOQ_PROTOCOL_VERSION`]. This
/// mirrors `blockmatrix::ipc::protocol::protocol_versions_compatible` but adds
/// the empty-string escape hatch for forward-compatibility with v0 peers.
fn peer_version_compatible(peer_version: &str) -> bool {
    if peer_version.is_empty() {
        return true;
    }
    major_version(peer_version) == major_version(STOQ_PROTOCOL_VERSION)
}

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
    /// P6: STOQ bilateral-protocol version (semver). `#[serde(default)]` yields
    /// an empty string for pre-P6 peers that omit it, which is treated as
    /// compatible so old nodes still handshake.
    #[serde(default)]
    protocol_version: String,
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
    proof_bytes: String, // hex
    signature: String,   // hex — FALCON sig over BLAKE3(nonce_a || proof)
    /// P6: STOQ bilateral-protocol version (semver). See [`Msg1`].
    #[serde(default)]
    protocol_version: String,
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
    let nonce_a = generate_nonce();
    let msg1 = make_msg1(signer, &nonce_a, local_coordinate);
    let msg1_bytes = serde_json::to_vec(&msg1)?;
    stream.write_msg(&msg1_bytes).await?;
    debug!("Bilateral handshake: sent Msg1 (initiator)");
    let msg2_bytes = stream.read_msg().await?;
    let msg2: Msg2 = serde_json::from_slice(&msg2_bytes)?;
    check_peer_version(&msg2.protocol_version)?;
    let peer_pubkey = verify_identity_binding(&msg2.node_id, &msg2.falcon_pubkey)?;
    let peer_proof_bytes =
        hex::decode(&msg2.proof_bytes).map_err(|e| anyhow!("Invalid hex in proof_bytes: {e}"))?;
    let peer_signature =
        hex::decode(&msg2.signature).map_err(|e| anyhow!("Invalid hex in signature: {e}"))?;
    verify_challenge_response(&peer_pubkey, &nonce_a, &peer_proof_bytes, &peer_signature)?;
    if !proof_provider.validate_proof(&peer_proof_bytes).await? {
        return Err(anyhow!("Peer state proof validation failed"));
    }
    let proof_signer = extract_signer_pubkey(&peer_proof_bytes)?;
    check_proof_signer(&proof_signer, &peer_pubkey)?;
    let nonce_b =
        hex::decode(&msg2.nonce).map_err(|e| anyhow!("Invalid hex in peer nonce: {e}"))?;
    check_nonce_length(&nonce_b)?;
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
    let msg1_bytes = stream.read_msg().await?;
    let msg1: Msg1 = serde_json::from_slice(&msg1_bytes)?;
    check_peer_version(&msg1.protocol_version)?;
    let peer_pubkey = verify_identity_binding(&msg1.node_id, &msg1.falcon_pubkey)?;
    let nonce_a =
        hex::decode(&msg1.nonce).map_err(|e| anyhow!("Invalid hex in peer nonce: {e}"))?;
    check_nonce_length(&nonce_a)?;
    let nonce_b = generate_nonce();
    let our_proof = proof_provider.generate_proof().await?;
    let our_signature = sign_challenge(signer, &nonce_a, &our_proof)?;
    let msg2 = make_msg2(
        signer,
        &nonce_b,
        local_coordinate,
        &our_proof,
        &our_signature,
    );
    let msg2_bytes = serde_json::to_vec(&msg2)?;
    stream.write_msg(&msg2_bytes).await?;
    debug!("Bilateral handshake: sent Msg2 (responder)");
    let msg3_bytes = stream.read_msg().await?;
    let msg3: Msg3 = serde_json::from_slice(&msg3_bytes)?;
    let peer_proof_bytes =
        hex::decode(&msg3.proof_bytes).map_err(|e| anyhow!("Invalid hex in proof_bytes: {e}"))?;
    let peer_signature =
        hex::decode(&msg3.signature).map_err(|e| anyhow!("Invalid hex in signature: {e}"))?;
    verify_challenge_response(&peer_pubkey, &nonce_b, &peer_proof_bytes, &peer_signature)?;
    if !proof_provider.validate_proof(&peer_proof_bytes).await? {
        return Err(anyhow!("Peer state proof validation failed"));
    }
    let proof_signer = extract_signer_pubkey(&peer_proof_bytes)?;
    check_proof_signer(&proof_signer, &peer_pubkey)?;
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

// Synchronous checks preserve the caller's validation order and error text.
fn check_peer_version(version: &str) -> Result<()> {
    if !peer_version_compatible(version) {
        return Err(anyhow!(
            "STOQ protocol version mismatch: peer '{}' incompatible with '{}'",
            version,
            STOQ_PROTOCOL_VERSION,
        ));
    }
    Ok(())
}

fn check_nonce_length(nonce: &[u8]) -> Result<()> {
    if nonce.len() != 32 {
        return Err(anyhow!("Invalid peer nonce length: {}", nonce.len()));
    }
    Ok(())
}

// Bind the signer of the state proof to the handshake-authenticated identity.
fn check_proof_signer(proof_signer: &[u8], peer_pubkey: &[u8]) -> Result<()> {
    if proof_signer != peer_pubkey {
        return Err(anyhow!("proof signer does not match peer identity"));
    }
    Ok(())
}

fn make_msg1(signer: &dyn NodeSigner, nonce: &[u8], coordinate: (i64, i64, i64)) -> Msg1 {
    Msg1 {
        node_id: signer.node_id().to_string(),
        falcon_pubkey: hex::encode(signer.public_key_bytes()),
        nonce: hex::encode(nonce),
        coordinate,
        protocol_version: STOQ_PROTOCOL_VERSION.to_string(),
        rotation_chain: signer.rotation_chain(),
    }
}

fn make_msg2(
    signer: &dyn NodeSigner,
    nonce: &[u8],
    coordinate: (i64, i64, i64),
    proof: &[u8],
    signature: &[u8],
) -> Msg2 {
    Msg2 {
        node_id: signer.node_id().to_string(),
        falcon_pubkey: hex::encode(signer.public_key_bytes()),
        nonce: hex::encode(nonce),
        coordinate,
        proof_bytes: hex::encode(proof),
        signature: hex::encode(signature),
        protocol_version: STOQ_PROTOCOL_VERSION.to_string(),
        rotation_chain: signer.rotation_chain(),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn generate_nonce() -> [u8; 32] {
    let mut nonce = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);
    nonce
}

/// Extract the FALCON signer public key from a `WireSignedProof` payload.
///
/// STOQ deliberately does NOT depend on trustchain, so we decode only the
/// `signer_pubkey` field structurally (serde ignores the other fields). The
/// wire format is the JSON serialization of trustchain's `WireSignedProof`,
/// where `signer_pubkey: Vec<u8>` is the raw FALCON-1024 public key.
///
/// Used to enforce the F2 signer↔identity binding: the proof's signer must
/// equal the peer's authenticated handshake key.
fn extract_signer_pubkey(proof_bytes: &[u8]) -> Result<Vec<u8>> {
    #[derive(Deserialize)]
    struct WireSignerView {
        signer_pubkey: Vec<u8>,
    }
    let view: WireSignerView = serde_json::from_slice(proof_bytes).map_err(|e| {
        anyhow!("Cannot extract proof signer key (not a signed WireSignedProof): {e}")
    })?;
    Ok(view.signer_pubkey)
}

/// Verify BLAKE3(falcon_pubkey) == declared node_id.
fn verify_identity_binding(declared_id: &str, pubkey_hex: &str) -> Result<Vec<u8>> {
    let pubkey =
        hex::decode(pubkey_hex).map_err(|e| anyhow!("Invalid hex in falcon_pubkey: {e}"))?;
    let computed_id = blake3::hash(&pubkey).to_hex().to_string();
    if computed_id != declared_id {
        let mut end = 16.min(declared_id.len());
        while !declared_id.is_char_boundary(end) {
            end -= 1;
        }
        return Err(anyhow!(
            "Identity binding failed: BLAKE3(pubkey)={} != declared node_id={}",
            &computed_id[..16],
            &declared_id[..end],
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
#[path = "tests/bilateral.rs"]
mod tests;
