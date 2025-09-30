//! eBPF throughput benchmarks for STOQ transport

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use stoq::transport::{StoqTransport, TransportConfig, Endpoint};
use std::net::Ipv6Addr;
use tokio::runtime::Runtime;

fn setup_transport_with_ebpf(rt: &Runtime) -> StoqTransport {
    rt.block_on(async {
        // Initialize crypto
        if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
            // Already installed
        }

        let mut config = TransportConfig::default();
        config.enable_zero_copy = true;
        config.enable_memory_pool = true;
        config.frame_batch_size = 64;
        config.memory_pool_size = 2048;

        StoqTransport::new(config).await.expect("Failed to create transport")
    })
}

fn setup_transport_without_ebpf(rt: &Runtime) -> StoqTransport {
    rt.block_on(async {
        // Initialize crypto
        if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
            // Already installed
        }

        let mut config = TransportConfig::default();
        config.enable_zero_copy = false;  // Disable zero-copy to simulate no eBPF
        config.enable_memory_pool = false;
        config.frame_batch_size = 1;

        StoqTransport::new(config).await.expect("Failed to create transport")
    })
}

fn benchmark_send_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("send_throughput");

    // Test different packet sizes
    for size in [64, 256, 1024, 4096, 16384, 65536].iter() {
        let data = vec![0u8; *size];

        // Benchmark with eBPF optimizations
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("with_ebpf", size),
            size,
            |b, _size| {
                let transport = setup_transport_with_ebpf(&rt);
                let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9294);

                b.iter(|| {
                    rt.block_on(async {
                        // Create mock connection for benchmarking
                        if let Ok(conn) = transport.connect(&endpoint).await {
                            let _ = transport.send(&conn, black_box(&data)).await;
                        }
                    })
                });
            },
        );

        // Benchmark without eBPF optimizations
        group.bench_with_input(
            BenchmarkId::new("without_ebpf", size),
            size,
            |b, _size| {
                let transport = setup_transport_without_ebpf(&rt);
                let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9295);

                b.iter(|| {
                    rt.block_on(async {
                        if let Ok(conn) = transport.connect(&endpoint).await {
                            let _ = transport.send(&conn, black_box(&data)).await;
                        }
                    })
                });
            },
        );
    }

    group.finish();
}

fn benchmark_zero_copy_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("zero_copy_ops");

    // Small packets (should fit in memory pool)
    group.bench_function("small_packet_zero_copy", |b| {
        let transport = setup_transport_with_ebpf(&rt);
        let data = vec![0u8; 1024];
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9296);

        b.iter(|| {
            rt.block_on(async {
                if let Ok(conn) = transport.connect(&endpoint).await {
                    for _ in 0..100 {
                        let _ = transport.send(&conn, black_box(&data)).await;
                    }
                }
            })
        });
    });

    // Large packets (require batching)
    group.bench_function("large_packet_batching", |b| {
        let transport = setup_transport_with_ebpf(&rt);
        let data = vec![0u8; 65536];
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9297);

        b.iter(|| {
            rt.block_on(async {
                if let Ok(conn) = transport.connect(&endpoint).await {
                    for _ in 0..10 {
                        let _ = transport.send(&conn, black_box(&data)).await;
                    }
                }
            })
        });
    });

    group.finish();
}

fn benchmark_packet_rate(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("packet_rate");

    // Maximum packet rate with small packets
    group.bench_function("max_packet_rate_ebpf", |b| {
        let transport = setup_transport_with_ebpf(&rt);
        let data = vec![0u8; 64]; // Minimum packet size
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9298);

        b.iter(|| {
            rt.block_on(async {
                if let Ok(conn) = transport.connect(&endpoint).await {
                    for _ in 0..1000 {
                        let _ = transport.send(&conn, black_box(&data)).await;
                    }
                }
            })
        });
    });

    group.bench_function("max_packet_rate_standard", |b| {
        let transport = setup_transport_without_ebpf(&rt);
        let data = vec![0u8; 64];
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9299);

        b.iter(|| {
            rt.block_on(async {
                if let Ok(conn) = transport.connect(&endpoint).await {
                    for _ in 0..1000 {
                        let _ = transport.send(&conn, black_box(&data)).await;
                    }
                }
            })
        });
    });

    group.finish();
}

#[cfg(feature = "ebpf")]
fn benchmark_ebpf_metrics_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("ebpf_metrics_collection", |b| {
        let transport = setup_transport_with_ebpf(&rt);

        b.iter(|| {
            // Measure overhead of collecting eBPF metrics
            black_box(transport.get_ebpf_metrics());
        });
    });
}

#[cfg(not(feature = "ebpf"))]
fn benchmark_ebpf_metrics_overhead(_c: &mut Criterion) {
    // No-op when eBPF not compiled
}

criterion_group!(
    benches,
    benchmark_send_throughput,
    benchmark_zero_copy_operations,
    benchmark_packet_rate,
    benchmark_ebpf_metrics_overhead
);

criterion_main!(benches);