# Network-Aware Certificate Strategy Guide

## Overview

STOQ now supports network-specific certificate strategies, allowing a single node to participate in multiple network types with completely isolated trust models. This implementation follows the Multi-Network Trust Architecture specification.

## Network Types and Their Trust Models

### 1. Anonymous Network
- **No certificates required**: Purely ephemeral sessions
- **No identity tracking**: Complete anonymity
- **No trust validation**: All connections accepted
- **Use case**: Privacy-first communications, Tor-like anonymity

```rust
use stoq::transport::{StoqTransport, NetworkType, TransportConfig};

// Create transport for anonymous network
let transport = StoqTransport::new_for_network(
    config,
    NetworkType::Anonymous,
).await?;
```

### 2. P2P Network
- **Self-signed certificates**: Generated locally
- **Direct peer exchange**: No CA involvement
- **Manual trust decisions**: User controls peer acceptance
- **Use case**: Direct peer-to-peer connections, SSH-like trust model

```rust
// Create transport for P2P network
let transport = StoqTransport::new_for_network(
    config,
    NetworkType::P2P,
).await?;

// Access P2P strategy for peer management
if let NetworkType::P2P = network_type {
    let strategy = P2PCertificateStrategy::new(
        node_id,
        common_name,
        ipv6_addresses,
    )?;

    // Add trusted peer
    strategy.add_trusted_peer("peer1".to_string(), peer_cert).await;

    // List trusted peers
    let peers = strategy.list_trusted_peers().await;
}
```

### 3. Federated Network
- **Federation gateway managed**: Certificates from specific gateway
- **Scoped trust**: Trust limited to federation boundary
- **Federation membership**: Only federation members can connect
- **Use case**: Enterprise networks, private federations

```rust
// Create transport for federated network
let transport = StoqTransport::new_for_network(
    config,
    NetworkType::Federated {
        gateway_url: "gateway.company.internal:8443".to_string(),
    },
).await?;
```

### 4. Public Network
- **Global CA managed**: Certificates from trust.hypermesh.online
- **Blockchain registration**: Certificates registered on-chain
- **Full transparency**: Maximum visibility and rewards
- **Use case**: Public HyperMesh network, CAESAR rewards

```rust
// Create transport for public network
let transport = StoqTransport::new_for_network(
    config,
    NetworkType::Public,
).await?;
```

## Certificate Strategy Pattern

The implementation uses a strategy pattern with the following trait:

```rust
#[async_trait]
pub trait CertificateStrategy: Send + Sync {
    /// Get certificate for this network (None for Anonymous)
    async fn get_certificate(&self) -> Result<Option<StoqNodeCertificate>>;

    /// Validate peer certificate in network context
    async fn validate_certificate(&self, cert: &StoqNodeCertificate) -> Result<bool>;

    /// Strategy name for debugging
    fn strategy_name(&self) -> &str;

    /// Check if strategy requires certificates
    fn requires_certificate(&self) -> bool;
}
```

## Implementation Details

### Certificate Manager Integration

The `CertificateManager` now supports three modes:
1. **LocalhostTesting**: Original self-signed for testing
2. **TrustChainProduction**: Original production CA mode
3. **NetworkStrategy**: New network-aware mode

### Backward Compatibility

The original API is fully preserved:

```rust
// Original API still works (defaults to localhost testing)
let transport = StoqTransport::new(config).await?;

// Or for production
let cert_config = CertificateConfig::production(
    node_id,
    common_name,
    ipv6_addresses,
);
```

### Custom Certificate Strategies

You can implement custom strategies:

```rust
pub struct CustomStrategy;

#[async_trait]
impl CertificateStrategy for CustomStrategy {
    async fn get_certificate(&self) -> Result<Option<StoqNodeCertificate>> {
        // Custom certificate generation
    }

    async fn validate_certificate(&self, cert: &StoqNodeCertificate) -> Result<bool> {
        // Custom validation logic
    }

    fn strategy_name(&self) -> &str {
        "Custom"
    }
}

// Use custom strategy
let strategy = Arc::new(CustomStrategy);
let cert_manager = CertificateManager::with_strategy(strategy).await?;
```

## Security Considerations

### Anonymous Network
- Ephemeral certificates generated for QUIC compatibility
- No persistent identity storage
- All validation bypassed

### P2P Network
- Trust decisions stored locally
- No automatic trust transitivity
- User responsible for peer verification

### Federated Network
- Trust scoped to federation
- Gateway controls membership
- No cross-federation communication

### Public Network
- Full blockchain validation
- Certificate Transparency logs
- Maximum security and audit trail

## Migration Guide

### From Original API

```rust
// Before: Single trust model
let transport = StoqTransport::new(config).await?;

// After: Network-specific trust
let transport = StoqTransport::new_for_network(
    config,
    NetworkType::P2P, // or Anonymous, Federated, Public
).await?;
```

### Multi-Network Participation

A single node can participate in multiple networks simultaneously:

```rust
// Create transports for different networks
let anon_transport = StoqTransport::new_for_network(
    config.clone(),
    NetworkType::Anonymous,
).await?;

let p2p_transport = StoqTransport::new_for_network(
    config.clone(),
    NetworkType::P2P,
).await?;

let public_transport = StoqTransport::new_for_network(
    config.clone(),
    NetworkType::Public,
).await?;

// Each transport has isolated trust model
```

## Testing

Run the network strategy tests:

```bash
cargo test --test network_strategy_test
```

## Performance Impact

- **Anonymous**: Fastest (no validation overhead)
- **P2P**: Fast (local validation only)
- **Federated**: Moderate (gateway validation)
- **Public**: Slower (blockchain validation)

## Future Enhancements

1. **Certificate rotation**: Automatic renewal for long-running connections
2. **Trust propagation**: Optional trust transitivity for P2P
3. **Federation bridging**: Cross-federation communication (with policy)
4. **Performance optimization**: Certificate caching improvements