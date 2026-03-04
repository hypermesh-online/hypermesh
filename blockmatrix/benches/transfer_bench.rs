// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Benchmarks for IPv6 asset addressing and transfer operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use std::time::Duration;

use blockmatrix::blockchain::node_chain::NodeBlockchain;
use blockmatrix::proof_of_state::validation::{DefaultStateAuthenticator, StateAuthenticator};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::transfer::{
    create_transfer_intent, proof_to_bytes, StateProofBytes, TransferEngine,
};
use hypermesh_lib::{AssetAddress, ContentHash};
use trustchain::proof_of_state::StateProof;

/// Get valid test proof bytes.
fn test_proof() -> StateProofBytes {
    let proof = StateProof::new_for_testing();
    proof_to_bytes(&proof).unwrap()
}

fn address_ops(c: &mut Criterion) {
    let hash = ContentHash::from_bytes([0xAB; 32]);

    c.bench_function("asset_address_new", |b| {
        b.iter(|| {
            black_box(AssetAddress::new(
                black_box(42),
                black_box(-17),
                black_box(99),
                &hash,
            ))
        });
    });

    let addr = AssetAddress::new(42, -17, 99, &hash).unwrap();

    c.bench_function("asset_address_to_ipv6", |b| {
        b.iter(|| black_box(addr.to_ipv6()));
    });

    let ipv6 = addr.to_ipv6();
    c.bench_function("asset_address_from_ipv6", |b| {
        b.iter(|| black_box(AssetAddress::from_ipv6(black_box(ipv6))));
    });

    c.bench_function("asset_address_shard_derive_14", |b| {
        b.iter(|| {
            for i in 1..=14u8 {
                black_box(addr.shard(i).unwrap());
            }
        });
    });

    c.bench_function("asset_address_matrix_coords", |b| {
        b.iter(|| black_box(addr.matrix_coords()));
    });
}

fn proof_validation(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let validator = Arc::new(DefaultStateAuthenticator::for_testing());
    let proof_bytes = test_proof();

    c.bench_function("pos_validate_proof", |b| {
        b.to_async(&rt).iter(|| {
            let v = validator.clone();
            let pb = proof_bytes.clone();
            async move {
                black_box(v.validate(pb.as_bytes()).await.unwrap());
            }
        });
    });
}

fn transfer_e2e(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let validator = Arc::new(DefaultStateAuthenticator::for_testing());
    let engine = Arc::new(TransferEngine::new(validator));

    let source_coord = MatrixCoordinate::new(10, 20, 30).unwrap();
    let target_coord = MatrixCoordinate::new(40, 50, 60).unwrap();
    let hash = ContentHash::from_bytes([0xAB; 32]);
    let addr = AssetAddress::new(source_coord.x, source_coord.y, source_coord.z, &hash).unwrap();

    c.bench_function("transfer_e2e_single", |b| {
        b.to_async(&rt).iter(|| {
            let eng = engine.clone();
            let sc = source_coord;
            let tc = target_coord;
            let a = addr;
            async move {
                let source_chain = NodeBlockchain::new(sc);
                let target_chain = NodeBlockchain::new(tc);
                let intent = create_transfer_intent(a, sc, tc, test_proof(), vec![]);
                let tp = test_proof();
                black_box(
                    eng.execute_transfer(&intent, &tp, &source_chain, &target_chain)
                        .await
                        .unwrap(),
                );
            }
        });
    });
}

fn transfer_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let validator = Arc::new(DefaultStateAuthenticator::for_testing());
    let engine = Arc::new(TransferEngine::new(validator));

    let hash = ContentHash::from_bytes([0xAB; 32]);

    c.bench_function("transfer_throughput_100", |b| {
        b.to_async(&rt).iter(|| {
            let eng = engine.clone();
            let h = hash;
            async move {
                let sc = MatrixCoordinate::new(10, 20, 30).unwrap();
                let tc = MatrixCoordinate::new(40, 50, 60).unwrap();
                let source_chain = NodeBlockchain::new(sc);
                let target_chain = NodeBlockchain::new(tc);

                let mut current_addr = AssetAddress::new(sc.x, sc.y, sc.z, &h).unwrap();

                for _ in 0..100 {
                    let intent = create_transfer_intent(current_addr, sc, tc, test_proof(), vec![]);
                    let receipt = eng
                        .execute_transfer(&intent, &test_proof(), &source_chain, &target_chain)
                        .await
                        .unwrap();
                    current_addr = AssetAddress::new(
                        sc.x,
                        sc.y,
                        sc.z,
                        &ContentHash::from_bytes(*receipt.receipt_hash.as_bytes()),
                    )
                    .unwrap();
                }
            }
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(3))
        .warm_up_time(Duration::from_secs(1));
    targets = address_ops,
              proof_validation,
              transfer_e2e,
              transfer_throughput
}

criterion_main!(benches);
