#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! criterion = "0.5"
//! blake3 = "1.5"
//! serde = { version = "1.0", features = ["derive"] }
//! bincode = "1.3"
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::{Duration, Instant};

// Quick standalone benchmark that doesn't require full project compilation

fn main() {
    println!("===========================================");
    println!("HyperMesh Quick Performance Baseline");
    println!("===========================================");
    println!("Date: {}", chrono::Local::now());
    println!();

    // Memory allocation benchmarks
    println!("Memory Allocation Tests:");
    let start = Instant::now();
    for _ in 0..1000 {
        let _data = vec![0u8; 1024 * 1024]; // 1MB
    }
    let elapsed = start.elapsed();
    println!("  1MB allocation x1000: {:?} ({:.2} MB/s)",
             elapsed,
             1000.0 / elapsed.as_secs_f64());

    let start = Instant::now();
    for _ in 0..100 {
        let _data = vec![0u8; 10 * 1024 * 1024]; // 10MB
    }
    let elapsed = start.elapsed();
    println!("  10MB allocation x100: {:?} ({:.2} MB/s)",
             elapsed,
             1000.0 / elapsed.as_secs_f64());
    println!();

    // Hash benchmarks
    println!("Blake3 Hash Performance:");
    use blake3::Hasher;

    let data_1kb = vec![0xAB; 1024];
    let data_1mb = vec![0xCD; 1024 * 1024];
    let data_10mb = vec![0xEF; 10 * 1024 * 1024];

    let iterations = 10000;
    let start = Instant::now();
    for _ in 0..iterations {
        let mut hasher = Hasher::new();
        hasher.update(&data_1kb);
        let _ = hasher.finalize();
    }
    let elapsed = start.elapsed();
    println!("  1KB hash x{}: {:?} ({:.2} MB/s)",
             iterations,
             elapsed,
             (iterations as f64 * 1.0) / elapsed.as_secs_f64());

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let mut hasher = Hasher::new();
        hasher.update(&data_1mb);
        let _ = hasher.finalize();
    }
    let elapsed = start.elapsed();
    println!("  1MB hash x{}: {:?} ({:.2} MB/s)",
             iterations,
             elapsed,
             (iterations as f64 * 1.0) / elapsed.as_secs_f64());

    let iterations = 10;
    let start = Instant::now();
    for _ in 0..iterations {
        let mut hasher = Hasher::new();
        hasher.update(&data_10mb);
        let _ = hasher.finalize();
    }
    let elapsed = start.elapsed();
    println!("  10MB hash x{}: {:?} ({:.2} MB/s)",
             iterations,
             elapsed,
             (iterations as f64 * 10.0) / elapsed.as_secs_f64());
    println!();

    // Concurrency benchmarks
    println!("Concurrency Tests:");
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    let counter = Arc::new(AtomicU64::new(0));
    let iterations = 1_000_000;
    let start = Instant::now();
    for _ in 0..iterations {
        counter.fetch_add(1, Ordering::Relaxed);
    }
    let elapsed = start.elapsed();
    println!("  Atomic counter x{}: {:?} ({:.2}M ops/s)",
             iterations,
             elapsed,
             (iterations as f64 / 1_000_000.0) / elapsed.as_secs_f64());

    let counter = Arc::new(Mutex::new(0u64));
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let mut c = counter.lock().unwrap();
        *c += 1;
    }
    let elapsed = start.elapsed();
    println!("  Mutex counter x{}: {:?} ({:.2}K ops/s)",
             iterations,
             elapsed,
             (iterations as f64 / 1000.0) / elapsed.as_secs_f64());
    println!();

    // Serialization benchmarks
    println!("Serialization Tests:");
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize)]
    struct TestData {
        id: u64,
        name: String,
        values: Vec<f64>,
    }

    let test_data = TestData {
        id: 42,
        name: "benchmark_test".to_string(),
        values: vec![1.0; 1000], // 1000 floats
    };

    let iterations = 10000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = bincode::serialize(&test_data).unwrap();
    }
    let elapsed = start.elapsed();
    println!("  Bincode serialize x{}: {:?} ({:.2}K ops/s)",
             iterations,
             elapsed,
             (iterations as f64 / 1000.0) / elapsed.as_secs_f64());

    let serialized = bincode::serialize(&test_data).unwrap();
    let start = Instant::now();
    for _ in 0..iterations {
        let _: TestData = bincode::deserialize(&serialized).unwrap();
    }
    let elapsed = start.elapsed();
    println!("  Bincode deserialize x{}: {:?} ({:.2}K ops/s)",
             iterations,
             elapsed,
             (iterations as f64 / 1000.0) / elapsed.as_secs_f64());
    println!();

    // System info
    println!("System Information:");
    println!("  CPU cores: {}", num_cpus::get());
    println!("  Physical cores: {}", num_cpus::get_physical());

    println!();
    println!("===========================================");
    println!("Baseline Performance Metrics Collected");
    println!("===========================================");
}