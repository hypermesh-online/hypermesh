# Catalog Plugin Integration Tests

## Status: AWAITING CATALOG IMPLEMENTATION

**All tests in this module are marked `#[ignore]` and will not compile until the Catalog extension is implemented.**

## Purpose

These tests are prepared for when the Catalog extension is implemented in BlockMatrix. They provide comprehensive coverage of:

1. **Lifecycle** (`lifecycle.rs`) - Plugin loading, unloading, configuration, hot-reload
2. **Integration** (`integration.rs`) - Asset registration, handlers, extension traits, API endpoints
3. **Operations** (`operations.rs`) - Library operations, P2P distribution, consensus integration
4. **Reliability** (`reliability.rs`) - Error handling, crash recovery, resource isolation, concurrent operations

## Current Compilation Status

**Expected Compilation Errors** (91 errors total):

### Missing API Components

1. **`blockmatrix::consensus::ProofType`** - Not exported from consensus module
2. **`hypermesh` crate** - Unresolved module references (extensions API)
3. **Extension API fields** - Various API mismatches:
   - `ExtensionLoader.config` is private
   - `ResourceLimits` field names differ from test expectations
   - `ResourceQuotas` struct doesn't match test usage
   - Extension trait methods not yet implemented

### Why These Errors Are Expected

The Catalog extension system designed in these tests requires:
- Full extension loading/unloading infrastructure
- Asset type registration system
- Security and capability management
- Extension lifecycle management
- VM execution integration

**None of these components currently exist in BlockMatrix.**

## File Organization

```
catalog_plugin/
├── mod.rs            - Common utilities and test helpers (81 lines)
├── lifecycle.rs      - Plugin lifecycle tests (410 lines)
├── integration.rs    - Extension integration tests (291 lines)
├── operations.rs     - Operation tests (283 lines)
└── reliability.rs    - Error handling and reliability (476 lines)
```

**Total: 1,541 lines** (split from original 1,484-line monolithic file)

All files comply with <500 line limit.

## Running Tests (When Implemented)

```bash
# Run all catalog plugin tests (when ready)
cargo test --test integration catalog_plugin

# Run specific test category
cargo test --test integration catalog_plugin::lifecycle
cargo test --test integration catalog_plugin::integration
cargo test --test integration catalog_plugin::operations
cargo test --test integration catalog_plugin::reliability

# Run with ignored tests (when implementation is ready)
cargo test --test integration catalog_plugin -- --ignored
```

## Implementation Checklist

When implementing the Catalog extension, ensure:

- [ ] Extension loading/unloading infrastructure
- [ ] Asset type registration system
- [ ] Security manager and capability checking
- [ ] Extension trait implementation
- [ ] API endpoint handlers
- [ ] Consensus integration
- [ ] P2P distribution support
- [ ] TrustChain verification
- [ ] Resource isolation and quotas
- [ ] State persistence and recovery
- [ ] Hot-reload functionality
- [ ] Error handling for all edge cases

## Test Coverage

- **Discovery & Loading**: 2 tests
- **Configuration & Security**: 2 tests
- **Lifecycle Management**: 3 tests
- **Asset Integration**: 3 tests
- **Operations & Sync**: 5 tests
- **Reliability & Errors**: 7 tests

**Total: 22 comprehensive integration tests**

## Notes

- All tests use `#[ignore]` attribute with clear reason
- Tests are ready to enable once Catalog API is implemented
- Test structure follows BlockMatrix conventions
- Helper functions centralized in `mod.rs`
- Clear separation of concerns across modules
