# Phase 1: STOQ Architecture Cleanup - Dependency Analysis

## Executive Summary
STOQ has become contaminated with application-layer code that violates the pure transport protocol principle. This analysis identifies all contamination and provides a roadmap for extraction.

## Contamination Identified

### Application Layer Code in STOQ (To Be Removed)
1. **Phoenix SDK** (`src/phoenix.rs`)
   - Application-layer SDK wrapper
   - Connection pooling logic
   - Performance monitoring at application level
   - Developer-friendly API abstractions
   - **Dependencies**: Uses core transport (StoqTransport), should be external consumer

2. **Performance Monitor** (`src/performance_monitor.rs`)
   - Real-time performance monitoring system
   - Threshold alerting
   - Statistics aggregation
   - **Dependencies**: Standalone monitoring, should be external observer

3. **Monitoring Module** (`src/monitoring.rs`)
   - Dashboard integration features
   - Historical metrics storage
   - API for external monitoring systems
   - **Dependencies**: Uses transport stats, should be external consumer

4. **Example Applications** (To Move)
   - `examples/phoenix_demo.rs` - Phoenix SDK demo
   - `examples/performance_monitor.rs` - Performance monitoring demo
   - `examples/monitoring_demo.rs` - Monitoring dashboard demo
   - `examples/realistic_performance_demo.rs` - Performance testing
   - `examples/benchmark_real.rs` - Real benchmarking tool
   - `examples/throughput_test.rs` - Throughput testing

### Pure Transport Code (To Keep)
1. **Core Transport** (`src/transport/`)
   - QUIC protocol implementation
   - Connection management
   - Stream handling
   - Certificate management
   - Falcon post-quantum encryption
   - Basic transport metrics (bytes sent/received, connections)

2. **Protocol Extensions** (`src/extensions.rs`)
   - Packet tokenization
   - Packet sharding
   - Hop routing
   - Seed node management

3. **Transport Config** (`src/config/`)
   - Transport configuration
   - Network settings

4. **Pure Examples** (To Keep)
   - `examples/integrated_echo_server.rs` - Basic echo server demonstrating transport

## Dependency Graph

```
Current (Contaminated):
┌─────────────────────────────────────┐
│             STOQ Package             │
├─────────────────────────────────────┤
│  Application Layer (CONTAMINATION)   │
│  ┌─────────────┐ ┌─────────────┐    │
│  │  Phoenix    │ │  Monitoring │    │
│  │    SDK      │ │   System    │    │
│  └──────┬──────┘ └──────┬──────┘    │
│         │               │            │
│         └───────┬───────┘            │
│                 ▼                    │
│  ┌─────────────────────────────┐    │
│  │    Transport Layer (PURE)    │    │
│  │  - QUIC Protocol            │    │
│  │  - Connection Management     │    │
│  │  - Stream Handling          │    │
│  │  - Transport Metrics        │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘

Target (Clean):
┌─────────────────┐     ┌─────────────────┐
│   HyperMesh     │     │  External Apps   │
│  ┌───────────┐  │     │                 │
│  │  Phoenix  │  │     │                 │
│  │    SDK    │  │     │                 │
│  └─────┬─────┘  │     │                 │
│  ┌─────┴─────┐  │     │                 │
│  │Monitoring │  │     │                 │
│  └─────┬─────┘  │     │                 │
└────────┼────────┘     └────────┬────────┘
         │                       │
         └───────────┬───────────┘
                     ▼
         ┌───────────────────────┐
         │    STOQ (Pure)        │
         │  Transport Protocol   │
         │  - QUIC/IPv6         │
         │  - Zero application  │
         │    logic             │
         └───────────────────────┘
```

## Migration Plan

### Step 1: Create Target Directories in HyperMesh
```bash
hypermesh/src/runtime/phoenix/     # Phoenix SDK
hypermesh/src/monitoring/          # Monitoring system
hypermesh/examples/phoenix/        # Phoenix examples
```

### Step 2: Extract Phoenix SDK
- Move `stoq/src/phoenix.rs` → `hypermesh/src/runtime/phoenix/mod.rs`
- Update to use STOQ as external dependency
- Convert from internal transport access to public API

### Step 3: Extract Monitoring
- Move `stoq/src/monitoring.rs` → `hypermesh/src/monitoring/stoq_monitor.rs`
- Move `stoq/src/performance_monitor.rs` → `hypermesh/src/monitoring/performance.rs`
- Update to observe STOQ externally

### Step 4: Move Examples
- Move Phoenix/monitoring examples to HyperMesh
- Keep only pure transport examples in STOQ

### Step 5: Clean STOQ lib.rs
- Remove Phoenix exports
- Remove monitoring exports
- Keep only pure transport exports

## Risk Assessment

### Low Risk
- Phoenix SDK extraction (clean boundaries)
- Example moves (no production impact)

### Medium Risk
- Monitoring extraction (may be used by tests)
- Need to update import paths

### Mitigation
- Run full test suite after each extraction
- Update documentation immediately
- Maintain backward compatibility where needed

## Success Criteria
1. STOQ compiles with zero application logic
2. STOQ exports only transport primitives
3. HyperMesh contains all application features
4. All tests pass in both projects
5. Clean architectural boundaries established

## Timeline
- Day 1-2: Dependency analysis ✓ (COMPLETE)
- Day 3-4: Code extraction (NEXT)
- Day 5: Validation and documentation

## Next Actions
1. Begin code extraction starting with Phoenix SDK
2. Create target directories in HyperMesh
3. Move code with proper dependency updates
4. Validate compilation at each step