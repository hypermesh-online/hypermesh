# Contributing to HyperMesh

Thank you for your interest in contributing to the HyperMesh Web3 ecosystem. This document covers the development workflow, constraints, and expectations for contributions.

## Prerequisites

- **Rust**: Nightly toolchain (`rustup default nightly`)
- **Node.js**: 20+ (for `ui/` frontend only)
- **cargo-criterion**: For benchmarks (`cargo install cargo-criterion`)
- **Linux**: Required for eBPF features (kernel 5.15+)

## Building

```bash
# Check all crates compile
cargo check --workspace

# Run all tests
cargo test --workspace

# Release build
cargo build --workspace --release

# Lint (must pass with zero warnings)
cargo clippy --workspace -- -D warnings
```

## Code Style

- **Formatting**: `cargo fmt --all` using default rustfmt configuration
- **Linting**: `cargo clippy --workspace -- -D warnings` must produce zero warnings
- **File size**: Maximum 500 lines per file. Split into modules if exceeded.
- **Function size**: Maximum 50 lines per function. Extract subfunctions if exceeded.
- **Nesting**: Maximum 3 levels of indentation. Abstract into functions or modules.
- **Naming**: Descriptive, self-documenting names. No cryptic abbreviations.
- **Error handling**: No silent failures. All errors must be handled or propagated.

## Pre-commit Hook

The repository includes a pre-commit hook that enforces production code safety:

- **Blocked**: `.unwrap()` and `panic!()` in production Rust code (exit code 1)
- **Warned**: `.expect()` in production code (non-blocking)
- **Excluded**: Files under `tests/` and `benches/` directories

For non-test code where you need a runtime assertion, use `.expect("descriptive reason")` instead of `.unwrap()`. For unreachable match arms in source files, use `unreachable!()` which is not blocked.

## Testing

```bash
# Test a specific crate
cargo test -p stoq
cargo test -p trustchain
cargo test -p catalog

# BlockMatrix: full --lib test suite may hang. Use module filters:
cargo test -p blockmatrix -- transfer::tests
cargo test -p blockmatrix -- verification::tests

# UI tests
cd ui/frontend && npm test
```

## Architecture Constraints

HyperMesh components are **system-level daemons** (similar to DHCP or DNS services), not containerized applications. The following are firm constraints:

- **No Docker, Kubernetes, or cloud-native PRs.** See `systemd/` for service unit files.
- **No HTTP/REST APIs.** All communication uses the STOQ protocol (QUIC + eBPF).
- **IPv6 only.** No IPv4 support in the transport layer.
- **System-level deployment.** Components run as native processes managed by systemd.

## Cryptographic Constraints

All contributions must use the approved cryptographic primitives:

| Purpose | Algorithm | Notes |
|---------|-----------|-------|
| Signing | FALCON-1024 | TrustChain CA certs, STOQ handshake |
| Encryption | Kyber-1024 | Asset encryption via KEM + AES-GCM |
| Hashing | BLAKE3 | All content hashing |

**Exceptions**: SHA-256 is permitted only for X.509 certificate fingerprints and OCI image digests (standard compliance).

Do not introduce SHA-256, RSA, Ed25519, or other non-quantum-resistant primitives for new functionality.

## Workspace Structure

| Crate | Description |
|-------|-------------|
| `lib` | Shared types (canonical source of truth) |
| `blockmatrix` | Block-MATRIX node, asset system, blockchain |
| `stoq` | QUIC transport with eBPF integration |
| `trustchain` | FALCON-1024 certificate authority |
| `caesar` | EVP gold-gram protocol |
| `caesar-sdk` | UPI adapter traits for Caesar integration |
| `catalog` | Asset package registry and DHT distribution |
| `gateway` | HTTP/3 + STOQ dual-listener gateway |
| `engauge` | Analytics, streaming metrics, marketplace |
| `hypermesh-ebpf` | XDP packet processing with AF_XDP |
| `ui` | TypeScript/React frontend |

## Branch Naming

- `feature/<name>` -- New functionality
- `fix/<name>` -- Bug fixes
- `refactor/<name>` -- Code restructuring without behavior change

## Pull Request Process

1. Fork the repository
2. Create a branch from `main` using the naming convention above
3. Make your changes, ensuring all tests pass and clippy is clean
4. Submit a pull request against `main`
5. Provide a clear description of the change and its motivation
6. Link any related issues

## License

By contributing, you agree that your contributions will be licensed under the Business Source License 1.1 (BSL-1.1). See `LICENSE` for details.
