# Security Policy

## Reporting Vulnerabilities

**Do NOT open public issues for security vulnerabilities.**

Please report security vulnerabilities by email to: **security@hypermesh.online**

Include the following in your report:

- Description of the vulnerability
- Steps to reproduce
- Affected component(s) and version(s)
- Potential impact assessment

## Scope

The following components are in scope for security reports:

- **TrustChain**: FALCON-1024 certificate authority, certificate issuance and revocation
- **BlockMatrix**: Kyber-1024 asset encryption, state proof validation, asset pipeline
- **STOQ**: Transport security, PoS token validation, connection authentication
- **hypermesh-ebpf**: XDP packet processing, AF_XDP zero-copy I/O, policy enforcement
- **Caesar**: EVP conservation invariant, settlement logic, gold-gram integrity
- **Gateway**: TLS termination, rate limiting, federation trust boundaries
- **Binary authentication**: Proof of State validation (PoSpace/PoStake/PoWork/PoTime)
- **BLAKE3 content integrity**: Hash verification across all crates

## Supported Versions

| Version | Supported |
|---------|-----------|
| `main` branch (0.1.0-alpha) | Yes |
| All other branches | No |

HyperMesh is alpha software under active development. Only the current `main` branch receives security fixes.

## Response Timeline

- **Acknowledgment**: Within 48 hours of report
- **Initial assessment**: Within 7 days
- **Critical fixes**: Within 30 days
- **Non-critical fixes**: Addressed in the next development sprint

## Disclosure

We follow coordinated disclosure. Please allow us reasonable time to address the issue before any public disclosure.
