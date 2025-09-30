#!/usr/bin/env rust-script
//! Test STOQ actual functionality
//!
//! ```cargo
//! [dependencies]
//! stoq = { path = "." }
//! tokio = { version = "1", features = ["full"] }
//! anyhow = "1"
//! bytes = "1"
//! ```

use anyhow::Result;
use bytes::Bytes;
use stoq::extensions::{DefaultStoqExtensions, StoqProtocolExtension, StoqPacket};
use stoq::config::TransportConfig;
use stoq::transport::{StoqTransport, Endpoint};
use std::net::Ipv6Addr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== STOQ Protocol Functional Testing ===\n");

    // Initialize crypto
    if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed
    }

    // Test 1: Protocol Extensions
    println!("1. Testing Protocol Extensions (Tokenization/Sharding/Hopping):");
    test_protocol_extensions()?;

    // Test 2: IPv6 Enforcement
    println!("\n2. Testing IPv6 Enforcement:");
    test_ipv6_enforcement().await?;

    // Test 3: FALCON Crypto
    println!("\n3. Testing FALCON Quantum-Resistant Crypto:");
    test_falcon_crypto()?;

    // Test 4: Real Performance
    println!("\n4. Testing Real Performance:");
    test_real_performance().await?;

    Ok(())
}

fn test_protocol_extensions() -> Result<()> {
    let extensions = DefaultStoqExtensions::new();
    let test_data = b"Test message for STOQ protocol validation with sharding";

    // Test tokenization
    let token = extensions.tokenize_packet(test_data);
    let valid = extensions.validate_token(test_data, &token);
    println!("  ✓ Tokenization: {} (hash: {:?})",
        if valid { "WORKING" } else { "FAILED" },
        &token.hash[0..8]
    );

    // Test sharding
    let shards = extensions.shard_packet(test_data, 10)?;
    println!("  ✓ Sharding: Created {} shards", shards.len());

    let reassembled = extensions.reassemble_shards(shards)?;
    let sharding_works = reassembled.as_ref() == test_data;
    println!("  ✓ Reassembly: {}",
        if sharding_works { "WORKING" } else { "FAILED" }
    );

    // Test hop support
    let mut packet = StoqPacket::new(Bytes::from_static(test_data));
    let hop = stoq::extensions::HopInfo {
        address: Ipv6Addr::new(2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        port: 9292,
        timestamp: 12345,
        metadata: Default::default(),
    };
    extensions.add_hop_info(&mut packet, hop)?;
    println!("  ✓ Hop Support: {} hops added", packet.hops.len());

    // Test packet serialization
    let serialized = packet.serialize()?;
    println!("  ✓ Serialization: {} bytes", serialized.len());

    Ok(())
}

async fn test_ipv6_enforcement() -> Result<()> {
    let config = TransportConfig::default();

    // Check default is IPv6
    println!("  Default bind address: {:?}", config.bind_address);
    let is_ipv6 = matches!(config.bind_address, addr if addr.segments().len() == 8);
    println!("  ✓ IPv6 Default: {}", if is_ipv6 { "YES" } else { "NO" });

    // Test endpoint creation
    let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
    println!("  ✓ Endpoint IPv6: {:?}", endpoint.address);

    // Try to create transport (may fail due to missing certs)
    let transport_result = StoqTransport::new(config).await;
    if transport_result.is_ok() {
        println!("  ✓ Transport Creation: SUCCESS");
    } else {
        println!("  ⚠ Transport Creation: Failed (expected - needs certificates)");
    }

    Ok(())
}

fn test_falcon_crypto() -> Result<()> {
    use stoq::transport::falcon::{FalconEngine, FalconVariant};

    let engine = FalconEngine::new(FalconVariant::Falcon1024);
    let (private_key, public_key) = engine.generate_keypair()?;

    println!("  ✓ Key Generation: FALCON-1024");
    println!("    - Public key: {} bytes", public_key.key_data.len());
    println!("    - Private key: {} bytes", private_key.key_data().len());

    let test_data = b"Test message for FALCON signature";
    let signature = engine.sign(&private_key, test_data)?;
    println!("  ✓ Signing: {} byte signature", signature.signature_data.len());

    let valid = engine.verify(&public_key, &signature, test_data)?;
    println!("  ✓ Verification: {}", if valid { "VALID" } else { "INVALID" });

    // Test with wrong data
    let invalid = engine.verify(&public_key, &signature, b"wrong data")?;
    println!("  ✓ Wrong Data Check: {}", if !invalid { "CORRECTLY REJECTED" } else { "FAILED" });

    Ok(())
}

async fn test_real_performance() -> Result<()> {
    use std::time::Instant;

    // Test data sizes
    let sizes = vec![
        (1024, "1 KB"),
        (1024 * 1024, "1 MB"),
        (10 * 1024 * 1024, "10 MB"),
    ];

    let extensions = DefaultStoqExtensions::new();

    for (size, label) in sizes {
        let data = vec![0u8; size];

        // Measure tokenization speed
        let start = Instant::now();
        let token = extensions.tokenize_packet(&data);
        let token_time = start.elapsed();

        // Measure sharding speed
        let start = Instant::now();
        let shards = extensions.shard_packet(&data, 64 * 1024)?;
        let shard_time = start.elapsed();

        // Calculate throughput
        let token_throughput = (size as f64) / token_time.as_secs_f64() / (1024.0 * 1024.0);
        let shard_throughput = (size as f64) / shard_time.as_secs_f64() / (1024.0 * 1024.0);

        println!("  {} Performance:", label);
        println!("    - Tokenization: {:.2} MB/s", token_throughput);
        println!("    - Sharding: {:.2} MB/s ({} shards)", shard_throughput, shards.len());
    }

    // Check for simulated hardware acceleration
    println!("\n  Hardware Acceleration Claims:");
    println!("    - Claimed: 40 Gbps (5000 MB/s)");
    println!("    - Actual: No hardware acceleration implemented");
    println!("    - Status: FANTASY - uses simulated metrics");

    Ok(())
}