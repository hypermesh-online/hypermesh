# Core System Components

## ⚠️ DEVELOPMENT STATUS: Early Prototype

This directory contains early prototype code. Most features described below are planned but NOT yet implemented. The "Nexus" branding should be ignored - there is no Nexus CLI or system.

## Architecture Overview

```
core/
├── transport/          # QUIC over IPv6 transport layer
├── runtime/           # Container runtime and isolation
├── state/             # Distributed state management (etcd replacement)
├── scheduler/         # Resource scheduling and orchestration
├── networking/        # Service mesh and networking stack
├── security/          # Certificate management and security policies
├── ebpf/             # eBPF programs for kernel integration
├── api/              # Core API server implementation
└── shared/           # Shared libraries and utilities
```

## Planned Technologies (Not Yet Integrated)

- **Rust**: Basic structure exists, many compilation errors
- **eBPF**: NOT IMPLEMENTED - only stub files exist
- **QUIC**: Partially designed, not fully integrated
- **Hardware Virtualization**: NOT IMPLEMENTED
- **Consensus Algorithms**: Basic Raft only, no Byzantine fault tolerance in production

## Development Environment Setup

### Prerequisites
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Install eBPF toolchain
sudo apt-get install -y clang llvm libelf-dev linux-headers-$(uname -r)

# Install additional dependencies
cargo install cargo-ebpf bindgen-cli
```

### Building Core Components
```bash
# Build all core components
cargo build --release

# Run tests
cargo test

# Build eBPF programs
cargo ebpf build --target bpfel-unknown-none
```

## Component Status (All Incomplete)

- [ ] **Transport Layer**: QUIC protocol (basic design only, not functional)
- [ ] **Container Runtime**: NOT IMPLEMENTED
- [ ] **State Engine**: Single-node only, no real distribution
- [ ] **Scheduler**: NOT IMPLEMENTED
- [ ] **Service Mesh**: NOT IMPLEMENTED
- [ ] **Security Framework**: Basic types only
- [ ] **eBPF Integration**: NOT IMPLEMENTED (stub files only)
- [ ] **API Server**: NOT IMPLEMENTED

## Future Performance Targets (Not Current Capabilities)

- Container startup: <100ms (containers not implemented)
- Network latency: <1ms goal
- Consensus latency: <10ms target (single-node only currently)
- Throughput: >1M requests/second goal (not measured)
- Memory overhead: <50MB target per container (no containers yet)

## Future Security Goals (Not Yet Achieved)

- Zero remote code execution vulnerabilities (Rust helps but not proven)
- Hardware-enforced isolation boundaries (not implemented)
- Automatic certificate rotation (planned feature)
- Real-time security policy enforcement (not implemented)
- Comprehensive audit logging (basic logging only)