# Multi-Network Coordinator Usage Guide

## Overview

The Multi-Network Coordinator enables a single BlockMatrix node to participate in multiple network types simultaneously while maintaining complete isolation between networks. This allows organizations to operate across different trust boundaries without compromising security.

## Network Types

### 1. Anonymous Network
- **Trust Model**: No persistent identity, ephemeral connections
- **Use Case**: Completely anonymous participation
- **Features**:
  - No certificate validation
  - Ephemeral session keys
  - No identity tracking
  - No trust propagation

### 2. P2P Network
- **Trust Model**: Direct peer-to-peer with self-signed certificates
- **Use Case**: Trusted peer groups
- **Features**:
  - Manual peer trust decisions
  - Self-signed certificate exchange
  - No central authority
  - SSH-like known_hosts model

### 3. Federated Network
- **Trust Model**: Federation gateway acts as trust anchor
- **Use Case**: Enterprise federations
- **Features**:
  - Federation-scoped trust
  - Gateway-issued certificates
  - Limited to federation members
  - Custom gateway URLs

### 4. Public Network
- **Trust Model**: Global CA with blockchain-registered certificates
- **Use Case**: Public HyperMesh participation
- **Features**:
  - Full Proof of State validation
  - Blockchain-registered certificates
  - DNS-as-Asset registration
  - CAESAR rewards enabled

## Basic Usage

### Creating the Coordinator

```rust
use blockmatrix::network::{
    multi_network::{MultiNetworkCoordinator, NetworkConfig},
    isolation::DefaultIsolationManager,
};
use std::sync::Arc;

// Create with default isolation
let mut coordinator = MultiNetworkCoordinator::new_default();

// Or with custom isolation manager
let isolation = Arc::new(DefaultIsolationManager::new());
let mut coordinator = MultiNetworkCoordinator::new(isolation);
```

### Joining Networks

#### Anonymous Network

```rust
use blockmatrix::network::trust::NetworkType;

let network_id = coordinator.join_network(
    NetworkType::Anonymous,
    NetworkConfig::anonymous(),
).await?;
```

#### P2P Network

```rust
let network_id = coordinator.join_network(
    NetworkType::P2P,
    NetworkConfig::p2p(vec![
        "peer1.local:8080".to_string(),
        "peer2.local:8080".to_string(),
    ]),
).await?;
```

#### Federated Network

```rust
let network_id = coordinator.join_network(
    NetworkType::Federated {
        gateway_url: "gateway.company.internal".to_string()
    },
    NetworkConfig::federated("gateway.company.internal".to_string()),
).await?;
```

#### Public Network

```rust
use blockmatrix::network::trust::ProofOfState;

let proof = ProofOfState {
    proof_of_space: vec![/* space proof */],
    proof_of_stake: vec![/* stake proof */],
    proof_of_work: vec![/* work proof */],
    proof_of_time: vec![/* time proof */],
};

let network_id = coordinator.join_network(
    NetworkType::Public,
    NetworkConfig::public(
        "mynode.hypermesh.online".to_string(),
        proof,
    ),
).await?;
```

### Managing Asset Visibility

Assets can be made visible to specific networks only:

```rust
use lib::assets::AssetId;

// Create or get an asset
let asset_id = AssetId { /* ... */ };

// Make visible to specific networks
coordinator.set_asset_visibility(
    asset_id,
    vec![network1_id, network2_id],
).await?;
```

### Handling Asset Requests

Process asset requests with network-specific authorization:

```rust
let response = coordinator.handle_asset_request(
    network_id,
    asset_id,
).await?;

if response.authorized {
    // Asset is visible to this network
    println!("Access granted");
} else {
    // Asset not visible to this network
    println!("Access denied");
}
```

### Querying Network Status

```rust
// Get all active networks
let active_networks = coordinator.active_networks().await;

// Check if connected to specific network
let is_connected = coordinator.is_connected(network_id).await;

// Get network type
let network_type = coordinator.get_network_type(&network_id).await;

// Get statistics
let stats = coordinator.get_network_stats().await;
println!("Anonymous: {}", stats.anonymous_count);
println!("P2P: {}", stats.p2p_count);
println!("Federated: {}", stats.federated_count);
println!("Public: {}", stats.public_count);

// List all networks with types
for (id, network_type) in coordinator.list_networks().await {
    println!("{}: {:?}", id, network_type);
}
```

### Leaving Networks

```rust
// Leave a specific network
coordinator.leave_network(network_id).await?;
```

## Advanced Configuration

### Custom Network Configuration

```rust
let config = NetworkConfig {
    peer_addresses: vec!["peer1:8080".to_string()],
    federation_gateway: Some("gateway.url".to_string()),
    dns_name: Some("node.name".to_string()),
    proof_of_state: Some(proof),
    stoq_port: Some(8443),
};
```

### Node-Level Configuration

```rust
use blockmatrix::network::config::{NodeConfig, AssetVisibilityPolicy};

let node_config = NodeConfig {
    disable_public: false,
    disable_anonymous: false,
    default_federation_gateway: Some("default.gateway".to_string()),
    default_p2p_peers: vec!["peer1".to_string()],
    max_networks: 10,
    resource_limits: ResourceLimits {
        cpu_per_network: 25.0,
        memory_mb_per_network: 2048,
        storage_gb_per_network: 50,
        bandwidth_mbps_per_network: 100,
    },
    asset_visibility_policy: AssetVisibilityPolicy::Explicit,
};
```

## Isolation Guarantees

The Multi-Network Coordinator ensures complete isolation between networks:

1. **Packet-Level Isolation**: No packets can cross network boundaries
2. **Connection Pool Isolation**: Each network has its own connection pool
3. **Resource Quota Isolation**: Per-network resource limits
4. **Certificate Isolation**: Network-specific certificate validation
5. **Asset Visibility Control**: Fine-grained per-network asset access

## Security Considerations

### Network-Specific Trust Models
- Anonymous: No trust validation, ephemeral everything
- P2P: Manual peer trust decisions
- Federated: Trust limited to federation boundary
- Public: Full blockchain validation

### Isolation Enforcement
- Packets crossing network boundaries are blocked
- Violations are logged for audit
- No shared state between networks
- Independent STOQ transport instances

### Asset Protection
- Assets must be explicitly shared with networks
- Default policy is private (no visibility)
- Network authorization checked on every request

## Example Scenarios

### Enterprise Multi-Network Setup

```rust
// Company operates on multiple networks simultaneously

// 1. Public network for customer-facing services
let public_id = coordinator.join_network(
    NetworkType::Public,
    NetworkConfig::public("company.hypermesh.online".to_string(), proof),
).await?;

// 2. Federated network for B2B partners
let federated_id = coordinator.join_network(
    NetworkType::Federated {
        gateway_url: "partners.company.internal".to_string()
    },
    NetworkConfig::federated("partners.company.internal".to_string()),
).await?;

// 3. P2P network for internal team
let p2p_id = coordinator.join_network(
    NetworkType::P2P,
    NetworkConfig::p2p(internal_peers),
).await?;

// 4. Anonymous network for research
let anon_id = coordinator.join_network(
    NetworkType::Anonymous,
    NetworkConfig::anonymous(),
).await?;
```

### Privacy-Preserving Asset Sharing

```rust
// Share different assets with different networks

// Public catalog visible to all
coordinator.set_asset_visibility(
    public_catalog_asset,
    vec![public_id, federated_id, p2p_id, anon_id],
).await?;

// Internal data only for P2P network
coordinator.set_asset_visibility(
    internal_data_asset,
    vec![p2p_id],
).await?;

// Partner data for federated network
coordinator.set_asset_visibility(
    partner_data_asset,
    vec![federated_id],
).await?;

// Anonymous research data
coordinator.set_asset_visibility(
    research_asset,
    vec![anon_id],
).await?;
```

## Running the Example

```bash
# Run the multi-network coordinator example
cargo run --example multi_network_coordinator

# With detailed logging
RUST_LOG=info cargo run --example multi_network_coordinator
```

## Testing

```bash
# Run unit tests
cargo test -p blockmatrix network::multi_network

# Run integration tests
cargo test -p blockmatrix --test multi_network_integration

# Run with logging
RUST_LOG=debug cargo test -p blockmatrix network::multi_network
```

## Troubleshooting

### Common Issues

1. **Network Join Failures**
   - Verify network configuration is valid
   - Check network connectivity
   - Ensure required ports are open

2. **Asset Visibility Issues**
   - Confirm network is connected
   - Verify asset ID is correct
   - Check visibility configuration

3. **Isolation Violations**
   - Review packet routing
   - Check for configuration errors
   - Examine violation logs

### Debug Commands

```rust
// Check isolation violations
let violations = isolation.check_violations().await;
for violation in violations {
    println!("Violation: {:?}", violation);
}

// Clear violations after review
isolation.clear_violations().await?;
```

## Performance Considerations

- Each network connection uses independent resources
- Asset visibility checks are O(1) lookups
- Network isolation adds minimal overhead
- Connection pools are lazily initialized

## Future Enhancements

- Automatic network discovery
- Dynamic trust level adjustment
- Cross-network asset bridges (with explicit approval)
- Network reputation systems
- Advanced routing algorithms