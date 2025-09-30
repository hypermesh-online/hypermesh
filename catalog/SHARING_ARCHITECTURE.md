# Catalog Decentralized Sharing Architecture

## Overview

The Catalog sharing system enables fully decentralized asset library distribution across the HyperMesh network, eliminating single points of failure and enabling true peer-to-peer package management.

## Core Components

### 1. Sharing Manager (`sharing/mod.rs`)
- **Purpose**: Orchestrates all sharing operations
- **Features**:
  - Peer connectivity management
  - Package sharing with permissions
  - Automatic mirroring of popular packages
  - Network-wide search capabilities
  - Bandwidth and resource management
  - Incentive tracking for contributions

### 2. Synchronization Module (`sharing/synchronization.rs`)
- **Purpose**: Keeps libraries synchronized across nodes
- **Strategies**:
  - **Full Sync**: Complete library synchronization
  - **Incremental**: Only changes since last sync
  - **Selective**: Sync specific categories/tags
  - **Priority-based**: Sync high-priority packages first
  - **Differential**: Merkle tree-based efficient sync
- **Conflict Resolution**:
  - NewestWins: Use most recent version
  - ConsensusWins: Use version with highest consensus
  - Merge: Attempt to merge changes
  - KeepBoth: Maintain both versions
  - Manual: Require user intervention

### 3. Mirroring Module (`sharing/mirroring.rs`)
- **Purpose**: Ensures package availability and redundancy
- **Strategies**:
  - **Popularity-based**: Mirror frequently accessed packages
  - **Geographic**: Distribute across regions for optimal access
  - **Access Pattern**: Mirror based on usage patterns
  - **Priority-based**: Ensure critical packages are replicated
  - **Adaptive**: Adjust based on network conditions
- **Features**:
  - Configurable replication factors
  - Storage capacity management
  - Health monitoring of mirror nodes
  - Automatic failover

### 4. Discovery Service (`sharing/discovery.rs`)
- **Purpose**: Enable global asset discovery
- **Capabilities**:
  - Local index management
  - Federated search across network
  - Full-text search
  - Fuzzy matching for typos
  - Recommendation engine
  - Usage statistics tracking
- **Performance**:
  - Cached search results
  - Relevance scoring
  - Parallel search across peers

### 5. Sharing Protocols (`sharing/protocols.rs`)
- **Purpose**: Secure and efficient data transfer
- **Features**:
  - STOQ-based transport
  - Bandwidth management and throttling
  - Fair-use policies
  - Permission enforcement
  - Contribution tracking
  - Incentive mechanisms
- **Transfer Priorities**:
  - Critical: System-critical transfers
  - High: User-initiated transfers
  - Normal: Standard operations
  - Low: Background synchronization

### 6. Network Topology (`sharing/topology.rs`)
- **Purpose**: Optimize routing and handle network dynamics
- **Routing Strategies**:
  - Shortest path
  - Lowest latency
  - Highest bandwidth
  - Geographic proximity
  - Load balanced
  - Fault tolerant (multiple paths)
- **Features**:
  - Network partition detection
  - Automatic recovery
  - Node health monitoring
  - Geographic awareness

## Data Flow

```
1. Package Publication
   User → Catalog → SharingManager → Discovery Service → Network

2. Package Discovery
   User → Search Query → Discovery Service → Federated Search → Results

3. Package Installation
   User → Request → SharingManager → Find Best Node → Download → Install

4. Synchronization
   SyncManager → Compare Merkle Trees → Identify Deltas → Transfer → Update

5. Mirroring
   MirrorManager → Identify Popular → Select Nodes → Replicate → Monitor
```

## Permission Model

- **Public**: Accessible to all nodes
- **Private**: Owner-only access
- **Restricted**: Specific nodes only
- **Friends**: Trusted peer group
- **Anonymous**: No identity tracking
- **Verified**: Requires consensus proofs

## Incentive System

### Contribution Tracking
- Bytes uploaded/downloaded
- Upload/download ratio
- Contribution score
- Earned credits

### Reward Calculation
- Based on data contributed
- Ratio multiplier (encourages sharing)
- Network health contribution
- Uptime and reliability

## Performance Characteristics

### Synchronization
- **Incremental sync**: < 100ms for small deltas
- **Full sync**: Scales linearly with library size
- **Merkle tree operations**: O(log n) complexity

### Mirroring
- **Auto-mirror decision**: < 10ms
- **Replication**: Parallel transfers to multiple nodes
- **Health checks**: Periodic, low overhead

### Discovery
- **Local search**: < 5ms for indexed packages
- **Network search**: < 500ms typical (parallel queries)
- **Cache hit rate**: > 80% for popular queries

### Bandwidth Management
- **Per-peer limits**: Configurable (default 1MB/s)
- **Burst allowance**: 5MB for 5 seconds
- **Priority queuing**: Critical transfers first
- **Fair scheduling**: Round-robin for equal priority

## Fault Tolerance

### Network Partitions
- Automatic detection via health checks
- Graceful degradation to local operations
- Queue operations for later sync
- Automatic recovery when partition heals

### Node Failures
- Multiple mirrors ensure availability
- Automatic failover to healthy nodes
- Redistribution of popular packages
- Health score tracking

### Data Integrity
- SHA-256 checksums for all packages
- Merkle tree verification
- Consensus validation for critical packages
- Signature verification

## Security Considerations

### Authentication
- TrustChain certificate validation
- Peer identity verification
- Consensus proof requirements

### Authorization
- Permission-based access control
- Share-level granularity
- Revocable permissions

### Transport Security
- STOQ encryption (QUIC + TLS 1.3)
- Perfect forward secrecy
- Certificate pinning

## Integration with HyperMesh

### Asset Management
- All packages are HyperMesh assets
- Consensus validation for publishing
- Resource allocation via AssetAdapters

### Network Services
- Uses HyperMesh routing infrastructure
- Integrates with TrustChain DNS
- Leverages consensus for verification

### Extension Framework
- Registered as AssetLibraryExtension
- Provides asset handlers for package types
- Exposes sharing APIs via extension interface

## Usage Examples

### Connect to Peer Network
```rust
let config = SharingConfig::default();
let manager = SharingManager::new(config).await?;
let peer_id = manager.connect_peer("peer.hypermesh.online").await?;
```

### Share a Package
```rust
let package = create_package();
manager.share_package(&package, SharePermission::Public).await?;
```

### Search Across Network
```rust
let results = manager.search_packages("virtual machine").await?;
```

### Synchronize with Peer
```rust
manager.sync_with_peer(&peer_id).await?;
```

### Mirror Popular Packages
```rust
let mirrored = manager.auto_mirror_packages().await?;
```

## Configuration

### Sharing Configuration
```rust
SharingConfig {
    node_id: "unique-node-id",
    max_mirror_storage: 10 * 1024 * 1024 * 1024, // 10GB
    max_bandwidth: 10 * 1024 * 1024, // 10MB/s
    replication_factor: 3,
    sync_interval: Duration::from_secs(300),
    auto_mirror_popular: true,
    enable_incentives: true,
    fair_use_limit: 1024 * 1024, // 1MB/s per peer
}
```

## Monitoring and Metrics

### Available Metrics
- Total packages shared/mirrored
- Bandwidth consumed/contributed
- Active peer connections
- Sync success/failure rates
- Cache hit ratios
- Network health score

### Health Indicators
- Node uptime and reliability
- Average response times
- Storage utilization
- Network bandwidth usage
- Error rates and types

## Future Enhancements

1. **Machine Learning Integration**
   - Predictive caching
   - Intelligent routing
   - Anomaly detection

2. **Advanced Consensus**
   - Multi-signature requirements
   - Threshold consensus
   - Time-locked packages

3. **Economic Models**
   - Token-based incentives
   - Storage markets
   - Bandwidth trading

4. **Performance Optimizations**
   - Content-addressed storage
   - Erasure coding
   - Delta compression