// STOQ Phase 5: Comprehensive Performance Benchmarking Suite
// Validates 10+ Gbps throughput, <1ms latency, and scalability claims

use stoq::{StoqTransport, Config, NetworkTier};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio::time::{Duration, Instant};
use bytes::Bytes;

mod throughput_benchmarks {
    use super::*;

    #[tokio::test]
    #[ignore] // Run with: cargo test --test phase5_performance_benchmarks throughput -- --ignored
    async fn benchmark_single_connection_throughput() {
        println!("\n=== Single Connection Throughput Benchmark ===");

        let mut config = Config::default();
        config.network_tier = NetworkTier::Lan;
        config.enable_ebpf = true; // Enable if available

        let server = StoqTransport::new(config.clone());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let bytes_received = Arc::new(AtomicU64::new(0));
        let bytes_clone = bytes_received.clone();

        // Server receive loop
        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 65536]; // 64KB buffer

            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(10) {
                match stream.read(&mut buf).await {
                    Ok(n) if n > 0 => {
                        bytes_clone.fetch_add(n as u64, Ordering::Relaxed);
                    }
                    _ => break,
                }
            }
        });

        // Client send loop
        let client = StoqTransport::new(config);
        let mut stream = client.connect(server_addr).await.unwrap();

        let data = vec![0xFF; 65536]; // 64KB chunks
        let start = Instant::now();
        let test_duration = Duration::from_secs(10);

        while start.elapsed() < test_duration {
            stream.write_all(&data).await.unwrap();
        }

        drop(stream); // Close connection
        server_handle.await.unwrap();

        // Calculate throughput
        let total_bytes = bytes_received.load(Ordering::Relaxed);
        let throughput_gbps = (total_bytes as f64 * 8.0) / (10.0 * 1_000_000_000.0);

        println!("Single connection throughput: {:.2} Gbps", throughput_gbps);
        println!("Total data transferred: {:.2} GB", total_bytes as f64 / 1_000_000_000.0);

        // Verify meets claims
        #[cfg(feature = "ebpf")]
        assert!(throughput_gbps >= 9.0, "Should achieve >9 Gbps with eBPF");

        #[cfg(not(feature = "ebpf"))]
        assert!(throughput_gbps >= 2.5, "Should achieve >2.5 Gbps without eBPF");
    }

    #[tokio::test]
    #[ignore]
    async fn benchmark_multi_connection_aggregate() {
        println!("\n=== Multi-Connection Aggregate Throughput Benchmark ===");

        const NUM_CONNECTIONS: usize = 10;

        let server = Arc::new(StoqTransport::new(Config::default()));
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let total_bytes = Arc::new(AtomicU64::new(0));

        // Server accept multiple connections
        let bytes_clone = total_bytes.clone();
        let server_handle = tokio::spawn(async move {
            let mut handles = Vec::new();

            for _ in 0..NUM_CONNECTIONS {
                let (mut stream, _) = listener.accept().await.unwrap();
                let bytes = bytes_clone.clone();

                let handle = tokio::spawn(async move {
                    let mut buf = vec![0u8; 65536];
                    let start = Instant::now();

                    while start.elapsed() < Duration::from_secs(10) {
                        match stream.read(&mut buf).await {
                            Ok(n) if n > 0 => {
                                bytes.fetch_add(n as u64, Ordering::Relaxed);
                            }
                            _ => break,
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.await.unwrap();
            }
        });

        // Create multiple client connections
        let mut client_handles = Vec::new();
        for _ in 0..NUM_CONNECTIONS {
            let handle = tokio::spawn(async move {
                let client = StoqTransport::new(Config::default());
                let mut stream = client.connect(server_addr).await.unwrap();

                let data = vec![0xFF; 65536];
                let start = Instant::now();

                while start.elapsed() < Duration::from_secs(10) {
                    let _ = stream.write_all(&data).await;
                }
            });
            client_handles.push(handle);
        }

        for handle in client_handles {
            handle.await.unwrap();
        }

        server_handle.await.unwrap();

        // Calculate aggregate throughput
        let bytes = total_bytes.load(Ordering::Relaxed);
        let throughput_gbps = (bytes as f64 * 8.0) / (10.0 * 1_000_000_000.0);

        println!("Aggregate throughput ({} connections): {:.2} Gbps", NUM_CONNECTIONS, throughput_gbps);
        println!("Per-connection average: {:.2} Gbps", throughput_gbps / NUM_CONNECTIONS as f64);

        assert!(throughput_gbps >= 5.0, "Aggregate should exceed 5 Gbps");
    }

    #[tokio::test]
    #[ignore]
    async fn benchmark_large_file_transfer() {
        println!("\n=== Large File Transfer Benchmark ===");

        const FILE_SIZE: usize = 1024 * 1024 * 1024; // 1GB

        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut received = 0usize;
            let mut buf = vec![0u8; 1024 * 1024]; // 1MB buffer

            let start = Instant::now();
            while received < FILE_SIZE {
                let n = stream.read(&mut buf).await.unwrap();
                received += n;
            }
            let duration = start.elapsed();

            let throughput_mbps = (FILE_SIZE as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0);
            println!("Server received 1GB in {:.2}s ({:.0} Mbps)", duration.as_secs_f64(), throughput_mbps);
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        // Send 1GB file
        let chunk = vec![0xFF; 1024 * 1024]; // 1MB chunks
        let chunks = FILE_SIZE / chunk.len();

        let start = Instant::now();
        for _ in 0..chunks {
            stream.write_all(&chunk).await.unwrap();
        }
        let duration = start.elapsed();

        let throughput_mbps = (FILE_SIZE as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0);
        println!("Client sent 1GB in {:.2}s ({:.0} Mbps)", duration.as_secs_f64(), throughput_mbps);

        server_handle.await.unwrap();

        assert!(throughput_mbps >= 1000.0, "Should exceed 1 Gbps for large files");
    }

    #[tokio::test]
    #[ignore]
    async fn benchmark_small_packet_performance() {
        println!("\n=== Small Packet Performance Benchmark ===");

        const PACKET_SIZE: usize = 64; // Small packets
        const NUM_PACKETS: usize = 1_000_000;

        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let packets_received = Arc::new(AtomicUsize::new(0));
        let packets_clone = packets_received.clone();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; PACKET_SIZE];

            while packets_clone.load(Ordering::Relaxed) < NUM_PACKETS {
                let n = stream.read(&mut buf).await.unwrap();
                if n == PACKET_SIZE {
                    packets_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        let packet = vec![0xAA; PACKET_SIZE];
        let start = Instant::now();

        for _ in 0..NUM_PACKETS {
            stream.write_all(&packet).await.unwrap();
        }

        server_handle.await.unwrap();
        let duration = start.elapsed();

        let packets_per_second = NUM_PACKETS as f64 / duration.as_secs_f64();
        let throughput_mbps = (NUM_PACKETS * PACKET_SIZE * 8) as f64 / (duration.as_secs_f64() * 1_000_000.0);

        println!("Small packet rate: {:.0} packets/second", packets_per_second);
        println!("Effective throughput: {:.2} Mbps", throughput_mbps);

        assert!(packets_per_second >= 100_000.0, "Should handle >100K packets/sec");
    }
}

mod latency_benchmarks {
    use super::*;
    use std::collections::VecDeque;

    #[tokio::test]
    async fn benchmark_round_trip_time() {
        println!("\n=== Round Trip Time (RTT) Benchmark ===");

        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Echo server
        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 1024];

            for _ in 0..1000 {
                let n = stream.read(&mut buf).await.unwrap();
                stream.write_all(&buf[..n]).await.unwrap(); // Echo back
            }
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        let mut rtts = Vec::new();
        let payload = b"ping";

        // Measure RTTs
        for _ in 0..1000 {
            let start = Instant::now();

            stream.write_all(payload).await.unwrap();
            let mut buf = vec![0u8; payload.len()];
            stream.read_exact(&mut buf).await.unwrap();

            let rtt = start.elapsed();
            rtts.push(rtt);
        }

        server_handle.await.unwrap();

        // Calculate statistics
        rtts.sort();
        let min = rtts[0];
        let max = rtts[rtts.len() - 1];
        let median = rtts[rtts.len() / 2];
        let p95 = rtts[(rtts.len() * 95) / 100];
        let p99 = rtts[(rtts.len() * 99) / 100];
        let avg = rtts.iter().sum::<Duration>() / rtts.len() as u32;

        println!("RTT Statistics (1000 samples):");
        println!("  Min: {:?}", min);
        println!("  Median: {:?}", median);
        println!("  Average: {:?}", avg);
        println!("  P95: {:?}", p95);
        println!("  P99: {:?}", p99);
        println!("  Max: {:?}", max);

        // Verify LAN latency claim
        assert!(median < Duration::from_millis(1), "Median RTT should be <1ms for LAN");
        assert!(p99 < Duration::from_millis(2), "P99 RTT should be <2ms for LAN");
    }

    #[tokio::test]
    async fn benchmark_latency_under_load() {
        println!("\n=== Latency Under Load Benchmark ===");

        let server = Arc::new(StoqTransport::new(Config::default()));
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Server handles both background traffic and latency measurements
        let server_handle = tokio::spawn(async move {
            let mut handles = Vec::new();

            // Accept multiple connections
            for i in 0..11 {
                let (mut stream, _) = listener.accept().await.unwrap();

                let handle = if i == 0 {
                    // First connection: latency measurement (echo)
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 64];
                        for _ in 0..100 {
                            let n = stream.read(&mut buf).await.unwrap();
                            stream.write_all(&buf[..n]).await.unwrap();
                        }
                    })
                } else {
                    // Other connections: background load
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 65536];
                        loop {
                            if stream.read(&mut buf).await.is_err() {
                                break;
                            }
                        }
                    })
                };
                handles.push(handle);
            }

            for handle in handles {
                let _ = handle.await;
            }
        });

        // Create latency measurement client
        let latency_client = StoqTransport::new(Config::default());
        let mut latency_stream = latency_client.connect(server_addr).await.unwrap();

        // Create background load clients
        let mut load_handles = Vec::new();
        for _ in 0..10 {
            let handle = tokio::spawn(async move {
                let client = StoqTransport::new(Config::default());
                let mut stream = client.connect(server_addr).await.unwrap();
                let data = vec![0xFF; 65536];

                for _ in 0..1000 {
                    let _ = stream.write_all(&data).await;
                }
            });
            load_handles.push(handle);
        }

        // Measure latency while under load
        let mut loaded_rtts = Vec::new();
        let payload = b"latency_test";

        tokio::time::sleep(Duration::from_millis(100)).await; // Let load stabilize

        for _ in 0..100 {
            let start = Instant::now();

            latency_stream.write_all(payload).await.unwrap();
            let mut buf = vec![0u8; payload.len()];
            latency_stream.read_exact(&mut buf).await.unwrap();

            loaded_rtts.push(start.elapsed());
        }

        // Stop load generators
        for handle in load_handles {
            handle.abort();
        }

        server_handle.abort();

        // Analyze results
        loaded_rtts.sort();
        let loaded_median = loaded_rtts[loaded_rtts.len() / 2];
        let loaded_p99 = loaded_rtts[(loaded_rtts.len() * 99) / 100];

        println!("Latency under load:");
        println!("  Median: {:?}", loaded_median);
        println!("  P99: {:?}", loaded_p99);

        // Should maintain low latency even under load
        assert!(loaded_median < Duration::from_millis(5), "Median should be <5ms under load");
        assert!(loaded_p99 < Duration::from_millis(10), "P99 should be <10ms under load");
    }

    #[tokio::test]
    async fn benchmark_jitter_measurement() {
        println!("\n=== Jitter Measurement Benchmark ===");

        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 64];

            for _ in 0..1000 {
                let n = stream.read(&mut buf).await.unwrap();
                stream.write_all(&buf[..n]).await.unwrap();
            }
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        let mut rtts = VecDeque::new();
        let payload = b"jitter_test";

        // Measure RTTs for jitter calculation
        for _ in 0..1000 {
            let start = Instant::now();

            stream.write_all(payload).await.unwrap();
            let mut buf = vec![0u8; payload.len()];
            stream.read_exact(&mut buf).await.unwrap();

            rtts.push_back(start.elapsed().as_micros() as f64);
        }

        server_handle.await.unwrap();

        // Calculate jitter (variation in latency)
        let mut jitters = Vec::new();
        for i in 1..rtts.len() {
            let jitter = (rtts[i] - rtts[i-1]).abs();
            jitters.push(jitter);
        }

        jitters.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_jitter = jitters.iter().sum::<f64>() / jitters.len() as f64;
        let max_jitter = jitters[jitters.len() - 1];
        let p95_jitter = jitters[(jitters.len() * 95) / 100];

        println!("Jitter Statistics:");
        println!("  Average: {:.2} µs", avg_jitter);
        println!("  P95: {:.2} µs", p95_jitter);
        println!("  Max: {:.2} µs", max_jitter);

        // Good networks should have low jitter
        assert!(avg_jitter < 100.0, "Average jitter should be <100µs");
        assert!(p95_jitter < 500.0, "P95 jitter should be <500µs");
    }

    #[tokio::test]
    #[ignore]
    async fn benchmark_network_tier_latencies() {
        println!("\n=== Network Tier Latency Comparison ===");

        // Test different network tiers
        let tiers = vec![
            (NetworkTier::Lan, Duration::from_millis(1)),
            (NetworkTier::Metro, Duration::from_millis(5)),
            (NetworkTier::Wan, Duration::from_millis(50)),
            (NetworkTier::Satellite, Duration::from_millis(600)),
        ];

        for (tier, expected_latency) in tiers {
            let mut config = Config::default();
            config.network_tier = tier.clone();
            config.simulated_latency = Some(expected_latency);

            let server = StoqTransport::new(config.clone());
            let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
            let server_addr = listener.local_addr().unwrap();

            let server_handle = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 64];

                for _ in 0..100 {
                    let n = stream.read(&mut buf).await.unwrap();
                    stream.write_all(&buf[..n]).await.unwrap();
                }
            });

            let client = StoqTransport::new(config);
            let mut stream = client.connect(server_addr).await.unwrap();

            let mut rtts = Vec::new();
            for _ in 0..100 {
                let start = Instant::now();
                stream.write_all(b"test").await.unwrap();
                let mut buf = vec![0u8; 4];
                stream.read_exact(&mut buf).await.unwrap();
                rtts.push(start.elapsed());
            }

            server_handle.await.unwrap();

            rtts.sort();
            let median = rtts[rtts.len() / 2];

            println!("{:?} tier median RTT: {:?}", tier, median);

            // Verify tier-appropriate latency
            match tier {
                NetworkTier::Lan => assert!(median < Duration::from_millis(2)),
                NetworkTier::Metro => assert!(median < Duration::from_millis(10)),
                NetworkTier::Wan => assert!(median < Duration::from_millis(100)),
                NetworkTier::Satellite => assert!(median < Duration::from_secs(1)),
                _ => {}
            }
        }
    }
}

mod scalability_benchmarks {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn benchmark_connection_scaling() {
        println!("\n=== Connection Scaling Benchmark ===");

        let mut config = Config::default();
        config.max_connections = 10_000;

        let server = Arc::new(StoqTransport::new(config));
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let active_connections = Arc::new(AtomicUsize::new(0));
        let connections_clone = active_connections.clone();

        // Server accepts connections
        let server_handle = tokio::spawn(async move {
            let mut handles = Vec::new();

            loop {
                match tokio::time::timeout(Duration::from_millis(100), listener.accept()).await {
                    Ok(Ok((stream, _))) => {
                        let conns = connections_clone.clone();
                        conns.fetch_add(1, Ordering::Relaxed);

                        let handle = tokio::spawn(async move {
                            // Keep connection alive
                            let mut buf = vec![0u8; 64];
                            while stream.read(&mut buf).await.is_ok() {
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                            conns.fetch_sub(1, Ordering::Relaxed);
                        });
                        handles.push(handle);
                    }
                    _ => {
                        if connections_clone.load(Ordering::Relaxed) >= 10_000 {
                            break;
                        }
                    }
                }
            }

            // Keep connections alive
            tokio::time::sleep(Duration::from_secs(5)).await;
        });

        // Create many connections
        let mut client_handles = Vec::new();
        let mut established = 0;

        for i in 0..10_000 {
            let handle = tokio::spawn(async move {
                let client = StoqTransport::new(Config::default());
                match tokio::time::timeout(
                    Duration::from_secs(5),
                    client.connect(server_addr)
                ).await {
                    Ok(Ok(mut stream)) => {
                        // Keep alive
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        let _ = stream.write_all(b"bye").await;
                        true
                    }
                    _ => false
                }
            });
            client_handles.push(handle);

            // Pace connection creation
            if i % 100 == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
                println!("Created {} connections...", i);
            }
        }

        // Wait for all clients
        for handle in client_handles {
            if handle.await.unwrap() {
                established += 1;
            }
        }

        server_handle.abort();

        let peak_connections = active_connections.load(Ordering::Relaxed);
        println!("Peak concurrent connections: {}", peak_connections);
        println!("Successfully established: {}", established);

        assert!(established >= 9000, "Should handle >9000 concurrent connections");
    }

    #[tokio::test]
    async fn benchmark_cpu_efficiency() {
        println!("\n=== CPU Efficiency Benchmark ===");

        use sysinfo::{System, SystemExt, ProcessExt};

        let mut system = System::new_all();
        system.refresh_all();

        let process_id = sysinfo::get_current_pid().unwrap();
        let initial_cpu = system.process(process_id).unwrap().cpu_usage();

        // Run high-throughput test
        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 65536];
            let mut total = 0u64;

            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(10) {
                if let Ok(n) = stream.read(&mut buf).await {
                    total += n as u64;
                }
            }
            total
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        let data = vec![0xFF; 65536];
        let start = Instant::now();

        while start.elapsed() < Duration::from_secs(10) {
            let _ = stream.write_all(&data).await;
        }

        let total_bytes = server_handle.await.unwrap();

        // Measure CPU after test
        system.refresh_process(process_id);
        let final_cpu = system.process(process_id).unwrap().cpu_usage();
        let cpu_delta = final_cpu - initial_cpu;

        let throughput_gbps = (total_bytes as f64 * 8.0) / (10.0 * 1_000_000_000.0);
        let cpu_per_gbps = cpu_delta / throughput_gbps as f32;

        println!("Throughput: {:.2} Gbps", throughput_gbps);
        println!("CPU usage: {:.1}%", cpu_delta);
        println!("CPU per Gbps: {:.2}%", cpu_per_gbps);

        #[cfg(feature = "ebpf")]
        assert!(cpu_per_gbps < 1.0, "Should use <1% CPU per Gbps with eBPF");

        #[cfg(not(feature = "ebpf"))]
        assert!(cpu_per_gbps < 5.0, "Should use <5% CPU per Gbps without eBPF");
    }

    #[tokio::test]
    async fn benchmark_memory_usage() {
        println!("\n=== Memory Usage Benchmark ===");

        use sysinfo::{System, SystemExt, ProcessExt};

        let mut system = System::new_all();
        system.refresh_all();

        let process_id = sysinfo::get_current_pid().unwrap();
        let initial_memory = system.process(process_id).unwrap().memory(); // KB

        // Create many connections
        let server = Arc::new(StoqTransport::new(Config::default()));
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let mut streams = Vec::new();
            for _ in 0..1000 {
                if let Ok((stream, _)) = listener.accept().await {
                    streams.push(stream);
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        });

        let mut clients = Vec::new();
        for _ in 0..1000 {
            let client = StoqTransport::new(Config::default());
            if let Ok(stream) = client.connect(server_addr).await {
                clients.push(stream);
            }
        }

        // Measure memory with connections
        system.refresh_process(process_id);
        let peak_memory = system.process(process_id).unwrap().memory();
        let memory_per_connection = (peak_memory - initial_memory) / 1000; // KB per connection

        println!("Initial memory: {} MB", initial_memory / 1024);
        println!("Peak memory: {} MB", peak_memory / 1024);
        println!("Memory per connection: {} KB", memory_per_connection);

        server_handle.abort();

        assert!(memory_per_connection < 100, "Should use <100KB per connection");
    }

    #[tokio::test]
    #[ignore]
    async fn benchmark_connection_establishment_rate() {
        println!("\n=== Connection Establishment Rate Benchmark ===");

        let server = Arc::new(StoqTransport::new(Config::default()));
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let connections_accepted = Arc::new(AtomicUsize::new(0));
        let accepted_clone = connections_accepted.clone();

        // Server accepts and immediately closes
        let server_handle = tokio::spawn(async move {
            loop {
                match tokio::time::timeout(Duration::from_millis(10), listener.accept()).await {
                    Ok(Ok((stream, _))) => {
                        accepted_clone.fetch_add(1, Ordering::Relaxed);
                        drop(stream); // Immediately close
                    }
                    _ => continue,
                }
            }
        });

        // Measure connection establishment rate
        let start = Instant::now();
        let mut handles = Vec::new();

        for _ in 0..10_000 {
            let handle = tokio::spawn(async move {
                let client = StoqTransport::new(Config::default());
                let _ = client.connect(server_addr).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let duration = start.elapsed();
        server_handle.abort();

        let total_accepted = connections_accepted.load(Ordering::Relaxed);
        let rate = total_accepted as f64 / duration.as_secs_f64();

        println!("Established {} connections in {:?}", total_accepted, duration);
        println!("Connection establishment rate: {:.0} conn/sec", rate);

        assert!(rate >= 1000.0, "Should establish >1000 connections/sec");
    }
}

mod stress_tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn stress_test_high_packet_rate() {
        println!("\n=== High Packet Rate Stress Test ===");

        const TARGET_PPS: usize = 1_000_000; // 1M packets/sec
        const TEST_DURATION: Duration = Duration::from_secs(10);

        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let packets_received = Arc::new(AtomicU64::new(0));
        let packets_clone = packets_received.clone();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 64]; // Small packets

            let start = Instant::now();
            while start.elapsed() < TEST_DURATION {
                if stream.read(&mut buf).await.is_ok() {
                    packets_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        let packet = vec![0xAB; 64];
        let start = Instant::now();
        let mut sent = 0u64;

        while start.elapsed() < TEST_DURATION {
            stream.write_all(&packet).await.unwrap();
            sent += 1;

            // Pace to target rate
            if sent % 1000 == 0 {
                let elapsed = start.elapsed();
                let target_sent = (TARGET_PPS as f64 * elapsed.as_secs_f64()) as u64;
                if sent > target_sent {
                    tokio::time::sleep(Duration::from_micros(100)).await;
                }
            }
        }

        server_handle.await.unwrap();

        let received = packets_received.load(Ordering::Relaxed);
        let pps = received as f64 / TEST_DURATION.as_secs_f64();

        println!("Sent {} packets", sent);
        println!("Received {} packets", received);
        println!("Packet rate: {:.0} pps", pps);
        println!("Packet loss: {:.2}%", (1.0 - received as f64 / sent as f64) * 100.0);

        assert!(pps >= 500_000.0, "Should handle >500K packets/sec");
    }

    #[tokio::test]
    #[ignore]
    async fn stress_test_network_congestion() {
        println!("\n=== Network Congestion Stress Test ===");

        let mut config = Config::default();
        config.adaptive_optimization = true;

        let server = StoqTransport::new(config.clone());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 65536];
            let mut total = 0u64;

            while total < 1_000_000_000 {
                // 1GB
                if let Ok(n) = stream.read(&mut buf).await {
                    total += n as u64;
                    // Simulate congestion
                    if total % 10_000_000 == 0 {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            }
        });

        let client = StoqTransport::new(config);
        let mut stream = client.connect(server_addr).await.unwrap();

        let data = vec![0xFF; 65536];
        let start = Instant::now();
        let mut sent = 0u64;

        while sent < 1_000_000_000 {
            stream.write_all(&data).await.unwrap();
            sent += data.len() as u64;
        }

        server_handle.await.unwrap();
        let duration = start.elapsed();

        let throughput_mbps = (sent as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0);
        println!("Throughput under congestion: {:.2} Mbps", throughput_mbps);

        // Verify adaptation worked
        let final_tier = stream.current_tier();
        println!("Adapted to tier: {:?}", final_tier);

        assert!(throughput_mbps >= 100.0, "Should maintain >100 Mbps even under congestion");
    }

    #[tokio::test]
    #[ignore]
    async fn stress_test_connection_churn() {
        println!("\n=== Connection Churn Stress Test ===");

        let server = Arc::new(StoqTransport::new(Config::default()));
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let total_connections = Arc::new(AtomicUsize::new(0));
        let connections_clone = total_connections.clone();

        // Server handles rapid connect/disconnect
        let server_handle = tokio::spawn(async move {
            loop {
                match tokio::time::timeout(Duration::from_millis(1), listener.accept()).await {
                    Ok(Ok((mut stream, _))) => {
                        connections_clone.fetch_add(1, Ordering::Relaxed);
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 64];
                            let _ = stream.read(&mut buf).await;
                        });
                    }
                    _ => continue,
                }
            }
        });

        // Rapid connect/disconnect cycles
        let start = Instant::now();
        let mut cycles = 0;

        while start.elapsed() < Duration::from_secs(30) {
            let client = StoqTransport::new(Config::default());
            if let Ok(mut stream) = client.connect(server_addr).await {
                stream.write_all(b"churn").await.unwrap();
                drop(stream); // Immediate disconnect
                cycles += 1;
            }
        }

        server_handle.abort();

        let total = total_connections.load(Ordering::Relaxed);
        let rate = cycles as f64 / start.elapsed().as_secs_f64();

        println!("Completed {} connection cycles", cycles);
        println!("Churn rate: {:.0} cycles/sec", rate);
        println!("Total connections handled: {}", total);

        assert!(rate >= 100.0, "Should handle >100 connect/disconnect cycles per second");
    }

    #[tokio::test]
    #[ignore]
    async fn stress_test_long_running_stability() {
        println!("\n=== Long Running Stability Test (5 minutes) ===");

        const TEST_DURATION: Duration = Duration::from_secs(300); // 5 minutes

        let server = StoqTransport::new(Config::default());
        let listener = server.listen("[::1]:0".parse().unwrap()).await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        let bytes_transferred = Arc::new(AtomicU64::new(0));
        let errors = Arc::new(AtomicUsize::new(0));

        let bytes_clone = bytes_transferred.clone();
        let errors_clone = errors.clone();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 65536];

            let start = Instant::now();
            while start.elapsed() < TEST_DURATION {
                match stream.read(&mut buf).await {
                    Ok(n) if n > 0 => {
                        bytes_clone.fetch_add(n as u64, Ordering::Relaxed);
                    }
                    Err(_) => {
                        errors_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    _ => break,
                }
            }
        });

        let client = StoqTransport::new(Config::default());
        let mut stream = client.connect(server_addr).await.unwrap();

        let data = vec![0xFF; 65536];
        let start = Instant::now();

        while start.elapsed() < TEST_DURATION {
            if stream.write_all(&data).await.is_err() {
                errors.fetch_add(1, Ordering::Relaxed);
            }
            tokio::time::sleep(Duration::from_millis(1)).await; // Pace transfers
        }

        server_handle.await.unwrap();

        let total_bytes = bytes_transferred.load(Ordering::Relaxed);
        let total_errors = errors.load(Ordering::Relaxed);
        let throughput_mbps = (total_bytes as f64 * 8.0) / (TEST_DURATION.as_secs_f64() * 1_000_000.0);

        println!("Long-running test results:");
        println!("  Duration: {:?}", TEST_DURATION);
        println!("  Data transferred: {:.2} GB", total_bytes as f64 / 1_000_000_000.0);
        println!("  Average throughput: {:.2} Mbps", throughput_mbps);
        println!("  Errors: {}", total_errors);

        assert_eq!(total_errors, 0, "Should have zero errors in long-running test");
        assert!(throughput_mbps >= 100.0, "Should maintain >100 Mbps average");
    }
}

// Run all performance benchmarks with:
// cargo test --test phase5_performance_benchmarks -- --ignored --test-threads=1