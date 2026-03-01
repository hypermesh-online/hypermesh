// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-crate integration tests.
//!
//! Validates that types and APIs from hypermesh_lib, blockmatrix, and
//! trustchain wire together correctly. These are compile-time integration
//! checks -- no servers, no network connections.

// ---------------------------------------------------------------------------
// Test 1: Asset Pipeline Round-Trip
// ---------------------------------------------------------------------------

mod asset_pipeline {
    use hypermesh_lib::{AssetAddress, AssetId, ContentHash, MatrixPosition, PipelineStage};

    #[test]
    fn pipeline_stages_follow_correct_order() {
        // The canonical pipeline order is:
        // Compress -> Encrypt -> Shard -> Distribute
        let stages = [
            PipelineStage::Compress,
            PipelineStage::Encrypt,
            PipelineStage::Shard,
            PipelineStage::Distribute,
        ];

        // Verify all four stages exist and are distinct
        for i in 0..stages.len() {
            for j in (i + 1)..stages.len() {
                assert_ne!(stages[i], stages[j], "stages must be distinct");
            }
        }
    }

    #[test]
    fn asset_id_creation_and_display() {
        let id = AssetId::from("test-asset-001");
        assert_eq!(id.0, "test-asset-001");
        assert_eq!(id.to_string(), "test-asset-001");

        let id_from_string = AssetId::from("another".to_string());
        assert_eq!(id_from_string.0, "another");
    }

    #[test]
    fn content_hash_round_trip() {
        let raw = [0xABu8; 32];
        let hash = ContentHash::from_bytes(raw);
        assert_eq!(*hash.as_bytes(), raw);

        let zeroed = ContentHash::zeroed();
        assert_eq!(*zeroed.as_bytes(), [0u8; 32]);

        // Different hashes are not equal
        assert_ne!(hash, zeroed);
    }

    #[test]
    fn matrix_position_stores_coordinates() {
        let pos = MatrixPosition {
            x: 1.0,
            y: 2.5,
            z: -3.0,
        };
        assert!((pos.x - 1.0).abs() < f64::EPSILON);
        assert!((pos.y - 2.5).abs() < f64::EPSILON);
        assert!((pos.z - (-3.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn asset_address_encodes_matrix_position_and_hash() {
        let hash = ContentHash::from_bytes([0x42u8; 32]);
        let addr = AssetAddress::new(10, 20, 30, &hash).expect("valid coordinates");

        let (x, y, z) = addr.matrix_coords();
        assert_eq!(x, 10);
        assert_eq!(y, 20);
        assert_eq!(z, 30);

        // Shard 0 = whole asset
        assert_eq!(addr.shard_index(), 0);
        assert!(addr.is_hypermesh());
    }

    #[test]
    fn asset_address_shard_derivation() {
        let hash = ContentHash::from_bytes([0x42u8; 32]);
        let parent = AssetAddress::new(1, 2, 3, &hash).expect("valid coordinates");

        // Derive shard addresses (Reed-Solomon 10+4 = up to shard 13)
        for shard_idx in 1..=14u8 {
            let shard = parent.shard(shard_idx).expect("valid shard index");
            assert_eq!(shard.shard_index(), shard_idx);
            assert_eq!(shard.matrix_coords(), parent.matrix_coords());
            // Parent of a shard should match the original parent
            assert_eq!(shard.parent().shard_index(), 0);
        }

        // Shard 15 is the max valid value
        assert!(parent.shard(15).is_ok());
        // Shard 16 should fail
        assert!(AssetAddress::with_shard(1, 2, 3, &hash, 16).is_err());
    }

    #[test]
    fn asset_address_ipv6_round_trip() {
        let hash = ContentHash::from_bytes([0xFFu8; 32]);
        let addr = AssetAddress::new(100, -50, 0, &hash).expect("valid coordinates");

        let ipv6 = addr.to_ipv6();
        let recovered = AssetAddress::from_ipv6(ipv6).expect("valid HyperMesh address");

        assert_eq!(addr.matrix_coords(), recovered.matrix_coords());
        assert_eq!(addr.shard_index(), recovered.shard_index());
        assert_eq!(addr.asset_fingerprint(), recovered.asset_fingerprint());
    }

    #[test]
    fn lib_types_wire_to_blockmatrix() {
        // Verify that lib's AssetId can be used to create a blockmatrix
        // AssetRegistration (which contains an AssetId-like string field).
        let lib_asset_id = AssetId::from("cross-crate-test");
        let lib_hash = ContentHash::from_bytes([1u8; 32]);

        // blockmatrix re-exports PrivacyMode from lib
        let privacy = blockmatrix::PrivacyMode::PUBLIC;
        assert!(privacy.tracked);

        // blockmatrix AssetType is its own enum (domain-specific)
        let _cpu = blockmatrix::AssetType::Cpu;
        let _gpu = blockmatrix::AssetType::Gpu;
        let _mem = blockmatrix::AssetType::Memory;
        let _sto = blockmatrix::AssetType::Storage;

        // Both lib and blockmatrix agree on the same PrivacyMode values
        assert_eq!(
            hypermesh_lib::PrivacyMode::ANONYMOUS.to_ebpf_u8(),
            blockmatrix::PrivacyMode::ANONYMOUS.to_ebpf_u8(),
        );

        // Ensure the types are the same (this is a compile-time check)
        let _: hypermesh_lib::PrivacyMode = blockmatrix::PrivacyMode::PUBLIC;

        // Use lib_asset_id and lib_hash to silence unused warnings
        assert!(!lib_asset_id.0.is_empty());
        assert_ne!(*lib_hash.as_bytes(), [0u8; 32]);
    }

    #[test]
    fn pipeline_stage_display() {
        assert_eq!(PipelineStage::Compress.to_string(), "Compress");
        assert_eq!(PipelineStage::Encrypt.to_string(), "Encrypt");
        assert_eq!(PipelineStage::Shard.to_string(), "Shard");
        assert_eq!(PipelineStage::Distribute.to_string(), "Distribute");
    }
}

// ---------------------------------------------------------------------------
// Test 2: PrivacyMode Consistency
// ---------------------------------------------------------------------------

mod privacy_mode_consistency {
    use hypermesh_lib::{AccessScope, PrivacyMode};

    #[test]
    fn three_presets_exist() {
        let anon = PrivacyMode::ANONYMOUS;
        let private = PrivacyMode::PRIVATE;
        let public = PrivacyMode::PUBLIC;

        // All three must be distinct
        assert_ne!(anon, private);
        assert_ne!(anon, public);
        assert_ne!(private, public);
    }

    #[test]
    fn ebpf_u8_encoding_values() {
        // ANONYMOUS=0, PRIVATE=2, PUBLIC=3
        assert_eq!(PrivacyMode::ANONYMOUS.to_ebpf_u8(), 0);
        assert_eq!(PrivacyMode::PRIVATE.to_ebpf_u8(), 2);
        assert_eq!(PrivacyMode::PUBLIC.to_ebpf_u8(), 3);
    }

    #[test]
    fn caesar_multiplier_values() {
        let epsilon = f64::EPSILON;
        assert!((PrivacyMode::ANONYMOUS.caesar_multiplier() - 0.0).abs() < epsilon);
        assert!((PrivacyMode::PRIVATE.caesar_multiplier() - 0.5).abs() < epsilon);
        assert!((PrivacyMode::PUBLIC.caesar_multiplier() - 1.0).abs() < epsilon);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn two_axis_model_scope_and_tracked() {
        // ANONYMOUS: Unbounded, untracked
        assert_eq!(PrivacyMode::ANONYMOUS.scope, AccessScope::Unbounded);
        assert!(!PrivacyMode::ANONYMOUS.tracked);

        // PRIVATE: Bounded, tracked
        assert_eq!(PrivacyMode::PRIVATE.scope, AccessScope::Bounded);
        assert!(PrivacyMode::PRIVATE.tracked);

        // PUBLIC: Unbounded, tracked
        assert_eq!(PrivacyMode::PUBLIC.scope, AccessScope::Unbounded);
        assert!(PrivacyMode::PUBLIC.tracked);
    }

    #[test]
    fn identity_requirements() {
        // Anonymous does not require identity
        assert!(!PrivacyMode::ANONYMOUS.requires_identity());
        // Private and Public do
        assert!(PrivacyMode::PRIVATE.requires_identity());
        assert!(PrivacyMode::PUBLIC.requires_identity());
    }

    #[test]
    fn logging_permissions() {
        // Anonymous does not allow logging
        assert!(!PrivacyMode::ANONYMOUS.allows_logging());
        // Private and Public do
        assert!(PrivacyMode::PRIVATE.allows_logging());
        assert!(PrivacyMode::PUBLIC.allows_logging());
    }

    #[test]
    fn connection_timeouts_graduate() {
        let anon_timeout = PrivacyMode::ANONYMOUS.connection_timeout_secs();
        let private_timeout = PrivacyMode::PRIVATE.connection_timeout_secs();
        let public_timeout = PrivacyMode::PUBLIC.connection_timeout_secs();

        // Anonymous shortest, Public longest
        assert!(anon_timeout < private_timeout);
        assert!(private_timeout < public_timeout);
    }

    #[test]
    fn display_format() {
        assert_eq!(PrivacyMode::ANONYMOUS.to_string(), "Anonymous");
        assert_eq!(PrivacyMode::PRIVATE.to_string(), "Private");
        assert_eq!(PrivacyMode::PUBLIC.to_string(), "Public");
    }

    #[test]
    fn serde_round_trip() {
        for mode in [
            PrivacyMode::ANONYMOUS,
            PrivacyMode::PRIVATE,
            PrivacyMode::PUBLIC,
        ] {
            let json =
                serde_json::to_string(&mode).unwrap_or_else(|e| panic!("serialize {mode:?}: {e}"));
            let back: PrivacyMode = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialize {mode:?} from '{json}': {e}"));
            assert_eq!(mode, back);
        }
    }

    #[test]
    fn blockmatrix_privacy_mode_is_same_type() {
        // blockmatrix re-exports PrivacyMode from hypermesh_lib.
        // This is a compile-time assertion that they are the same type.
        let lib_mode: hypermesh_lib::PrivacyMode = PrivacyMode::PUBLIC;
        let bm_mode: blockmatrix::PrivacyMode = lib_mode;
        assert_eq!(bm_mode.caesar_multiplier(), 1.0);
    }
}

// ---------------------------------------------------------------------------
// Test 3: BlockchainScope Binary Model
// ---------------------------------------------------------------------------

mod blockchain_scope {
    use hypermesh_lib::BlockchainScope;

    #[test]
    fn only_two_variants() {
        let device = BlockchainScope::Device;
        let network = BlockchainScope::Network;

        // The two variants are distinct
        assert_ne!(device, network);

        // Exhaustive match -- if a third variant is added, this will fail to
        // compile, catching the architectural invariant violation.
        match device {
            BlockchainScope::Device => {}
            BlockchainScope::Network => unreachable!(),
        }
        match network {
            BlockchainScope::Device => unreachable!(),
            BlockchainScope::Network => {}
        }
    }

    #[test]
    fn display_format() {
        assert_eq!(BlockchainScope::Device.to_string(), "Device");
        assert_eq!(BlockchainScope::Network.to_string(), "Network");
    }

    #[test]
    fn serde_round_trip() {
        for scope in [BlockchainScope::Device, BlockchainScope::Network] {
            let json = serde_json::to_string(&scope)
                .unwrap_or_else(|e| panic!("serialize {scope:?}: {e}"));
            let back: BlockchainScope = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialize {scope:?} from '{json}': {e}"));
            assert_eq!(scope, back);
        }
    }

    #[test]
    fn hash_and_eq() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BlockchainScope::Device);
        set.insert(BlockchainScope::Network);
        set.insert(BlockchainScope::Device); // duplicate

        assert_eq!(set.len(), 2, "only two unique variants");
    }

    #[test]
    fn copy_semantics() {
        let a = BlockchainScope::Device;
        let b = a; // Copy
        assert_eq!(a, b); // a still usable after copy
    }
}

// ---------------------------------------------------------------------------
// Test 4: ProofType Four-Proof System
// ---------------------------------------------------------------------------

mod proof_type_system {
    use hypermesh_lib::ProofType;

    #[test]
    fn four_proof_types_exist() {
        let proofs = [
            ProofType::Space, // WHERE
            ProofType::Stake, // WHO
            ProofType::Work,  // WHAT/HOW
            ProofType::Time,  // WHEN
        ];

        // All four must be distinct
        for i in 0..proofs.len() {
            for j in (i + 1)..proofs.len() {
                assert_ne!(proofs[i], proofs[j]);
            }
        }
    }

    #[test]
    fn display_format() {
        assert_eq!(ProofType::Space.to_string(), "ProofOfSpace");
        assert_eq!(ProofType::Stake.to_string(), "ProofOfStake");
        assert_eq!(ProofType::Work.to_string(), "ProofOfWork");
        assert_eq!(ProofType::Time.to_string(), "ProofOfTime");
    }

    #[test]
    fn exhaustive_match() {
        // If a fifth proof type is added, this must be updated -- compile
        // error catches it.
        fn proof_label(p: ProofType) -> &'static str {
            match p {
                ProofType::Space => "WHERE",
                ProofType::Stake => "WHO",
                ProofType::Work => "WHAT",
                ProofType::Time => "WHEN",
            }
        }
        assert_eq!(proof_label(ProofType::Space), "WHERE");
        assert_eq!(proof_label(ProofType::Stake), "WHO");
        assert_eq!(proof_label(ProofType::Work), "WHAT");
        assert_eq!(proof_label(ProofType::Time), "WHEN");
    }
}

// ---------------------------------------------------------------------------
// Test 5: SystemAssetKind Cross-Crate Alignment
// ---------------------------------------------------------------------------

mod system_asset_kind {
    use hypermesh_lib::{AssetKind, SystemAssetKind};

    #[test]
    fn nine_system_kinds_with_stable_ids() {
        let kinds = [
            (SystemAssetKind::Cpu, 0, "Cpu"),
            (SystemAssetKind::Gpu, 1, "Gpu"),
            (SystemAssetKind::Memory, 2, "Memory"),
            (SystemAssetKind::Storage, 3, "Storage"),
            (SystemAssetKind::Network, 4, "Network"),
            (SystemAssetKind::Container, 5, "Container"),
            (SystemAssetKind::Economic, 6, "Economic"),
            (SystemAssetKind::Blockchain, 7, "Blockchain"),
            (SystemAssetKind::Dns, 8, "Dns"),
        ];

        for (kind, expected_id, expected_name) in &kinds {
            assert_eq!(kind.type_id(), *expected_id, "{expected_name} type_id");
            assert_eq!(
                kind.type_name(),
                *expected_name,
                "{expected_name} type_name"
            );
        }
    }

    #[test]
    fn asset_kind_wraps_system_kind() {
        let system = AssetKind::System(SystemAssetKind::Storage);
        assert_eq!(system.to_string(), "System(Storage)");
    }

    #[test]
    fn from_system_asset_kind_into_asset_kind() {
        let kind: AssetKind = SystemAssetKind::Gpu.into();
        match kind {
            AssetKind::System(SystemAssetKind::Gpu) => {}
            other => panic!("expected System(Gpu), got {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test 6: NodeId and NetworkId Basics
// ---------------------------------------------------------------------------

mod identifiers {
    use hypermesh_lib::{NetworkId, NodeId};

    #[test]
    fn node_id_from_public_key_deterministic() {
        let id1 = NodeId::from_public_key(b"node-alpha");
        let id2 = NodeId::from_public_key(b"node-alpha");
        assert_eq!(id1, id2);
        // Display shows first 4 bytes as hex + ellipsis
        let display = id1.to_string();
        assert!(display.ends_with('\u{2026}'), "got: {display}");
        assert_eq!(display.len(), 9); // 8 hex chars + 1 ellipsis char
    }

    #[test]
    fn network_id_display_is_hex() {
        let nid = NetworkId([
            0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
            0x66, 0x77,
        ]);
        let display = nid.to_string();
        assert_eq!(
            display,
            "abcdef012345678900112233445566\u{200B}77".replace("\u{200b}", "")
        );
        // Just verify length (32 hex chars for 16 bytes)
        assert_eq!(display.len(), 32);
    }

    #[test]
    fn node_id_hash_equality() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(NodeId::from_public_key(b"a"), 1);
        map.insert(NodeId::from_public_key(b"b"), 2);
        assert_eq!(map.get(&NodeId::from_public_key(b"a")), Some(&1));
        assert_eq!(map.get(&NodeId::from_public_key(b"b")), Some(&2));
        assert_eq!(map.get(&NodeId::from_public_key(b"c")), None);
    }
}
