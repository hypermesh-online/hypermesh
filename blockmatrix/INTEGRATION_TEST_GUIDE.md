# HyperMesh Integration Test Guide

## Overview

This guide covers running and interpreting the comprehensive integration test suite for HyperMesh's OS abstraction layer, including platform-specific features like eBPF (Linux) and Performance Counters (Windows).

## Test Organization

```
tests/
├── test_ebpf_integration.rs      # Linux eBPF kernel integration
├── test_windows_performance.rs   # Windows Performance Counters
├── test_performance_validation.rs # Cross-platform performance benchmarks
├── test_multi_node.rs            # Multi-node cluster simulation
├── test_load.rs                  # Load and stress testing
└── os_integration_test.rs        # Core OS abstraction + asset adapters
```

## Platform Requirements

### Linux
- **Kernel**: 4.4+ (eBPF support)
- **Privileges**: CAP_BPF or root for eBPF tests
- **Dependencies**: libbpf-dev, clang (for eBPF compilation)

### Windows
- **Version**: Windows 10+ (Performance Counter APIs)
- **Privileges**: Administrator for some counters
- **Dependencies**: Windows SDK

### macOS
- **Version**: 10.14+ (Mojave or later)
- **Privileges**: Standard user (elevated for some features)
- **Dependencies**: Xcode Command Line Tools

### BSD
- **Version**: FreeBSD 12+, OpenBSD 6.8+
- **Privileges**: Standard user
- **Dependencies**: Base system

## Running Tests

### Quick Start

```bash
# Run all integration tests
cargo test --tests

# Run with output
cargo test --tests -- --nocapture

# Run specific test suite
cargo test --test test_multi_node

# Run single test
cargo test --test test_load test_1000_consecutive_cpu_detections
```

### Platform-Specific Tests

#### Linux eBPF Tests
```bash
# Requires elevated privileges
sudo cargo test --test test_ebpf_integration -- --nocapture

# Or with capabilities
sudo setcap cap_bpf+ep ./target/debug/deps/test_ebpf_integration-*
cargo test --test test_ebpf_integration -- --nocapture

# Individual eBPF tests
cargo test test_xdp_program_lifecycle
cargo test test_kprobe_syscall_monitoring
cargo test test_multiple_programs_simultaneous
```

#### Windows Performance Tests
```powershell
# Run as Administrator for full access
cargo test --test test_windows_performance -- --nocapture

# Individual Windows tests
cargo test test_cpu_usage_tracking
cargo test test_network_io_tracking
cargo test test_disk_io_tracking
```

### Performance Benchmarks

```bash
# Full performance validation suite
cargo test --test test_performance_validation -- --nocapture

# Specific benchmarks
cargo test benchmark_full_system_profile
cargo test benchmark_resource_usage_sampling
cargo test benchmark_ebpf_metric_read  # Linux only
```

### Load Testing

```bash
# Run load tests sequentially (recommended)
cargo test --test test_load -- --nocapture --test-threads=1

# Individual load tests
cargo test test_1000_consecutive_cpu_detections
cargo test test_concurrent_detection_from_10_threads
cargo test test_high_frequency_sampling_100hz
```

### Multi-Node Tests

```bash
# Multi-node simulation
cargo test --test test_multi_node -- --nocapture

# Specific scenarios
cargo test test_parallel_metric_collection
cargo test test_node_failure_handling
cargo test test_rolling_node_updates
```

## Test Configuration

### Environment Variables

```bash
# Skip privileged tests
export SKIP_PRIVILEGED_TESTS=1

# Set test timeout (seconds)
export TEST_TIMEOUT=300

# Enable debug output
export RUST_LOG=debug

# Specify test data directory
export TEST_DATA_DIR=/path/to/test/data
```

### Cargo.toml Test Settings

```toml
[profile.test]
opt-level = 2          # Optimize for better performance testing
debug = true           # Keep debug symbols
overflow-checks = true # Catch arithmetic overflows

[[test]]
name = "test_ebpf_integration"
required-features = ["ebpf"]
```

## Interpreting Results

### Success Indicators

✅ **Green Test Output**
```
test test_cpu_detection_returns_valid_data ... ok
test test_memory_detection_never_panics ... ok
```

✅ **Performance Within Targets**
```
Full system profile completed in 142ms (target: <500ms) ✅
Resource sampling: 4.2ms average (target: <50ms) ✅
```

✅ **No Memory Leaks**
```
Memory after 100 iterations: 65MB (baseline: 42MB)
No memory leak detected ✅
```

### Warning Indicators

⚠️ **Skipped Tests**
```
Skipping test: requires CAP_BPF or root privileges
Skipping test: eBPF not supported on this system
```

⚠️ **Performance Degradation**
```
Performance degradation: 15.3% (threshold: 20%)
P99 latency: 95ms (approaching 100ms limit)
```

### Failure Indicators

❌ **Test Failures**
```
thread 'test_name' panicked at 'assertion failed: ...'
test test_name ... FAILED
```

❌ **Performance Violations**
```
Full profile took 650ms, max allowed: 500ms
Memory leak detected: 150MB increase after 100 iterations
```

## Common Issues & Solutions

### Issue: eBPF Tests Fail with Permission Denied

**Solution**:
```bash
# Option 1: Run with sudo
sudo cargo test --test test_ebpf_integration

# Option 2: Add capabilities
sudo setcap cap_bpf+ep ./target/debug/deps/test_ebpf_integration-*

# Option 3: Check kernel support
uname -r  # Should be 4.4+
ls /sys/fs/bpf  # Should exist
```

### Issue: Windows Tests Fail with Access Denied

**Solution**:
```powershell
# Run PowerShell as Administrator
# Or adjust user permissions for Performance Counters
perfmon.exe  # Configure permissions
```

### Issue: Tests Timeout

**Solution**:
```bash
# Increase timeout
cargo test -- --test-threads=1 --timeout=600

# Run specific test in isolation
cargo test test_specific_name -- --exact
```

### Issue: Inconsistent Performance Results

**Solution**:
```bash
# Ensure consistent environment
# 1. Close other applications
# 2. Disable CPU frequency scaling
sudo cpupower frequency-set -g performance

# 3. Run multiple iterations
for i in {1..5}; do
    cargo test benchmark_full_system_profile
done
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libbpf-dev clang
      - name: Run tests
        run: |
          cargo test --tests -- --nocapture
          sudo cargo test --test test_ebpf_integration

  test-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
      - name: Run tests
        run: cargo test --tests -- --nocapture

  test-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
      - name: Run tests
        run: cargo test --tests -- --nocapture
```

### GitLab CI

```yaml
stages:
  - test

test:linux:
  stage: test
  image: rust:latest
  script:
    - apt-get update && apt-get install -y libbpf-dev clang
    - cargo test --tests -- --nocapture
    - cargo test --test test_ebpf_integration || true  # Allow failure

test:windows:
  stage: test
  tags:
    - windows
  script:
    - cargo test --tests -- --nocapture
```

## Performance Baselines

### Expected Performance by Platform

| Operation | Linux | Windows | macOS | BSD |
|-----------|-------|---------|-------|-----|
| CPU Detection | <10ms | <50ms | <10ms | <20ms |
| Memory Detection | <5ms | <10ms | <5ms | <10ms |
| Storage Detection | <50ms | <100ms | <50ms | <75ms |
| Resource Sampling | <10ms | <20ms | <10ms | <15ms |
| Full Profile | <200ms | <300ms | <150ms | <250ms |

### Load Test Targets

| Test | Target | Acceptable | Failure |
|------|--------|------------|---------|
| 1000 Operations | <10s | <15s | >20s |
| 10-Thread Concurrent | >100 ops/s | >50 ops/s | <50 ops/s |
| 100Hz Sampling | >80 samples/s | >60 samples/s | <60 samples/s |
| Memory Growth | <50MB | <100MB | >100MB |
| Error Rate | <0.1% | <1% | >1% |

## Advanced Testing

### Custom Test Scenarios

```rust
// Create custom integration test
#[test]
fn test_custom_scenario() {
    let os = create_os_abstraction().unwrap();

    // Your test logic
    let cpu = os.detect_cpu().unwrap();
    assert!(cpu.cores > 0);

    // Benchmark
    let start = Instant::now();
    for _ in 0..100 {
        os.get_resource_usage().unwrap();
    }
    let duration = start.elapsed();

    assert!(duration < Duration::from_secs(1));
}
```

### Stress Testing

```bash
# Run extended stress test
STRESS_TEST_DURATION=3600 cargo test test_sustained_mixed_workload

# Memory stress test
MEMORY_STRESS_GB=8 cargo test test_memory_pressure_under_load

# CPU stress test
CPU_STRESS_THREADS=16 cargo test test_concurrent_detection
```

### Integration with External Systems

```bash
# Test with monitoring stack
docker-compose up -d prometheus grafana
cargo test --features monitoring

# Test with real eBPF programs
sudo cargo test --test test_ebpf_integration --features production-ebpf
```

## Debugging Failed Tests

### Enable Debug Logging

```bash
RUST_LOG=debug cargo test failing_test_name -- --nocapture
```

### Run with GDB

```bash
cargo test --no-run
gdb target/debug/deps/test_name-*
(gdb) run --test-threads=1
```

### Valgrind Memory Check (Linux)

```bash
cargo test --no-run
valgrind --leak-check=full --track-origins=yes \
    target/debug/deps/test_name-* --test-threads=1
```

### Performance Profiling

```bash
# Using perf (Linux)
perf record cargo test benchmark_test_name
perf report

# Using Instruments (macOS)
instruments -t "Time Profiler" cargo test benchmark_test_name
```

## Test Maintenance

### Adding New Tests

1. Create test file in `tests/` directory
2. Add platform-specific compilation guards
3. Document requirements and expected behavior
4. Update this guide with new test information

### Updating Baselines

```bash
# Re-run baseline tests
cargo test --test test_performance_validation -- --nocapture > baselines.txt

# Update expected values in tests
# Edit test files with new baseline values
```

### Test Coverage

```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# View report
open coverage/index.html
```

## Support

For test-related issues:
1. Check this guide for common solutions
2. Review test output carefully for error messages
3. Ensure platform requirements are met
4. File issues with test logs and system information

Remember: Integration tests validate real system behavior. Some variation in performance results is normal across different hardware and system loads.