# STOQ Protocol Refactoring & CDN Implementation Plan

## Overview
Based on the comprehensive codebase audit, this document outlines the surgical refactoring approach to evolve STOQ into a standalone protocol with CDN/edge network capabilities while maintaining clean separation from Nexus and HyperMesh.

## Current State Analysis

### Critical Issues Identified
1. **Simulated Performance**: STOQ benchmark uses `sleep()` instead of real testing (line 88 in stoq_benchmark.rs)
2. **Tight Coupling**: Transport layer directly integrated with Nexus
3. **Missing CDN Primitives**: No chunking, caching, or route optimization
4. **Limited Scalability**: Hardcoded 10K connection limit
5. **No Edge Support**: Missing geographic awareness and edge node management

## Refactoring Strategy

### Phase 1: Protocol Extraction (Weeks 1-2)

#### 1.1 Create Standalone STOQ Module
```
/protocols/
└── stoq/
    ├── Cargo.toml
    ├── src/
    │   ├── lib.rs           # Public API
    │   ├── transport/       # QUIC/IPv6 transport
    │   │   ├── mod.rs
    │   │   ├── quic.rs      # QUIC implementation
    │   │   ├── certificates.rs
    │   │   └── streams.rs
    │   ├── routing/         # CDN routing logic
    │   │   ├── mod.rs
    │   │   ├── matrix.rs    # Routing matrix
    │   │   ├── discovery.rs # Route discovery
    │   │   └── optimization.rs
    │   ├── chunking/        # Data chunking
    │   │   ├── mod.rs
    │   │   ├── engine.rs    # Chunking algorithms
    │   │   ├── dedup.rs     # Deduplication
    │   │   └── distribution.rs
    │   └── config.rs        # Configuration
    └── tests/
```

#### 1.2 Extract Core Components
- Move transport logic from `/core/transport/` to `/protocols/stoq/src/transport/`
- Decouple certificate management into standalone module
- Create clean API boundaries with trait-based interfaces

### Phase 2: CDN Core Implementation (Weeks 3-6)

#### 2.1 Routing Matrix Implementation
```rust
// protocols/stoq/src/routing/matrix.rs
pub struct RoutingMatrix {
    nodes: Vec<NodeMetrics>,
    topology: Array2<f64>,  // Latency matrix
    bandwidth: Array2<f64>, // Bandwidth matrix
    ml_model: Option<RoutePredictor>,
}

impl RoutingMatrix {
    pub fn find_optimal_route(&self, src: NodeId, dst: NodeId) -> Route {
        // Multi-factor optimization:
        // 1. Latency (RTT measurements)
        // 2. Available bandwidth
        // 3. Node load
        // 4. Geographic proximity
        // 5. Historical performance
    }
    
    pub fn update_metrics(&mut self, metrics: NodeMetrics) {
        // Real-time matrix updates
        // ML model retraining
    }
}
```

#### 2.2 Chunk Management System
```rust
// protocols/stoq/src/chunking/engine.rs
pub struct ChunkEngine {
    algorithm: ChunkAlgorithm,
    dedup_index: DedupIndex,
    distribution_map: DistributionMap,
}

impl ChunkEngine {
    pub fn chunk_data(&self, data: &[u8]) -> Vec<Chunk> {
        // Content-aware chunking
        // Variable size based on content type
        // Deduplication at chunk level
    }
    
    pub fn distribute_chunks(&self, chunks: Vec<Chunk>) -> Distribution {
        // Geographic distribution
        // Redundancy planning
        // Erasure coding
    }
}
```

#### 2.3 Edge Network Support
```rust
// protocols/stoq/src/edge/mod.rs
pub struct EdgeNetwork {
    edge_nodes: HashMap<Region, Vec<EdgeNode>>,
    cache_policy: CachePolicy,
    sync_protocol: SyncProtocol,
}

impl EdgeNetwork {
    pub fn register_edge_node(&mut self, node: EdgeNode) {
        // Geographic registration
        // Capacity planning
    }
    
    pub fn find_nearest_edge(&self, client: ClientLocation) -> EdgeNode {
        // Geolocation-based routing
        // Load balancing
    }
}
```

### Phase 3: Performance Optimization (Weeks 7-8)

#### 3.1 Real Performance Testing
Replace simulated benchmarks with actual performance tests:

```rust
// protocols/stoq/benches/real_throughput.rs
pub async fn benchmark_real_throughput() -> ThroughputResults {
    // Setup actual QUIC connections
    let transport = StoqTransport::new(config).await?;
    
    // Generate real traffic patterns
    let traffic = generate_cdn_traffic_patterns();
    
    // Measure actual throughput
    let start = Instant::now();
    let bytes_transferred = transport.transfer(traffic).await?;
    let duration = start.elapsed();
    
    // Calculate real metrics
    ThroughputResults {
        peak_sustained_throughput_gbps: calculate_throughput(bytes_transferred, duration),
        packet_loss_rate_percent: measure_packet_loss(),
        // ... actual measurements
    }
}
```

#### 3.2 Scalability Improvements
```rust
// Remove hardcoded limits
pub struct TransportConfig {
    pub max_connections: Option<u32>,  // None = unlimited
    pub connection_pool_size: usize,   // Dynamic pooling
    pub auto_scale: bool,              // Auto-scaling based on load
}
```

### Phase 4: Integration & Testing (Weeks 9-10)

#### 4.1 Nexus Integration Layer
```rust
// nexus/src/stoq_integration.rs
pub trait StoqProvider {
    async fn get_transport(&self) -> Arc<dyn Transport>;
    async fn get_router(&self) -> Arc<dyn Router>;
    async fn get_chunk_engine(&self) -> Arc<dyn ChunkEngine>;
}

// Clean dependency injection
pub struct NexusOrchestrator {
    stoq: Arc<dyn StoqProvider>,
    // Nexus-specific logic
}
```

#### 4.2 Comprehensive Testing Suite
```rust
// protocols/stoq/tests/integration/
mod cdn_tests {
    #[tokio::test]
    async fn test_multi_source_chunk_retrieval() { }
    
    #[tokio::test]
    async fn test_geographic_routing() { }
    
    #[tokio::test]
    async fn test_edge_cache_performance() { }
    
    #[tokio::test]
    async fn test_failover_scenarios() { }
}
```

## Implementation Checklist

### Immediate Actions (Week 1)
- [ ] Create `/protocols/stoq/` directory structure
- [ ] Move transport code to new location
- [ ] Create trait-based interfaces
- [ ] Set up independent Cargo.toml

### Short Term (Weeks 2-4)
- [ ] Implement routing matrix
- [ ] Build chunk engine
- [ ] Add edge network support
- [ ] Create real benchmarks

### Medium Term (Weeks 5-8)
- [ ] Integrate CDN capabilities
- [ ] Optimize performance
- [ ] Add monitoring/observability
- [ ] Implement failover mechanisms

### Long Term (Weeks 9-12)
- [ ] Complete integration testing
- [ ] Performance validation
- [ ] Documentation update
- [ ] Production deployment

## Success Metrics

### Functional Requirements
- ✅ STOQ works standalone without Nexus
- ✅ Clean trait-based interfaces
- ✅ Full IPv6 support (no IPv4)
- ✅ CDN routing capabilities
- ✅ Edge network support

### Performance Requirements
- ✅ Real 40+ Gbps throughput (not simulated)
- ✅ 100K+ concurrent connections
- ✅ <10ms route discovery
- ✅ <100ms chunk retrieval
- ✅ >90% network utilization

### Architecture Requirements
- ✅ Complete separation of concerns
- ✅ Modular design
- ✅ Extensible configuration
- ✅ Plugin architecture for extensions

## Risk Mitigation

### Technical Risks
1. **Performance Regression**: Continuous benchmarking during refactor
2. **Breaking Changes**: Maintain compatibility layer during transition
3. **Integration Issues**: Comprehensive integration test suite

### Schedule Risks
1. **Scope Creep**: Strict adherence to phased approach
2. **Dependencies**: Early identification and resolution
3. **Testing Delays**: Parallel test development

## Next Steps

1. **Create Working Branch**: `feature/stoq-cdn-refactor`
2. **Set Up Module Structure**: Initialize `/protocols/stoq/`
3. **Begin Extraction**: Move transport components
4. **Implement CDN Core**: Start with routing matrix
5. **Validate Progress**: Weekly performance benchmarks

## Documentation Updates Required

- [ ] STOQ Protocol Specification
- [ ] CDN Architecture Guide
- [ ] Edge Network Operations Manual
- [ ] Performance Tuning Guide
- [ ] Migration Guide for Existing Code

## Conclusion

This refactoring plan provides a methodical approach to evolving STOQ into a standalone, CDN-capable protocol while maintaining the high-performance characteristics of the current implementation. The phased approach ensures continuous validation and minimizes risk while delivering incremental value.