# Archived Test Files

These test files were disabled and have been archived because they reference APIs
that were never implemented. They appear to be specification tests written before
the actual STOQ implementation.

## Archived Files (*.rs.archived)

### 1. phase5_unit_tests.rs.archived
**Reason**: Missing core APIs
**Missing Components**:
- `AdaptiveOptimizer` - Not implemented
- `ProtocolExtensions`, `TokenizedStreams`, `ShardedData` - Protocol extensions don't exist
- `FalconHandshake` - Crypto API different from spec
- `EbpfManager` - eBPF integration not as specified
- `StoqError` - Error types differ from spec

**Status**: Would require implementing 5+ major missing components
**Recommendation**: Rewrite from scratch if these features are needed

### 2. protocol_integration_test.rs.archived
**Reason**: Protocol extension framework not implemented
**Missing Components**:
- `StoqProtocolHandler` - No protocol handler abstraction
- `StoqFrame`, `TokenFrame`, `ShardFrame`, `FalconSigFrame` - Frame types not implemented
- `StoqHandshakeExtension` - Handshake extensions not implemented
- `StoqParameters` - Parameter negotiation different from spec

**Status**: Entire protocol extension layer missing
**Recommendation**: Archive - Protocol extensions are conceptual only

### 3. phoenix_quality_gates.rs.archived
**Reason**: Phoenix SDK doesn't exist
**Missing Components**:
- `PhoenixTransport` - Entire Phoenix SDK not implemented
- `PhoenixConfig` - Phoenix configuration layer missing

**Status**: Phoenix is conceptual, never implemented
**Recommendation**: Archive - Phoenix SDK is vaporware

### 4. real_performance_validation.rs.archived
**Reason**: PerformanceMonitor module not public API
**Missing Components**:
- `PerformanceMonitor` - Exists in lib.rs but not exported
- Custom `NetworkTier` type (different from config::NetworkTier)

**Status**: Could be migrated if performance_monitor is made public
**Recommendation**: Rewrite using public APIs or export performance_monitor

## Migration Results

**Total Files**: 8
**Successfully Migrated**: 4 (50%)
- adaptive_test.rs ✅
- performance_real.rs ✅
- phase5_integration_tests.rs ✅
- phase5_performance_benchmarks.rs ✅

**Archived**: 4 (50%)
- phase5_unit_tests.rs.archived
- protocol_integration_test.rs.archived
- phoenix_quality_gates.rs.archived
- real_performance_validation.rs.archived

## Why Were These Tests Written?

These tests appear to be **specification tests** written during Phase 5 planning:
1. They document desired features (tokenization, sharding, Phoenix SDK)
2. They define ideal APIs that were never implemented
3. They represent a vision of STOQ that diverged from actual implementation

## Current STOQ Reality

The actual STOQ implementation is **simpler and more focused**:
- Core QUIC transport with FALCON-1024 crypto ✅
- Adaptive network tier configuration ✅
- Basic connection pooling and optimization ✅
- eBPF integration (different from spec) ✅

**Missing from spec**:
- Protocol extension framework ❌
- Token-based streaming ❌
- Automatic sharding ❌
- Phoenix SDK ❌
- Advanced adaptive optimizer ❌

## Going Forward

**Test what exists**, not what was planned:
1. Focus on transport layer tests (working)
2. Test actual FALCON crypto implementation
3. Test real adaptive behavior
4. Test actual eBPF integration
5. Performance benchmarks on implemented features

**Don't test vaporware**.
