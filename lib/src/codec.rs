// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Canonical binary serialization using postcard
//!
//! All HyperMesh types use postcard for compact, deterministic binary encoding.
//! Postcard is a non-self-describing format optimized for embedded and
//! wire-protocol usage with minimal overhead.
//!
//! # Compatibility note
//!
//! Types with custom `Deserialize` implementations that call `deserialize_any`
//! (e.g. [`PrivacyMode`]) are NOT compatible with postcard because it is a
//! non-self-describing format. Use [`serde_json`] or another self-describing
//! format for those types.

use serde::{Serialize, de::DeserializeOwned};
use crate::error::HypermeshError;

/// Encode a value to postcard binary format.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, HypermeshError> {
    postcard::to_allocvec(value)
        .map_err(|e| HypermeshError::Serialization(format!("postcard encode: {}", e)))
}

/// Decode a value from postcard binary format.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, HypermeshError> {
    postcard::from_bytes(bytes)
        .map_err(|e| HypermeshError::Serialization(format!("postcard decode: {}", e)))
}

/// Calculate the encoded size of a value without a separate allocation.
///
/// Internally encodes to get the exact byte count. For hot paths consider
/// caching the result.
pub fn encoded_size<T: Serialize>(value: &T) -> Result<usize, HypermeshError> {
    let bytes = encode(value)?;
    Ok(bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use crate::proof::*;
    use crate::asset::*;
    use crate::economic::*;
    use std::time::Duration;

    // -----------------------------------------------------------------
    // types.rs roundtrips
    // -----------------------------------------------------------------

    #[test]
    fn roundtrip_node_id() {
        let node = NodeId::from_public_key(b"test-node-42");
        let bytes = encode(&node).expect("test: encode NodeId");
        let decoded: NodeId = decode(&bytes).expect("test: decode NodeId");
        assert_eq!(node, decoded);
    }

    #[test]
    fn roundtrip_asset_id() {
        let id = AssetId("asset-abc-123".to_string());
        let bytes = encode(&id).expect("test: encode AssetId");
        let decoded: AssetId = decode(&bytes).expect("test: decode AssetId");
        assert_eq!(id, decoded);
    }

    #[test]
    fn roundtrip_network_id() {
        let id = NetworkId([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let bytes = encode(&id).expect("test: encode NetworkId");
        let decoded: NetworkId = decode(&bytes).expect("test: decode NetworkId");
        assert_eq!(id, decoded);
    }

    #[test]
    fn roundtrip_content_hash() {
        let hash = ContentHash([0xAB; 32]);
        let bytes = encode(&hash).expect("test: encode ContentHash");
        let decoded: ContentHash = decode(&bytes).expect("test: decode ContentHash");
        assert_eq!(hash, decoded);
    }

    #[test]
    fn roundtrip_blockchain_scope() {
        for scope in [BlockchainScope::Device, BlockchainScope::Network] {
            let bytes = encode(&scope).expect("test: encode BlockchainScope");
            let decoded: BlockchainScope = decode(&bytes).expect("test: decode BlockchainScope");
            assert_eq!(scope, decoded);
        }
    }

    #[test]
    fn roundtrip_proof_type() {
        let types = [ProofType::Space, ProofType::Stake, ProofType::Work, ProofType::Time];
        for pt in types {
            let bytes = encode(&pt).expect("test: encode ProofType");
            let decoded: ProofType = decode(&bytes).expect("test: decode ProofType");
            assert_eq!(pt, decoded);
        }
    }

    #[test]
    fn roundtrip_access_scope() {
        for scope in [AccessScope::Bounded, AccessScope::Unbounded] {
            let bytes = encode(&scope).expect("test: encode AccessScope");
            let decoded: AccessScope = decode(&bytes).expect("test: decode AccessScope");
            assert_eq!(scope, decoded);
        }
    }

    #[test]
    fn roundtrip_pipeline_stage() {
        let stages = [
            PipelineStage::Compress,
            PipelineStage::Encrypt,
            PipelineStage::Shard,
            PipelineStage::Distribute,
        ];
        for stage in stages {
            let bytes = encode(&stage).expect("test: encode PipelineStage");
            let decoded: PipelineStage = decode(&bytes).expect("test: decode PipelineStage");
            assert_eq!(stage, decoded);
        }
    }

    #[test]
    fn roundtrip_matrix_position() {
        let pos = MatrixPosition { x: 1.5, y: -2.3, z: 99.0 };
        let bytes = encode(&pos).expect("test: encode MatrixPosition");
        let decoded: MatrixPosition = decode(&bytes).expect("test: decode MatrixPosition");
        assert_eq!(pos, decoded);
    }

    // -----------------------------------------------------------------
    // asset.rs roundtrips
    // -----------------------------------------------------------------

    #[test]
    fn roundtrip_base_state() {
        let states = [
            BaseState::Available,
            BaseState::Allocated,
            BaseState::InUse,
            BaseState::Suspended,
            BaseState::Maintenance,
            BaseState::Failed,
        ];
        for state in states {
            let bytes = encode(&state).expect("test: encode BaseState");
            let decoded: BaseState = decode(&bytes).expect("test: decode BaseState");
            assert_eq!(state, decoded);
        }
    }

    #[test]
    fn roundtrip_system_asset_kind() {
        let kinds = [
            SystemAssetKind::Cpu,
            SystemAssetKind::Gpu,
            SystemAssetKind::Memory,
            SystemAssetKind::Storage,
            SystemAssetKind::Network,
            SystemAssetKind::Container,
            SystemAssetKind::Economic,
            SystemAssetKind::Blockchain,
            SystemAssetKind::Dns,
        ];
        for kind in kinds {
            let bytes = encode(&kind).expect("test: encode SystemAssetKind");
            let decoded: SystemAssetKind = decode(&bytes).expect("test: decode SystemAssetKind");
            assert_eq!(kind, decoded);
        }
    }

    #[test]
    fn roundtrip_asset_kind_system() {
        let kind = AssetKind::System(SystemAssetKind::Gpu);
        let bytes = encode(&kind).expect("test: encode AssetKind::System");
        let decoded: AssetKind = decode(&bytes).expect("test: decode AssetKind::System");
        assert_eq!(kind, decoded);
    }

    #[test]
    fn roundtrip_asset_kind_user_defined() {
        let kind = AssetKind::UserDefined(UserAssetKind {
            type_name: "MyWidget".to_string(),
            type_hash: ContentHash::zeroed(),
        });
        let bytes = encode(&kind).expect("test: encode AssetKind::UserDefined");
        let decoded: AssetKind = decode(&bytes).expect("test: decode AssetKind::UserDefined");
        assert_eq!(kind, decoded);
    }

    // -----------------------------------------------------------------
    // proof.rs roundtrips
    // -----------------------------------------------------------------

    #[test]
    fn roundtrip_space_proof() {
        let proof = SpaceProof {
            node_id: NodeId::from_public_key(b"node-1"),
            matrix_position: MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
            stored_bytes: 1000,
            committed_bytes: 5000,
            content_hash: ContentHash([0; 32]),
            timestamp_ms: 12345,
        };
        let bytes = encode(&proof).expect("test: encode SpaceProof");
        let decoded: SpaceProof = decode(&bytes).expect("test: decode SpaceProof");
        assert_eq!(proof, decoded);
    }

    #[test]
    fn roundtrip_stake_proof() {
        let proof = StakeProof {
            node_id: NodeId::from_public_key(b"node-1"),
            asset_id: Some(AssetId("asset-001".to_string())),
            stake_amount: 10000,
            signature: vec![1, 2, 3],
            timestamp_ms: 12345,
        };
        let bytes = encode(&proof).expect("test: encode StakeProof");
        let decoded: StakeProof = decode(&bytes).expect("test: decode StakeProof");
        assert_eq!(proof, decoded);
    }

    #[test]
    fn roundtrip_stake_proof_no_asset() {
        let proof = StakeProof {
            node_id: NodeId::from_public_key(b"node-2"),
            asset_id: None,
            stake_amount: 0,
            signature: vec![],
            timestamp_ms: 99999,
        };
        let bytes = encode(&proof).expect("test: encode StakeProof(no asset)");
        let decoded: StakeProof = decode(&bytes).expect("test: decode StakeProof(no asset)");
        assert_eq!(proof, decoded);
    }

    #[test]
    fn roundtrip_work_proof() {
        let proof = WorkProof {
            node_id: NodeId::from_public_key(b"node-1"),
            compute_units: 1000000,
            work_category: WorkCategory::Compute,
            challenge_proof: vec![0xCA, 0xFE],
            timestamp_ms: 12345,
        };
        let bytes = encode(&proof).expect("test: encode WorkProof");
        let decoded: WorkProof = decode(&bytes).expect("test: decode WorkProof");
        assert_eq!(proof, decoded);
    }

    #[test]
    fn roundtrip_time_proof() {
        let proof = TimeProof {
            time_offset: Duration::from_millis(150),
            nonce: 42,
            proof_hash: vec![0xBE, 0xEF],
            timestamp_ms: 12345,
        };
        let bytes = encode(&proof).expect("test: encode TimeProof");
        let decoded: TimeProof = decode(&bytes).expect("test: decode TimeProof");
        assert_eq!(proof, decoded);
    }

    #[test]
    fn roundtrip_proof_of_state() {
        let pos = ProofOfState {
            space: SpaceProof {
                node_id: NodeId::from_public_key(b"node-1"),
                matrix_position: MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
                stored_bytes: 1000,
                committed_bytes: 5000,
                content_hash: ContentHash([0; 32]),
                timestamp_ms: 12345,
            },
            stake: StakeProof {
                node_id: NodeId::from_public_key(b"node-1"),
                asset_id: Some(AssetId("asset-001".to_string())),
                stake_amount: 10000,
                signature: vec![1, 2, 3],
                timestamp_ms: 12345,
            },
            work: WorkProof {
                node_id: NodeId::from_public_key(b"node-1"),
                compute_units: 1000000,
                work_category: WorkCategory::Compute,
                challenge_proof: vec![0xCA, 0xFE],
                timestamp_ms: 12345,
            },
            time: TimeProof {
                time_offset: Duration::from_millis(150),
                nonce: 42,
                proof_hash: vec![0xBE, 0xEF],
                timestamp_ms: 12345,
            },
        };
        let bytes = encode(&pos).expect("test: encode ProofOfState");
        let decoded: ProofOfState = decode(&bytes).expect("test: decode ProofOfState");
        assert_eq!(pos, decoded);
    }

    #[test]
    fn roundtrip_work_category_all_variants() {
        let categories = [
            WorkCategory::Compute,
            WorkCategory::Network,
            WorkCategory::Storage,
            WorkCategory::Cryptographic,
            WorkCategory::Validation,
        ];
        for cat in categories {
            let bytes = encode(&cat).expect("test: encode WorkCategory");
            let decoded: WorkCategory = decode(&bytes).expect("test: decode WorkCategory");
            assert_eq!(cat, decoded);
        }
    }

    #[test]
    fn roundtrip_proof_validation_result() {
        let result = ProofValidationResult {
            space_valid: true,
            stake_valid: false,
            work_valid: true,
            time_valid: false,
        };
        let bytes = encode(&result).expect("test: encode ProofValidationResult");
        let decoded: ProofValidationResult =
            decode(&bytes).expect("test: decode ProofValidationResult");
        assert_eq!(result, decoded);
    }

    // -----------------------------------------------------------------
    // economic.rs roundtrips
    // -----------------------------------------------------------------

    #[test]
    fn roundtrip_packet_id() {
        let mut data = [0u8; 32];
        data[0] = 0xAB;
        data[7] = 0xFF;
        let id = PacketId::new(data);
        let bytes = encode(&id).expect("test: encode PacketId");
        let decoded: PacketId = decode(&bytes).expect("test: decode PacketId");
        assert_eq!(id, decoded);
    }

    #[test]
    fn roundtrip_market_tier() {
        let tiers = [MarketTier::L0, MarketTier::L1, MarketTier::L2, MarketTier::L3];
        for tier in tiers {
            let bytes = encode(&tier).expect("test: encode MarketTier");
            let decoded: MarketTier = decode(&bytes).expect("test: decode MarketTier");
            assert_eq!(tier, decoded);
        }
    }

    #[test]
    fn roundtrip_packet_state_all_variants() {
        let states = [
            PacketState::Minted,
            PacketState::InTransit,
            PacketState::Delivered,
            PacketState::Settling,
            PacketState::Settled,
            PacketState::Held,
            PacketState::Stalled,
            PacketState::Dispersed,
            PacketState::Expired,
            PacketState::Refunded,
            PacketState::Dissolved,
        ];
        for state in states {
            let bytes = encode(&state).expect("test: encode PacketState");
            let decoded: PacketState = decode(&bytes).expect("test: decode PacketState");
            assert_eq!(state, decoded);
        }
    }

    #[test]
    fn roundtrip_demurrage_rate() {
        let rate = DemurrageRate {
            lambda: 1.39e-5,
            max_ttl_secs: 86_400,
        };
        let bytes = encode(&rate).expect("test: encode DemurrageRate");
        let decoded: DemurrageRate = decode(&bytes).expect("test: decode DemurrageRate");
        assert_eq!(rate, decoded);
    }

    // -----------------------------------------------------------------
    // Utility function tests
    // -----------------------------------------------------------------

    #[test]
    fn encoded_size_matches_encode_len() {
        let node = NodeId::from_public_key(b"test");
        let size = encoded_size(&node).expect("test: encoded_size");
        let bytes = encode(&node).expect("test: encode");
        assert_eq!(size, bytes.len());
    }

    #[test]
    fn decode_invalid_data_returns_error() {
        let result: Result<NodeId, _> = decode(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("postcard decode"), "got: {}", err_msg);
    }

    #[test]
    fn decode_empty_data_returns_error() {
        let result: Result<ContentHash, _> = decode(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn postcard_is_compact() {
        // Postcard should be significantly smaller than JSON for structured data
        let kind = SystemAssetKind::Cpu;
        let postcard_bytes = encode(&kind).expect("test: encode");
        let json_bytes = serde_json::to_vec(&kind).expect("test: json encode");
        assert!(
            postcard_bytes.len() < json_bytes.len(),
            "postcard ({} bytes) should be smaller than JSON ({} bytes)",
            postcard_bytes.len(),
            json_bytes.len(),
        );
    }

    // -----------------------------------------------------------------
    // PrivacyMode: NOT compatible with postcard
    // -----------------------------------------------------------------
    // PrivacyMode uses `deserialize_any` in its custom Deserialize impl,
    // which is unsupported by postcard (non-self-describing format).
    // Use serde_json or another self-describing format for PrivacyMode.
    //
    // GoldGrams wraps rust_decimal::Decimal which also uses deserialize_any.
    // Same restriction applies.
}
