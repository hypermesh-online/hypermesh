// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! End-to-end system tests for Catalog functionality
//!
//! This test suite validates core Catalog subsystems including:
//! - CatalogRegistry (type registration, search, dependency resolution)
//! - AssetTypeDefinition (validation rules, schema checking)
//! - Content addressing (ContentAddress, MerkleTree, ContentChunker)
//! - DHT package discovery (DhtNodeId, Distance)

mod common;

use std::time::Duration;

use serde_json::json;

use catalog::registry::{
    AssetTypeDefinition, CatalogRegistry, RegistryConfig, SearchQuery, TrustPolicy, ValidationRule,
    ValidationRuleType,
};

use catalog::distribution::content_addressing::{
    CompressionType, ContentAddress, ContentChunker, MerkleTree,
};

use catalog::distribution::dht::DhtNodeId;

use blockmatrix::assets::ConsensusProof;
use blockmatrix::consensus::proof_of_state_integration::{
    SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
};
use hypermesh_lib::PrivacyMode;

// ---------------------------------------------------------------------------
// Helper: construct a valid ConsensusProof for tests
// ---------------------------------------------------------------------------
fn test_consensus_proof() -> ConsensusProof {
    let stake = StakeProof::new("test-holder".to_string(), "test-id".to_string(), 1000);
    let space = SpaceProof::new("test-node".to_string(), "/test".to_string(), 1024);
    let work = WorkProof::new(
        "test-owner".to_string(),
        "test-workload".to_string(),
        12345,
        100,
        WorkloadType::Compute,
        WorkState::Completed,
    );
    let time = TimeProof::new(Duration::from_secs(10));
    ConsensusProof::new(stake, time, space, work)
}

// ===========================================================================
// CatalogRegistry
// ===========================================================================

#[tokio::test]
async fn test_registry_register_and_find_type() {
    let registry = CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    );

    let schema = json!({
        "type": "object",
        "properties": {
            "vin": { "type": "string" },
            "make": { "type": "string" }
        },
        "required": ["vin", "make"]
    });

    let type_def = AssetTypeDefinition::new("Vehicle".to_string(), schema, test_consensus_proof());

    let asset_id = registry.register_type(type_def).await.unwrap();
    let found_id = registry.find_type("Vehicle").await.unwrap();
    assert_eq!(asset_id, found_id);
}

#[tokio::test]
async fn test_registry_duplicate_type_fails() {
    let registry = CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    );

    let schema = json!({"type": "object"});
    let type1 = AssetTypeDefinition::new(
        "DuplicateType".to_string(),
        schema.clone(),
        test_consensus_proof(),
    );
    let type2 =
        AssetTypeDefinition::new("DuplicateType".to_string(), schema, test_consensus_proof());

    registry.register_type(type1).await.unwrap();
    let result = registry.register_type(type2).await;
    assert!(result.is_err(), "Registering a duplicate type should fail");
}

#[tokio::test]
async fn test_registry_search_types_with_scoring() {
    let registry = CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    );

    // Register multiple types
    for name in &["Vehicle", "VehicleInsurance", "Driver", "DriverLicense"] {
        let schema = json!({"type": "object"});
        let type_def = AssetTypeDefinition::new(name.to_string(), schema, test_consensus_proof());
        registry.register_type(type_def).await.unwrap();
    }

    // Search for "Vehicle" should match 2 types
    let query = SearchQuery {
        query: "Vehicle".to_string(),
        ..Default::default()
    };
    let results = registry.search_types(&query).await.unwrap();
    assert_eq!(
        results.results.len(),
        2,
        "Should find Vehicle and VehicleInsurance"
    );

    // Search for "Driver" should match 2 types
    let query = SearchQuery {
        query: "Driver".to_string(),
        ..Default::default()
    };
    let results = registry.search_types(&query).await.unwrap();
    assert_eq!(
        results.results.len(),
        2,
        "Should find Driver and DriverLicense"
    );

    // Empty query should return all
    let query = SearchQuery {
        query: String::new(),
        ..Default::default()
    };
    let results = registry.search_types(&query).await.unwrap();
    assert_eq!(
        results.total_count, 4,
        "Empty query should return all types"
    );
}

#[tokio::test]
async fn test_registry_resolve_dependencies() {
    let registry = CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    );

    let schema = json!({"type": "object"});
    let type_def = AssetTypeDefinition::new("DepTest".to_string(), schema, test_consensus_proof());
    registry.register_type(type_def).await.unwrap();

    // resolve_dependencies returns empty for now (stub)
    let deps = registry.resolve_dependencies("DepTest").await.unwrap();
    assert!(
        deps.is_empty(),
        "resolve_dependencies currently returns empty"
    );
}

#[tokio::test]
async fn test_registry_list_types() {
    let registry = CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    );

    let names = vec!["Alpha", "Beta", "Gamma"];
    for name in &names {
        let schema = json!({"type": "object"});
        let type_def = AssetTypeDefinition::new(name.to_string(), schema, test_consensus_proof());
        registry.register_type(type_def).await.unwrap();
    }

    let listed = registry.list_types().await.unwrap();
    assert_eq!(listed.len(), 3);
    for name in &names {
        assert!(listed.contains(&name.to_string()));
    }
}

#[tokio::test]
async fn test_registry_statistics() {
    let registry = CatalogRegistry::new(
        PrivacyMode::PUBLIC,
        TrustPolicy::default(),
        RegistryConfig::default(),
    );

    let stats = registry.get_statistics().await;
    assert_eq!(stats.total_types, 0);
    // PrivacyMode::PUBLIC Debug format includes "Unbounded" and "tracked: true"
    assert!(
        stats.privacy_level.contains("Unbounded") && stats.privacy_level.contains("true"),
        "Privacy level should represent PUBLIC mode, got: {}",
        stats.privacy_level
    );

    let schema = json!({"type": "object"});
    let type_def =
        AssetTypeDefinition::new("StatsTest".to_string(), schema, test_consensus_proof());
    registry.register_type(type_def).await.unwrap();

    let stats = registry.get_statistics().await;
    assert_eq!(stats.total_types, 1);
}

// ===========================================================================
// AssetTypeDefinition validation
// ===========================================================================

#[test]
fn test_type_definition_validate_valid_instance() {
    let schema = json!({
        "type": "object",
        "properties": {
            "vin": { "type": "string" },
            "make": { "type": "string" }
        }
    });

    let type_def = AssetTypeDefinition::new("Vehicle".to_string(), schema, test_consensus_proof());

    let instance = json!({"vin": "1HGBH41JXMN109186", "make": "Honda"});
    let result = type_def.validate_instance(&instance).unwrap();
    assert!(result.valid, "Valid object instance should pass validation");
    assert!(result.errors.is_empty());
}

#[test]
fn test_type_definition_validate_invalid_instance() {
    let schema = json!({"type": "object"});
    let type_def =
        AssetTypeDefinition::new("StrictType".to_string(), schema, test_consensus_proof());

    // A string is not an object, so validation should fail
    let instance = json!("not an object");
    let result = type_def.validate_instance(&instance).unwrap();
    assert!(
        !result.valid,
        "Non-object instance should fail schema validation"
    );
    assert!(!result.errors.is_empty());
}

#[test]
fn test_type_definition_with_validation_rules() {
    let schema = json!({"type": "object"});
    let mut type_def =
        AssetTypeDefinition::new("RuledType".to_string(), schema, test_consensus_proof());

    // Add Schema validation rule
    type_def.add_validation_rule(ValidationRule {
        name: "schema-check".to_string(),
        rule_type: ValidationRuleType::Schema,
        definition: json!({}),
        error_message: "Schema validation failed".to_string(),
    });

    // Add Range rule (currently stub, should pass)
    type_def.add_validation_rule(ValidationRule {
        name: "range-check".to_string(),
        rule_type: ValidationRuleType::Range,
        definition: json!({"min": 0, "max": 100}),
        error_message: "Range validation failed".to_string(),
    });

    // Add Enum rule (currently stub, should pass)
    type_def.add_validation_rule(ValidationRule {
        name: "enum-check".to_string(),
        rule_type: ValidationRuleType::Enum,
        definition: json!({"values": ["A", "B", "C"]}),
        error_message: "Enum validation failed".to_string(),
    });

    let instance = json!({"value": 42});
    let result = type_def.validate_instance(&instance).unwrap();
    assert!(result.valid, "Object with stub rules should pass");
}

#[test]
fn test_type_definition_add_dependency() {
    let schema = json!({"type": "object"});
    let mut type_def =
        AssetTypeDefinition::new("WithDeps".to_string(), schema, test_consensus_proof());

    type_def.add_dependency("BaseType".to_string());
    type_def.add_dependency("MixinType".to_string());
    assert_eq!(type_def.dependencies.len(), 2);
}

#[test]
fn test_type_definition_serialization_roundtrip() {
    let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
    let type_def = AssetTypeDefinition::new("SerTest".to_string(), schema, test_consensus_proof());

    let bytes = type_def.to_storage_format().unwrap();
    let restored = AssetTypeDefinition::from_storage_format(&bytes).unwrap();
    assert_eq!(restored.type_name, "SerTest");
}

// ===========================================================================
// Content Addressing
// ===========================================================================

#[test]
fn test_content_address_from_data_deterministic() {
    let data = b"hello hypermesh world";
    let addr1 = ContentAddress::from_data(data);
    let addr2 = ContentAddress::from_data(data);
    assert_eq!(
        addr1, addr2,
        "Same data should produce same content address"
    );
}

#[test]
fn test_content_address_different_data_different_hash() {
    let addr1 = ContentAddress::from_data(b"data-a");
    let addr2 = ContentAddress::from_data(b"data-b");
    assert_ne!(
        addr1, addr2,
        "Different data should produce different addresses"
    );
}

#[test]
fn test_content_address_hex_roundtrip() {
    let addr = ContentAddress::from_data(b"roundtrip test");
    let hex = addr.to_hex();
    assert_eq!(hex.len(), 64, "SHA-256 hex should be 64 characters");
    let restored = ContentAddress::from_hex(&hex).unwrap();
    assert_eq!(addr, restored);
}

#[test]
fn test_content_address_invalid_hex() {
    let result = ContentAddress::from_hex("invalid_hex!");
    assert!(result.is_err());

    // Wrong length
    let result = ContentAddress::from_hex("aabb");
    assert!(result.is_err());
}

// ===========================================================================
// Merkle Tree
// ===========================================================================

#[test]
fn test_merkle_tree_from_chunks_single() {
    let chunks = vec![b"only chunk".to_vec()];
    let tree = MerkleTree::from_chunks(&chunks).unwrap();
    assert_eq!(tree.chunk_count(), 1);
    assert!(tree.verify_chunk(0, &chunks[0]).unwrap());
}

#[test]
fn test_merkle_tree_from_chunks_multiple() {
    let chunks: Vec<Vec<u8>> = (0..4).map(|i| format!("chunk-{i}").into_bytes()).collect();
    let tree = MerkleTree::from_chunks(&chunks).unwrap();
    assert_eq!(tree.chunk_count(), 4);

    // Verify each chunk
    for (i, chunk) in chunks.iter().enumerate() {
        assert!(
            tree.verify_chunk(i, chunk).unwrap(),
            "Chunk {i} should verify"
        );
    }
}

#[test]
fn test_merkle_tree_tampered_chunk_fails() {
    let chunks = vec![b"chunk-0".to_vec(), b"chunk-1".to_vec()];
    let tree = MerkleTree::from_chunks(&chunks).unwrap();

    // Tampered data should fail verification
    assert!(!tree.verify_chunk(0, b"tampered").unwrap());
}

#[test]
fn test_merkle_tree_proof_generation_and_verification() {
    let chunks: Vec<Vec<u8>> = (0..8)
        .map(|i| format!("data-block-{i}").into_bytes())
        .collect();
    let tree = MerkleTree::from_chunks(&chunks).unwrap();

    // Get and verify proof for chunk 3
    let proof = tree.get_proof(3).unwrap();
    assert!(
        MerkleTree::verify_proof(&proof, &chunks[3]),
        "Proof should verify for correct data"
    );

    // Proof should fail for wrong data
    assert!(
        !MerkleTree::verify_proof(&proof, b"wrong data"),
        "Proof should fail for tampered data"
    );
}

#[test]
fn test_merkle_tree_empty_chunks_fails() {
    let chunks: Vec<Vec<u8>> = vec![];
    let result = MerkleTree::from_chunks(&chunks);
    assert!(result.is_err(), "Empty chunks should fail");
}

#[test]
fn test_merkle_tree_root_hash_consistency() {
    let chunks: Vec<Vec<u8>> = vec![b"a".to_vec(), b"b".to_vec()];
    let tree1 = MerkleTree::from_chunks(&chunks).unwrap();
    let tree2 = MerkleTree::from_chunks(&chunks).unwrap();
    assert_eq!(
        tree1.root_hash(),
        tree2.root_hash(),
        "Same chunks should produce same root"
    );
}

// ===========================================================================
// ContentChunker
// ===========================================================================

#[test]
fn test_content_chunker_split_and_reassemble() {
    let data = b"Hello, this is a test of the content chunking system for HyperMesh catalog!";
    let chunker = ContentChunker::new(16, CompressionType::None);

    let chunks = chunker.chunk_data(data).unwrap();
    assert!(
        chunks.len() > 1,
        "Data should be split into multiple chunks"
    );

    let reassembled = chunker.reassemble(&chunks).unwrap();
    assert_eq!(
        reassembled.as_slice(),
        data.as_slice(),
        "Reassembled data should match original"
    );
}

#[test]
fn test_content_chunker_single_chunk() {
    let data = b"small";
    let chunker = ContentChunker::new(1024, CompressionType::None);

    let chunks = chunker.chunk_data(data).unwrap();
    assert_eq!(chunks.len(), 1);

    let reassembled = chunker.reassemble(&chunks).unwrap();
    assert_eq!(reassembled.as_slice(), data.as_slice());
}

// ===========================================================================
// DHT Node ID and Distance
// ===========================================================================

#[test]
fn test_dht_node_id_random_uniqueness() {
    let id1 = DhtNodeId::random();
    let id2 = DhtNodeId::random();
    assert_ne!(id1, id2, "Random DHT node IDs should be unique");
}

#[test]
fn test_dht_node_id_xor_distance_self_is_zero() {
    let id = DhtNodeId::random();
    let dist = id.distance(&id);
    assert_eq!(
        dist.bucket_index(),
        0,
        "Distance to self should have bucket 0"
    );
}

#[test]
fn test_dht_node_id_xor_distance_symmetry() {
    let id1 = DhtNodeId::random();
    let id2 = DhtNodeId::random();
    let d1 = id1.distance(&id2);
    let d2 = id2.distance(&id1);
    assert_eq!(d1, d2, "XOR distance should be symmetric");
}

#[test]
fn test_dht_node_id_hex_display() {
    let id = DhtNodeId::random();
    let hex = id.to_hex();
    assert_eq!(hex.len(), 64, "DHT node ID hex should be 64 characters");
    // Display shows first 8 chars
    let display = format!("{id}");
    assert_eq!(display.len(), 8);
    assert_eq!(&display, &hex[..8]);
}

// ===========================================================================
// Legacy tests (gated behind future-tests feature)
// ===========================================================================

#[cfg(feature = "future-tests")]
mod future_full_system_tests {
    use blockmatrix::assets::core::{AssetId, AssetManager, AssetType};
    use blockmatrix::consensus::{ConsensusProof, ProofType};
    use blockmatrix::extensions::{Extension, ExtensionRequest, ExtensionResponse};
    use catalog::{
        AssetLibrary, CatalogConfig, CatalogExtension, DistributionConfig, P2PNode, Package,
        PackageVersion, SecurityConfig, ValidationReport,
    };
    use hypermesh_lib::PrivacyMode;
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, Instant, SystemTime};
    use stoq::transport::{QuicTransport, TransportConfig};
    use tokio::time::sleep;
    use tracing::{debug, error, info, warn};
    use trustchain::{CertificateChain, TrustChainClient, VerificationResult};

    mod common {
        pub use crate::common::*;
    }

    fn init_test_logging() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter("debug")
            .try_init();
    }

    async fn create_test_catalog() -> CatalogExtension {
        let config = CatalogConfig {
            storage_path: PathBuf::from(format!("./test-data/catalog-{}", uuid::Uuid::new_v4())),
            max_libraries: 100,
            max_packages_per_library: 1000,
            enable_p2p: false,
            enable_trustchain: false,
            enable_consensus: false,
            cache_size_mb: 10,
            auto_verify: false,
            security_config: SecurityConfig {
                enforce_signatures: false,
                require_consensus: false,
                audit_enabled: true,
                max_package_size_mb: 10,
            },
        };

        CatalogExtension::new(config).await.unwrap()
    }

    async fn create_test_library(catalog: &CatalogExtension, name: &str) -> String {
        let request = ExtensionRequest {
            id: format!("create-lib-{}", name),
            method: "create_library".to_string(),
            params: serde_json::json!({
                "name": name,
                "description": format!("Test library {}", name),
                "tags": ["test"]
            }),
            consensus_proof: None,
        };

        let response = catalog.handle_request(request).await.unwrap();
        response.data["id"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_full_system_initialization() {
        init_test_logging();
        let catalog = create_test_catalog().await;
        let handlers = catalog.register_assets().await.unwrap();
        assert!(!handlers.is_empty());
    }
}
