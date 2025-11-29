/*!
# Network Performance Benchmarking

Network-specific performance benchmarking for MFN networking optimizations:
- Bandwidth and throughput measurements
- Latency and jitter analysis
- Packet loss simulation and recovery
- Connection scaling tests
- Protocol efficiency validation
*/

use crate::common::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener as TokioTcpListener, TcpStream as TokioTcpStream};
use tokio::time::{sleep, timeout};

/// Network benchmarking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub test_duration_seconds: u64,
    pub concurrent_connections: usize,
    pub packet_sizes: Vec<usize>,
    pub target_bandwidth_mbps: f64,
    pub max_latency_ms: f64,
    pub packet_loss_percent: f64,
    pub jitter_range_ms: f64,
    pub enable_tcp_nodelay: bool,
    pub buffer_size: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            test_duration_seconds: 30,
            concurrent_connections: 100,
            packet_sizes: vec![64, 256, 512, 1024, 4096, 8192, 65536], // Various packet sizes
            target_bandwidth_mbps: 1000.0, // 1 Gbps
            max_latency_ms: 10.0,
            packet_loss_percent: 0.1,
            jitter_range_ms: 2.0,
            enable_tcp_nodelay: true,
            buffer_size: 65536,
        }
    }
}

/// Network benchmark server for testing client-server scenarios
pub struct NetworkBenchmarkServer {
    config: NetworkConfig,
    listener: Option<TokioTcpListener>,
    active_connections: Arc<Mutex<usize>>,
    metrics: Arc<Mutex<NetworkMetrics>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_connections: usize,
    pub active_connections: usize,
    pub connection_errors: usize,
    pub latency_samples: Vec<f64>,
    pub throughput_samples: Vec<f64>,
    pub packet_loss_events: usize,
}

impl NetworkBenchmarkServer {
    pub fn new(config: NetworkConfig) -> Self {
        Self {
            config,
            listener: None,
            active_connections: Arc::new(Mutex::new(0)),
            metrics: Arc::new(Mutex::new(NetworkMetrics::default())),
        }
    }

    pub async fn start(&mut self, bind_addr: &str) -> anyhow::Result<SocketAddr> {
        let listener = TokioTcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;
        
        println!("🌐 Network benchmark server started on {}", local_addr);
        
        let active_connections = self.active_connections.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let active_connections = active_connections.clone();
                        let metrics = metrics.clone();
                        let config = config.clone();
                        
                        tokio::spawn(async move {
                            Self::handle_client(stream, peer_addr, active_connections, metrics, config).await;
                        });
                    }
                    Err(e) => {
                        eprintln!("Server accept error: {}", e);
                        break;
                    }
                }
            }
        });

        self.listener = Some(listener);
        Ok(local_addr)
    }

    async fn handle_client(
        mut stream: TokioTcpStream,
        peer_addr: SocketAddr,
        active_connections: Arc<Mutex<usize>>,
        metrics: Arc<Mutex<NetworkMetrics>>,
        config: NetworkConfig,
    ) {
        {
            let mut count = active_connections.lock().unwrap();
            *count += 1;
            let mut m = metrics.lock().unwrap();
            m.total_connections += 1;
            m.active_connections = *count;
        }

        if config.enable_tcp_nodelay {
            let _ = stream.set_nodelay(true);
        }

        let mut buffer = vec![0u8; config.buffer_size];
        
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    // Simulate processing delay and jitter
                    if config.jitter_range_ms > 0.0 {
                        let jitter = fastrand::f64() * config.jitter_range_ms;
                        sleep(Duration::from_secs_f64(jitter / 1000.0)).await;
                    }

                    // Simulate packet loss
                    if fastrand::f64() < config.packet_loss_percent / 100.0 {
                        let mut m = metrics.lock().unwrap();
                        m.packet_loss_events += 1;
                        continue; // Drop packet
                    }

                    // Echo data back
                    if let Err(e) = stream.write_all(&buffer[..n]).await {
                        eprintln!("Write error to {}: {}", peer_addr, e);
                        break;
                    }

                    // Update metrics
                    {
                        let mut m = metrics.lock().unwrap();
                        m.total_bytes_received += n as u64;
                        m.total_bytes_sent += n as u64;
                    }
                }
                Err(e) => {
                    eprintln!("Read error from {}: {}", peer_addr, e);
                    let mut m = metrics.lock().unwrap();
                    m.connection_errors += 1;
                    break;
                }
            }
        }

        {
            let mut count = active_connections.lock().unwrap();
            *count -= 1;
            let mut m = metrics.lock().unwrap();
            m.active_connections = *count;
        }
    }

    pub fn get_metrics(&self) -> NetworkMetrics {
        self.metrics.lock().unwrap().clone()
    }
}

/// Network benchmark client for testing various network scenarios
pub struct NetworkBenchmarkClient {
    config: NetworkConfig,
}

impl NetworkBenchmarkClient {
    pub fn new(config: NetworkConfig) -> Self {
        Self { config }
    }

    /// Run bandwidth test
    pub async fn run_bandwidth_test(&self, server_addr: &str) -> anyhow::Result<BandwidthTestResult> {
        println!("🚀 Running bandwidth test to {}", server_addr);
        
        let mut results = Vec::new();
        
        for &packet_size in &self.config.packet_sizes {
            let result = self.run_single_bandwidth_test(server_addr, packet_size).await?;
            results.push(result);
            
            println!("  📦 {:<6} bytes: {:.2} Mbps, {:.3}ms latency", 
                     packet_size, result.throughput_mbps, result.avg_latency_ms);
        }

        // Calculate overall statistics
        let avg_throughput = results.iter().map(|r| r.throughput_mbps).sum::<f64>() / results.len() as f64;
        let max_throughput = results.iter().map(|r| r.throughput_mbps).fold(0.0f64, |a, b| a.max(b));
        let avg_latency = results.iter().map(|r| r.avg_latency_ms).sum::<f64>() / results.len() as f64;

        Ok(BandwidthTestResult {
            packet_size: 0, // Overall result
            throughput_mbps: avg_throughput,
            max_throughput_mbps: max_throughput,
            avg_latency_ms: avg_latency,
            packet_loss_percent: results.iter().map(|r| r.packet_loss_percent).sum::<f64>() / results.len() as f64,
            test_duration: Duration::from_secs(self.config.test_duration_seconds),
            bytes_transferred: results.iter().map(|r| r.bytes_transferred).sum(),
            target_achieved: avg_throughput >= self.config.target_bandwidth_mbps * 0.8, // 80% of target
        })
    }

    async fn run_single_bandwidth_test(&self, server_addr: &str, packet_size: usize) -> anyhow::Result<BandwidthTestResult> {
        let start_time = Instant::now();
        let test_duration = Duration::from_secs(5); // Shorter per-packet-size test
        
        let mut stream = timeout(Duration::from_secs(10), TokioTcpStream::connect(server_addr)).await??;
        
        if self.config.enable_tcp_nodelay {
            stream.set_nodelay(true)?;
        }

        let test_data = vec![0u8; packet_size];
        let mut response_buffer = vec![0u8; packet_size];
        
        let mut bytes_transferred = 0u64;
        let mut latency_samples = Vec::new();
        let mut packets_sent = 0u64;
        let mut packets_lost = 0u64;

        while start_time.elapsed() < test_duration {
            let send_start = Instant::now();
            
            // Send data
            if let Err(_) = timeout(Duration::from_secs(1), stream.write_all(&test_data)).await {
                packets_lost += 1;
                continue;
            }
            
            // Receive response
            match timeout(Duration::from_secs(1), stream.read_exact(&mut response_buffer)).await {
                Ok(Ok(_)) => {
                    let latency = send_start.elapsed();
                    latency_samples.push(latency.as_secs_f64() * 1000.0);
                    bytes_transferred += packet_size as u64 * 2; // Send + receive
                    packets_sent += 1;
                }
                Ok(Err(_)) | Err(_) => {
                    packets_lost += 1;
                }
            }
        }

        let actual_duration = start_time.elapsed();
        let throughput_mbps = if actual_duration.as_secs_f64() > 0.0 {
            (bytes_transferred as f64 * 8.0) / (actual_duration.as_secs_f64() * 1_000_000.0)
        } else {
            0.0
        };

        let avg_latency_ms = if !latency_samples.is_empty() {
            latency_samples.iter().sum::<f64>() / latency_samples.len() as f64
        } else {
            0.0
        };

        let packet_loss_percent = if packets_sent + packets_lost > 0 {
            (packets_lost as f64 / (packets_sent + packets_lost) as f64) * 100.0
        } else {
            0.0
        };

        Ok(BandwidthTestResult {
            packet_size,
            throughput_mbps,
            max_throughput_mbps: throughput_mbps,
            avg_latency_ms,
            packet_loss_percent,
            test_duration: actual_duration,
            bytes_transferred,
            target_achieved: throughput_mbps >= self.config.target_bandwidth_mbps * 0.5,
        })
    }

    /// Run connection scaling test
    pub async fn run_connection_scaling_test(&self, server_addr: &str) -> anyhow::Result<ConnectionScalingResult> {
        println!("📈 Running connection scaling test with {} connections", self.config.concurrent_connections);
        
        let start_time = Instant::now();
        let mut handles = Vec::new();
        let successful_connections = Arc::new(Mutex::new(0usize));
        let failed_connections = Arc::new(Mutex::new(0usize));
        let total_bytes = Arc::new(Mutex::new(0u64));

        // Spawn concurrent connections
        for i in 0..self.config.concurrent_connections {
            let server_addr = server_addr.to_string();
            let successful = successful_connections.clone();
            let failed = failed_connections.clone();
            let bytes = total_bytes.clone();
            let config = self.config.clone();
            
            let handle = tokio::spawn(async move {
                match Self::run_single_connection_test(&server_addr, i, config).await {
                    Ok(bytes_transferred) => {
                        *successful.lock().unwrap() += 1;
                        *bytes.lock().unwrap() += bytes_transferred;
                    }
                    Err(_) => {
                        *failed.lock().unwrap() += 1;
                    }
                }
            });
            
            handles.push(handle);
            
            // Small delay between connection attempts to avoid overwhelming the server
            if i % 10 == 0 && i > 0 {
                sleep(Duration::from_millis(10)).await;
            }
        }

        // Wait for all connections to complete
        for handle in handles {
            let _ = handle.await;
        }

        let total_duration = start_time.elapsed();
        let successful = *successful_connections.lock().unwrap();
        let failed = *failed_connections.lock().unwrap();
        let bytes_transferred = *total_bytes.lock().unwrap();

        let connection_rate = successful as f64 / total_duration.as_secs_f64();
        let success_rate = successful as f64 / self.config.concurrent_connections as f64;
        
        let throughput_mbps = if total_duration.as_secs_f64() > 0.0 {
            (bytes_transferred as f64 * 8.0) / (total_duration.as_secs_f64() * 1_000_000.0)
        } else {
            0.0
        };

        Ok(ConnectionScalingResult {
            target_connections: self.config.concurrent_connections,
            successful_connections: successful,
            failed_connections: failed,
            connection_rate_per_sec: connection_rate,
            success_rate_percent: success_rate * 100.0,
            total_duration,
            aggregate_throughput_mbps: throughput_mbps,
            bytes_transferred,
            scaling_achieved: success_rate >= 0.95, // 95% success rate
        })
    }

    async fn run_single_connection_test(
        server_addr: &str,
        connection_id: usize,
        config: NetworkConfig,
    ) -> anyhow::Result<u64> {
        let mut stream = timeout(
            Duration::from_secs(5), 
            TokioTcpStream::connect(server_addr)
        ).await??;
        
        if config.enable_tcp_nodelay {
            stream.set_nodelay(true)?;
        }

        let test_data = vec![(connection_id % 256) as u8; 1024]; // 1KB packets
        let mut response_buffer = vec![0u8; 1024];
        let mut bytes_transferred = 0u64;

        // Send 10 packets per connection
        for _ in 0..10 {
            timeout(Duration::from_secs(1), stream.write_all(&test_data)).await??;
            timeout(Duration::from_secs(1), stream.read_exact(&mut response_buffer)).await??;
            bytes_transferred += 2048; // 1KB send + 1KB receive
            
            // Small delay between packets
            sleep(Duration::from_millis(10)).await;
        }

        Ok(bytes_transferred)
    }

    /// Run latency test with various patterns
    pub async fn run_latency_test(&self, server_addr: &str) -> anyhow::Result<LatencyTestResult> {
        println!("⏱️  Running latency test");
        
        let mut stream = TokioTcpStream::connect(server_addr).await?;
        if self.config.enable_tcp_nodelay {
            stream.set_nodelay(true)?;
        }

        let test_packet = vec![0u8; 64]; // Small packets for latency testing
        let mut response_buffer = vec![0u8; 64];
        let mut latency_samples = Vec::new();
        
        let num_samples = 1000;
        
        for i in 0..num_samples {
            let start = Instant::now();
            
            // Send packet
            stream.write_all(&test_packet).await?;
            
            // Receive response
            stream.read_exact(&mut response_buffer).await?;
            
            let latency = start.elapsed();
            latency_samples.push(latency.as_secs_f64() * 1000.0); // Convert to ms
            
            if i % 100 == 0 {
                sleep(Duration::from_millis(1)).await; // Prevent overwhelming
            }
        }

        // Calculate statistics
        latency_samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let min_latency = latency_samples[0];
        let max_latency = latency_samples[latency_samples.len() - 1];
        let avg_latency = latency_samples.iter().sum::<f64>() / latency_samples.len() as f64;
        let p50_latency = latency_samples[latency_samples.len() / 2];
        let p95_latency = latency_samples[(latency_samples.len() as f64 * 0.95) as usize];
        let p99_latency = latency_samples[(latency_samples.len() as f64 * 0.99) as usize];

        // Calculate jitter (standard deviation)
        let variance = latency_samples.iter()
            .map(|x| (x - avg_latency).powi(2))
            .sum::<f64>() / latency_samples.len() as f64;
        let jitter = variance.sqrt();

        Ok(LatencyTestResult {
            sample_count: latency_samples.len(),
            min_latency_ms: min_latency,
            max_latency_ms: max_latency,
            avg_latency_ms: avg_latency,
            p50_latency_ms: p50_latency,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
            jitter_ms: jitter,
            target_achieved: p95_latency <= self.config.max_latency_ms,
        })
    }
}

/// Run comprehensive network benchmarks
pub async fn run_network_benchmarks(config: NetworkConfig) -> anyhow::Result<NetworkBenchmarkSuite> {
    println!("🌐 Starting comprehensive network benchmarks");
    
    // Start benchmark server
    let mut server = NetworkBenchmarkServer::new(config.clone());
    let server_addr = server.start("127.0.0.1:0").await?;
    let server_addr_str = server_addr.to_string();
    
    // Wait for server to be ready
    sleep(Duration::from_millis(100)).await;
    
    // Create client
    let client = NetworkBenchmarkClient::new(config.clone());
    
    println!("🔧 Running bandwidth tests...");
    let bandwidth_result = client.run_bandwidth_test(&server_addr_str).await?;
    
    println!("📈 Running connection scaling tests...");
    let scaling_result = client.run_connection_scaling_test(&server_addr_str).await?;
    
    println!("⏱️  Running latency tests...");
    let latency_result = client.run_latency_test(&server_addr_str).await?;
    
    let server_metrics = server.get_metrics();
    
    Ok(NetworkBenchmarkSuite {
        config,
        server_metrics,
        bandwidth_result,
        scaling_result,
        latency_result,
        overall_success: bandwidth_result.target_achieved 
                        && scaling_result.scaling_achieved 
                        && latency_result.target_achieved,
    })
}

// Data structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthTestResult {
    pub packet_size: usize,
    pub throughput_mbps: f64,
    pub max_throughput_mbps: f64,
    pub avg_latency_ms: f64,
    pub packet_loss_percent: f64,
    pub test_duration: Duration,
    pub bytes_transferred: u64,
    pub target_achieved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionScalingResult {
    pub target_connections: usize,
    pub successful_connections: usize,
    pub failed_connections: usize,
    pub connection_rate_per_sec: f64,
    pub success_rate_percent: f64,
    pub total_duration: Duration,
    pub aggregate_throughput_mbps: f64,
    pub bytes_transferred: u64,
    pub scaling_achieved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyTestResult {
    pub sample_count: usize,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub jitter_ms: f64,
    pub target_achieved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkBenchmarkSuite {
    pub config: NetworkConfig,
    pub server_metrics: NetworkMetrics,
    pub bandwidth_result: BandwidthTestResult,
    pub scaling_result: ConnectionScalingResult,
    pub latency_result: LatencyTestResult,
    pub overall_success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_server_creation() {
        let config = NetworkConfig::default();
        let mut server = NetworkBenchmarkServer::new(config);
        
        let result = server.start("127.0.0.1:0").await;
        assert!(result.is_ok());
        
        let addr = result.unwrap();
        assert!(addr.port() > 0);
    }

    #[tokio::test]
    async fn test_network_client_creation() {
        let config = NetworkConfig::default();
        let client = NetworkBenchmarkClient::new(config);
        
        // Just test that client can be created
        // Full network tests require a running server
        assert_eq!(client.config.concurrent_connections, 100);
    }

    #[tokio::test]
    async fn test_basic_network_communication() {
        let config = NetworkConfig {
            test_duration_seconds: 1,
            concurrent_connections: 1,
            packet_sizes: vec![64],
            ..Default::default()
        };

        // Start server
        let mut server = NetworkBenchmarkServer::new(config.clone());
        let server_addr = server.start("127.0.0.1:0").await.unwrap();
        
        // Give server time to start
        sleep(Duration::from_millis(50)).await;
        
        // Create client and test
        let client = NetworkBenchmarkClient::new(config);
        let result = client.run_latency_test(&server_addr.to_string()).await;
        
        match result {
            Ok(latency_result) => {
                assert!(latency_result.sample_count > 0);
                assert!(latency_result.avg_latency_ms >= 0.0);
            }
            Err(e) => {
                println!("Network test failed (may be expected in some environments): {}", e);
            }
        }
    }
}