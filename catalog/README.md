# Catalog

Asset package registry and template library for the HyperMesh ecosystem. Catalog is a package manager -- it manages asset definitions, versioning, distribution, and discovery. It is not a marketplace, not a VM, and does not execute code locally.

**Status**: 100% Complete (alpha) | 78 files | ~27,900 lines | 52 tests

**Network Address**: `catalog.hypermesh.online` (via TrustChain DNS)

## What Catalog Does

- **Package Management**: Publish, install, search, version, and resolve dependencies for asset packages
- **Template Generation**: Create new asset packages from built-in templates
- **Distribution**: Content-addressed storage (DHT) with STOQ transport and P2P sharing
- **Security**: TrustChain FALCON-1024 certificate lifecycle, binary publisher authentication (pass/fail, no reputation scoring)
- **Validation**: Proof of State validation for packages, syntax validation (Julia/Lua/WASM -- validation only, not execution)
- **Rewards**: Caesar contribution rewards via `CatalogRewardAdapter` and `ContributionTracker` (30% publications, 30% references, 25% validation, 15% maintenance)
- **Discovery**: Typedef registry with browsing, multi-factor search scoring, and featured listings
- **Hashing**: BLAKE3 content hashing throughout

## What Catalog Does Not Do

- Does not execute code locally -- execution is delegated to remote HyperMesh nodes
- Does not run a VM or sandbox
- Is not a commerce marketplace
- Does not manage runtime resources (that is BlockMatrix's job via Asset Adapters)

## Architecture

```
catalog.hypermesh.online
    |- Registry         (publish/install/search, semantic versioning, dependency resolution)
    |- Distribution     (DHT content-addressed storage, STOQ transport, P2P sharing)
    |- Templates        (built-in asset templates, generation framework)
    |- Security         (TrustChain certs, binary publisher auth, PoS validation)
    |- Asset SDK        (clean public API facade for package operations)
    |- Asset Handlers   (Blockchain, Dns, Dataset, Template handlers)
    |- Sharing          (peer-to-peer package sharing, discovery, synchronization)
    |- Rewards          (Caesar integration, contribution tracking)
    '- STOQ API         (CatalogStoqApi: browse/search/package/publisher/stats/health)
```

### Execution Delegation

Catalog packages assets with metadata and resource requirements. Actual execution happens on remote HyperMesh nodes, with resources allocated through BlockMatrix Asset Adapters (CPU/GPU/Memory/Storage).

## Quick Start

```bash
cargo build -p catalog --release
cargo test -p catalog
```

## STOQ API

`CatalogStoqApi` binds on `[::1]:9295` with 6 handlers:
- `browse` -- browse available packages
- `search` -- multi-factor search with scoring
- `package` -- get package details
- `publisher` -- publisher information
- `stats` -- registry statistics
- `health` -- service health check

## Integration

- **STOQ**: Transport for all distribution and API traffic
- **TrustChain**: Certificate-based authentication, FALCON-1024 signing for publishers
- **BlockMatrix**: Asset system integration, resource mapping for execution delegation
- **Caesar**: Contribution reward distribution for publishers and validators

## License

Business Source License 1.1
