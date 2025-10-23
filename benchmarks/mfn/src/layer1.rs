/*!
# Layer 1 (IFR) Benchmarking

Benchmarks for the Immediate Flow Registry layer focusing on:
- Exact matching performance with Robin Hood hashing
- Unix socket IPC performance
- Bloom filter efficiency
- Flow cache hit rates
- Overall coordination latency

Performance targets:
- <0.1ms exact matching latency
- >10M operations per second
- 88.6% latency improvement over network calls
- <10MB memory footprint
*/

use crate::common::*;
use std::collections::HashMap;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use blake3::Hasher;
use ahash::AHashMap;

/// IFR-specific benchmark configuration
#[derive(Debug, Clone)]
pub struct IfrBenchmarkConfig {
    pub base: BenchmarkConfig,
    pub hash_table_size: usize,
    pub bloom_filter_count: usize,
    pub cache_size_mb: usize,
    pub concurrent_connections: usize,
    pub flow_record_count: usize,
}

impl Default for IfrBenchmarkConfig {
    fn default() -> Self {
        Self {
            base: BenchmarkConfig {
                warmup_iterations: 1000,
                measurement_iterations: 10000,
                statistical_confidence: 0.95,
                regression_threshold: 0.05,
                memory_limit_mb: 32,
                timeout_seconds: 120,
                parallel_workers: num_cpus::get(),
                output_format: OutputFormat::Json,
                enable_flamegraph: false,
                enable_perf_counters: true,
            },
            hash_table_size: 1048576, // 1M entries
            bloom_filter_count: 16,
            cache_size_mb: 100,
            concurrent_connections: 1000,
            flow_record_count: 1000000,
        }
    }
}

/// Flow record structure matching the Zig implementation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlowRecord {
    pub key: [u8; 32],
    pub component_id: u32,
    pub timestamp: u64,
    pub metadata: [u8; 8],
}

/// Robin Hood hash table implementation for benchmarking
pub struct RobinHoodHashTable {
    buckets: Vec<Option<(FlowRecord, u32)>>, // (record, distance)
    size: usize,
    count: usize,
}

impl RobinHoodHashTable {
    pub fn new(size: usize) -> Self {
        Self {
            buckets: vec![None; size],
            size,
            count: 0,
        }
    }

    pub fn insert(&mut self, record: FlowRecord) -> bool {
        if self.count >= self.size * 3 / 4 {
            return false; // Table too full
        }

        let hash = self.hash_key(&record.key);
        let mut pos = hash % self.size;
        let mut dist = 0u32;
        let mut to_insert = Some((record, dist));

        loop {
            match &self.buckets[pos] {
                None => {
                    self.buckets[pos] = to_insert;
                    self.count += 1;
                    return true;
                }
                Some((_, existing_dist)) => {
                    if dist > *existing_dist {
                        // Robin Hood: steal from the rich
                        let old_entry = self.buckets[pos].take();
                        self.buckets[pos] = to_insert;
                        to_insert = old_entry.map(|(record, _)| (record, dist));
                        dist = *existing_dist;
                    }
                }
            }
            
            pos = (pos + 1) % self.size;
            dist += 1;
            
            if dist > 100 {
                return false; // Prevent infinite loops
            }
        }
    }

    pub fn lookup(&self, key: &[u8; 32]) -> Option<&FlowRecord> {
        let hash = self.hash_key(key);
        let mut pos = hash % self.size;
        let mut dist = 0u32;

        loop {
            match &self.buckets[pos] {
                None => return None,
                Some((record, existing_dist)) => {
                    if dist > *existing_dist {
                        return None; // Would have been displaced
                    }
                    if &record.key == key {
                        return Some(record);
                    }
                }
            }
            
            pos = (pos + 1) % self.size;
            dist += 1;
            
            if dist > 100 {
                return None; // Prevent infinite loops
            }
        }
    }

    fn hash_key(&self, key: &[u8; 32]) -> usize {
        let mut hasher = Hasher::new();
        hasher.update(key);
        let hash = hasher.finalize();
        let hash_bytes = hash.as_bytes();
        
        // Convert first 8 bytes to usize
        let mut result = 0usize;
        for (i, &byte) in hash_bytes.iter().take(8).enumerate() {
            result |= (byte as usize) << (i * 8);
        }
        result
    }

    pub fn load_factor(&self) -> f64 {
        self.count as f64 / self.size as f64
    }
}

/// Bloom filter bank for negative lookups
pub struct BloomFilterBank {
    filters: Vec<BloomFilter>,
    current_filter: usize,
    rotation_count: usize,
}

pub struct BloomFilter {
    bits: Vec<bool>,
    size: usize,
    hash_functions: usize,
}

impl BloomFilter {
    pub fn new(size: usize, hash_functions: usize) -> Self {
        Self {
            bits: vec![false; size],
            size,
            hash_functions,
        }
    }

    pub fn add(&mut self, key: &[u8; 32]) {
        for i in 0..self.hash_functions {
            let hash = self.hash_with_seed(key, i as u64);
            let index = (hash % self.size as u64) as usize;
            self.bits[index] = true;
        }
    }

    pub fn might_contain(&self, key: &[u8; 32]) -> bool {
        for i in 0..self.hash_functions {
            let hash = self.hash_with_seed(key, i as u64);
            let index = (hash % self.size as u64) as usize;
            if !self.bits[index] {
                return false;
            }
        }
        true
    }

    fn hash_with_seed(&self, key: &[u8; 32], seed: u64) -> u64 {
        let mut hasher = Hasher::new();
        hasher.update(&seed.to_le_bytes());
        hasher.update(key);
        let hash = hasher.finalize();
        let hash_bytes = hash.as_bytes();
        
        // Convert to u64
        let mut result = 0u64;
        for (i, &byte) in hash_bytes.iter().take(8).enumerate() {
            result |= (byte as u64) << (i * 8);
        }
        result
    }
}

impl BloomFilterBank {
    pub fn new(filter_count: usize, bits_per_filter: usize, hash_functions: usize) -> Self {
        let filters = (0..filter_count)
            .map(|_| BloomFilter::new(bits_per_filter, hash_functions))
            .collect();
        
        Self {
            filters,
            current_filter: 0,
            rotation_count: 0,
        }
    }

    pub fn add(&mut self, key: &[u8; 32]) {
        self.filters[self.current_filter].add(key);
    }

    pub fn might_contain(&self, key: &[u8; 32]) -> bool {
        self.filters.iter().any(|filter| filter.might_contain(key))
    }

    pub fn rotate(&mut self) {
        self.current_filter = (self.current_filter + 1) % self.filters.len();
        self.rotation_count += 1;
        
        // Clear the new current filter
        let size = self.filters[self.current_filter].size;
        let hash_functions = self.filters[self.current_filter].hash_functions;
        self.filters[self.current_filter] = BloomFilter::new(size, hash_functions);
    }
}

/// Unix socket server for IPC benchmarking
pub struct IpcServer {
    listener: UnixListener,
    socket_path: String,
    active_connections: Arc<Mutex<usize>>,
}

impl IpcServer {
    pub fn new(socket_path: &str) -> anyhow::Result<Self> {
        // Remove existing socket file
        let _ = std::fs::remove_file(socket_path);
        
        let listener = UnixListener::bind(socket_path)?;
        
        Ok(Self {
            listener,
            socket_path: socket_path.to_string(),
            active_connections: Arc::new(Mutex::new(0)),
        })
    }

    pub async fn start_server(&self) -> anyhow::Result<()> {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let connections = self.active_connections.clone();
                    {
                        let mut count = connections.lock().unwrap();
                        *count += 1;
                    }
                    
                    tokio::spawn(async move {
                        let _ = Self::handle_connection(stream).await;
                        let mut count = connections.lock().unwrap();
                        *count -= 1;
                    });
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_connection(mut _stream: UnixStream) -> anyhow::Result<()> {
        // Simple echo server for benchmarking
        Ok(())
    }

    pub fn get_connection_count(&self) -> usize {
        *self.active_connections.lock().unwrap()
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Main IFR benchmark suite
pub async fn run_ifr_benchmarks(config: IfrBenchmarkConfig) -> anyhow::Result<Vec<BenchmarkResult>> {
    let mut harness = BenchmarkHarness::new(config.base.clone());
    let mut results = Vec::new();

    println!("🏗️  Starting Layer 1 (IFR) Benchmarks");
    println!("    Hash table size: {}", config.hash_table_size);
    println!("    Flow records: {}", config.flow_record_count);
    println!("    Bloom filters: {}", config.bloom_filter_count);

    // Generate test data
    let flow_records = generate_test_flows(config.flow_record_count);
    let lookup_keys = flow_records.iter()
        .take(config.flow_record_count / 10)
        .map(|r| r.key)
        .collect::<Vec<_>>();

    // Benchmark 1: Exact Matcher Performance
    results.push(run_exact_matcher_benchmark(&mut harness, &config, &flow_records, &lookup_keys).await?);
    
    // Benchmark 2: Bloom Filter Performance  
    results.push(run_bloom_filter_benchmark(&mut harness, &config, &flow_records, &lookup_keys).await?);
    
    // Benchmark 3: Unix Socket IPC Performance
    results.push(run_unix_socket_benchmark(&mut harness, &config).await?);
    
    // Benchmark 4: Flow Cache Performance
    results.push(run_flow_cache_benchmark(&mut harness, &config, &flow_records, &lookup_keys).await?);
    
    // Benchmark 5: Integrated IFR Performance
    results.push(run_integrated_ifr_benchmark(&mut harness, &config, &flow_records, &lookup_keys).await?);

    Ok(results)
}

async fn run_exact_matcher_benchmark(
    harness: &mut BenchmarkHarness,
    config: &IfrBenchmarkConfig,
    flow_records: &[FlowRecord],
    lookup_keys: &[[u8; 32]],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "exact_matcher_lookup",
        MfnLayer::Layer1Ifr,
        {
            let mut hash_table = RobinHoodHashTable::new(config.hash_table_size);
            
            // Pre-populate hash table
            for record in flow_records {
                hash_table.insert(record.clone());
            }
            
            let keys = lookup_keys.to_vec();
            
            move || {
                let start = Instant::now();
                let key = &keys[fastrand::usize(0..keys.len())];
                let _ = hash_table.lookup(key);
                let duration = start.elapsed();
                
                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_bloom_filter_benchmark(
    harness: &mut BenchmarkHarness,
    config: &IfrBenchmarkConfig,
    flow_records: &[FlowRecord],
    lookup_keys: &[[u8; 32]],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "bloom_filter_lookup",
        MfnLayer::Layer1Ifr,
        {
            let mut bloom_bank = BloomFilterBank::new(
                config.bloom_filter_count,
                131072, // 128KB per filter
                3,      // 3 hash functions
            );
            
            // Pre-populate bloom filters
            for record in flow_records {
                bloom_bank.add(&record.key);
            }
            
            let keys = lookup_keys.to_vec();
            
            move || {
                let start = Instant::now();
                let key = &keys[fastrand::usize(0..keys.len())];
                let _ = bloom_bank.might_contain(key);
                let duration = start.elapsed();
                
                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_unix_socket_benchmark(
    harness: &mut BenchmarkHarness,
    config: &IfrBenchmarkConfig,
) -> anyhow::Result<BenchmarkResult> {
    let socket_path = "/tmp/ifr_bench.sock";
    let _server = IpcServer::new(socket_path)?;
    
    harness.run_benchmark(
        "unix_socket_connection",
        MfnLayer::Layer1Ifr,
        move || {
            let socket_path = socket_path.to_string();
            async move {
                let start = Instant::now();
                let result = timeout(
                    Duration::from_millis(100),
                    async {
                        let _stream = UnixStream::connect(&socket_path)?;
                        Ok::<(), anyhow::Error>(())
                    }
                ).await;
                
                let duration = start.elapsed();
                
                match result {
                    Ok(_) => Ok(duration),
                    Err(_) => Ok(Duration::from_millis(100)), // Timeout case
                }
            }
        }
    ).await
}

async fn run_flow_cache_benchmark(
    harness: &mut BenchmarkHarness,
    _config: &IfrBenchmarkConfig,
    flow_records: &[FlowRecord],
    lookup_keys: &[[u8; 32]],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "flow_cache_lookup",
        MfnLayer::Layer1Ifr,
        {
            let mut cache = AHashMap::new();
            
            // Pre-populate cache
            for record in flow_records.iter().take(10000) {
                cache.insert(record.key, record.clone());
            }
            
            let keys = lookup_keys.to_vec();
            
            move || {
                let start = Instant::now();
                let key = &keys[fastrand::usize(0..keys.len())];
                let _ = cache.get(key);
                let duration = start.elapsed();
                
                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_integrated_ifr_benchmark(
    harness: &mut BenchmarkHarness,
    config: &IfrBenchmarkConfig,
    flow_records: &[FlowRecord],
    lookup_keys: &[[u8; 32]],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "integrated_ifr_lookup",
        MfnLayer::Layer1Ifr,
        {
            // Create integrated system
            let mut hash_table = RobinHoodHashTable::new(config.hash_table_size);
            let mut bloom_bank = BloomFilterBank::new(config.bloom_filter_count, 131072, 3);
            let mut cache = AHashMap::new();
            
            // Pre-populate all components
            for record in flow_records {
                hash_table.insert(record.clone());
                bloom_bank.add(&record.key);
                if cache.len() < 10000 {
                    cache.insert(record.key, record.clone());
                }
            }
            
            let keys = lookup_keys.to_vec();
            
            move || {
                let start = Instant::now();
                let key = &keys[fastrand::usize(0..keys.len())];
                
                // Simulate full IFR lookup pipeline
                let result = if let Some(cached) = cache.get(key) {
                    Some(cached.clone())
                } else if bloom_bank.might_contain(key) {
                    hash_table.lookup(key).cloned()
                } else {
                    None
                };
                
                let duration = start.elapsed();
                let _ = result; // Consume result
                
                async move { Ok(duration) }
            }
        }
    ).await
}

fn generate_test_flows(count: usize) -> Vec<FlowRecord> {
    (0..count)
        .map(|i| {
            let mut key = [0u8; 32];
            let i_bytes = (i as u64).to_le_bytes();
            key[..8].copy_from_slice(&i_bytes);
            
            // Add some randomness to the rest of the key
            for j in 8..32 {
                key[j] = fastrand::u8(0..=255);
            }
            
            FlowRecord {
                key,
                component_id: fastrand::u32(1000..9999),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
                metadata: [0; 8],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robin_hood_hash_table() {
        let mut table = RobinHoodHashTable::new(1000);
        
        let record = FlowRecord {
            key: [1; 32],
            component_id: 1234,
            timestamp: 1000,
            metadata: [0; 8],
        };
        
        assert!(table.insert(record.clone()));
        assert_eq!(table.lookup(&[1; 32]), Some(&record));
        assert_eq!(table.lookup(&[2; 32]), None);
    }

    #[test]
    fn test_bloom_filter() {
        let mut filter = BloomFilter::new(1000, 3);
        let key = [1; 32];
        
        assert!(!filter.might_contain(&key));
        filter.add(&key);
        assert!(filter.might_contain(&key));
    }

    #[test]
    fn test_bloom_filter_bank() {
        let mut bank = BloomFilterBank::new(4, 1000, 3);
        let key = [1; 32];
        
        assert!(!bank.might_contain(&key));
        bank.add(&key);
        assert!(bank.might_contain(&key));
        
        // Test rotation
        bank.rotate();
        bank.add(&[2; 32]);
        assert!(bank.might_contain(&key)); // Should still be in previous filters
        assert!(bank.might_contain(&[2; 32]));
    }

    #[tokio::test]
    async fn test_ipc_server() {
        let socket_path = "/tmp/test_ifr_bench.sock";
        let server = IpcServer::new(socket_path).unwrap();
        
        // Test connection count
        assert_eq!(server.get_connection_count(), 0);
        
        // Cleanup
        drop(server);
        assert!(!std::path::Path::new(socket_path).exists());
    }
}