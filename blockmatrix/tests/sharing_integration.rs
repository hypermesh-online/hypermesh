// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for P2P file sharing.
//!
//! Exercises the full share lifecycle: identity generation, asset processing
//! through the streaming pipeline, Kyber-1024 key wrapping, FALCON-1024
//! signed invitations, inbox management, and reconstruction.

use blockmatrix::assets::pipeline::compression::CompressionAlgorithm;
use blockmatrix::assets::pipeline::encryption::AesKey;
use blockmatrix::assets::pipeline::orchestrator::DecryptionKey;
use blockmatrix::assets::pipeline::sharding::Shard;
use blockmatrix::assets::pipeline::streaming_pipeline::{
    StreamingAssetPipeline, StreamingPipelineConfig,
};
use blockmatrix::assets::pipeline::PipelineInputMetadata;
use blockmatrix::identity::FalconIdentity;
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::sharing::inbox::InboxStore;
use blockmatrix::sharing::invite::ShareInvite;
use blockmatrix::sharing::key_wrap::{unwrap_key, wrap_key_for_recipient};
use hypermesh_lib::ContentHash;

use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{PublicKey, SecretKey};

fn test_metadata(name: &str, size: usize) -> PipelineInputMetadata {
    PipelineInputMetadata {
        name: name.to_string(),
        content_type: "application/octet-stream".to_string(),
        size,
        created_at: 1_700_000_000,
        custom: Default::default(),
    }
}

fn make_test_invite(invite_id: &str) -> ShareInvite {
    ShareInvite::new(
        invite_id.into(),
        "asset-test-123".into(),
        "sender-node".into(),
        Some("TestSender".into()),
        "recipient-node".into(),
        "testfile.bin".into(),
        4096,
        6,
        b"[]".to_vec(),
        vec![0xAA; 32],
        vec![0xBB; 64],
        1_700_000_000,
    )
}

// ── Test 1: Full share lifecycle (alice -> bob) ─────────────────────────────

#[tokio::test]
async fn test_full_share_lifecycle() {
    // 1. Generate dual-key identities for alice and bob
    let alice = FalconIdentity::generate();
    let bob = FalconIdentity::generate();

    // 2. Alice creates an asset through the streaming pipeline
    let data: Vec<u8> = b"Hello Bob, this is a secret file shared over HyperMesh!"
        .repeat(200);

    let pipeline = StreamingAssetPipeline::new(StreamingPipelineConfig {
        segment_size: 10_000,
        compression: CompressionAlgorithm::None,
        compression_level: 0,
        rs_data_shards: 4,
        rs_parity_shards: 2,
        content_type: "application/octet-stream".to_string(),
    })
    .expect("test: pipeline creation");

    let metadata = test_metadata("secret.txt", data.len());
    let (manifest, decryption_key, shard_sets) = pipeline
        .process_segmented(&data, &metadata)
        .expect("test: process_segmented");

    // 3. Alice stores shards in her ShardStore
    let alice_dir = tempfile::tempdir().expect("test: alice tempdir");
    let alice_store = ShardStore::new_with_dir(alice_dir.path().to_path_buf());
    for set in &shard_sets {
        for shard in &set.shards {
            let hash = ContentHash(*blake3::hash(&shard.data).as_bytes());
            alice_store.store(hash, shard.data.clone()).await;
        }
    }

    // 4. Alice wraps the decryption key for bob using bob's Kyber public key
    let (encrypted_key, kem_ct) =
        wrap_key_for_recipient(&decryption_key, &bob.kyber_public_key)
            .expect("test: wrap_key_for_recipient");

    // 5. Alice builds and signs a ShareInvite
    let shard_count: u32 = shard_sets.iter().map(|s| s.shards.len() as u32).sum();
    let shard_map_json = serde_json::to_vec(&manifest).expect("test: serialize manifest");

    let mut invite = ShareInvite::new(
        "inv-lifecycle-001".into(),
        hex::encode(manifest.content_hash),
        alice.node_id.clone(),
        Some("alice".into()),
        bob.node_id.clone(),
        "secret.txt".into(),
        data.len() as u64,
        shard_count,
        shard_map_json,
        encrypted_key,
        kem_ct,
        1_700_000_000,
    );
    invite.sign(&alice).expect("test: sign invite");

    // 6. Verify invite signature with alice's public key
    assert!(
        invite.verify_signature(&alice.public_key),
        "Alice's signature must verify"
    );

    // 7. Bob receives invite and stores in his inbox
    let bob_inbox_dir = tempfile::tempdir().expect("test: bob inbox tempdir");
    let bob_inbox = InboxStore::new(Some(bob_inbox_dir.path().to_path_buf()));
    bob_inbox
        .add(invite.clone())
        .await
        .expect("test: add to bob inbox");
    assert_eq!(bob_inbox.count().await, 1);

    // 8. Bob retrieves the invite and verifies signature
    let received = bob_inbox
        .get("inv-lifecycle-001")
        .await
        .expect("test: invite must be in inbox");
    assert!(
        received.verify_signature(&alice.public_key),
        "Received invite signature must verify"
    );

    // 9. Bob accepts: unwraps the decryption key using his Kyber secret key
    let unwrapped_key = unwrap_key(
        &received.encrypted_key,
        &received.key_kem_ciphertext,
        bob.kyber_secret_key_bytes(),
    )
    .expect("test: unwrap_key");

    // 10. Bob fetches shards and reconstructs the asset
    let all_shards: Vec<Vec<Shard>> =
        shard_sets.iter().map(|s| s.shards.clone()).collect();
    let reconstructed = pipeline
        .reconstruct_segmented(&manifest, &unwrapped_key, &all_shards)
        .expect("test: reconstruct_segmented");

    // 11. Verify data integrity
    assert_eq!(
        reconstructed.len(),
        data.len(),
        "Reconstructed size must match original"
    );
    assert_eq!(reconstructed, data, "Reconstructed data must match original");
}

// ── Test 2: Reject flow clears inbox ────────────────────────────────────────

#[tokio::test]
async fn test_share_reject_clears_inbox() {
    let inbox = InboxStore::new(None);
    let invite = make_test_invite("inv-reject-1");
    inbox.add(invite).await.expect("test: add invite");
    assert_eq!(inbox.count().await, 1);

    let removed = inbox.remove("inv-reject-1").await;
    assert!(removed.is_some(), "Remove must return the invite");
    assert_eq!(inbox.count().await, 0, "Inbox must be empty after reject");
    assert!(
        inbox.get("inv-reject-1").await.is_none(),
        "Rejected invite must not be retrievable"
    );
}

// ── Test 3: Wrong recipient key fails unwrap ────────────────────────────────

#[test]
fn test_wrong_recipient_key_fails() {
    let alice_kyber = kyber1024::keypair();
    let bob_kyber = kyber1024::keypair();

    let dk = DecryptionKey::Aes(AesKey {
        key: vec![0xCC; 32],
        nonce: vec![0xDD; 12],
    });

    // Wrap with alice's public key
    let (encrypted, kem_ct) =
        wrap_key_for_recipient(&dk, alice_kyber.0.as_bytes())
            .expect("test: wrap with alice pubkey");

    // Try unwrap with bob's secret key — must fail
    let result = unwrap_key(&encrypted, &kem_ct, bob_kyber.1.as_bytes());
    assert!(result.is_err(), "Unwrap with wrong key must fail");
}

// ── Test 4: Invalid signature rejected ──────────────────────────────────────

#[test]
fn test_invalid_signature_rejected() {
    let alice = FalconIdentity::generate();
    let mut invite = make_test_invite("inv-tampered");
    invite.sender_node_id = alice.node_id.clone();
    invite.sign(&alice).expect("test: sign invite");

    // Tamper with the asset_name after signing
    invite.asset_name = "TAMPERED.exe".to_string();

    // Signature verification must fail for tampered content
    assert!(
        !invite.verify_signature(&alice.public_key),
        "Tampered invite must fail verification"
    );
}

// ── Test 5: Unsigned invite rejected ────────────────────────────────────────

#[test]
fn test_unsigned_invite_rejected() {
    let alice = FalconIdentity::generate();
    let invite = make_test_invite("inv-unsigned");

    // Verify without signing — empty signature must return false
    assert!(
        !invite.verify_signature(&alice.public_key),
        "Unsigned invite must fail verification"
    );
}

// ── Test 6: Inbox persistence roundtrip ─────────────────────────────────────

#[tokio::test]
async fn test_inbox_persistence_across_restarts() {
    let dir = tempfile::tempdir().expect("test: tempdir");
    let inbox_path = dir.path().to_path_buf();

    // Create inbox with invites and persist
    {
        let inbox = InboxStore::new(Some(inbox_path.clone()));
        inbox
            .add(make_test_invite("inv-persist-1"))
            .await
            .expect("test: add 1");
        inbox
            .add(make_test_invite("inv-persist-2"))
            .await
            .expect("test: add 2");
        inbox.persist().await.expect("test: persist");
    }

    // Load into a fresh inbox (simulates restart)
    let inbox2 = InboxStore::new(Some(inbox_path));
    inbox2.load().await.expect("test: load");

    assert_eq!(inbox2.count().await, 2, "Both invites must survive restart");
    assert!(inbox2.get("inv-persist-1").await.is_some());
    assert!(inbox2.get("inv-persist-2").await.is_some());
}

// ── Test 7: Multi-node shard distribution and share ─────────────────────────

#[tokio::test]
async fn test_multi_node_share_and_fetch() {
    // Simulate 3 storage nodes + 1 recipient
    let node_a = FalconIdentity::generate();
    let _node_b = FalconIdentity::generate();
    let _node_c = FalconIdentity::generate();
    let recipient = FalconIdentity::generate();

    // Create asset
    let data: Vec<u8> = (0..5000u32)
        .flat_map(|i| i.to_le_bytes())
        .collect();

    let pipeline = StreamingAssetPipeline::new(StreamingPipelineConfig {
        segment_size: 8_000,
        compression: CompressionAlgorithm::None,
        compression_level: 0,
        rs_data_shards: 3,
        rs_parity_shards: 2,
        content_type: "application/octet-stream".to_string(),
    })
    .expect("test: pipeline");

    let metadata = test_metadata("distributed.bin", data.len());
    let (manifest, decryption_key, shard_sets) = pipeline
        .process_segmented(&data, &metadata)
        .expect("test: process");

    // Distribute shards across 3 node stores (round-robin)
    let dir_a = tempfile::tempdir().expect("test: dir_a");
    let dir_b = tempfile::tempdir().expect("test: dir_b");
    let dir_c = tempfile::tempdir().expect("test: dir_c");
    let store_a = ShardStore::new_with_dir(dir_a.path().to_path_buf());
    let store_b = ShardStore::new_with_dir(dir_b.path().to_path_buf());
    let store_c = ShardStore::new_with_dir(dir_c.path().to_path_buf());
    let stores = [&store_a, &store_b, &store_c];

    let mut shard_idx = 0usize;
    for set in &shard_sets {
        for shard in &set.shards {
            let hash = ContentHash(*blake3::hash(&shard.data).as_bytes());
            stores[shard_idx % 3]
                .store(hash, shard.data.clone())
                .await;
            shard_idx += 1;
        }
    }

    // Wrap key for recipient
    let (encrypted_key, kem_ct) =
        wrap_key_for_recipient(&decryption_key, &recipient.kyber_public_key)
            .expect("test: wrap key");

    // Build and sign invite from node_a (the sharer)
    let shard_count: u32 = shard_sets.iter().map(|s| s.shards.len() as u32).sum();
    let shard_map_json = serde_json::to_vec(&manifest).expect("test: serialize");
    let mut invite = ShareInvite::new(
        "inv-multi-001".into(),
        hex::encode(manifest.content_hash),
        node_a.node_id.clone(),
        Some("node-a".into()),
        recipient.node_id.clone(),
        "distributed.bin".into(),
        data.len() as u64,
        shard_count,
        shard_map_json,
        encrypted_key,
        kem_ct,
        1_700_000_000,
    );
    invite.sign(&node_a).expect("test: sign");

    // Recipient verifies signature
    assert!(invite.verify_signature(&node_a.public_key));

    // Recipient unwraps key
    let unwrapped = unwrap_key(
        &invite.encrypted_key,
        &invite.key_kem_ciphertext,
        recipient.kyber_secret_key_bytes(),
    )
    .expect("test: unwrap");

    // Recipient fetches shards from all 3 nodes (in real system this
    // would be over STOQ; here we just gather from the stores)
    let mut fetched_idx = 0usize;
    let mut fetched_sets: Vec<Vec<Shard>> = Vec::new();
    for set in &shard_sets {
        let mut fetched_shards = Vec::new();
        for shard in &set.shards {
            let hash = ContentHash(*blake3::hash(&shard.data).as_bytes());
            let store = stores[fetched_idx % 3];
            let shard_data = store
                .get(&hash)
                .await
                .expect("test: shard must exist in store");
            // Reconstruct Shard with original metadata
            fetched_shards.push(Shard {
                data: shard_data,
                metadata: shard.metadata.clone(),
            });
            fetched_idx += 1;
        }
        fetched_sets.push(fetched_shards);
    }

    // Reconstruct
    let reconstructed = pipeline
        .reconstruct_segmented(&manifest, &unwrapped, &fetched_sets)
        .expect("test: reconstruct");

    assert_eq!(reconstructed, data, "Multi-node reconstruction must match");
}

// ── Test 8: Key wrapping preserves all DecryptionKey variants ───────────────

#[test]
fn test_key_wrap_preserves_kyber_segmented_variant() {
    let recipient = FalconIdentity::generate();

    let dk = DecryptionKey::KyberSegmented {
        ciphertext_kem: vec![0x11; 80],
        secret_key: vec![0x22; 80],
        segment_count: 5,
        original_size: 50_000,
    };

    let (encrypted, kem_ct) =
        wrap_key_for_recipient(&dk, &recipient.kyber_public_key)
            .expect("test: wrap KyberSegmented");

    let restored = unwrap_key(&encrypted, &kem_ct, recipient.kyber_secret_key_bytes())
        .expect("test: unwrap KyberSegmented");

    assert!(
        matches!(restored, DecryptionKey::KyberSegmented { segment_count: 5, original_size: 50_000, .. }),
        "KyberSegmented variant must be preserved with correct fields"
    );
}

// ── Test 9: Invite serialization roundtrip ──────────────────────────────────

#[test]
fn test_invite_serialization_preserves_all_fields() {
    let alice = FalconIdentity::generate();
    let mut invite = ShareInvite::new(
        "inv-ser-001".into(),
        "asset-xyz".into(),
        alice.node_id.clone(),
        Some("alice".into()),
        "bob-node".into(),
        "document.pdf".into(),
        1_048_576,
        14,
        b"[{\"shard\":0}]".to_vec(),
        vec![0xAA; 64],
        vec![0xBB; 128],
        1_700_000_000,
    );
    invite.sign(&alice).expect("test: sign");

    let json = serde_json::to_vec(&invite).expect("test: serialize");
    let restored: ShareInvite =
        serde_json::from_slice(&json).expect("test: deserialize");

    assert_eq!(restored.invite_id, invite.invite_id);
    assert_eq!(restored.asset_id, invite.asset_id);
    assert_eq!(restored.sender_node_id, invite.sender_node_id);
    assert_eq!(restored.sender_name, invite.sender_name);
    assert_eq!(restored.recipient_node_id, invite.recipient_node_id);
    assert_eq!(restored.asset_name, invite.asset_name);
    assert_eq!(restored.asset_size, invite.asset_size);
    assert_eq!(restored.shard_count, invite.shard_count);
    assert_eq!(restored.shard_map_json, invite.shard_map_json);
    assert_eq!(restored.encrypted_key, invite.encrypted_key);
    assert_eq!(restored.key_kem_ciphertext, invite.key_kem_ciphertext);
    assert_eq!(restored.created_at, invite.created_at);
    assert_eq!(restored.signature, invite.signature);

    // Signature must still verify after roundtrip
    assert!(
        restored.verify_signature(&alice.public_key),
        "Signature must verify after serialization roundtrip"
    );
}

// ── Test 10: Inbox ordering (newest first) ──────────────────────────────────

#[tokio::test]
async fn test_inbox_list_orders_newest_first() {
    let inbox = InboxStore::new(None);

    let mut old = make_test_invite("inv-old");
    old.created_at = 1_000;
    let mut mid = make_test_invite("inv-mid");
    mid.created_at = 2_000;
    let mut new = make_test_invite("inv-new");
    new.created_at = 3_000;

    // Add in random order
    inbox.add(mid).await.expect("test: add mid");
    inbox.add(old).await.expect("test: add old");
    inbox.add(new).await.expect("test: add new");

    let list = inbox.list().await;
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].invite_id, "inv-new");
    assert_eq!(list[1].invite_id, "inv-mid");
    assert_eq!(list[2].invite_id, "inv-old");
}
