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
        let pk = falcon1024::PublicKey::from_bytes(pubkey).map_err(|e| anyhow!("test: {e}"))?;
        let sig = falcon1024::DetachedSignature::from_bytes(signature)
            .map_err(|e| anyhow!("test: {e}"))?;
        Ok(falcon1024::verify_detached_signature(&sig, data, &pk).is_ok())
    }
}

/// Minimal StateProofProvider for testing.
#[allow(dead_code)]
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
    let result = verify_identity_binding(&signer.node_id, &hex::encode(&signer.pubkey));
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

    let sig = sign_challenge(&signer, &nonce, proof).expect("test: signing");
    verify_challenge_response(&signer.pubkey, &nonce, proof, &sig).expect("test: verification");
}

#[test]
fn test_challenge_wrong_nonce_fails() {
    let signer = TestSigner::generate();
    let nonce = generate_nonce();
    let wrong_nonce = generate_nonce();
    let proof = b"test-proof-data";

    let sig = sign_challenge(&signer, &nonce, proof).expect("test: signing");
    let result = verify_challenge_response(&signer.pubkey, &wrong_nonce, proof, &sig);
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
        protocol_version: STOQ_PROTOCOL_VERSION.to_string(),
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
    assert!(
        msg.rotation_chain.is_empty(),
        "Missing field should default to empty"
    );
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
        protocol_version: STOQ_PROTOCOL_VERSION.to_string(),
        rotation_chain: chain.clone(),
    };
    let json = serde_json::to_string(&msg).expect("test: serialize Msg2");
    assert!(
        json.contains("rotation_chain"),
        "Non-empty chain should be serialized"
    );
    let back: Msg2 = serde_json::from_str(&json).expect("test: deserialize Msg2");
    assert_eq!(back.rotation_chain, chain);
}

// -- P6 protocol-version negotiation tests -----------------------------

#[test]
fn test_version_same_major_compatible() {
    // Exact and minor/patch-different but same-major → compatible.
    assert!(peer_version_compatible(STOQ_PROTOCOL_VERSION));
    assert!(peer_version_compatible("1.4.2"));
    assert!(peer_version_compatible("1.0.0-rc1"));
}

#[test]
fn test_version_different_major_rejected() {
    assert!(!peer_version_compatible("2.0.0"));
    assert!(!peer_version_compatible("0.9.0"));
}

#[test]
fn test_version_empty_is_backward_compatible() {
    // A pre-P6 peer omits the field → serde default "" → accepted.
    assert!(
        peer_version_compatible(""),
        "old peers omitting protocol_version must still handshake"
    );
}

#[test]
fn test_msg1_defaults_protocol_version_for_old_peer() {
    // Old node's Msg1 (no protocol_version) deserializes with "" default.
    let json = r#"{"node_id":"abc","falcon_pubkey":"def","nonce":"012","coordinate":[1,2,3]}"#;
    let msg: Msg1 = serde_json::from_str(json).expect("test: deserialize Msg1");
    assert_eq!(msg.protocol_version, "");
    assert!(peer_version_compatible(&msg.protocol_version));
}

#[test]
fn test_msg1_carries_protocol_version_when_present() {
    let json = r#"{"node_id":"a","falcon_pubkey":"b","nonce":"c","coordinate":[0,0,0],"protocol_version":"2.3.4"}"#;
    let msg: Msg1 = serde_json::from_str(json).expect("test: deserialize Msg1");
    assert_eq!(msg.protocol_version, "2.3.4");
    assert!(
        !peer_version_compatible(&msg.protocol_version),
        "major 2 must be rejected against our major 1"
    );
}

#[test]
fn test_identity_binding_unicode_split_at_byte_16() {
    let error = verify_identity_binding("123456789012345é", "00").unwrap_err();
    let expected_hash = blake3::hash(&[0]).to_hex().to_string();
    assert_eq!(
        error.to_string(),
        format!(
            "Identity binding failed: BLAKE3(pubkey)={} != declared node_id=123456789012345",
            &expected_hash[..16]
        )
    );
}

#[test]
fn test_identity_binding_ascii_and_boundary_diagnostics() {
    let key = [0];
    let id = blake3::hash(&key).to_hex().to_string();
    assert_eq!(verify_identity_binding(&id, "00").unwrap(), key);
    for (declared, prefix) in [
        ("12345678901234567890", "1234567890123456"),
        ("12345678901234éz", "12345678901234é"),
        ("", ""),
        ("short", "short"),
    ] {
        let error = verify_identity_binding(declared, "00").unwrap_err();
        assert_eq!(
            error.to_string(),
            format!(
                "Identity binding failed: BLAKE3(pubkey)={} != declared node_id={}",
                &id[..16],
                prefix
            )
        );
    }
}
