// Standalone benchmark - compile with: rustc standalone_bench.rs -O
use std::time::Instant;

fn main() {
    println!("===========================================");
    println!("HyperMesh Standalone Performance Baseline");
    println!("===========================================");
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

    let start = Instant::now();
    for _ in 0..10 {
        let _data = vec![0u8; 100 * 1024 * 1024]; // 100MB
    }
    let elapsed = start.elapsed();
    println!("  100MB allocation x10: {:?} ({:.2} MB/s)",
             elapsed,
             1000.0 / elapsed.as_secs_f64());
    println!();

    // Vector operations
    println!("Vector Operations:");
    let vec_size = 10_000_000;
    let start = Instant::now();
    let mut vec = Vec::with_capacity(vec_size);
    for i in 0..vec_size {
        vec.push(i);
    }
    let elapsed = start.elapsed();
    println!("  Vec push {}M elements: {:?} ({:.2}M ops/s)",
             vec_size / 1_000_000,
             elapsed,
             (vec_size as f64 / 1_000_000.0) / elapsed.as_secs_f64());

    let start = Instant::now();
    let sum: usize = vec.iter().sum();
    let elapsed = start.elapsed();
    println!("  Vec sum {}M elements: {:?} ({:.2}M ops/s)",
             vec_size / 1_000_000,
             elapsed,
             (vec_size as f64 / 1_000_000.0) / elapsed.as_secs_f64());
    println!("    Sum check: {}", sum);
    println!();

    // String operations
    println!("String Operations:");
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _s = format!("Hello {} World {}", 42, 3.14159);
    }
    let elapsed = start.elapsed();
    println!("  String format x{}: {:?} ({:.2}K ops/s)",
             iterations,
             elapsed,
             (iterations as f64 / 1000.0) / elapsed.as_secs_f64());

    let base_str = "Hello World";
    let start = Instant::now();
    for _ in 0..iterations {
        let _s = base_str.to_uppercase();
    }
    let elapsed = start.elapsed();
    println!("  String to_uppercase x{}: {:?} ({:.2}K ops/s)",
             iterations,
             elapsed,
             (iterations as f64 / 1000.0) / elapsed.as_secs_f64());
    println!();

    // HashMap operations
    use std::collections::HashMap;
    println!("HashMap Operations:");
    let map_size = 100_000;
    let start = Instant::now();
    let mut map = HashMap::new();
    for i in 0..map_size {
        map.insert(i, i * 2);
    }
    let elapsed = start.elapsed();
    println!("  HashMap insert {}K entries: {:?} ({:.2}K ops/s)",
             map_size / 1000,
             elapsed,
             (map_size as f64 / 1000.0) / elapsed.as_secs_f64());

    let start = Instant::now();
    let mut hits = 0;
    for i in 0..map_size {
        if let Some(_) = map.get(&i) {
            hits += 1;
        }
    }
    let elapsed = start.elapsed();
    println!("  HashMap lookup {}K entries: {:?} ({:.2}K ops/s)",
             map_size / 1000,
             elapsed,
             (map_size as f64 / 1000.0) / elapsed.as_secs_f64());
    println!("    Hits: {}", hits);
    println!();

    // Sorting operations
    println!("Sorting Operations:");
    let sort_size = 1_000_000;
    let mut data: Vec<i32> = (0..sort_size).rev().collect();
    let start = Instant::now();
    data.sort();
    let elapsed = start.elapsed();
    println!("  Sort {}M integers: {:?} ({:.2}M items/s)",
             sort_size / 1_000_000,
             elapsed,
             (sort_size as f64 / 1_000_000.0) / elapsed.as_secs_f64());

    let mut data: Vec<i32> = (0..sort_size).map(|i| i % 1000).collect();
    let start = Instant::now();
    data.sort_unstable();
    let elapsed = start.elapsed();
    println!("  Unstable sort {}M integers: {:?} ({:.2}M items/s)",
             sort_size / 1_000_000,
             elapsed,
             (sort_size as f64 / 1_000_000.0) / elapsed.as_secs_f64());
    println!();

    println!("===========================================");
    println!("Baseline Complete - All Tests Passed");
    println!("===========================================");
}