// Load Testing for Sprint 3
// Tests system behavior under sustained high load

#[cfg(test)]
mod load_tests {
    use blockmatrix::os_integration::{create_os_abstraction, types::*};
    use std::sync::{Arc, Mutex, atomic::{AtomicU64, AtomicBool, Ordering}};
    use std::thread;
    use std::time::{Duration, Instant};
    use std::collections::VecDeque;

    #[test]
    fn test_1000_consecutive_cpu_detections() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        let mut timings = Vec::new();
        let mut errors = 0;
        let start = Instant::now();

        for i in 0..1000 {
            let op_start = Instant::now();
            match os.detect_cpu() {
                Ok(cpu_info) => {
                    // Verify consistent results
                    assert!(cpu_info.core_count > 0, "Invalid CPU count at iteration {}", i);
                    assert!(cpu_info.frequency_mhz > 0, "Invalid frequency at iteration {}", i);
                }
                Err(e) => {
                    eprintln!("CPU detection failed at iteration {}: {}", i, e);
                    errors += 1;
                }
            }
            timings.push(op_start.elapsed());
        }

        let total_duration = start.elapsed();

        // Calculate statistics
        timings.sort();
        let median = timings[timings.len() / 2];
        let p95 = timings[timings.len() * 95 / 100];
        let p99 = timings[timings.len() * 99 / 100];

        println!("1000 CPU detections completed:");
        println!("  Total time: {:?}", total_duration);
        println!("  Median: {:?}", median);
        println!("  P95: {:?}", p95);
        println!("  P99: {:?}", p99);
        println!("  Errors: {}", errors);

        // Verify no degradation
        assert_eq!(errors, 0, "CPU detection should not fail");
        assert!(p99 < Duration::from_millis(100), "P99 latency too high");

        // Compare first and last 100 operations
        let first_100_avg: Duration = timings[..100].iter().sum::<Duration>() / 100;
        let last_100_avg: Duration = timings[900..].iter().sum::<Duration>() / 100;

        let degradation = if last_100_avg > first_100_avg {
            ((last_100_avg.as_micros() as f64 / first_100_avg.as_micros() as f64) - 1.0) * 100.0
        } else {
            0.0
        };

        println!("Performance degradation: {:.2}%", degradation);
        assert!(degradation < 20.0, "Performance degraded by {:.2}%", degradation);
    }

    #[test]
    fn test_concurrent_detection_from_10_threads() {
        let os = Arc::new(create_os_abstraction().expect("Failed to create OS abstraction"));
        let errors = Arc::new(AtomicU64::new(0));
        let operations = Arc::new(AtomicU64::new(0));

        let mut handles = Vec::new();
        let start = Instant::now();

        for thread_id in 0..10 {
            let os_clone = Arc::clone(&os);
            let errors_clone = Arc::clone(&errors);
            let operations_clone = Arc::clone(&operations);

            let handle = thread::spawn(move || {
                let mut local_timings = Vec::new();

                for _ in 0..100 {
                    let op_start = Instant::now();

                    // Mix different detection types
                    match thread_id % 4 {
                        0 => {
                            if os_clone.detect_cpu().is_err() {
                                errors_clone.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        1 => {
                            if os_clone.detect_memory().is_err() {
                                errors_clone.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        2 => {
                            if os_clone.detect_storage().is_err() {
                                errors_clone.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        _ => {
                            if os_clone.get_resource_usage().is_err() {
                                errors_clone.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }

                    operations_clone.fetch_add(1, Ordering::Relaxed);
                    local_timings.push(op_start.elapsed());
                }

                local_timings
            });

            handles.push(handle);
        }

        // Collect results
        let mut all_timings = Vec::new();
        for handle in handles {
            let timings = handle.join().expect("Thread panicked");
            all_timings.extend(timings);
        }

        let duration = start.elapsed();
        let total_ops = operations.load(Ordering::Relaxed);
        let total_errors = errors.load(Ordering::Relaxed);

        // Calculate throughput
        let ops_per_sec = total_ops as f64 / duration.as_secs_f64();

        println!("Concurrent detection from 10 threads:");
        println!("  Total operations: {}", total_ops);
        println!("  Total time: {:?}", duration);
        println!("  Throughput: {:.2} ops/sec", ops_per_sec);
        println!("  Errors: {}", total_errors);

        assert_eq!(total_ops, 1000, "Should complete all operations");
        assert_eq!(total_errors, 0, "Should have no errors");
        assert!(ops_per_sec > 100.0, "Throughput too low: {:.2} ops/sec", ops_per_sec);

        // Verify thread safety (no crashes/panics)
        println!("Thread safety verified - no panics or data races");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_rapid_ebpf_load_unload_cycles() {
        // Check privileges
        if unsafe { libc::geteuid() } != 0 {
            eprintln!("Skipping test: requires root privileges");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        if !os.is_ebpf_supported() {
            eprintln!("Skipping test: eBPF not supported");
            return;
        }

        let program = match std::fs::read("ebpf/xdp_counter.o") {
            Ok(p) => p,
            Err(_) => {
                eprintln!("Skipping test: eBPF program not found");
                return;
            }
        };

        let mut timings = Vec::new();
        let mut errors = 0;
        let start = Instant::now();

        for i in 0..100 {
            let cycle_start = Instant::now();

            match os.load_ebpf_program(&program) {
                Ok(handle) => {
                    // Optional attach/detach on some iterations
                    if i % 5 == 0 {
                        let _ = os.attach_ebpf_monitor(handle, EbpfAttachType::Xdp {
                            interface: "lo".to_string(),
                            flags: XdpFlags::SkbMode,
                        });
                    }

                    // Always unload
                    if os.unload_ebpf_program(handle).is_err() {
                        errors += 1;
                    }
                }
                Err(_) => {
                    errors += 1;
                }
            }

            timings.push(cycle_start.elapsed());
        }

        let duration = start.elapsed();

        println!("100 eBPF load/unload cycles:");
        println!("  Total time: {:?}", duration);
        println!("  Average: {:?}", duration / 100);
        println!("  Errors: {}", errors);

        assert!(errors < 5, "Too many errors: {}/100", errors);
        assert!(duration < Duration::from_secs(10), "Load/unload too slow");
    }

    #[test]
    fn test_high_frequency_sampling_100hz() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        let sample_rate = Duration::from_millis(10); // 100Hz
        let test_duration = Duration::from_secs(10);

        let start = Instant::now();
        let mut samples = Vec::new();
        let mut sample_times = Vec::new();

        while start.elapsed() < test_duration {
            let sample_start = Instant::now();

            match os.get_resource_usage() {
                Ok(usage) => {
                    samples.push(usage);
                    sample_times.push(sample_start.elapsed());
                }
                Err(e) => {
                    eprintln!("Sampling error: {}", e);
                }
            }

            let remaining = sample_rate.saturating_sub(sample_start.elapsed());
            if remaining > Duration::ZERO {
                thread::sleep(remaining);
            }
        }

        let actual_duration = start.elapsed();
        let sample_count = samples.len();
        let actual_rate = sample_count as f64 / actual_duration.as_secs_f64();

        // Calculate sampling jitter
        let avg_sample_time = sample_times.iter().sum::<Duration>() / sample_times.len() as u32;
        let max_sample_time = sample_times.iter().max().unwrap();

        println!("High frequency sampling at 100Hz for 10 seconds:");
        println!("  Samples collected: {}", sample_count);
        println!("  Actual rate: {:.2} Hz", actual_rate);
        println!("  Average sample time: {:?}", avg_sample_time);
        println!("  Max sample time: {:?}", max_sample_time);

        // Should achieve close to target rate
        assert!(actual_rate > 80.0, "Sample rate too low: {:.2} Hz", actual_rate);
        assert!(actual_rate < 120.0, "Sample rate too high: {:.2} Hz", actual_rate);

        // Sampling should be consistent
        assert!(max_sample_time < Duration::from_millis(50),
            "Sample time too variable: {:?}", max_sample_time);
    }

    #[test]
    fn test_memory_pressure_under_load() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Baseline memory
        let baseline = os.detect_memory().expect("Failed to get baseline memory");
        let baseline_used = baseline.used_mb;

        println!("Baseline memory: {} MB used", baseline_used);

        // Create memory pressure while monitoring
        let mut allocations = Vec::new();
        let mut memory_samples = Vec::new();

        for i in 0..20 {
            // Allocate 50MB chunks
            let chunk = vec![0xFF_u8; 50 * 1024 * 1024];
            allocations.push(chunk);

            // Sample memory
            let mem = os.detect_memory().expect("Failed to detect memory");
            memory_samples.push(mem.used_mb);

            println!("After allocation {}: {} MB used", i + 1, mem.used_mb);

            thread::sleep(Duration::from_millis(100));
        }

        // Memory should increase roughly linearly
        let expected_increase = 20 * 50; // 1000 MB
        let actual_increase = memory_samples.last().unwrap() - baseline_used;

        println!("Memory increase: {} MB (expected ~{} MB)", actual_increase, expected_increase);

        // Allow for OS overhead and other processes
        assert!(
            actual_increase >= expected_increase * 80 / 100,
            "Memory tracking not accurate: {} MB vs {} MB expected",
            actual_increase, expected_increase
        );

        // Detection should still work under memory pressure
        let final_detection = os.detect_memory();
        assert!(final_detection.is_ok(), "Memory detection failed under pressure");
    }

    #[test]
    fn test_sustained_mixed_workload() {
        let os = Arc::new(create_os_abstraction().expect("Failed to create OS abstraction"));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();

        // CPU detection thread
        let os_cpu = Arc::clone(&os);
        let stop_cpu = Arc::clone(&stop_flag);
        handles.push(thread::spawn(move || {
            let mut count = 0;
            while !stop_cpu.load(Ordering::Relaxed) {
                if os_cpu.detect_cpu().is_ok() {
                    count += 1;
                }
                thread::sleep(Duration::from_millis(50));
            }
            count
        }));

        // Memory monitoring thread
        let os_mem = Arc::clone(&os);
        let stop_mem = Arc::clone(&stop_flag);
        handles.push(thread::spawn(move || {
            let mut count = 0;
            while !stop_mem.load(Ordering::Relaxed) {
                if os_mem.detect_memory().is_ok() {
                    count += 1;
                }
                thread::sleep(Duration::from_millis(100));
            }
            count
        }));

        // Resource usage sampling thread
        let os_usage = Arc::clone(&os);
        let stop_usage = Arc::clone(&stop_flag);
        handles.push(thread::spawn(move || {
            let mut count = 0;
            while !stop_usage.load(Ordering::Relaxed) {
                if os_usage.get_resource_usage().is_ok() {
                    count += 1;
                }
                thread::sleep(Duration::from_millis(20));
            }
            count
        }));

        // Storage detection thread
        let os_storage = Arc::clone(&os);
        let stop_storage = Arc::clone(&stop_flag);
        handles.push(thread::spawn(move || {
            let mut count = 0;
            while !stop_storage.load(Ordering::Relaxed) {
                if os_storage.detect_storage().is_ok() {
                    count += 1;
                }
                thread::sleep(Duration::from_millis(200));
            }
            count
        }));

        // Run for 5 seconds
        thread::sleep(Duration::from_secs(5));
        stop_flag.store(true, Ordering::Relaxed);

        // Collect results
        let mut total_operations = 0;
        for (i, handle) in handles.into_iter().enumerate() {
            let count = handle.join().expect("Thread panicked");
            total_operations += count;
            println!("Thread {} completed {} operations", i, count);
        }

        println!("Total operations across all threads: {}", total_operations);

        // Should complete many operations without issues
        assert!(total_operations > 500, "Too few operations completed");
    }

    #[test]
    fn test_error_rate_under_load() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        let mut success_count = 0;
        let mut error_count = 0;
        let mut error_types = std::collections::HashMap::new();

        // Run 1000 mixed operations
        for i in 0..1000 {
            let result = match i % 4 {
                0 => os.detect_cpu().map(|_| ()),
                1 => os.detect_memory().map(|_| ()),
                2 => os.detect_storage().map(|_| ()),
                _ => os.get_resource_usage().map(|_| ()),
            };

            match result {
                Ok(_) => success_count += 1,
                Err(e) => {
                    error_count += 1;
                    let error_str = format!("{}", e);
                    *error_types.entry(error_str).or_insert(0) += 1;
                }
            }
        }

        let error_rate = (error_count as f64 / 1000.0) * 100.0;

        println!("Error rate under load:");
        println!("  Success: {}/1000", success_count);
        println!("  Errors: {}/1000 ({:.2}%)", error_count, error_rate);

        if !error_types.is_empty() {
            println!("  Error types:");
            for (error, count) in error_types {
                println!("    {}: {}", error, count);
            }
        }

        // Error rate should be very low
        assert!(error_rate < 1.0, "Error rate too high: {:.2}%", error_rate);
    }

    #[test]
    fn test_graceful_degradation() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Create a sliding window for performance tracking
        let window_size = 100;
        let mut timing_window = VecDeque::with_capacity(window_size);

        // Track performance over time
        for i in 0..500 {
            let start = Instant::now();
            let _ = os.get_resource_usage();
            let duration = start.elapsed();

            timing_window.push_back(duration);
            if timing_window.len() > window_size {
                timing_window.pop_front();
            }

            // Every 100 operations, check if performance is degrading
            if i > 0 && i % 100 == 0 {
                let avg: Duration = timing_window.iter().sum::<Duration>() / timing_window.len() as u32;
                println!("Operations {}-{}: avg {:?}", i - 99, i, avg);

                // Performance shouldn't degrade significantly
                assert!(
                    avg < Duration::from_millis(100),
                    "Performance degraded at operation {}: {:?}",
                    i, avg
                );
            }
        }

        println!("No performance degradation observed over 500 operations");
    }
}