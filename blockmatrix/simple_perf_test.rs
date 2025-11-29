#!/usr/bin/env -S cargo +nightly -Zscript

//! Simple Performance Test for HyperMesh Components
//! This establishes baseline metrics for core operations

use std::time::{Duration, Instant};
use std::collections::HashMap;

#[derive(Debug)]
struct BenchmarkResult {
    name: String,
    iterations: u64,
    total_time: Duration,
    ops_per_second: f64,
    avg_latency_ns: u64,
}

impl BenchmarkResult {
    fn new(name: &str, iterations: u64, total_time: Duration) -> Self {
        let ops_per_second = iterations as f64 / total_time.as_secs_f64();
        let avg_latency_ns = total_time.as_nanos() as u64 / iterations;

        Self {
            name: name.to_string(),
            iterations,
            total_time,
            ops_per_second,
            avg_latency_ns,
        }
    }

    fn print(&self) {
        println!("Benchmark: {}", self.name);
        println!("  Iterations: {}", self.iterations);
        println!("  Total time: {:?}", self.total_time);
        println!("  Throughput: {:.2} ops/sec", self.ops_per_second);
        println!("  Avg latency: {} ns", self.avg_latency_ns);
        println!();
    }
}

fn bench_hashmap_operations() -> BenchmarkResult {
    let iterations = 1_000_000;
    let mut map = HashMap::new();

    let start = Instant::now();
    for i in 0..iterations {
        map.insert(i, i * 2);
    }
    let elapsed = start.elapsed();

    BenchmarkResult::new("HashMap Insert", iterations, elapsed)
}

fn bench_vector_operations() -> BenchmarkResult {
    let iterations = 1_000_000;
    let mut vec = Vec::with_capacity(iterations as usize);

    let start = Instant::now();
    for i in 0..iterations {
        vec.push(i);
    }
    let elapsed = start.elapsed();

    BenchmarkResult::new("Vector Push", iterations, elapsed)
}

fn bench_string_operations() -> BenchmarkResult {
    let iterations = 100_000;
    let base_string = "HyperMesh Performance Test";

    let start = Instant::now();
    for _ in 0..iterations {
        let _s = format!("{} - iteration", base_string);
    }
    let elapsed = start.elapsed();

    BenchmarkResult::new("String Format", iterations, elapsed)
}

fn bench_memory_allocation() -> BenchmarkResult {
    let iterations = 100_000;
    let allocation_size = 1024; // 1KB

    let start = Instant::now();
    for _ in 0..iterations {
        let _v: Vec<u8> = vec![0u8; allocation_size];
    }
    let elapsed = start.elapsed();

    BenchmarkResult::new("Memory Allocation (1KB)", iterations, elapsed)
}

fn bench_concurrent_operations() -> BenchmarkResult {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;

    let iterations = 1_000_000;
    let counter = Arc::new(AtomicU64::new(0));
    let num_threads = 4;
    let iterations_per_thread = iterations / num_threads;

    let start = Instant::now();
    let mut handles = vec![];

    for _ in 0..num_threads {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            for _ in 0..iterations_per_thread {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
    let elapsed = start.elapsed();

    BenchmarkResult::new("Atomic Counter (4 threads)", iterations, elapsed)
}

fn main() {
    println!("====================================");
    println!("HyperMesh Simple Performance Baseline");
    println!("====================================");
    println!("System: {}", std::env::consts::OS);
    println!("Architecture: {}", std::env::consts::ARCH);
    println!("CPUs: {}", num_cpus::get());
    println!();

    let mut results = vec![];

    println!("Running benchmarks...\n");

    // Run benchmarks
    results.push(bench_hashmap_operations());
    results.push(bench_vector_operations());
    results.push(bench_string_operations());
    results.push(bench_memory_allocation());
    results.push(bench_concurrent_operations());

    // Print results
    println!("====================================");
    println!("Results:");
    println!("====================================");
    for result in &results {
        result.print();
    }

    // Summary
    println!("====================================");
    println!("Performance Summary:");
    println!("====================================");
    println!("HashMap throughput: {:.2} Mops/sec", results[0].ops_per_second / 1_000_000.0);
    println!("Vector throughput: {:.2} Mops/sec", results[1].ops_per_second / 1_000_000.0);
    println!("String formatting: {:.2} Kops/sec", results[2].ops_per_second / 1_000.0);
    println!("Memory allocation: {:.2} MB/sec", results[3].ops_per_second * 1024.0 / 1_000_000.0);
    println!("Atomic ops (concurrent): {:.2} Mops/sec", results[4].ops_per_second / 1_000_000.0);
}

// Dependencies for cargo script
/*
[dependencies]
num_cpus = "1.16"
*/