# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.0-alpha] - 2026-02-25

### Added

- Block-MATRIX node with dual-scope blockchain (Device local-only, Network synchronized)
- STOQ QUIC transport with eBPF integration and adaptive bandwidth tiers (176 tests)
- TrustChain FALCON-1024 certificate authority with binary authentication (95% complete)
- Caesar EVP gold-gram protocol with conservation invariant and PID governor (220 tests)
- Catalog asset package registry with DHT distribution and STOQ API (264 tests)
- Gateway HTTP/3 + STOQ dual-listener with 4 roles: bootstrap, inbound proxy, outbound proxy, inter-network (155 tests)
- engauge analytics with streaming metrics, differential privacy, and resource marketplace (130 tests)
- caesar-sdk with IngressAdapter and EgressAdapter traits for external integration
- hypermesh-ebpf XDP packet processing with AF_XDP zero-copy I/O and policy sync
- IPv6 asset addressing with AssetAddress and TransferEngine
- Privacy model: PrivacyMode (Anonymous/Private/Public) independent from BlockchainScope (Device/Network)
- Quantum-resistant cryptography throughout: FALCON-1024 signing, Kyber-1024 encryption, BLAKE3 hashing
- Proof of State four-proof authentication: PoSpace, PoStake, PoWork, PoTime
- Asset pipeline: Compress (Brotli), Encrypt (Kyber-1024), Shard (Reed-Solomon 10+4), Distribute (tensor)
- Remote proxy/NAT system with IPv6-like addressing for resource access
- Tensor-based routing and resource allocation using Block-MATRIX topology
- Instruction-based retrieval (send maps, not files)
- hypermesh-lib shared types crate as single source of truth for all workspace members

[Unreleased]: https://github.com/hypermesh-online/core/compare/v0.1.0-alpha...HEAD
[0.1.0-alpha]: https://github.com/hypermesh-online/core/releases/tag/v0.1.0-alpha
