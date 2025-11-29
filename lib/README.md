# HyperMesh Shared Library

Core shared types and systems used across all HyperMesh components.

## Components

### Proof of State (Consensus System)

The universal consensus system that validates all operations across the HyperMesh ecosystem. Every asset and operation requires validation through all four proofs:

- **Proof of Space (PoSp)**: WHERE - Storage location and physical/network location
- **Proof of Stake (PoSt)**: WHO - Ownership, access rights, and economic stake
- **Proof of Work (PoWk)**: WHAT/HOW - Computational resources and processing
- **Proof of Time (PoTm)**: WHEN - Temporal ordering and timestamp validation

**Combined**: These create a unified "Consensus Proof" that answers WHERE/WHO/WHAT/WHEN for every block and asset.

### Asset System

Universal asset types and identifiers used throughout HyperMesh:

- `AssetId`: Blockchain-registered universal identifiers
- `AssetType`: CPU, GPU, Memory, Storage, Network, Container, Economic
- `AssetMetadata`: Common metadata for all assets

### Common Utilities

Shared error types, result wrappers, and utility functions.

## Usage

```rust
use hypermesh_lib::{AssetId, AssetType, ConsensusProof};

// Create an asset
let asset_id = AssetId::new(AssetType::Cpu);

// Access Proof of State system
use hypermesh_lib::proof_of_state::*;
```

## Architecture

This library is dependency-free of other HyperMesh components and serves as the foundation:

```
hypermesh-lib (this crate)
    ├── Proof of State ← Used by all components
    ├── Asset System ← Used by blockmatrix
    └── Common Types ← Used everywhere
```
