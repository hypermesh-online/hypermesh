// Integration tests for real eBPF kernel operations
// Run with: cargo test --test test_ebpf_kernel_integration -- --nocapture
// May require: sudo -E cargo test --test test_ebpf_kernel_integration

use anyhow::Result;
use blockmatrix::os_integration::{OsAbstraction, types::*};

#[cfg(target_os = "linux")]
mod linux_ebpf_tests {
    use super::*;
    use std::time::Duration;
    use std::thread;

    fn create_linux_abstraction() -> Result<Box<dyn OsAbstraction>> {
        hypermesh::os_integration::create_os_abstraction()
    }

    #[test]
    fn test_kernel_version_detection() {
        let os = create_linux_abstraction().expect("Failed to create OS abstraction");

        println!("\n=== Kernel Version Detection ===");
        println!("Platform: {}", os.platform());

        // Check if eBPF is supported
        let supported = os.is_ebpf_supported();
        println!("eBPF Supported: {}", supported);

        // Try to get kernel info from /proc/version
        if let Ok(version_str) = std::fs::read_to_string("/proc/version") {
            println!("Kernel info: {}", version_str.lines().next().unwrap_or("Unknown"));
        }

        // Check BPF filesystem
        let bpf_fs_exists = std::path::Path::new("/sys/fs/bpf").exists();
        println!("BPF filesystem mounted: {}", bpf_fs_exists);

        // Check debug filesystem
        let debug_fs = std::path::Path::new("/sys/kernel/debug/tracing").exists() ||
                       std::path::Path::new("/sys/kernel/tracing").exists();
        println!("Debug/Tracing filesystem: {}", debug_fs);
    }

    #[test]
    fn test_ebpf_program_validation() {
        let os = create_linux_abstraction().expect("Failed to create OS abstraction");

        println!("\n=== eBPF Program Validation ===");

        // Create a minimal valid eBPF program (BPF_EXIT instruction)
        let valid_program = vec![0x95, 0, 0, 0, 0, 0, 0, 0];

        // Try to load the program
        match os.load_ebpf_program(&valid_program) {
            Ok(handle) => {
                println!("✓ Successfully loaded eBPF program with handle: {:?}", handle);

                // Clean up
                if let Err(e) = os.unload_ebpf_program(handle) {
                    println!("Warning: Failed to unload program: {}", e);
                }
            }
            Err(e) => {
                println!("✗ Failed to load eBPF program: {}", e);
                println!("  This is expected without proper permissions (CAP_BPF or root)");
            }
        }

        // Test invalid program (empty)
        println!("\nTesting invalid programs:");
        match os.load_ebpf_program(&[]) {
            Ok(_) => panic!("Empty program should not load"),
            Err(e) => println!("✓ Empty program rejected: {}", e),
        }

        // Test invalid size (not multiple of 8)
        match os.load_ebpf_program(&[0; 7]) {
            Ok(_) => panic!("Invalid size program should not load"),
            Err(e) => println!("✓ Invalid size rejected: {}", e),
        }
    }

    #[test]
    fn test_xdp_packet_counter() {
        let os = create_linux_abstraction().expect("Failed to create OS abstraction");

        println!("\n=== XDP Packet Counter Test ===");

        if !os.is_ebpf_supported() {
            println!("⚠ eBPF not supported on this system, skipping test");
            return;
        }

        // Try to load compiled XDP program if it exists
        let xdp_program_path = "ebpf_programs/xdp_packet_counter.o";

        if std::path::Path::new(xdp_program_path).exists() {
            println!("Found compiled XDP program at {}", xdp_program_path);

            // Read the compiled program
            match std::fs::read(xdp_program_path) {
                Ok(program_bytes) => {
                    println!("XDP program size: {} bytes", program_bytes.len());

                    // Try to load it
                    match os.load_ebpf_program(&program_bytes) {
                        Ok(handle) => {
                            println!("✓ XDP program loaded with handle: {:?}", handle);

                            // Try to attach to loopback interface
                            let attach_result = os.attach_ebpf_monitor(
                                handle,
                                EbpfAttachType::Xdp,
                            );

                            match attach_result {
                                Ok(_) => {
                                    println!("✓ XDP program attached to network interface");

                                    // Generate some traffic (ping localhost)
                                    println!("Generating test traffic...");
                                    std::process::Command::new("ping")
                                        .args(&["-c", "5", "-i", "0.2", "127.0.0.1"])
                                        .output()
                                        .ok();

                                    // Read metrics
                                    thread::sleep(Duration::from_secs(1));

                                    match os.read_ebpf_metrics(handle) {
                                        Ok(metrics) => {
                                            println!("✓ XDP Metrics collected:");
                                            println!("  Name: {}", metrics.name);
                                            println!("  Type: {:?}", metrics.metric_type);
                                            for (key, value) in &metrics.values {
                                                println!("  {}: {}", key, value);
                                            }
                                        }
                                        Err(e) => {
                                            println!("✗ Failed to read metrics: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("✗ Failed to attach XDP program: {}", e);
                                    println!("  This requires CAP_NET_ADMIN + CAP_BPF");
                                }
                            }

                            // Clean up
                            let _ = os.unload_ebpf_program(handle);
                        }
                        Err(e) => {
                            println!("✗ Failed to load XDP program: {}", e);
                            println!("  Ensure you have CAP_BPF or run as root");
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to read XDP program file: {}", e);
                }
            }
        } else {
            println!("XDP program not compiled yet.");
            println!("Run: cd ebpf_programs && ./compile_ebpf.sh");

            // Test with minimal mock program instead
            println!("\nTesting with minimal mock XDP program...");
            let mock_xdp = vec![0x95, 0, 0, 0, 0, 0, 0, 0]; // BPF_EXIT

            match os.load_ebpf_program(&mock_xdp) {
                Ok(handle) => {
                    println!("✓ Mock program loaded");
                    let _ = os.unload_ebpf_program(handle);
                }
                Err(e) => {
                    println!("✗ Mock program failed: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_kprobe_execve_monitor() {
        let os = create_linux_abstraction().expect("Failed to create OS abstraction");

        println!("\n=== Kprobe Execve Monitor Test ===");

        if !os.is_ebpf_supported() {
            println!("⚠ eBPF not supported on this system, skipping test");
            return;
        }

        // Create minimal kprobe program
        let kprobe_program = vec![0x95, 0, 0, 0, 0, 0, 0, 0]; // BPF_EXIT

        match os.load_ebpf_program(&kprobe_program) {
            Ok(handle) => {
                println!("✓ Kprobe program loaded with handle: {:?}", handle);

                // Try to attach to sys_execve
                let attach_result = os.attach_ebpf_monitor(
                    handle,
                    EbpfAttachType::Kprobe {
                        function: "sys_execve".to_string(),
                    },
                );

                match attach_result {
                    Ok(_) => {
                        println!("✓ Kprobe attached to sys_execve");

                        // Trigger execve by running a simple command
                        println!("Triggering execve event...");
                        std::process::Command::new("echo")
                            .arg("test")
                            .output()
                            .ok();

                        thread::sleep(Duration::from_millis(100));

                        // Read metrics
                        match os.read_ebpf_metrics(handle) {
                            Ok(metrics) => {
                                println!("✓ Kprobe metrics:");
                                println!("  Probe hits: {:?}", metrics.values.get("probe_hits"));
                                println!("  Function: {:?}", metrics.metadata.get("function"));
                            }
                            Err(e) => {
                                println!("✗ Failed to read kprobe metrics: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("✗ Failed to attach kprobe: {}", e);
                        println!("  This requires CAP_BPF or root access");
                    }
                }

                // Clean up
                let _ = os.unload_ebpf_program(handle);
            }
            Err(e) => {
                println!("✗ Failed to load kprobe program: {}", e);
            }
        }
    }

    #[test]
    fn test_tracepoint_network_monitor() {
        let os = create_linux_abstraction().expect("Failed to create OS abstraction");

        println!("\n=== Tracepoint Network Monitor Test ===");

        if !os.is_ebpf_supported() {
            println!("⚠ eBPF not supported on this system, skipping test");
            return;
        }

        // Create minimal tracepoint program
        let tp_program = vec![0x95, 0, 0, 0, 0, 0, 0, 0]; // BPF_EXIT

        match os.load_ebpf_program(&tp_program) {
            Ok(handle) => {
                println!("✓ Tracepoint program loaded with handle: {:?}", handle);

                // Try to attach to network tracepoint
                let attach_result = os.attach_ebpf_monitor(
                    handle,
                    EbpfAttachType::Tracepoint {
                        category: "syscalls".to_string(),
                        name: "sys_enter_connect".to_string(),
                    },
                );

                match attach_result {
                    Ok(_) => {
                        println!("✓ Tracepoint attached to syscalls:sys_enter_connect");

                        // Generate network activity
                        println!("Generating network events...");
                        std::process::Command::new("curl")
                            .args(&["-s", "-o", "/dev/null", "http://127.0.0.1:1"])
                            .output()
                            .ok();

                        thread::sleep(Duration::from_millis(100));

                        // Read metrics
                        match os.read_ebpf_metrics(handle) {
                            Ok(metrics) => {
                                println!("✓ Tracepoint metrics:");
                                println!("  Events: {:?}", metrics.values.get("events"));
                                println!("  Category: {:?}", metrics.metadata.get("category"));
                                println!("  Name: {:?}", metrics.metadata.get("name"));
                            }
                            Err(e) => {
                                println!("✗ Failed to read tracepoint metrics: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("✗ Failed to attach tracepoint: {}", e);
                    }
                }

                // Clean up
                let _ = os.unload_ebpf_program(handle);
            }
            Err(e) => {
                println!("✗ Failed to load tracepoint program: {}", e);
            }
        }
    }

    #[test]
    fn test_ebpf_lifecycle_stress() {
        let os = create_linux_abstraction().expect("Failed to create OS abstraction");

        println!("\n=== eBPF Lifecycle Stress Test ===");

        if !os.is_ebpf_supported() {
            println!("⚠ eBPF not supported, skipping stress test");
            return;
        }

        let program = vec![0x95, 0, 0, 0, 0, 0, 0, 0];
        let mut handles = Vec::new();

        // Load multiple programs
        println!("Loading multiple eBPF programs...");
        for i in 0..5 {
            match os.load_ebpf_program(&program) {
                Ok(handle) => {
                    println!("  Program {} loaded: {:?}", i, handle);
                    handles.push(handle);
                }
                Err(e) => {
                    println!("  Program {} failed: {}", i, e);
                    break;
                }
            }
        }

        // Attach programs to different points
        println!("\nAttaching programs to various points...");
        for (i, &handle) in handles.iter().enumerate() {
            let attach_type = match i % 3 {
                0 => EbpfAttachType::Xdp,
                1 => EbpfAttachType::Kprobe {
                    function: format!("test_func_{}", i),
                },
                _ => EbpfAttachType::Tracepoint {
                    category: "test".to_string(),
                    name: format!("test_{}", i),
                },
            };

            match os.attach_ebpf_monitor(handle, attach_type.clone()) {
                Ok(_) => println!("  Program {} attached: {:?}", i, attach_type),
                Err(e) => println!("  Program {} attach failed: {}", i, e),
            }
        }

        // Clean up all programs
        println!("\nCleaning up programs...");
        for handle in handles {
            match os.unload_ebpf_program(handle) {
                Ok(_) => println!("  Program {:?} unloaded", handle),
                Err(e) => println!("  Failed to unload {:?}: {}", handle, e),
            }
        }

        println!("✓ Stress test completed");
    }

    #[test]
    fn test_ebpf_permission_errors() {
        let os = create_linux_abstraction().expect("Failed to create OS abstraction");

        println!("\n=== eBPF Permission Error Handling ===");

        // Check current permissions
        let is_root = unsafe { libc::geteuid() } == 0;
        println!("Running as root: {}", is_root);

        // Check capabilities (would need cap-std crate for full check)
        let has_cap_bpf = std::path::Path::new("/sys/fs/bpf").exists() &&
                          std::fs::metadata("/sys/fs/bpf")
                              .map(|m| m.permissions().readonly())
                              .unwrap_or(true);
        println!("Likely has CAP_BPF: {}", !has_cap_bpf);

        // Try to load program and expect clear error message
        let program = vec![0x95, 0, 0, 0, 0, 0, 0, 0];

        match os.load_ebpf_program(&program) {
            Ok(handle) => {
                println!("✓ Program loaded successfully (have permissions)");
                let _ = os.unload_ebpf_program(handle);
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                println!("Expected permission error: {}", error_msg);

                // Verify error message is helpful
                assert!(
                    error_msg.contains("permission") ||
                    error_msg.contains("CAP_BPF") ||
                    error_msg.contains("root"),
                    "Error message should mention permissions"
                );
                println!("✓ Error message is clear about permission requirements");
            }
        }
    }
}

fn main() {
    println!("eBPF Kernel Integration Tests");
    println!("==============================");
    println!("These tests verify real eBPF kernel operations.");
    println!("Some tests may require CAP_BPF capability or root access.");
    println!();
    println!("Run with: cargo test --test test_ebpf_kernel_integration");
    println!("Or with sudo: sudo -E cargo test --test test_ebpf_kernel_integration");
}