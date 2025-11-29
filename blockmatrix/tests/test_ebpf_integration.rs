// eBPF Integration Tests for Sprint 3
// Tests Linux eBPF kernel integration with XDP, kprobe, and map operations
// Requires: Linux kernel 4.4+, CAP_BPF or root privileges

#[cfg(target_os = "linux")]
#[cfg(test)]
mod ebpf_integration_tests {
    use blockmatrix::os_integration::{create_os_abstraction, types::*};
    use std::thread;
    use std::time::{Duration, Instant};
    use std::fs;
    use std::process::Command;
    use anyhow::Result;

    // Helper to check if we have required privileges
    fn has_ebpf_privileges() -> bool {
        // Check for CAP_BPF capability or root
        let output = Command::new("capsh")
            .args(&["--print"])
            .output()
            .ok();

        if let Some(output) = output {
            let caps = String::from_utf8_lossy(&output.stdout);
            if caps.contains("cap_bpf") || caps.contains("cap_sys_admin") {
                return true;
            }
        }

        // Check if running as root
        unsafe { libc::geteuid() == 0 }
    }

    #[test]
    fn test_xdp_program_lifecycle() {
        if !has_ebpf_privileges() {
            eprintln!("Skipping test: requires CAP_BPF or root privileges");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        if !os.is_ebpf_supported() {
            eprintln!("Skipping test: eBPF not supported on this system");
            return;
        }

        // Simple XDP program that counts packets
        let xdp_program = include_bytes!("../ebpf/xdp_counter.o");

        // Load the program
        let handle = os.load_ebpf_program(xdp_program)
            .expect("Failed to load XDP program");

        // Attach to loopback interface
        let attach_result = os.attach_ebpf_monitor(handle, EbpfAttachType::Xdp {
            interface: "lo".to_string(),
            flags: XdpFlags::SkbMode
        });

        assert!(attach_result.is_ok(), "Failed to attach XDP program: {:?}", attach_result);

        // Generate some network traffic on loopback
        Command::new("ping")
            .args(&["-c", "5", "-i", "0.1", "127.0.0.1"])
            .spawn()
            .expect("Failed to generate test traffic");

        thread::sleep(Duration::from_secs(1));

        // Read packet counters
        let metrics = os.read_ebpf_metrics(handle)
            .expect("Failed to read eBPF metrics");

        // Verify we captured packets
        assert!(metrics.packet_count > 0, "No packets captured by XDP");
        assert!(metrics.bytes_processed > 0, "No bytes processed by XDP");

        // Detach and unload
        let unload_result = os.unload_ebpf_program(handle);
        assert!(unload_result.is_ok(), "Failed to unload eBPF program");
    }

    #[test]
    fn test_kprobe_syscall_monitoring() {
        if !has_ebpf_privileges() {
            eprintln!("Skipping test: requires CAP_BPF or root privileges");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        if !os.is_ebpf_supported() {
            eprintln!("Skipping test: eBPF not supported on this system");
            return;
        }

        // Kprobe program that monitors open() syscalls
        let kprobe_program = include_bytes!("../ebpf/kprobe_open.o");

        // Load the program
        let handle = os.load_ebpf_program(kprobe_program)
            .expect("Failed to load kprobe program");

        // Attach to sys_open
        let attach_result = os.attach_ebpf_monitor(handle, EbpfAttachType::Kprobe {
            function: "__x64_sys_open".to_string()
        });

        assert!(attach_result.is_ok(), "Failed to attach kprobe: {:?}", attach_result);

        // Trigger some open() syscalls
        for i in 0..10 {
            let _ = fs::File::open("/etc/passwd");
            thread::sleep(Duration::from_millis(10));
        }

        // Read syscall counters
        let metrics = os.read_ebpf_metrics(handle)
            .expect("Failed to read kprobe metrics");

        // Verify we captured syscalls
        assert!(metrics.syscall_count >= 10, "Expected at least 10 syscalls, got {}", metrics.syscall_count);

        // Cleanup
        os.unload_ebpf_program(handle).expect("Failed to unload kprobe");
    }

    #[test]
    fn test_multiple_programs_simultaneous() {
        if !has_ebpf_privileges() {
            eprintln!("Skipping test: requires CAP_BPF or root privileges");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        if !os.is_ebpf_supported() {
            eprintln!("Skipping test: eBPF not supported on this system");
            return;
        }

        let xdp_program = include_bytes!("../ebpf/xdp_counter.o");
        let kprobe_program = include_bytes!("../ebpf/kprobe_open.o");

        // Load multiple programs
        let xdp_handle = os.load_ebpf_program(xdp_program)
            .expect("Failed to load XDP program");
        let kprobe_handle = os.load_ebpf_program(kprobe_program)
            .expect("Failed to load kprobe program");

        // Attach both
        os.attach_ebpf_monitor(xdp_handle, EbpfAttachType::Xdp {
            interface: "lo".to_string(),
            flags: XdpFlags::SkbMode
        }).expect("Failed to attach XDP");

        os.attach_ebpf_monitor(kprobe_handle, EbpfAttachType::Kprobe {
            function: "__x64_sys_open".to_string()
        }).expect("Failed to attach kprobe");

        // Generate mixed activity
        thread::spawn(|| {
            Command::new("ping")
                .args(&["-c", "10", "-i", "0.1", "127.0.0.1"])
                .spawn();
        });

        for _ in 0..5 {
            let _ = fs::File::open("/etc/hosts");
            thread::sleep(Duration::from_millis(50));
        }

        thread::sleep(Duration::from_secs(1));

        // Read metrics from both
        let xdp_metrics = os.read_ebpf_metrics(xdp_handle)
            .expect("Failed to read XDP metrics");
        let kprobe_metrics = os.read_ebpf_metrics(kprobe_handle)
            .expect("Failed to read kprobe metrics");

        assert!(xdp_metrics.packet_count > 0, "XDP didn't capture packets");
        assert!(kprobe_metrics.syscall_count > 0, "Kprobe didn't capture syscalls");

        // Cleanup both
        os.unload_ebpf_program(xdp_handle).expect("Failed to unload XDP");
        os.unload_ebpf_program(kprobe_handle).expect("Failed to unload kprobe");
    }

    #[test]
    fn test_ebpf_map_operations() {
        if !has_ebpf_privileges() {
            eprintln!("Skipping test: requires CAP_BPF or root privileges");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        if !os.is_ebpf_supported() {
            eprintln!("Skipping test: eBPF not supported on this system");
            return;
        }

        // Program with hash map for per-CPU stats
        let map_program = include_bytes!("../ebpf/percpu_stats.o");

        let handle = os.load_ebpf_program(map_program)
            .expect("Failed to load program with maps");

        // Initial read should have zeros
        let initial_metrics = os.read_ebpf_metrics(handle)
            .expect("Failed to read initial metrics");

        assert_eq!(initial_metrics.cpu_stats.len(), 0, "Expected empty initial CPU stats");

        // Attach to trigger updates
        os.attach_ebpf_monitor(handle, EbpfAttachType::Tracepoint {
            category: "sched".to_string(),
            name: "sched_switch".to_string()
        }).expect("Failed to attach to tracepoint");

        // Let it collect some data
        thread::sleep(Duration::from_millis(500));

        // Read updated metrics
        let updated_metrics = os.read_ebpf_metrics(handle)
            .expect("Failed to read updated metrics");

        // Should have per-CPU data now
        assert!(!updated_metrics.cpu_stats.is_empty(), "No per-CPU stats collected");

        // Verify reasonable values
        for (cpu, stats) in &updated_metrics.cpu_stats {
            assert!(*cpu < 256, "Invalid CPU ID: {}", cpu);
            assert!(stats.context_switches > 0, "No context switches on CPU {}", cpu);
        }

        os.unload_ebpf_program(handle).expect("Failed to unload program");
    }

    #[test]
    fn test_missing_privileges_graceful_error() {
        // Temporarily drop privileges if we have them
        let original_uid = unsafe { libc::geteuid() };

        if original_uid == 0 {
            // Switch to nobody user temporarily
            unsafe { libc::seteuid(65534) };
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        let dummy_program = vec![0u8; 100]; // Invalid program
        let load_result = os.load_ebpf_program(&dummy_program);

        // Should fail gracefully, not panic
        assert!(load_result.is_err(), "Expected error when loading without privileges");

        if let Err(e) = load_result {
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("permission") || error_msg.contains("CAP_BPF"),
                "Error should mention permissions: {}",
                error_msg
            );
        }

        // Restore original privileges
        if original_uid == 0 {
            unsafe { libc::seteuid(original_uid) };
        }
    }

    #[test]
    fn test_invalid_program_validation() {
        if !has_ebpf_privileges() {
            eprintln!("Skipping test: requires CAP_BPF or root privileges");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        if !os.is_ebpf_supported() {
            eprintln!("Skipping test: eBPF not supported on this system");
            return;
        }

        // Try to load invalid bytecode
        let invalid_program = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Not valid eBPF
        let load_result = os.load_ebpf_program(&invalid_program);

        assert!(load_result.is_err(), "Should fail to load invalid program");

        if let Err(e) = load_result {
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("invalid") || error_msg.contains("verifier"),
                "Error should mention validation failure: {}",
                error_msg
            );
        }
    }

    #[test]
    fn test_interface_not_found_error() {
        if !has_ebpf_privileges() {
            eprintln!("Skipping test: requires CAP_BPF or root privileges");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        if !os.is_ebpf_supported() {
            eprintln!("Skipping test: eBPF not supported on this system");
            return;
        }

        let xdp_program = include_bytes!("../ebpf/xdp_counter.o");
        let handle = os.load_ebpf_program(xdp_program)
            .expect("Failed to load XDP program");

        // Try to attach to non-existent interface
        let attach_result = os.attach_ebpf_monitor(handle, EbpfAttachType::Xdp {
            interface: "nonexistent999".to_string(),
            flags: XdpFlags::SkbMode
        });

        assert!(attach_result.is_err(), "Should fail to attach to non-existent interface");

        if let Err(e) = attach_result {
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("interface") || error_msg.contains("not found"),
                "Error should mention interface issue: {}",
                error_msg
            );
        }

        os.unload_ebpf_program(handle).expect("Failed to cleanup");
    }

    #[test]
    fn test_rapid_load_unload_cycles() {
        if !has_ebpf_privileges() {
            eprintln!("Skipping test: requires CAP_BPF or root privileges");
            return;
        }

        let os = create_os_abstraction().expect("Failed to create OS abstraction");

        if !os.is_ebpf_supported() {
            eprintln!("Skipping test: eBPF not supported on this system");
            return;
        }

        let xdp_program = include_bytes!("../ebpf/xdp_counter.o");
        let start = Instant::now();

        // Rapid load/unload cycles
        for i in 0..100 {
            let handle = os.load_ebpf_program(xdp_program)
                .expect(&format!("Failed to load program on iteration {}", i));

            // Optional: attach and detach
            if i % 10 == 0 {
                let _ = os.attach_ebpf_monitor(handle, EbpfAttachType::Xdp {
                    interface: "lo".to_string(),
                    flags: XdpFlags::SkbMode
                });
            }

            os.unload_ebpf_program(handle)
                .expect(&format!("Failed to unload program on iteration {}", i));
        }

        let duration = start.elapsed();
        println!("100 load/unload cycles completed in {:?}", duration);

        // Should complete reasonably fast (under 10 seconds)
        assert!(duration.as_secs() < 10, "Load/unload cycles took too long");
    }
}

// Export types for Windows test compilation
#[cfg(not(target_os = "linux"))]
#[cfg(test)]
mod stub_tests {
    #[test]
    fn test_ebpf_integration_linux_only() {
        println!("eBPF integration tests only run on Linux");
    }
}