// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Performance benchmarks for TrustChain certificate and state proof operations.
//!
//! Benchmarks real FALCON-1024 key generation, signing, verification,
//! BinaryAuthenticator pass/fail, and StateProof validation latency.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

use pqcrypto_falcon::falcon1024;

use trustchain::proof_of_state::{StateProof, StateRequirements};
use trustchain::security::BinaryAuthenticator;

/// Benchmark FALCON-1024 key-pair generation.
fn bench_falcon_keygen(c: &mut Criterion) {
    c.bench_function("falcon1024_keygen", |b| {
        b.iter(|| {
            let (pk, sk) = falcon1024::keypair();
            black_box((&pk, &sk));
        });
    });
}

/// Benchmark FALCON-1024 detached signing on a 256-byte message.
fn bench_falcon_sign(c: &mut Criterion) {
    let (_pk, sk) = falcon1024::keypair();
    let message: Vec<u8> = (0..256).map(|i| i as u8).collect();

    c.bench_function("falcon1024_sign_256B", |b| {
        b.iter(|| {
            let sig = falcon1024::detached_sign(black_box(&message), &sk);
            black_box(sig);
        });
    });
}

/// Benchmark FALCON-1024 detached signature verification.
fn bench_falcon_verify(c: &mut Criterion) {
    let (pk, sk) = falcon1024::keypair();
    let message: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let sig = falcon1024::detached_sign(&message, &sk);

    c.bench_function("falcon1024_verify_256B", |b| {
        b.iter(|| {
            let result = falcon1024::verify_detached_signature(
                black_box(&sig),
                black_box(&message),
                black_box(&pk),
            );
            let _ = black_box(result);
        });
    });
}

/// Benchmark BinaryAuthenticator pass path (node not revoked).
fn bench_binary_auth_pass(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let authenticator = BinaryAuthenticator::new();

    c.bench_function("binary_auth_pass", |b| {
        b.to_async(&rt).iter(|| {
            let auth = &authenticator;
            async move {
                let result = auth
                    .authenticate(black_box("bench_node_42"))
                    .await
                    .expect("auth should succeed");
                black_box(result);
            }
        });
    });
}

/// Benchmark BinaryAuthenticator fail path (node revoked, lookup in map).
fn bench_binary_auth_revoked(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let authenticator = BinaryAuthenticator::new();

    // Pre-populate 1000 revoked nodes so the lookup is non-trivial
    rt.block_on(async {
        for i in 0..1000 {
            let node = format!("revoked_node_{i}");
            // Use is_revoked just to confirm map access works;
            // we need to revoke via the available API
            authenticator
                .revoke(
                    &node,
                    &trustchain::security::ByzantineViolation::InvalidStakeSignature {
                        stake_holder_id: node.clone(),
                    },
                )
                .await
                .expect("revoke should succeed");
        }
    });

    c.bench_function("binary_auth_revoked_1k", |b| {
        b.to_async(&rt).iter(|| {
            let auth = &authenticator;
            async move {
                let result = auth
                    .authenticate(black_box("revoked_node_500"))
                    .await
                    .expect("auth should succeed");
                black_box(result);
            }
        });
    });
}

/// Benchmark StateProof local (synchronous) four-proof validation.
fn bench_pos_validate_sync(c: &mut Criterion) {
    let proof = StateProof::new_for_testing();

    c.bench_function("pos_validate_sync", |b| {
        b.iter(|| {
            let valid = black_box(&proof).validate();
            black_box(valid);
        });
    });
}

/// Benchmark StateProof validation against requirements.
fn bench_pos_validate_with_requirements(c: &mut Criterion) {
    let proof = StateProof::new_for_testing();
    let requirements = StateRequirements::localhost_testing();

    c.bench_function("pos_validate_requirements", |b| {
        b.iter(|| {
            let valid = black_box(&proof).validate_with_requirements(black_box(&requirements));
            black_box(valid);
        });
    });
}

/// Benchmark StateProof serialization round-trip (to_bytes + from_bytes).
fn bench_pos_serde_roundtrip(c: &mut Criterion) {
    let proof = StateProof::new_for_testing();

    c.bench_function("pos_serde_roundtrip", |b| {
        b.iter(|| {
            let bytes = black_box(&proof).to_bytes().expect("serialize");
            let decoded = StateProof::from_bytes(black_box(&bytes)).expect("deserialize");
            black_box(decoded);
        });
    });
}

/// Benchmark BLAKE3 hashing of a StateProof.
fn bench_pos_hash(c: &mut Criterion) {
    let proof = StateProof::new_for_testing();

    c.bench_function("pos_hash_blake3", |b| {
        b.iter(|| {
            let hash = black_box(&proof).hash().expect("hash");
            black_box(hash);
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_falcon_keygen,
              bench_falcon_sign,
              bench_falcon_verify,
              bench_binary_auth_pass,
              bench_binary_auth_revoked,
              bench_pos_validate_sync,
              bench_pos_validate_with_requirements,
              bench_pos_serde_roundtrip,
              bench_pos_hash
}

criterion_main!(benches);
