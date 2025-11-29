use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

// Simple baseline benchmarks that don't require full implementation

fn simple_allocation_benchmark(c: &mut Criterion) {
    c.bench_function("memory_allocation_1mb", |b| {
        b.iter(|| {
            let data = vec![0u8; black_box(1024 * 1024)];
            black_box(data);
        });
    });

    c.bench_function("memory_allocation_10mb", |b| {
        b.iter(|| {
            let data = vec![0u8; black_box(10 * 1024 * 1024)];
            black_box(data);
        });
    });
}

fn simple_hash_benchmark(c: &mut Criterion) {
    use blake3::Hasher;

    let data_1kb = vec![0xAB; 1024];
    let data_1mb = vec![0xCD; 1024 * 1024];

    c.bench_function("blake3_hash_1kb", |b| {
        b.iter(|| {
            let mut hasher = Hasher::new();
            hasher.update(&data_1kb);
            black_box(hasher.finalize());
        });
    });

    c.bench_function("blake3_hash_1mb", |b| {
        b.iter(|| {
            let mut hasher = Hasher::new();
            hasher.update(&data_1mb);
            black_box(hasher.finalize());
        });
    });
}

fn simple_concurrency_benchmark(c: &mut Criterion) {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    c.bench_function("atomic_counter_single_thread", |b| {
        let counter = Arc::new(AtomicU64::new(0));
        b.iter(|| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
    });

    c.bench_function("mutex_counter", |b| {
        use std::sync::Mutex;
        let counter = Arc::new(Mutex::new(0u64));
        b.iter(|| {
            let mut c = counter.lock().unwrap();
            *c += 1;
        });
    });
}

fn simple_serialization_benchmark(c: &mut Criterion) {
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
        values: vec![1.0, 2.0, 3.0, 4.0, 5.0],
    };

    c.bench_function("bincode_serialize", |b| {
        b.iter(|| {
            black_box(bincode::serialize(&test_data).unwrap());
        });
    });

    let serialized = bincode::serialize(&test_data).unwrap();

    c.bench_function("bincode_deserialize", |b| {
        b.iter(|| {
            black_box(bincode::deserialize::<TestData>(&serialized).unwrap());
        });
    });
}

// Configure criterion with shorter measurement times for quick baseline
criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(3))
        .warm_up_time(Duration::from_secs(1));
    targets = simple_allocation_benchmark,
              simple_hash_benchmark,
              simple_concurrency_benchmark,
              simple_serialization_benchmark
}

criterion_main!(benches);