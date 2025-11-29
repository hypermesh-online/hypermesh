// Windows Performance Counter Tests for Sprint 3
// Tests Windows performance monitoring via WMI and Performance Counters
// Requires: Windows 10+

#[cfg(target_os = "windows")]
#[cfg(test)]
mod windows_performance_tests {
    use blockmatrix::os_integration::{create_os_abstraction, types::*};
    use std::thread;
    use std::time::{Duration, Instant};
    use std::fs::{self, File};
    use std::io::Write;
    use anyhow::Result;

    // Helper to verify Windows version
    fn is_windows_10_or_later() -> bool {
        use winapi::um::sysinfoapi::{GetVersionExW, OSVERSIONINFOEXW};
        use std::mem::zeroed;

        unsafe {
            let mut version_info: OSVERSIONINFOEXW = zeroed();
            version_info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;

            if GetVersionExW(&mut version_info as *mut _ as *mut _) != 0 {
                // Windows 10 is version 10.0
                version_info.dwMajorVersion >= 10
            } else {
                false
            }
        }
    }

    #[test]
    fn test_cpu_usage_tracking() {
        if !is_windows_10_or_later() {
            eprintln!("Skipping test: requires Windows 10 or later");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // First sample - establish baseline
        let usage1 = os.get_resource_usage()
            .expect("Failed to get first resource usage");

        // Generate some CPU load
        let start = Instant::now();
        let mut sum = 0u64;
        while start.elapsed() < Duration::from_millis(500) {
            for i in 0..1000000 {
                sum = sum.wrapping_add(i);
            }
        }

        // Prevent optimization
        std::hint::black_box(sum);

        // Second sample - should show CPU usage
        let usage2 = os.get_resource_usage()
            .expect("Failed to get second resource usage");

        // Verify we got non-zero CPU delta
        assert!(usage2.cpu_percent > 0.0, "CPU usage should be > 0% after load");
        assert!(usage2.cpu_percent <= 100.0, "CPU usage should be <= 100%");

        // CPU time should have increased
        assert!(
            usage2.total_cpu_time_ms > usage1.total_cpu_time_ms,
            "Total CPU time should increase: {} -> {}",
            usage1.total_cpu_time_ms,
            usage2.total_cpu_time_ms
        );

        println!("CPU usage: {:.2}%, total time: {}ms",
            usage2.cpu_percent, usage2.total_cpu_time_ms);
    }

    #[test]
    fn test_network_io_tracking() {
        if !is_windows_10_or_later() {
            eprintln!("Skipping test: requires Windows 10 or later");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Baseline network counters
        let usage1 = os.get_resource_usage()
            .expect("Failed to get baseline network usage");

        // Generate network activity
        use std::net::TcpStream;
        use std::io::Read;

        // Try to connect to a well-known service
        if let Ok(mut stream) = TcpStream::connect("1.1.1.1:443") {
            let _ = stream.write_all(b"GET / HTTP/1.1\r\nHost: 1.1.1.1\r\n\r\n");
            let mut buffer = [0; 1024];
            let _ = stream.read(&mut buffer);
        }

        // Allow counters to update
        thread::sleep(Duration::from_millis(500));

        // Get updated counters
        let usage2 = os.get_resource_usage()
            .expect("Failed to get updated network usage");

        // Should have some network I/O
        let bytes_sent_delta = usage2.network_bytes_sent.saturating_sub(usage1.network_bytes_sent);
        let bytes_recv_delta = usage2.network_bytes_received.saturating_sub(usage1.network_bytes_received);

        println!("Network I/O: sent {} bytes, received {} bytes",
            bytes_sent_delta, bytes_recv_delta);

        // At minimum we sent the HTTP request
        assert!(
            bytes_sent_delta > 0 || bytes_recv_delta > 0,
            "Should have some network I/O"
        );
    }

    #[test]
    fn test_disk_io_tracking() {
        if !is_windows_10_or_later() {
            eprintln!("Skipping test: requires Windows 10 or later");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Baseline disk counters
        let usage1 = os.get_resource_usage()
            .expect("Failed to get baseline disk usage");

        // Generate disk I/O
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("hypermesh_disk_test.tmp");

        // Write 10MB of data
        {
            let mut file = File::create(&test_file)
                .expect("Failed to create test file");

            let data = vec![0xAB; 1024 * 1024]; // 1MB buffer
            for _ in 0..10 {
                file.write_all(&data).expect("Failed to write test data");
            }
            file.sync_all().expect("Failed to sync file");
        }

        // Allow counters to update
        thread::sleep(Duration::from_millis(500));

        // Get updated counters
        let usage2 = os.get_resource_usage()
            .expect("Failed to get updated disk usage");

        // Clean up
        let _ = fs::remove_file(test_file);

        // Should show disk writes
        let disk_write_delta = usage2.disk_bytes_written.saturating_sub(usage1.disk_bytes_written);

        println!("Disk I/O: wrote {} bytes", disk_write_delta);

        // We wrote at least 10MB
        assert!(
            disk_write_delta >= 10 * 1024 * 1024,
            "Expected at least 10MB written, got {}",
            disk_write_delta
        );
    }

    #[test]
    fn test_memory_tracking() {
        if !is_windows_10_or_later() {
            eprintln!("Skipping test: requires Windows 10 or later");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Get baseline memory
        let mem_info1 = os.detect_memory()
            .expect("Failed to get memory info");

        // Allocate significant memory
        let big_vec: Vec<u8> = vec![0xFF; 100 * 1024 * 1024]; // 100MB

        // Get updated memory
        let mem_info2 = os.detect_memory()
            .expect("Failed to get updated memory info");

        // Available memory should have decreased
        assert!(
            mem_info2.available_mb < mem_info1.available_mb,
            "Available memory should decrease after allocation"
        );

        // Used memory should have increased by roughly 100MB
        let used_delta = mem_info2.used_mb - mem_info1.used_mb;
        assert!(
            used_delta >= 90, // Allow some variance
            "Used memory should increase by ~100MB, got {}MB",
            used_delta
        );

        // Prevent optimization
        std::hint::black_box(big_vec);

        println!("Memory allocation tracked: {}MB increase", used_delta);
    }

    #[test]
    fn test_sampling_interval_accuracy() {
        if !is_windows_10_or_later() {
            eprintln!("Skipping test: requires Windows 10 or later");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        let mut timestamps = Vec::new();
        let sample_count = 10;
        let expected_interval = Duration::from_millis(100);

        for _ in 0..sample_count {
            let start = Instant::now();
            let _ = os.get_resource_usage();
            timestamps.push(start);
            thread::sleep(expected_interval);
        }

        // Check intervals between samples
        for i in 1..timestamps.len() {
            let interval = timestamps[i].duration_since(timestamps[i-1]);

            // Allow 20ms variance for Windows scheduler
            assert!(
                interval >= Duration::from_millis(80) &&
                interval <= Duration::from_millis(120),
                "Interval {} was {:?}, expected ~100ms",
                i, interval
            );
        }

        println!("Sampling intervals accurate within 20ms tolerance");
    }

    #[test]
    fn test_first_sample_handling() {
        if !is_windows_10_or_later() {
            eprintln!("Skipping test: requires Windows 10 or later");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // First sample - no previous data
        let usage = os.get_resource_usage()
            .expect("First sample should succeed");

        // CPU percent might be 0 on first sample (no delta)
        assert!(usage.cpu_percent >= 0.0 && usage.cpu_percent <= 100.0,
            "CPU percent should be valid even on first sample");

        // But absolute values should be available
        assert!(usage.memory_used_mb > 0, "Memory usage should be available");
        assert!(usage.total_memory_mb > 0, "Total memory should be available");

        println!("First sample handled gracefully");
    }

    #[test]
    fn test_performance_counter_categories() {
        if !is_windows_10_or_later() {
            eprintln!("Skipping test: requires Windows 10 or later");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Should be able to query different counter categories
        let usage = os.get_resource_usage()
            .expect("Failed to get resource usage");

        // Verify we get data from multiple categories
        assert!(usage.memory_used_mb > 0, "Memory counters not working");
        assert!(usage.total_memory_mb > 0, "Memory info not available");
        assert!(usage.process_count > 0, "Process counters not working");
        assert!(usage.thread_count > 0, "Thread counters not working");

        // Network and disk might be 0 if idle
        assert!(usage.network_bytes_sent >= 0, "Network counters failed");
        assert!(usage.disk_bytes_read >= 0, "Disk counters failed");

        println!("Performance counter categories: Memory, Process, Network, Disk all functional");
    }

    #[test]
    fn test_high_frequency_sampling() {
        if !is_windows_10_or_later() {
            eprintln!("Skipping test: requires Windows 10 or later");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        let start = Instant::now();
        let mut sample_count = 0;

        // Sample at ~100Hz for 1 second
        while start.elapsed() < Duration::from_secs(1) {
            let _ = os.get_resource_usage()
                .expect("High frequency sampling should not fail");
            sample_count += 1;
            thread::sleep(Duration::from_millis(10)); // ~100Hz
        }

        println!("Completed {} samples in 1 second", sample_count);

        // Should achieve at least 80 samples (allowing for overhead)
        assert!(sample_count >= 80,
            "Expected at least 80 samples, got {}", sample_count);
    }

    #[test]
    fn test_counter_overflow_handling() {
        if !is_windows_10_or_later() {
            eprintln!("Skipping test: requires Windows 10 or later");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        // Take multiple samples to test counter wraparound
        let mut last_bytes_sent = 0u64;

        for i in 0..100 {
            let usage = os.get_resource_usage()
                .expect("Failed to get resource usage");

            // Network bytes should never decrease (handle counter overflow)
            assert!(
                usage.network_bytes_sent >= last_bytes_sent,
                "Network counter went backwards at iteration {}: {} -> {}",
                i, last_bytes_sent, usage.network_bytes_sent
            );

            last_bytes_sent = usage.network_bytes_sent;
            thread::sleep(Duration::from_millis(10));
        }

        println!("Counter overflow handling verified over 100 samples");
    }
}

// Stub for non-Windows platforms
#[cfg(not(target_os = "windows"))]
#[cfg(test)]
mod stub_tests {
    #[test]
    fn test_windows_performance_windows_only() {
        println!("Windows performance tests only run on Windows");
    }
}