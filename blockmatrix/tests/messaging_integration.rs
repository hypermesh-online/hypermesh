// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for the private messaging system.
//!
//! Tests cover the full message lifecycle: creation, encryption, signing,
//! verification, decryption, threading, store operations, and YAML persistence.

use blockmatrix::identity::FalconIdentity;
use blockmatrix::messaging::message::DirectMessage;
use blockmatrix::messaging::store::MessageStore;
use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{PublicKey, SecretKey};

/// Helper: build a test message between two parties.
fn make_message(id: &str, sender: &str, recipient: &str, ts: i64) -> DirectMessage {
    DirectMessage {
        message_id: id.into(),
        sender_node_id: sender.into(),
        sender_name: None,
        recipient_node_id: recipient.into(),
        encrypted_body: vec![0xAA; 16],
        kem_ciphertext: vec![0xBB; 32],
        reply_to: None,
        content_type: "text/plain".into(),
        created_at: ts,
        signature: Vec::new(),
    }
}

// -----------------------------------------------------------------------
// 1. Full message lifecycle: keygen -> encrypt -> sign -> verify -> decrypt
// -----------------------------------------------------------------------

#[test]
fn test_full_message_lifecycle() {
    // Generate Kyber keypairs for alice (sender) and bob (recipient)
    let (bob_kyber_pk, bob_kyber_sk) = kyber1024::keypair();

    // Generate FALCON identity for alice (signing)
    let alice_identity = FalconIdentity::generate();

    // Alice creates a message destined for bob
    let mut msg = DirectMessage::new(
        alice_identity.node_id.clone(),
        Some("Alice".into()),
        "bob-node-id".into(),
        "text/plain".into(),
        None,
    );

    // Encrypt for bob's Kyber public key
    let plaintext = b"Hello Bob, this is a private message over HyperMesh!";
    msg.encrypt_body(plaintext, bob_kyber_pk.as_bytes())
        .expect("test: encrypt body for bob");

    assert!(
        !msg.encrypted_body.is_empty(),
        "encrypted body must not be empty after encryption"
    );
    assert!(
        !msg.kem_ciphertext.is_empty(),
        "KEM ciphertext must not be empty after encryption"
    );

    // Alice signs the message with FALCON-1024
    msg.sign(&alice_identity)
        .expect("test: sign message with alice identity");
    assert!(
        !msg.signature.is_empty(),
        "signature must not be empty after signing"
    );

    // Bob verifies the signature using alice's FALCON public key
    assert!(
        msg.verify_signature(&alice_identity.public_key),
        "bob must be able to verify alice's signature"
    );

    // Bob decrypts the body using his Kyber secret key
    let decrypted = msg
        .decrypt_body(bob_kyber_sk.as_bytes())
        .expect("test: bob decrypts message");
    assert_eq!(
        decrypted, plaintext,
        "decrypted text must match original plaintext"
    );
}

// -----------------------------------------------------------------------
// 2. Message threading via reply_to
// -----------------------------------------------------------------------

#[test]
fn test_message_threading() {
    let alice_id = FalconIdentity::generate();
    let bob_id = FalconIdentity::generate();

    // Alice sends initial message (no reply_to)
    let msg1 = DirectMessage::new(
        alice_id.node_id.clone(),
        Some("Alice".into()),
        bob_id.node_id.clone(),
        "text/plain".into(),
        None,
    );
    assert!(
        msg1.reply_to.is_none(),
        "initial message must have no reply_to"
    );
    let msg1_id = msg1.message_id.clone();

    // Bob replies to alice's message
    let msg2 = DirectMessage::new(
        bob_id.node_id.clone(),
        Some("Bob".into()),
        alice_id.node_id.clone(),
        "text/plain".into(),
        Some(msg1_id.clone()),
    );
    assert_eq!(
        msg2.reply_to.as_deref(),
        Some(msg1_id.as_str()),
        "bob's reply must reference alice's message"
    );
    let msg2_id = msg2.message_id.clone();

    // Alice replies to bob's reply
    let msg3 = DirectMessage::new(
        alice_id.node_id.clone(),
        Some("Alice".into()),
        bob_id.node_id.clone(),
        "text/plain".into(),
        Some(msg2_id.clone()),
    );
    assert_eq!(
        msg3.reply_to.as_deref(),
        Some(msg2_id.as_str()),
        "alice's second message must reference bob's reply"
    );

    // Verify all three messages have unique IDs
    assert_ne!(msg1.message_id, msg2.message_id);
    assert_ne!(msg2.message_id, msg3.message_id);
    assert_ne!(msg1.message_id, msg3.message_id);
}

// -----------------------------------------------------------------------
// 3. Wrong recipient key fails decryption
// -----------------------------------------------------------------------

#[test]
fn test_wrong_recipient_key_fails() {
    let (charlie_pk, _charlie_sk) = kyber1024::keypair();
    let (_bob_pk, bob_sk) = kyber1024::keypair();

    let mut msg = DirectMessage::new(
        "alice".into(),
        None,
        "charlie".into(),
        "text/plain".into(),
        None,
    );

    // Alice encrypts for charlie's public key
    msg.encrypt_body(b"secret data", charlie_pk.as_bytes())
        .expect("test: encrypt for charlie");

    // Bob tries to decrypt with his own secret key -- must fail
    let result = msg.decrypt_body(bob_sk.as_bytes());
    assert!(
        result.is_err(),
        "decrypting with wrong key must fail"
    );
}

// -----------------------------------------------------------------------
// 4. Invalid signature rejected
// -----------------------------------------------------------------------

#[test]
fn test_invalid_signature_rejected() {
    let alice = FalconIdentity::generate();

    let mut msg = DirectMessage::new(
        alice.node_id.clone(),
        Some("Alice".into()),
        "bob".into(),
        "text/plain".into(),
        None,
    );
    msg.encrypted_body = vec![0xCC; 32];
    msg.kem_ciphertext = vec![0xDD; 64];

    // Sign with alice's key
    msg.sign(&alice).expect("test: sign");
    assert!(
        msg.verify_signature(&alice.public_key),
        "untampered signature must verify"
    );

    // Tamper with the body after signing
    msg.encrypted_body = vec![0xFF; 32];

    // Signature verification must now fail
    assert!(
        !msg.verify_signature(&alice.public_key),
        "tampered message must fail signature verification"
    );
}

// -----------------------------------------------------------------------
// 5. Conversation history ordering in MessageStore
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_conversation_history_ordering() {
    let store = MessageStore::new(None);

    // Create 5 messages between alice and bob with ascending timestamps
    let timestamps = [100, 200, 300, 400, 500];
    for (i, &ts) in timestamps.iter().enumerate() {
        let sender = if i % 2 == 0 { "alice" } else { "bob" };
        let recipient = if i % 2 == 0 { "bob" } else { "alice" };
        let msg = make_message(&format!("hist-{i}"), sender, recipient, ts);
        store.add(msg).await.expect("test: add message");
    }

    // Retrieve history -- must be sorted by created_at ascending
    let history = store.history_with_peer("alice", "bob").await;
    assert_eq!(history.len(), 5, "all 5 messages in conversation");

    for (i, msg) in history.iter().enumerate() {
        assert_eq!(
            msg.message_id,
            format!("hist-{i}"),
            "messages must be in chronological order"
        );
        assert_eq!(msg.created_at, timestamps[i]);
    }
}

// -----------------------------------------------------------------------
// 6. MessageStore with YAML persistence roundtrip
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_message_store_yaml_persistence() {
    let dir = tempfile::tempdir().expect("test: tempdir");
    let msg_dir = dir.path().join("msg_store");

    // Store 1: write messages
    let store1 = MessageStore::new(Some(msg_dir.clone()));
    store1
        .add(make_message("y1", "alice", "bob", 1000))
        .await
        .expect("test: add y1");
    store1
        .add(make_message("y2", "bob", "alice", 2000))
        .await
        .expect("test: add y2");
    store1
        .add(make_message("y3", "carol", "alice", 3000))
        .await
        .expect("test: add y3");
    store1.persist().await.expect("test: persist");

    // Verify YAML file exists
    let yaml_path = msg_dir.join("messages.yaml");
    assert!(yaml_path.exists(), "YAML file must be created");

    // Store 2: reload from disk
    let store2 = MessageStore::new(Some(msg_dir));
    store2.load().await.expect("test: load");
    assert_eq!(store2.count().await, 3, "all 3 messages must reload");

    // Verify individual messages survived the roundtrip
    let y1 = store2.get("y1").await.expect("test: y1 must exist");
    assert_eq!(y1.sender_node_id, "alice");
    assert_eq!(y1.recipient_node_id, "bob");

    let y2 = store2.get("y2").await.expect("test: y2 must exist");
    assert_eq!(y2.sender_node_id, "bob");
    assert_eq!(y2.created_at, 2000);
}

// -----------------------------------------------------------------------
// 7. Inbox filtering: only recipient's messages returned
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_inbox_filtering_by_recipient() {
    let store = MessageStore::new(None);

    store
        .add(make_message("i1", "alice", "bob", 100))
        .await
        .expect("test: add");
    store
        .add(make_message("i2", "carol", "bob", 200))
        .await
        .expect("test: add");
    store
        .add(make_message("i3", "alice", "carol", 300))
        .await
        .expect("test: add");
    store
        .add(make_message("i4", "bob", "alice", 400))
        .await
        .expect("test: add");

    let bob_inbox = store.list_for_recipient("bob").await;
    assert_eq!(bob_inbox.len(), 2, "bob has 2 messages");
    // Newest first
    assert_eq!(bob_inbox[0].message_id, "i2");
    assert_eq!(bob_inbox[1].message_id, "i1");

    let alice_inbox = store.list_for_recipient("alice").await;
    assert_eq!(alice_inbox.len(), 1, "alice has 1 received message");
    assert_eq!(alice_inbox[0].message_id, "i4");

    let carol_inbox = store.list_for_recipient("carol").await;
    assert_eq!(carol_inbox.len(), 1, "carol has 1 received message");
    assert_eq!(carol_inbox[0].message_id, "i3");
}

// -----------------------------------------------------------------------
// 8. Encrypt-sign-verify full roundtrip with two identities
// -----------------------------------------------------------------------

#[test]
fn test_encrypt_sign_verify_full_roundtrip() {
    let alice = FalconIdentity::generate();
    let bob = FalconIdentity::generate();
    let (bob_kyber_pk, bob_kyber_sk) = kyber1024::keypair();

    let mut msg = DirectMessage::new(
        alice.node_id.clone(),
        Some("Alice".into()),
        bob.node_id.clone(),
        "application/json".into(),
        None,
    );

    let payload = br#"{"type": "asset-ref", "id": "asset-abc123"}"#;
    msg.encrypt_body(payload, bob_kyber_pk.as_bytes())
        .expect("test: encrypt");
    msg.sign(&alice).expect("test: sign");

    // Bob verifies alice's signature
    assert!(msg.verify_signature(&alice.public_key));

    // Bob should NOT verify with his own key
    assert!(!msg.verify_signature(&bob.public_key));

    // Bob decrypts
    let decrypted = msg
        .decrypt_body(bob_kyber_sk.as_bytes())
        .expect("test: decrypt");
    assert_eq!(decrypted, payload);
}
