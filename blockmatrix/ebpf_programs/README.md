# eBPF Programs for HyperMesh

This directory contains pre-built eBPF programs for HyperMesh's kernel-level monitoring on Linux.

## Programs

### 1. XDP Packet Counter (`xdp_packet_counter.c`)
- **Type**: XDP (eXpress Data Path)
- **Purpose**: Count network packets without dropping them
- **Metrics**:
  - Total packets processed
  - Total bytes processed
  - IPv4 vs IPv6 packet counts
- **Attach Point**: Network interface (e.g., lo, eth0)

### 2. Kprobe Execve Monitor (`kprobe_execve.c`)
- **Type**: Kprobe
- **Purpose**: Monitor process execution (sys_execve calls)
- **Metrics**:
  - Process execution events (PID, UID, command)
  - Execution count per user
- **Attach Point**: sys_execve kernel function

### 3. Tracepoint Network Monitor (`tracepoint_network.c`)
- **Type**: Tracepoint
- **Purpose**: Monitor network socket operations
- **Metrics**:
  - Connection events (connect, accept)
  - Data transfer (send, recv) with byte counts
  - Socket closures
- **Attach Points**: Various syscall tracepoints

## Compilation

### Prerequisites

1. Install required packages:
```bash
# Ubuntu/Debian
sudo apt-get install clang llvm libbpf-dev linux-headers-$(uname -r)

# Fedora/RHEL
sudo dnf install clang llvm libbpf-devel kernel-headers

# Arch Linux
sudo pacman -S clang llvm libbpf linux-headers
```

2. Install aya-tool (Rust eBPF compiler):
```bash
cargo install aya-tool
```

### Compile Programs

#### Method 1: Using clang directly

```bash
# Compile XDP packet counter
clang -O2 -target bpf -c xdp_packet_counter.c -o xdp_packet_counter.o

# Compile kprobe execve monitor
clang -O2 -target bpf -c kprobe_execve.c -o kprobe_execve.o

# Compile tracepoint network monitor
clang -O2 -target bpf -c tracepoint_network.c -o tracepoint_network.o
```

#### Method 2: Using aya-tool (recommended for Rust integration)

```bash
# Generate Rust bindings and compile
aya-tool generate xdp_packet_counter.c > xdp_packet_counter.rs
aya-tool build-ebpf xdp_packet_counter.c

aya-tool generate kprobe_execve.c > kprobe_execve.rs
aya-tool build-ebpf kprobe_execve.c

aya-tool generate tracepoint_network.c > tracepoint_network.rs
aya-tool build-ebpf tracepoint_network.c
```

### Compilation Script

```bash
#!/bin/bash
# compile_ebpf.sh - Compile all eBPF programs

set -e

echo "Compiling eBPF programs..."

for prog in *.c; do
    base="${prog%.c}"
    echo "Compiling $prog -> ${base}.o"
    clang -O2 -target bpf -c "$prog" -o "${base}.o"
done

echo "All programs compiled successfully!"
echo "Note: Run with CAP_BPF or as root to load these programs"
```

## Loading Programs in HyperMesh

These compiled eBPF programs (.o files) can be loaded by HyperMesh using the aya framework:

```rust
use aya::Bpf;
use aya::programs::{Xdp, XdpFlags, KProbe, TracePoint};

// Load XDP program
let mut bpf = Bpf::load_file("xdp_packet_counter.o")?;
let program: &mut Xdp = bpf.program_mut("xdp_packet_counter").unwrap().try_into()?;
program.load()?;
program.attach("lo", XdpFlags::default())?;

// Load kprobe program
let mut bpf = Bpf::load_file("kprobe_execve.o")?;
let program: &mut KProbe = bpf.program_mut("trace_execve").unwrap().try_into()?;
program.load()?;
program.attach("sys_execve", 0)?;

// Load tracepoint program
let mut bpf = Bpf::load_file("tracepoint_network.o")?;
let program: &mut TracePoint = bpf.program_mut("trace_connect_enter").unwrap().try_into()?;
program.load()?;
program.attach("syscalls", "sys_enter_connect")?;
```

## Security Requirements

### Required Capabilities
- `CAP_BPF` (Linux 5.8+) or `CAP_SYS_ADMIN` (older kernels)
- Read access to `/sys/fs/bpf` for pinning maps
- Access to `/sys/kernel/debug/tracing` or `/sys/kernel/tracing`

### Running with minimal privileges:
```bash
# Add CAP_BPF capability to binary
sudo setcap cap_bpf+ep ./hypermesh

# Or run with specific capability
sudo capsh --caps="cap_bpf+eip" -- -c "./hypermesh"
```

## Testing

Test the eBPF integration:
```bash
# Run tests (requires appropriate permissions)
cargo test --test test_ebpf_kernel_integration -- --nocapture

# Run with sudo if needed
sudo -E cargo test --test test_ebpf_kernel_integration -- --nocapture
```

## Troubleshooting

### Common Errors

1. **"Insufficient permissions"**
   - Run with CAP_BPF or as root
   - Check if /sys/fs/bpf is mounted: `mount | grep bpf`

2. **"eBPF not supported"**
   - Check kernel version: `uname -r` (needs >= 4.4)
   - Check if CONFIG_BPF is enabled: `grep CONFIG_BPF /boot/config-$(uname -r)`

3. **"Program too large"**
   - Kernel has instruction limits (typically 1M instructions)
   - Optimize program or split into smaller programs

4. **"Invalid attach type"**
   - Some attach types require specific kernel versions
   - XDP: kernel >= 4.8
   - LSM: kernel >= 5.7
   - BTF/CO-RE: kernel >= 5.0

## Performance Considerations

- eBPF programs run in kernel space - keep them efficient
- Use per-CPU maps for high-frequency counters
- Use ring buffers for event streaming (more efficient than perf buffers)
- Minimize map lookups in hot paths
- Use static inline functions to reduce overhead

## References

- [Linux eBPF Documentation](https://docs.kernel.org/bpf/)
- [Aya Rust eBPF Framework](https://aya-rs.dev/)
- [BPF CO-RE (Compile Once, Run Everywhere)](https://nakryiko.com/posts/bpf-portability-and-co-re/)