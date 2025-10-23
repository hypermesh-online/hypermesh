use std::time::{Duration, Instant};
use std::collections::HashMap;

fn main() {
    println!("====================================");
    println!("HyperMesh Basic Performance Baseline");
    println!("====================================");

    // Test 1: HashMap operations
    let iterations = 1_000_000;
    let mut map = HashMap::new();
    let start = Instant::now();
    for i in 0..iterations {
        map.insert(i, i * 2);
    }
    let elapsed = start.elapsed();
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

    println!("HashMap Insert Performance:");
    println!("  Iterations: {}", iterations);
    println!("  Time: {:?}", elapsed);
    println!("  Throughput: {:.2} ops/sec ({:.2} Mops/sec)", ops_per_sec, ops_per_sec / 1_000_000.0);
    println!("  Latency: {:.2} ns/op", elapsed.as_nanos() as f64 / iterations as f64);
    println!();

    // Test 2: Vector operations
    let mut vec = Vec::with_capacity(iterations as usize);
    let start = Instant::now();
    for i in 0..iterations {
        vec.push(i);
    }
    let elapsed = start.elapsed();
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

    println!("Vector Push Performance:");
    println!("  Iterations: {}", iterations);
    println!("  Time: {:?}", elapsed);
    println!("  Throughput: {:.2} ops/sec ({:.2} Mops/sec)", ops_per_sec, ops_per_sec / 1_000_000.0);
    println!("  Latency: {:.2} ns/op", elapsed.as_nanos() as f64 / iterations as f64);
    println!();

    // Test 3: Memory allocation
    let iterations = 100_000;
    let allocation_size = 4096; // 4KB
    let start = Instant::now();
    for _ in 0..iterations {
        let _v: Vec<u8> = vec![0u8; allocation_size];
    }
    let elapsed = start.elapsed();
    let mb_per_sec = (iterations as f64 * allocation_size as f64) / elapsed.as_secs_f64() / 1_000_000.0;

    println!("Memory Allocation Performance (4KB blocks):");
    println!("  Iterations: {}", iterations);
    println!("  Time: {:?}", elapsed);
    println!("  Throughput: {:.2} MB/sec", mb_per_sec);
    println!("  Latency: {:.2} μs/allocation", elapsed.as_micros() as f64 / iterations as f64);
    println!();

    // Summary
    println!("====================================");
    println!("Baseline established successfully");
    println!("====================================");
}