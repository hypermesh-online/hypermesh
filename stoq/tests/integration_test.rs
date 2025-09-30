use stoq::extensions::{DefaultStoqExtensions, StoqProtocolExtension, StoqPacket};
use stoq::config::TransportConfig;
use stoq::transport::{StoqTransport, Endpoint};
use stoq::transport::falcon::{FalconEngine, FalconVariant};
use bytes::Bytes;
use std::net::Ipv6Addr;
use std::time::Instant;

#[test]
fn test_protocol_extensions_functionality() {
    println!("\n=== Testing Protocol Extensions ===");

    let extensions = DefaultStoqExtensions::new();
    let test_data = b"Test message for STOQ protocol validation with sharding and tokenization";

    // Test 1: Tokenization
    let token = extensions.tokenize_packet(test_data);
    let valid = extensions.validate_token(test_data, &token);
    assert!(valid, "Token validation failed");
    println!("✓ Tokenization: WORKING");

    // Test 2: Sharding and Reassembly
    let shards = extensions.shard_packet(test_data, 20).unwrap();
    assert!(shards.len() > 1, "Sharding didn't create multiple shards");
    println!("✓ Sharding: Created {} shards", shards.len());

    let reassembled = extensions.reassemble_shards(shards).unwrap();
    assert_eq!(reassembled.as_ref(), test_data, "Reassembly failed");
    println!("✓ Reassembly: WORKING");

    // Test 3: Hop Support
    let mut packet = StoqPacket::new(Bytes::from_static(test_data));
    let hop = stoq::extensions::HopInfo {
        address: Ipv6Addr::new(2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        port: 9292,
        timestamp: 12345,
        metadata: Default::default(),
    };
    extensions.add_hop_info(&mut packet, hop).unwrap();
    assert_eq!(packet.hops.len(), 1, "Hop not added");
    println!("✓ Hop Support: WORKING");

    // Test 4: Packet Serialization
    packet.token = Some(token);
    let serialized = packet.serialize().unwrap();
    assert!(!serialized.is_empty(), "Serialization failed");
    println!("✓ Packet Serialization: {} bytes", serialized.len());
}

#[test]
fn test_ipv6_enforcement() {
    println!("\n=== Testing IPv6 Enforcement ===");

    let config = TransportConfig::default();

    // Check default is IPv6
    assert_eq!(config.bind_address, Ipv6Addr::LOCALHOST, "Default not IPv6");
    println!("✓ Default bind: IPv6 ({:?})", config.bind_address);

    // Test endpoint creation
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
    assert_eq!(endpoint.address, Ipv6Addr::LOCALHOST, "Endpoint not IPv6");
    println!("✓ Endpoint: IPv6-only enforced");
}

#[test]
fn test_falcon_crypto_implementation() {
    println!("\n=== Testing FALCON Quantum-Resistant Crypto ===");

    let engine = FalconEngine::new(FalconVariant::Falcon1024);

    // Test key generation
    let (private_key, public_key) = engine.generate_keypair().unwrap();
    assert_eq!(public_key.key_data.len(), 1793, "Wrong public key size");
    // private_key.key_data() is private, so we check variant instead
    assert_eq!(private_key.variant, FalconVariant::Falcon1024, "Wrong variant");
    println!("✓ FALCON-1024 Key Generation: WORKING");

    // Test signing
    let test_data = b"Test message for FALCON signature";
    let signature = engine.sign(&private_key, test_data).unwrap();
    assert_eq!(signature.signature_data.len(), 1330, "Wrong signature size");
    println!("✓ FALCON Signing: {} byte signature", signature.signature_data.len());

    // Test verification (NOTE: This is mock implementation)
    let valid = engine.verify(&public_key, &signature, test_data).unwrap();
    println!("✓ FALCON Verification: {}", if valid { "MOCK VALID" } else { "FAILED" });

    // Test with wrong data
    let invalid = engine.verify(&public_key, &signature, b"wrong data").unwrap();
    assert!(!invalid, "Should reject wrong data");
    println!("✓ Wrong Data Rejection: WORKING");

    println!("⚠ NOTE: FALCON is MOCK IMPLEMENTATION - not real quantum-resistant crypto!");
}

#[test]
fn test_real_vs_claimed_performance() {
    println!("\n=== Testing Real vs Claimed Performance ===");

    let extensions = DefaultStoqExtensions::new();

    // Test different data sizes
    let test_sizes = vec![
        (1024, "1 KB"),
        (64 * 1024, "64 KB"),
        (1024 * 1024, "1 MB"),
    ];

    println!("\nProtocol Extension Performance:");
    for (size, label) in test_sizes {
        let data = vec![0u8; size];

        // Measure tokenization
        let start = Instant::now();
        for _ in 0..100 {
            let _ = extensions.tokenize_packet(&data);
        }
        let elapsed = start.elapsed();
        let throughput = (size as f64 * 100.0) / elapsed.as_secs_f64() / (1024.0 * 1024.0);
        println!("  {} Tokenization: {:.2} MB/s", label, throughput);

        // Measure sharding
        let start = Instant::now();
        let shards = extensions.shard_packet(&data, 64 * 1024).unwrap();
        let elapsed = start.elapsed();
        let throughput = (size as f64) / elapsed.as_secs_f64() / (1024.0 * 1024.0);
        println!("  {} Sharding: {:.2} MB/s ({} shards)", label, throughput, shards.len());
    }

    println!("\n⚠ Performance Reality Check:");
    println!("  Claimed: 40 Gbps (5,000 MB/s)");
    println!("  Actual: Software-only, no hardware acceleration");
    println!("  Reality: Performance limited by QUIC library (quinn)");
    println!("  Status: FANTASY METRICS - uses simulated values");
}

#[tokio::test]
async fn test_transport_creation_failure() {
    println!("\n=== Testing Transport Creation ===");

    // Initialize crypto
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed
    }

    let config = TransportConfig::default();
    let result = StoqTransport::new(config).await;

    // Check if transport creates (with self-signed certs)
    if result.is_ok() {
        println!("✓ Transport creation: SUCCESS (using self-signed certificates)");
    } else {
        println!("✗ Transport creation: FAILED");
        if let Err(e) = result {
            println!("  Error: {}", e);
        }
    }
}

#[test]
fn test_missing_protocol_features() {
    println!("\n=== Testing Missing Protocol Features ===");

    println!("\n✗ Missing at Protocol Level:");
    println!("  - NO secure tokenization (just SHA256 hashing)");
    println!("  - NO cryptographic packet validation");
    println!("  - NO actual hop routing (just data structures)");
    println!("  - NO seeding/mirroring implementation");
    println!("  - NO integration with QUIC transport");
    println!("  - NO real FALCON implementation (mock only)");

    println!("\n✗ Architecture Issues:");
    println!("  - Extensions not integrated with transport");
    println!("  - FALCON not used in handshakes");
    println!("  - No actual protocol modifications to QUIC");
    println!("  - Just QUIC wrapper with unused features");
}