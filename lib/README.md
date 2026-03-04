# HyperMesh Shared Library (`hypermesh-lib`)

Canonical shared types for the HyperMesh ecosystem. All crates depend on this library as the single source of truth for identifiers, enums, and cross-cutting type definitions.

**Status**: Alpha (68% complete) | 12 files | ~3,200 lines | 97 tests

## Core Types

### Identifiers (`types.rs`)
- `NodeId(String)` -- unique node identifier in the Block-MATRIX topology
- `AssetId(String)` -- blockchain-registered universal asset identifier
- `NetworkId([u8; 16])` -- 128-bit network identifier
- `ContentHash([u8; 32])` -- 256-bit BLAKE3 digest
- `KeyPairId` -- cryptographic key pair reference
- `AssetAddress([u8; 16])` -- IPv6 asset address (`fd48:4d00` prefix, matrix coords, content fingerprint, shard sub-addressing)

### Enums (`types.rs`)
- `BlockchainScope` -- binary: `Device` (local-only, always running) | `Network` (synchronized via reflector/swarm)
- `ProofType` -- `PoSpace` | `PoStake` | `PoWork` | `PoTime`
- `PipelineStage` -- asset processing pipeline stages
- `CryptoAlgorithm` -- `Falcon` | `Kyber` | `Aes`

### Privacy (`types.rs`)
- `PrivacyMode` -- struct with 2 axes: `scope: AccessScope` + `tracked: bool`
- `AccessScope` -- `Bounded` | `Unbounded`
- Three canonical presets: `ANONYMOUS` (Unbounded, untracked), `PRIVATE` (Bounded, tracked), `PUBLIC` (Unbounded, tracked)

### Asset System (`asset.rs`)
- `SystemAssetKind` -- 9 variants: `Cpu`, `Gpu`, `Memory`, `Storage`, `Network`, `Container`, `Economic`, `Blockchain`, `Dns`
- `AssetMetadata` -- common metadata for all assets
- `BaseState` / `AssetStatusTrait` -- asset lifecycle states
- `AssetAdapter` -- trait for specialized asset handling (three-pillar system)

### state proofs (`proof.rs`)
- `SpaceProof`, `StakeProof`, `WorkProof`, `TimeProof` -- individual proof structs
- `ProofOfState` -- combined four-proof validation (WHERE/WHO/WHAT/WHEN)
- `Validatable` trait -- shared validation interface

### Economic Types (`economic.rs`)
- `PacketId`, `GoldGrams`, `MarketTier`, `PacketState`, `DemurrageRate` -- Caesar EVP types

### Position (`types.rs`)
- `MatrixPosition` -- (x, y, z) coordinate in the Block-MATRIX topology

## Usage

```rust
use hypermesh_lib::{
    AssetId, NodeId, ContentHash, NetworkId,
    PrivacyMode, BlockchainScope, ProofType,
    MatrixPosition, AssetAddress,
};
use hypermesh_lib::asset::SystemAssetKind;
use hypermesh_lib::proof::ProofOfState;

// Identifiers
let node = NodeId::from("node-alpha");
let asset = AssetId::from("asset-001");

// Privacy (struct, not enum -- use constants)
let mode = PrivacyMode::ANONYMOUS;

// Blockchain scope (binary)
let scope = BlockchainScope::Device;
```

## Architecture

This library has zero dependencies on other HyperMesh crates and serves as the foundation layer:

```
hypermesh-lib (this crate)
    |- Identifiers    (NodeId, AssetId, NetworkId, ContentHash, AssetAddress)
    |- Enums          (BlockchainScope, ProofType, PipelineStage, CryptoAlgorithm)
    |- Privacy        (PrivacyMode struct with 3 presets)
    |- Asset System   (SystemAssetKind, BaseState, AssetAdapter trait)
    |- Proofs         (SpaceProof, StakeProof, WorkProof, TimeProof, ProofOfState)
    |- Economic       (PacketId, GoldGrams, MarketTier, PacketState)
    '- Error          (HypermeshError unified error type)
```

## License

Business Source License 1.1
