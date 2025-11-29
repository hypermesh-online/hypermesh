// Cross-Platform Performance Validation Tests for Sprint 3
// Benchmarks and validates performance requirements across all platforms

#[cfg(test)]
mod performance_validation_tests {
    use blockmatrix::os_integration::{create_os_abstraction, types::*};
    use std::time::{Duration, Instant};
    use std::thread;
    use std::sync::{Arc, Mutex};
    use criterion::{black_box, Criterion};

    // Performance requirements
    const MAX_FULL_PROFILE_MS: u64 = 500;
    const MAX_RESOURCE_SAMPLE_MS: u64 = 50;
    const MAX_EBPF_READ_MS: u64 = 10;
    const MAX_MEMORY_USAGE_MB: usize = 100;

    #[test]
    fn benchmark_full_system_profile() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        let start = Instant::now();

        // Full system profile: all detection methods
        let cpu_info = os.detect_cpu().expect("CPU detection failed");
        let gpu_info = os.detect_gpu().expect("GPU detection failed");
        let memory_info = os.detect_memory().expect("Memory detection failed");
        let storage_info = os.detect_storage().expect("Storage detection failed");
        let resource_usage = os.get_resource_usage().expect("Resource usage failed");

        // On supported platforms, include eBPF
        #[cfg(target_os = "linux")]
        {
            if os.is_ebpf_supported() {
                if let Ok(program) = std::fs::read("ebpf/xdp_counter.o") {
                    if let Ok(handle) = os.load_ebpf_program(&program) {
                        let _ = os.read_ebpf_metrics(handle);
                        let _ = os.unload_ebpf_program(handle);
                    }
                }
            }
        }

        let duration = start.elapsed();

        println!("Full system profile completed in {:?}", duration);
        println!("  CPU: {} cores @ {} MHz", cpu_info.core_count, cpu_info.frequency_mhz);
        println!("  GPU: {} devices detected", gpu_info.len());
        println!("  Memory: {} MB total", memory_info.total_mb);
        println!("  Storage: {} devices", storage_info.len());

        assert!(
            duration.as_millis() < MAX_FULL_PROFILE_MS as u128,
            "Full profile took {:?}, max allowed: {}ms",
            duration, MAX_FULL_PROFILE_MS
        );
    }

    #[test]
    fn benchmark_resource_usage_sampling() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        let mut timings = Vec::new();

        // Take 100 samples
        for _ in 0..100 {
            let start = Instant::now();
            let _ = os.get_resource_usage().expect("Resource sampling failed");
            let duration = start.elapsed();
            timings.push(duration);
        }

        // Calculate statistics
        let total: Duration = timings.iter().sum();
        let avg = total / timings.len() as u32;
        let max = timings.iter().max().unwrap();
        let min = timings.iter().min().unwrap();

        println!("Resource usage sampling (100 samples):");
        println!("  Average: {:?}", avg);
        println!("  Min: {:?}", min);
        println!("  Max: {:?}", max);

        assert!(
            avg.as_millis() < MAX_RESOURCE_SAMPLE_MS as u128,
            "Average sampling time {:?} exceeds {}ms",
            avg, MAX_RESOURCE_SAMPLE_MS
        );

        assert!(
            max.as_millis() < (MAX_RESOURCE_SAMPLE_MS * 2) as u128,
            "Max sampling time {:?} exceeds {}ms",
            max, MAX_RESOURCE_SAMPLE_MS * 2
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn benchmark_ebpf_metric_read() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        if !os.is_ebpf_supported() {
            println!("eBPF not supported, skipping benchmark");
            return;
        }

        // Check privileges
        if unsafe { libc::geteuid() } != 0 {
            println!("eBPF benchmark requires root privileges, skipping");
            return;
        }

        let program = match std::fs::read("ebpf/xdp_counter.o") {
            Ok(p) => p,
            Err(_) => {
                println!("eBPF program not found, skipping benchmark");
                return;
            }
        };

        let handle = os.load_ebpf_program(&program)
            .expect("Failed to load eBPF program");

        let mut timings = Vec::new();

        // Benchmark metric reads
        for _ in 0..100 {
            let start = Instant::now();
            let _ = os.read_ebpf_metrics(handle);
            let duration = start.elapsed();
            timings.push(duration);
        }

        let avg = timings.iter().sum::<Duration>() / timings.len() as u32;
        let max = timings.iter().max().unwrap();

        println!("eBPF metric read (100 samples):");
        println!("  Average: {:?}", avg);
        println!("  Max: {:?}", max);

        assert!(
            avg.as_millis() < MAX_EBPF_READ_MS as u128,
            "Average eBPF read {:?} exceeds {}ms",
            avg, MAX_EBPF_READ_MS
        );

        os.unload_ebpf_program(handle).expect("Failed to unload eBPF");
    }

    #[test]
    fn test_memory_usage_during_profiling() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Get baseline memory
        let baseline_mem = get_process_memory_mb();
        println!("Baseline memory: {} MB", baseline_mem);

        // Run intensive profiling
        for _ in 0..10 {
            let _ = os.detect_cpu();
            let _ = os.detect_gpu();
            let _ = os.detect_memory();
            let _ = os.detect_storage();
            let _ = os.get_resource_usage();
        }

        // Check peak memory
        let peak_mem = get_process_memory_mb();
        let memory_increase = peak_mem - baseline_mem;

        println!("Peak memory: {} MB (increase: {} MB)", peak_mem, memory_increase);

        assert!(
            memory_increase < MAX_MEMORY_USAGE_MB,
            "Memory usage increased by {} MB, max allowed: {} MB",
            memory_increase, MAX_MEMORY_USAGE_MB
        );
    }

    #[test]
    fn test_memory_leak_detection() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Get baseline
        let baseline_mem = get_process_memory_mb();

        // Run many iterations
        for iteration in 0..100 {
            let _ = os.detect_cpu();
            let _ = os.detect_memory();
            let _ = os.get_resource_usage();

            // Check memory every 10 iterations
            if iteration % 10 == 9 {
                let current_mem = get_process_memory_mb();
                let increase = current_mem.saturating_sub(baseline_mem);

                // Memory shouldn't grow unbounded
                assert!(
                    increase < 50,
                    "Memory leak detected: {} MB increase after {} iterations",
                    increase, iteration + 1
                );
            }
        }

        let final_mem = get_process_memory_mb();
        println!("Memory after 100 iterations: {} MB (baseline: {} MB)",
            final_mem, baseline_mem);
    }

    #[test]
    fn test_concurrent_access_performance() {
        let os = Arc::new(create_os_abstraction().expect("Failed to create OS abstraction"));
        let mut handles = Vec::new();

        let start = Instant::now();

        // Spawn 10 threads doing concurrent operations
        for thread_id in 0..10 {
            let os_clone = Arc::clone(&os);
            let handle = thread::spawn(move || {
                let mut local_timings = Vec::new();

                for _ in 0..10 {
                    let op_start = Instant::now();
                    let _ = os_clone.get_resource_usage();
                    local_timings.push(op_start.elapsed());
                }

                local_timings
            });
            handles.push(handle);
        }

        // Wait for all threads
        let mut all_timings = Vec::new();
        for handle in handles {
            let timings = handle.join().expect("Thread panicked");
            all_timings.extend(timings);
        }

        let duration = start.elapsed();

        // Calculate statistics
        let avg = all_timings.iter().sum::<Duration>() / all_timings.len() as u32;
        let max = all_timings.iter().max().unwrap();

        println!("Concurrent access (10 threads, 100 total ops):");
        println!("  Total time: {:?}", duration);
        println!("  Average op: {:?}", avg);
        println!("  Max op: {:?}", max);

        // Even under contention, should meet requirements
        assert!(
            avg.as_millis() < (MAX_RESOURCE_SAMPLE_MS * 2) as u128,
            "Concurrent ops too slow: {:?}",
            avg
        );
    }

    #[test]
    fn test_detection_accuracy() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Run detection multiple times
        let mut cpu_results = Vec::new();
        let mut memory_results = Vec::new();

        for _ in 0..5 {
            let cpu = os.detect_cpu().expect("CPU detection failed");
            let mem = os.detect_memory().expect("Memory detection failed");

            cpu_results.push(cpu.core_count);
            memory_results.push(mem.total_mb);

            thread::sleep(Duration::from_millis(100));
        }

        // Results should be consistent
        let first_cpu = cpu_results[0];
        let first_mem = memory_results[0];

        for (i, &cpu_cores) in cpu_results.iter().enumerate() {
            assert_eq!(
                cpu_cores, first_cpu,
                "Inconsistent CPU core count at iteration {}: {} vs {}",
                i, cpu_cores, first_cpu
            );
        }

        // Memory might vary slightly but should be close
        for (i, &mem_mb) in memory_results.iter().enumerate() {
            let diff = (mem_mb as i64 - first_mem as i64).abs();
            assert!(
                diff < 100,
                "Memory detection varied too much at iteration {}: {} vs {}",
                i, mem_mb, first_mem
            );
        }

        println!("Detection consistency verified across 5 iterations");
    }

    #[test]
    fn test_error_recovery_performance() {
        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Measure performance when operations fail
        let start = Instant::now();

        // Try operations that might fail gracefully
        #[cfg(target_os = "linux")]
        {
            // Try to load non-existent eBPF program
            let _ = os.load_ebpf_program(&[0xFF, 0xFF]);
        }

        #[cfg(target_os = "windows")]
        {
            // Windows operations generally shouldn't fail
            let _ = os.get_resource_usage();
        }

        // Even with errors, should be fast
        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 100,
            "Error recovery took too long: {:?}",
            duration
        );
    }

    // Helper function to get process memory usage
    fn get_process_memory_mb() -> usize {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            let status = fs::read_to_string("/proc/self/status")
                .unwrap_or_default();

            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<usize>() {
                            return kb / 1024;
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use winapi::um::processthreadsapi::GetCurrentProcess;
            use winapi::um::psapi::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
            use std::mem;

            unsafe {
                let mut counters: PROCESS_MEMORY_COUNTERS = mem::zeroed();
                counters.cb = mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

                if GetProcessMemoryInfo(
                    GetCurrentProcess(),
                    &mut counters as *mut _,
                    counters.cb
                ) != 0 {
                    return (counters.WorkingSetSize / (1024 * 1024)) as usize;
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let output = Command::new("ps")
                .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
                .output()
                .unwrap_or_default();

            if let Ok(text) = String::from_utf8(output.stdout) {
                if let Ok(kb) = text.trim().parse::<usize>() {
                    return kb / 1024;
                }
            }
        }

        // Fallback
        0
    }
}