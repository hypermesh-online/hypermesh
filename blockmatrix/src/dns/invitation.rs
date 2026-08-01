// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Domain Invitation System
//!
//! Creates and verifies BLAKE3-keyed invitation tokens for private domain
//! networks. Invitations can be open (any node) or targeted (specific node).
//! Tokens are encoded as hex for transport.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A domain invitation token allowing a node to join a domain network.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DomainInvitation {
    /// Domain name this invitation grants access to.
    pub domain_name: String,
    /// Derived network ID for the domain.
    pub network_id: String,
    /// Target node ID (empty string = open invitation).
    pub invitee_node_id: String,
    /// Unix timestamp when the invitation expires.
    pub expires_at: u64,
    /// BLAKE3 keyed-hash token binding domain + invitee + expiry.
    pub token: [u8; 32],
}

/// Create a domain invitation token.
///
/// `owner_proof_bytes` is used as the BLAKE3 keying material (up to 32 bytes).
/// If `invitee_node_id` is `None`, the invitation is open to any node.
pub fn create_invitation(
    domain_name: &str,
    owner_proof_bytes: &[u8],
    invitee_node_id: Option<&str>,
    ttl_secs: u64,
) -> DomainInvitation {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let expires_at = now.saturating_add(ttl_secs);
    let invitee = invitee_node_id.unwrap_or("");
    // `DomainInvitation.network_id` is a serialized wire field; keep its
    // canonical hex string form (byte-identical to the pre-retype value).
    let network_id = super::domain::derive_network_id(domain_name).to_string();

    let key = derive_key(owner_proof_bytes);
    let payload = format!("{domain_name}:{invitee}:{expires_at}");
    let token = *blake3::keyed_hash(&key, payload.as_bytes()).as_bytes();

    DomainInvitation {
        domain_name: domain_name.to_string(),
        network_id,
        invitee_node_id: invitee.to_string(),
        expires_at,
        token,
    }
}

/// Verify a domain invitation against the owner's proof bytes.
///
/// Returns `false` if expired or if the token does not match.
pub fn verify_invitation(invitation: &DomainInvitation, owner_proof_bytes: &[u8]) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now > invitation.expires_at {
        return false;
    }

    let key = derive_key(owner_proof_bytes);
    let payload = format!(
        "{}:{}:{}",
        invitation.domain_name, invitation.invitee_node_id, invitation.expires_at
    );
    let expected = *blake3::keyed_hash(&key, payload.as_bytes()).as_bytes();
    invitation.token == expected
}

/// Encode an invitation as a hex string for transport.
pub fn encode_invitation(invitation: &DomainInvitation) -> Result<String, serde_json::Error> {
    let json = serde_json::to_vec(invitation)?;
    Ok(hex::encode(json))
}

/// Decode an invitation from a hex-encoded string.
pub fn decode_invitation(token_str: &str) -> Result<DomainInvitation, String> {
    let bytes =
        hex::decode(token_str).map_err(|e| format!("Invalid token encoding: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Invalid token format: {e}"))
}

/// Derive a 32-byte BLAKE3 key from arbitrary-length proof bytes.
fn derive_key(proof_bytes: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    let copy_len = proof_bytes.len().min(32);
    key[..copy_len].copy_from_slice(&proof_bytes[..copy_len]);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invitation_create_and_verify() {
        let proof = b"owner-proof-secret-key-material!";
        let inv = create_invitation("home.hypermesh", proof, Some("node-42"), 3600);

        assert_eq!(inv.domain_name, "home.hypermesh");
        assert_eq!(inv.invitee_node_id, "node-42");
        assert!(verify_invitation(&inv, proof));
    }

    #[test]
    fn test_invitation_wrong_key() {
        let proof_a = b"correct-proof-aaaaaaaaaaaaaaaaa!";
        let proof_b = b"wrong-proof-bbbbbbbbbbbbbbbbbbb!";
        let inv = create_invitation("test.domain", proof_a, Some("peer-1"), 3600);

        assert!(
            !verify_invitation(&inv, proof_b),
            "verification must fail with wrong key"
        );
    }

    #[test]
    fn test_invitation_encode_decode_roundtrip() {
        let proof = b"roundtrip-key-material-32bytes!x";
        let inv = create_invitation("rt.domain", proof, Some("node-rt"), 7200);

        let encoded = encode_invitation(&inv).expect("test: encode");
        let decoded = decode_invitation(&encoded).expect("test: decode");

        assert_eq!(inv, decoded);
    }

    #[test]
    fn test_invitation_open_vs_targeted() {
        let proof = b"open-vs-targeted-proof-material!";

        // Open invitation (no specific invitee)
        let open = create_invitation("open.domain", proof, None, 3600);
        assert_eq!(open.invitee_node_id, "");
        assert!(verify_invitation(&open, proof));

        // Targeted invitation
        let targeted = create_invitation("open.domain", proof, Some("specific-node"), 3600);
        assert_eq!(targeted.invitee_node_id, "specific-node");
        assert!(verify_invitation(&targeted, proof));

        // Tokens differ because invitee is part of the hash payload
        assert_ne!(open.token, targeted.token);
    }

    #[test]
    fn test_invitation_expired() {
        let proof = b"expired-proof-material-here!!!!!";

        // Create invitation that is already expired (expires_at in the past)
        let mut inv = create_invitation("exp.domain", proof, None, 3600);
        // Force expiry to a past timestamp
        inv.expires_at = 1;
        // Re-derive token with the past timestamp so the HMAC matches
        let key = derive_key(proof);
        let payload = format!("{}:{}:{}", inv.domain_name, inv.invitee_node_id, inv.expires_at);
        inv.token = *blake3::keyed_hash(&key, payload.as_bytes()).as_bytes();

        assert!(
            !verify_invitation(&inv, proof),
            "expired invitation must not verify"
        );
    }
}
