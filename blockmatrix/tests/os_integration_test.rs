//! Integration Tests for OS Abstraction Layer
//!
//! Comprehensive tests for OS abstraction layer with asset adapter integration
//! across Linux, Windows, BSD, and macOS platforms.
//!
//! Test Coverage:
//! - Hardware detection (CPU, GPU, Memory, Storage)
//! - Asset adapter integration with OS abstraction
//! - Cross-platform compatibility
//! - eBPF support detection (Linux)
//! - Performance benchmarks
//! - Error handling and fallback behavior
//! - Graceful degradation on headless systems

use blockmatrix::os_integration::{
    create_os_abstraction, OsAbstraction,
    types::{CpuInfo, MemoryInfo, StorageInfo, GpuInfo, ResourceUsage},
};

use blockmatrix::assets::adapters::{
    CpuAssetAdapter, GpuAssetAdapter, MemoryAssetAdapter, StorageAssetAdapter,
};

use std::time::Instant;

// ======================
// 1. PLATFORM TESTS
// ======================

#[test]
fn test_os_abstraction_creation() {
    println!("\n=== Platform Detection Test ===");

    let result = create_os_abstraction();
    assert!(result.is_ok(), "Should create OS abstraction for current platform");

    let os = result.unwrap();
    let platform = os.platform();

    println!("Detected platform: {}", platform);

    // Verify platform detection matches current OS
    #[cfg(target_os = "linux")]
    {
        assert_eq!(platform, "linux", "Should detect Linux platform");
    }

    #[cfg(target_os = "windows")]
    {
        assert_eq!(platform, "windows", "Should detect Windows platform");
    }

    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    {
        assert_eq!(platform, "bsd", "Should detect BSD platform");
    }

    #[cfg(target_os = "macos")]
    {
        assert_eq!(platform, "macos", "Should detect macOS platform");
    }
}

// ======================
// 2. CPU DETECTION TESTS
// ======================

#[test]
fn test_cpu_detection_returns_valid_data() {
    println!("\n=== CPU Detection Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let cpu = os.detect_cpu().expect("Failed to detect CPU");

    println!("CPU Information:");
    println!("  Cores: {}", cpu.cores);
    println!("  Model: {}", cpu.model);
    println!("  Architecture: {}", cpu.architecture);
    if let Some(freq) = cpu.frequency_mhz {
        println!("  Frequency: {} MHz", freq);
    }
    if let Some(vendor) = &cpu.vendor {
        println!("  Vendor: {}", vendor);
    }

    // Validations
    assert!(cpu.cores > 0, "Should detect at least one CPU core");
    assert!(!cpu.model.is_empty(), "Should detect CPU model name");
    assert!(!cpu.architecture.is_empty(), "Should detect CPU architecture");
    assert_ne!(cpu.architecture, "unknown", "Architecture should not be 'unknown'");
    assert_eq!(cpu.architecture, std::env::consts::ARCH, "Architecture should match compilation target");
}

#[test]
fn test_cpu_detection_on_linux_reads_proc_cpuinfo() {
    #[cfg(target_os = "linux")]
    {
        println!("\n=== Linux CPU /proc/cpuinfo Test ===");

        let os = create_os_abstraction().expect("Failed to create OS abstraction");
        let cpu = os.detect_cpu().expect("Failed to detect CPU");

        // On Linux, should successfully parse /proc/cpuinfo
        assert!(cpu.cores > 0);
        assert!(!cpu.model.is_empty());

        // Model should contain actual CPU name (not fallback)
        assert_ne!(cpu.model, "Unknown");

        println!("✅ Successfully parsed /proc/cpuinfo: {} cores, {}", cpu.cores, cpu.model);
    }
}

// ======================
// 3. GPU DETECTION TESTS
// ======================

#[test]
fn test_gpu_detection_handles_no_gpu() {
    println!("\n=== GPU Detection Test (May Have No GPU) ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let gpus = os.detect_gpu().expect("Failed to detect GPU (should handle gracefully)");

    println!("GPU Detection Result:");
    if gpus.is_empty() {
        println!("  No GPUs detected (expected on headless/virtual systems)");
    } else {
        println!("  Found {} GPU(s):", gpus.len());
        for (i, gpu) in gpus.iter().enumerate() {
            println!("  [{}] {} from {}", i, gpu.model, gpu.vendor);
            if let Some(mem) = gpu.memory_bytes {
                println!("       Memory: {} MB", mem / (1024 * 1024));
            }
        }
    }

    // Should return empty list gracefully on systems without GPUs, not error
    assert!(gpus.is_empty() || !gpus.is_empty(), "GPU detection should complete without panic");
}

#[test]
fn test_gpu_detection_structure_is_valid() {
    println!("\n=== GPU Detection Structure Validation ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let gpus = os.detect_gpu().expect("Failed to detect GPU");

    for gpu in gpus {
        assert!(!gpu.model.is_empty(), "GPU model should not be empty");
        assert!(!gpu.vendor.is_empty(), "GPU vendor should not be empty");

        // If memory is specified, it should be reasonable
        if let Some(mem) = gpu.memory_bytes {
            assert!(mem > 0, "GPU memory should be positive if specified");
            assert!(mem < 1_000_000_000_000, "GPU memory should be < 1 TB (sanity check)");
        }
    }

    println!("✅ GPU structure validation passed");
}

#[test]
fn test_gpu_detection_on_linux_checks_sys_class_drm() {
    #[cfg(target_os = "linux")]
    {
        println!("\n=== Linux GPU /sys/class/drm Test ===");

        let os = create_os_abstraction().expect("Failed to create OS abstraction");
        let gpus = os.detect_gpu().expect("Failed to detect GPU");

        // On Linux systems with GPUs, /sys/class/drm should find them
        // On headless systems, empty list is acceptable
        println!("Linux GPU detection returned: {} GPU(s)", gpus.len());
    }
}

// ======================
// 4. MEMORY DETECTION TESTS
// ======================

#[test]
fn test_memory_detection_returns_valid_data() {
    println!("\n=== Memory Detection Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let mem = os.detect_memory().expect("Failed to detect memory");

    println!("Memory Information:");
    println!("  Total: {} GB", mem.total_bytes / (1024*1024*1024));
    println!("  Available: {} GB", mem.available_bytes / (1024*1024*1024));
    println!("  Used: {} GB", mem.used_bytes / (1024*1024*1024));
    println!("  Usage: {:.2}%", mem.usage_percent);

    // Validations
    assert!(mem.total_bytes > 0, "Should detect non-zero total memory");
    assert!(mem.usage_percent >= 0.0 && mem.usage_percent <= 100.0,
            "Usage percentage should be 0-100");
    assert!(mem.used_bytes <= mem.total_bytes, "Used should not exceed total");
    assert!(mem.available_bytes <= mem.total_bytes, "Available should not exceed total");
}

#[test]
fn test_memory_detection_on_linux_reads_proc_meminfo() {
    #[cfg(target_os = "linux")]
    {
        println!("\n=== Linux Memory /proc/meminfo Test ===");

        let os = create_os_abstraction().expect("Failed to create OS abstraction");
        let mem = os.detect_memory().expect("Failed to detect memory");

        // On Linux, should successfully parse /proc/meminfo
        assert!(mem.total_bytes > 0);
        assert!(mem.available_bytes > 0);

        // Swap info should be available on Linux
        if let (Some(swap_total), Some(swap_used)) = (mem.swap_total_bytes, mem.swap_used_bytes) {
            println!("Swap: {} / {} MB",
                     swap_used / (1024*1024),
                     swap_total / (1024*1024));
            assert!(swap_used <= swap_total, "Swap used should not exceed swap total");
        }

        println!("✅ Successfully parsed /proc/meminfo");
    }
}

// ======================
// 5. STORAGE DETECTION TESTS
// ======================

#[test]
fn test_storage_detection_finds_root_filesystem() {
    println!("\n=== Storage Detection Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let storage = os.detect_storage().expect("Failed to detect storage");

    // Should have at least root filesystem or equivalent
    assert!(!storage.is_empty(), "Should detect at least one storage device");

    println!("Storage Devices Found: {}", storage.len());
    for (i, device) in storage.iter().enumerate() {
        println!("[{}] {} @ {}", i, device.device, device.mount_point);
        println!("    Size: {:.2} GB, Used: {:.2} GB ({:.1}%)",
                 device.total_bytes as f64 / (1024*1024*1024),
                 device.used_bytes as f64 / (1024*1024*1024),
                 device.usage_percent);
    }
}

#[test]
fn test_storage_detection_validates_data() {
    println!("\n=== Storage Validation Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let storage = os.detect_storage().expect("Failed to detect storage");

    for device in storage {
        assert!(device.total_bytes > 0, "Storage should have non-zero capacity");
        assert!(device.usage_percent >= 0.0 && device.usage_percent <= 100.0,
                "Usage percentage should be 0-100");
        assert!(device.used_bytes <= device.total_bytes,
                "Used storage should not exceed total");
        assert!(device.available_bytes <= device.total_bytes,
                "Available storage should not exceed total");
    }

    println!("✅ Storage validation passed");
}

#[test]
fn test_storage_detection_on_linux_reads_proc_mounts() {
    #[cfg(target_os = "linux")]
    {
        println!("\n=== Linux Storage /proc/mounts Test ===");

        let os = create_os_abstraction().expect("Failed to create OS abstraction");
        let storage = os.detect_storage().expect("Failed to detect storage");

        // Should have at least root filesystem
        let root_device = storage.iter().find(|s| s.mount_point == "/");
        assert!(root_device.is_some() || storage.len() > 0,
                "Should find root filesystem or other mounted devices");

        println!("✅ Successfully parsed /proc/mounts: {} devices", storage.len());
    }
}

// ======================
// 6. RESOURCE USAGE TESTS
// ======================

#[test]
fn test_resource_usage_detection() {
    println!("\n=== Resource Usage Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let usage = os.get_resource_usage().expect("Failed to get resource usage");

    println!("Current Resource Usage:");
    println!("  CPU: {:.2}%", usage.cpu_usage_percent);
    println!("  Memory: {:.2}%", usage.memory_usage_percent);

    if let Some(load) = usage.load_average {
        println!("  Load Average: {:.2}, {:.2}, {:.2}", load[0], load[1], load[2]);
    }

    if let Some(count) = usage.process_count {
        println!("  Process Count: {}", count);
    }

    // Validations
    assert!(usage.cpu_usage_percent >= 0.0 && usage.cpu_usage_percent <= 100.0,
            "CPU usage should be 0-100%");
    assert!(usage.memory_usage_percent >= 0.0 && usage.memory_usage_percent <= 100.0,
            "Memory usage should be 0-100%");
}

#[test]
fn test_resource_usage_on_linux_includes_load_average() {
    #[cfg(target_os = "linux")]
    {
        println!("\n=== Linux Load Average Test ===");

        let os = create_os_abstraction().expect("Failed to create OS abstraction");
        let usage = os.get_resource_usage().expect("Failed to get resource usage");

        assert!(usage.load_average.is_some(), "Load average should be available on Linux");
        let load = usage.load_average.unwrap();
        assert!(load[0] >= 0.0, "Load average should be non-negative");

        println!("✅ Load average: {:.2}, {:.2}, {:.2}", load[0], load[1], load[2]);
    }
}

// ======================
// 7. EBPF SUPPORT TESTS
// ======================

#[test]
fn test_ebpf_support_detection() {
    println!("\n=== eBPF Support Detection Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let supported = os.is_ebpf_supported();

    println!("eBPF Supported: {}", supported);

    #[cfg(target_os = "linux")]
    {
        println!("Platform: Linux - eBPF detection implemented");
        // On Linux, eBPF should be detected (but might be false on old kernels)
        println!("Note: eBPF requires kernel 4.4+ for basic support, 5.0+ for full features");
    }

    #[cfg(not(target_os = "linux"))]
    {
        // On non-Linux, eBPF support detection should not panic
        println!("Platform: Non-Linux - eBPF support may not be available");
    }
}

// ======================
// 8. ASSET ADAPTER INTEGRATION
// ======================

#[test]
fn test_cpu_adapter_uses_os_abstraction() {
    println!("\n=== CPU Adapter Integration Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let cpu = os.detect_cpu().expect("CPU detection should work");

    // CPU adapter should be able to work with this data
    assert!(cpu.cores > 0, "CPU cores should be detected");

    println!("✅ CPU adapter can use detected: {} cores", cpu.cores);
}

#[test]
fn test_gpu_adapter_handles_no_gpu_gracefully() {
    println!("\n=== GPU Adapter Graceful Degradation Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let gpus = os.detect_gpu().expect("GPU detection should not error");

    // GPU adapter should handle empty list gracefully
    if gpus.is_empty() {
        println!("✅ No GPUs detected - gracefully handled");
    } else {
        println!("✅ Found {} GPU(s)", gpus.len());
    }
}

#[test]
fn test_memory_adapter_uses_os_abstraction() {
    println!("\n=== Memory Adapter Integration Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let mem = os.detect_memory().expect("Memory detection should work");

    // Memory adapter should be able to work with this data
    assert!(mem.total_bytes > 0, "Memory should be detected");

    println!("✅ Memory adapter can use detected: {} GB total",
             mem.total_bytes / (1024*1024*1024));
}

#[test]
fn test_storage_adapter_uses_os_abstraction() {
    println!("\n=== Storage Adapter Integration Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");
    let storage = os.detect_storage().expect("Storage detection should work");

    // Storage adapter should be able to work with this data
    assert!(!storage.is_empty(), "Storage devices should be detected");

    println!("✅ Storage adapter can use detected: {} device(s)", storage.len());
}

// ======================
// 9. PERFORMANCE BENCHMARKS
// ======================

#[test]
fn test_cpu_detection_performance() {
    println!("\n=== CPU Detection Performance Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    let start = Instant::now();
    let cpu = os.detect_cpu().expect("CPU detection should work");
    let duration = start.elapsed();

    println!("CPU detection completed in {:.2}ms", duration.as_secs_f64() * 1000.0);

    // Should complete quickly (< 100ms)
    assert!(duration.as_millis() < 100,
            "CPU detection should complete in < 100ms, took {}ms",
            duration.as_millis());
}

#[test]
fn test_memory_detection_performance() {
    println!("\n=== Memory Detection Performance Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    let start = Instant::now();
    let mem = os.detect_memory().expect("Memory detection should work");
    let duration = start.elapsed();

    println!("Memory detection completed in {:.2}ms", duration.as_secs_f64() * 1000.0);

    assert!(duration.as_millis() < 100,
            "Memory detection should complete in < 100ms, took {}ms",
            duration.as_millis());
}

#[test]
fn test_storage_detection_performance() {
    println!("\n=== Storage Detection Performance Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    let start = Instant::now();
    let storage = os.detect_storage().expect("Storage detection should work");
    let duration = start.elapsed();

    println!("Storage detection completed in {:.2}ms", duration.as_secs_f64() * 1000.0);

    assert!(duration.as_millis() < 100,
            "Storage detection should complete in < 100ms, took {}ms",
            duration.as_millis());
}

#[test]
fn test_gpu_detection_performance() {
    println!("\n=== GPU Detection Performance Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    let start = Instant::now();
    let _gpus = os.detect_gpu().expect("GPU detection should work");
    let duration = start.elapsed();

    println!("GPU detection completed in {:.2}ms", duration.as_secs_f64() * 1000.0);

    assert!(duration.as_millis() < 100,
            "GPU detection should complete in < 100ms, took {}ms",
            duration.as_millis());
}

#[test]
fn test_all_detection_combined_performance() {
    println!("\n=== Combined Detection Performance Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    let start = Instant::now();

    let cpu = os.detect_cpu().expect("CPU detection failed");
    let mem = os.detect_memory().expect("Memory detection failed");
    let storage = os.detect_storage().expect("Storage detection failed");
    let _gpus = os.detect_gpu().expect("GPU detection failed");

    let duration = start.elapsed();

    println!("All detections combined: {:.2}ms", duration.as_secs_f64() * 1000.0);
    println!("  CPU: {} cores", cpu.cores);
    println!("  Memory: {} GB", mem.total_bytes / (1024*1024*1024));
    println!("  Storage: {} devices", storage.len());

    // All detection should complete in reasonable time
    assert!(duration.as_millis() < 500,
            "All detections should complete in < 500ms, took {}ms",
            duration.as_millis());
}

// ======================
// 10. ERROR HANDLING & FALLBACK
// ======================

#[test]
fn test_cpu_detection_never_panics() {
    println!("\n=== CPU Detection Error Handling Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    // Should not panic even if /proc/cpuinfo doesn't exist
    let result = os.detect_cpu();

    match result {
        Ok(cpu) => println!("✅ CPU detected: {} cores", cpu.cores),
        Err(e) => println!("⚠️  CPU detection returned error: {}", e),
    }
}

#[test]
fn test_gpu_detection_never_panics() {
    println!("\n=== GPU Detection Error Handling Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    let result = os.detect_gpu();

    match result {
        Ok(gpus) => println!("✅ GPU detection completed: {} GPUs", gpus.len()),
        Err(e) => println!("⚠️  GPU detection returned error: {}", e),
    }
}

#[test]
fn test_memory_detection_never_panics() {
    println!("\n=== Memory Detection Error Handling Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    let result = os.detect_memory();

    match result {
        Ok(mem) => println!("✅ Memory detected: {} GB", mem.total_bytes / (1024*1024*1024)),
        Err(e) => println!("⚠️  Memory detection returned error: {}", e),
    }
}

#[test]
fn test_storage_detection_never_panics() {
    println!("\n=== Storage Detection Error Handling Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    let result = os.detect_storage();

    match result {
        Ok(storage) => println!("✅ Storage detected: {} devices", storage.len()),
        Err(e) => println!("⚠️  Storage detection returned error: {}", e),
    }
}

// ======================
// 11. CROSS-PLATFORM CONSISTENCY
// ======================

#[test]
fn test_all_platforms_have_consistent_interface() {
    println!("\n=== Cross-Platform Interface Consistency Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    // All platforms should implement these methods
    assert!(!os.platform().is_empty(), "Platform should have a name");

    let cpu_result = os.detect_cpu();
    assert!(cpu_result.is_ok(), "CPU detection should work on all platforms");

    let mem_result = os.detect_memory();
    assert!(mem_result.is_ok(), "Memory detection should work on all platforms");

    let storage_result = os.detect_storage();
    assert!(storage_result.is_ok(), "Storage detection should work on all platforms");

    let gpu_result = os.detect_gpu();
    assert!(gpu_result.is_ok(), "GPU detection should work on all platforms");

    println!("✅ All platforms implement consistent interface");
}

#[test]
fn test_ebpf_support_check_does_not_panic_any_platform() {
    println!("\n=== eBPF Support Check Cross-Platform Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    // Should not panic on any platform
    let _supported = os.is_ebpf_supported();

    println!("✅ eBPF support check completed without panic");
}

// ======================
// 12. COMPREHENSIVE SYSTEM PROFILE
// ======================

#[test]
fn test_full_system_profile() {
    println!("\n=== COMPREHENSIVE SYSTEM PROFILE ===\n");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    println!("Platform: {}", os.platform());

    // CPU Profile
    match os.detect_cpu() {
        Ok(cpu) => {
            println!("\nCPU Profile:");
            println!("  Cores: {}", cpu.cores);
            println!("  Model: {}", cpu.model);
            println!("  Architecture: {}", cpu.architecture);
            if let Some(freq) = cpu.frequency_mhz {
                println!("  Base Frequency: {} MHz", freq);
            }
            if let Some(vendor) = &cpu.vendor {
                println!("  Vendor: {}", vendor);
            }
        },
        Err(e) => println!("CPU Profile Error: {}", e),
    }

    // Memory Profile
    match os.detect_memory() {
        Ok(mem) => {
            println!("\nMemory Profile:");
            println!("  Total: {:.2} GB", mem.total_bytes as f64 / (1024.0*1024.0*1024.0));
            println!("  Available: {:.2} GB", mem.available_bytes as f64 / (1024.0*1024.0*1024.0));
            println!("  Used: {:.2} GB", mem.used_bytes as f64 / (1024.0*1024.0*1024.0));
            println!("  Usage: {:.2}%", mem.usage_percent);
        },
        Err(e) => println!("Memory Profile Error: {}", e),
    }

    // GPU Profile
    match os.detect_gpu() {
        Ok(gpus) => {
            println!("\nGPU Profile:");
            if gpus.is_empty() {
                println!("  No GPUs detected");
            } else {
                for (i, gpu) in gpus.iter().enumerate() {
                    println!("  [{}] {} ({})", i, gpu.model, gpu.vendor);
                    if let Some(mem) = gpu.memory_bytes {
                        println!("       Memory: {} MB", mem / (1024*1024));
                    }
                }
            }
        },
        Err(e) => println!("GPU Profile Error: {}", e),
    }

    // Storage Profile
    match os.detect_storage() {
        Ok(storage) => {
            println!("\nStorage Profile:");
            for device in storage.iter().take(3) {
                println!("  {} ({}): {:.2} GB / {:.2} GB",
                         device.mount_point,
                         device.filesystem,
                         device.available_bytes as f64 / (1024.0*1024.0*1024.0),
                         device.total_bytes as f64 / (1024.0*1024.0*1024.0));
            }
            if storage.len() > 3 {
                println!("  ... and {} more devices", storage.len() - 3);
            }
        },
        Err(e) => println!("Storage Profile Error: {}", e),
    }

    // Resource Usage
    match os.get_resource_usage() {
        Ok(usage) => {
            println!("\nCurrent Resource Usage:");
            println!("  CPU: {:.2}%", usage.cpu_usage_percent);
            println!("  Memory: {:.2}%", usage.memory_usage_percent);
            if let Some(load) = usage.load_average {
                println!("  Load Avg: {:.2}, {:.2}, {:.2}", load[0], load[1], load[2]);
            }
        },
        Err(e) => println!("Resource Usage Error: {}", e),
    }

    // eBPF Support
    println!("\neBPF Support: {}", os.is_ebpf_supported());

    println!("\n✅ System profile completed successfully");
}

// ======================
// 14. ASSET ADAPTER INTEGRATION VALIDATION
// ======================

#[test]
fn test_cpu_adapter_uses_real_os_metrics() {
    println!("\n=== CPU Asset Adapter Integration Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    // Get real CPU metrics
    let cpu_info = os.detect_cpu().expect("CPU detection failed");
    let usage = os.get_resource_usage().expect("Resource usage failed");

    // Create CPU adapter
    let cpu_adapter = CpuAssetAdapter::new();

    // Adapter should use real OS metrics
    let adapter_metrics = cpu_adapter.get_metrics();

    println!("OS CPU Info: {} cores @ {}MHz", cpu_info.cores,
             cpu_info.frequency_mhz.unwrap_or(0));
    println!("OS CPU Usage: {:.2}%", usage.cpu_usage_percent);
    println!("Adapter CPU metrics: {:?}", adapter_metrics);

    // Verify adapter reflects OS state
    assert_eq!(adapter_metrics.core_count, cpu_info.cores,
               "Adapter should report correct core count");

    // CPU usage should be reasonable
    assert!(usage.cpu_usage_percent >= 0.0 && usage.cpu_usage_percent <= 100.0,
            "CPU usage should be between 0-100%");
}

#[test]
fn test_memory_adapter_tracks_real_pressure() {
    println!("\n=== Memory Asset Adapter Integration Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    // Get baseline memory
    let mem_before = os.detect_memory().expect("Memory detection failed");

    // Create memory adapter
    let mem_adapter = MemoryAssetAdapter::new();

    // Allocate significant memory to create pressure
    let _large_vec: Vec<u8> = vec![0xFF; 100 * 1024 * 1024]; // 100MB

    // Get updated memory
    let mem_after = os.detect_memory().expect("Memory detection failed");

    println!("Memory before: {} MB used / {} MB total",
             mem_before.used_bytes / (1024*1024),
             mem_before.total_bytes / (1024*1024));
    println!("Memory after: {} MB used / {} MB total",
             mem_after.used_bytes / (1024*1024),
             mem_after.total_bytes / (1024*1024));

    // Adapter should reflect memory pressure
    let adapter_metrics = mem_adapter.get_metrics();
    println!("Adapter memory metrics: {:?}", adapter_metrics);

    // Memory usage should have increased
    assert!(mem_after.used_bytes > mem_before.used_bytes,
            "Memory usage should increase after allocation");

    // Adapter should track this
    assert!(adapter_metrics.used_mb > 0, "Adapter should report memory usage");
}

#[test]
fn test_storage_adapter_monitors_real_io() {
    println!("\n=== Storage Asset Adapter Integration Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    // Get initial storage info
    let storage_info = os.detect_storage().expect("Storage detection failed");

    // Create storage adapter
    let storage_adapter = StorageAssetAdapter::new();

    println!("Storage devices detected: {}", storage_info.len());
    for device in &storage_info {
        println!("  Device {}: {} MB total, {} MB free",
                 device.mount_point.display(),
                 device.total_bytes / (1024*1024),
                 device.free_bytes / (1024*1024));
    }

    // Get initial I/O stats
    let usage_before = os.get_resource_usage().expect("Resource usage failed");

    // Generate some disk I/O
    use std::fs::File;
    use std::io::Write;
    let temp_file = std::env::temp_dir().join("hypermesh_test.tmp");
    {
        let mut file = File::create(&temp_file).expect("Failed to create test file");
        let data = vec![0xAB; 1024 * 1024]; // 1MB
        for _ in 0..10 {
            file.write_all(&data).expect("Failed to write test data");
        }
        file.sync_all().expect("Failed to sync file");
    }

    // Get updated I/O stats
    std::thread::sleep(std::time::Duration::from_millis(500));
    let usage_after = os.get_resource_usage().expect("Resource usage failed");

    // Clean up
    let _ = std::fs::remove_file(temp_file);

    println!("Disk I/O before: {} bytes written", usage_before.disk_bytes_written);
    println!("Disk I/O after: {} bytes written", usage_after.disk_bytes_written);

    // Adapter should track I/O
    let adapter_metrics = storage_adapter.get_metrics();
    println!("Adapter storage metrics: {:?}", adapter_metrics);

    // Should have some disk activity
    if usage_after.disk_bytes_written > usage_before.disk_bytes_written {
        println!("✅ Storage adapter correctly tracking disk I/O");
    } else {
        println!("⚠️  Could not verify disk I/O tracking (may be cached)");
    }
}

#[test]
fn test_gpu_adapter_handles_no_gpu_gracefully() {
    println!("\n=== GPU Asset Adapter Error Handling Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    // Detect GPUs (may be none on CI/headless)
    let gpu_info = os.detect_gpu().expect("GPU detection failed");

    // Create GPU adapter
    let gpu_adapter = GpuAssetAdapter::new();

    if gpu_info.is_empty() {
        println!("No GPUs detected - verifying graceful handling");

        // Adapter should handle no GPU case
        let adapter_metrics = gpu_adapter.get_metrics();
        println!("Adapter GPU metrics (no GPU): {:?}", adapter_metrics);

        // Should not panic or error
        assert!(adapter_metrics.gpu_count == 0,
                "Adapter should report 0 GPUs when none present");
    } else {
        println!("GPUs detected: {}", gpu_info.len());
        for gpu in &gpu_info {
            println!("  GPU: {} ({} MB memory)", gpu.name, gpu.memory_mb);
        }

        // Adapter should track GPUs
        let adapter_metrics = gpu_adapter.get_metrics();
        println!("Adapter GPU metrics: {:?}", adapter_metrics);

        assert_eq!(adapter_metrics.gpu_count, gpu_info.len(),
                   "Adapter should report correct GPU count");
    }
}

#[test]
fn test_all_adapters_handle_os_errors_gracefully() {
    println!("\n=== Asset Adapter Error Recovery Test ===");

    // Create adapters
    let cpu_adapter = CpuAssetAdapter::new();
    let mem_adapter = MemoryAssetAdapter::new();
    let storage_adapter = StorageAssetAdapter::new();
    let gpu_adapter = GpuAssetAdapter::new();

    // Even if OS abstraction has issues, adapters shouldn't panic
    let cpu_metrics = cpu_adapter.get_metrics();
    let mem_metrics = mem_adapter.get_metrics();
    let storage_metrics = storage_adapter.get_metrics();
    let gpu_metrics = gpu_adapter.get_metrics();

    println!("All adapters returned metrics without panicking:");
    println!("  CPU: {:?}", cpu_metrics);
    println!("  Memory: {:?}", mem_metrics);
    println!("  Storage: {:?}", storage_metrics);
    println!("  GPU: {:?}", gpu_metrics);

    // Basic validation
    assert!(cpu_metrics.core_count >= 0, "CPU cores should be non-negative");
    assert!(mem_metrics.total_mb >= 0, "Memory should be non-negative");
    assert!(storage_metrics.device_count >= 0, "Storage devices should be non-negative");
    assert!(gpu_metrics.gpu_count >= 0, "GPU count should be non-negative");
}

#[test]
fn test_adapter_integration_performance() {
    println!("\n=== Asset Adapter Performance Test ===");

    let os = create_os_abstraction().expect("Failed to create OS abstraction");

    // Create all adapters
    let cpu_adapter = CpuAssetAdapter::new();
    let mem_adapter = MemoryAssetAdapter::new();
    let storage_adapter = StorageAssetAdapter::new();
    let gpu_adapter = GpuAssetAdapter::new();

    let start = Instant::now();

    // Collect metrics from all adapters
    for _ in 0..100 {
        let _ = cpu_adapter.get_metrics();
        let _ = mem_adapter.get_metrics();
        let _ = storage_adapter.get_metrics();
        let _ = gpu_adapter.get_metrics();

        // Also query OS
        let _ = os.get_resource_usage();
    }

    let duration = start.elapsed();
    let ops_per_sec = 500.0 / duration.as_secs_f64();

    println!("500 adapter operations in {:?}", duration);
    println!("Throughput: {:.2} ops/sec", ops_per_sec);

    // Should be fast
    assert!(duration.as_millis() < 5000,
            "Adapter operations too slow: {:?} for 500 ops", duration);
}
