# Multi-Network Trust Architecture & Quality Gates

## Executive Summary

This document defines the comprehensive architecture for BlockMatrix's multi-network trust system, where a single node can simultaneously connect to all four network types (Anonymous, P2P, Federated, Public) with completely isolated trust models and no cross-network data leakage.

**Critical Design Principle**: Trust anchors are network-specific, NOT universal. The system architecture is fundamentally wrong in attempting to connect to `trust.hypermesh.online` for all privacy modes.

## 1. Network-Specific Trust Models

### 1.1 Anonymous Network Trust Model

**Core Principle**: No persistent identity, no trust validation, ephemeral everything.

**Trust Architecture**:
```rust
struct AnonymousTrust {
    // NO certificate requests ever
    certificate: None,

    // Ephemeral session keys only
    session_key: EphemeralKey,

    // No signing, no validation
    signature_validation: Disabled,

    // Accept all connections
    peer_validation: AcceptAll,

    // No trust anchor
    trust_anchor: None,
}
```

**Connection Flow**:
1. Generate ephemeral session key (destroyed on disconnect)
2. Accept incoming connections without validation
3. No certificate exchange
4. No identity persistence
5. No trust propagation

**Implementation Requirements**:
- NEVER request certificates from ANY CA
- Use ephemeral Diffie-Hellman for key exchange
- Destroy all session data on disconnect
- No peer database or connection history
- Similar to Tor hidden services

### 1.2 P2P Network Trust Model

**Core Principle**: Direct peer trust exchange without intermediary CA.

**Trust Architecture**:
```rust
struct P2PTrust {
    // Self-signed certificate for peer exchange
    local_certificate: SelfSignedCert,

    // Peer certificates stored locally
    peer_certificates: HashMap<PeerId, PeerCertificate>,

    // Manual trust decisions
    trust_decisions: HashMap<PeerId, TrustDecision>,

    // No CA involved
    trust_anchor: None,

    // Direct peer validation
    validation_method: DirectPeerExchange,
}
```

**Connection Flow**:
1. Generate self-signed certificate locally
2. User provides peer address (out-of-band exchange)
3. Connect to peer directly
4. Exchange self-signed certificates
5. User manually accepts/rejects peer certificate
6. Store trust decision locally
7. No network-wide trust propagation

**Implementation Requirements**:
- Certificate exchange happens peer-to-peer
- Trust decisions stored locally per peer
- No automatic trust transitivity
- User controls all trust decisions
- Similar to SSH known_hosts model

### 1.3 Federated Network Trust Model

**Core Principle**: Federation gateway acts as trust anchor for that specific federation.

**Trust Architecture**:
```rust
struct FederatedTrust {
    // Federation-specific gateway
    federation_gateway: String, // e.g., "gateway.bank.internal"

    // Certificate from federation CA
    federation_certificate: FederationCert,

    // Federation members trust list
    federation_members: HashSet<NodeId>,

    // Federation-specific trust anchor
    trust_anchor: FederationCA,

    // Validation within federation only
    validation_scope: FederationOnly,
}
```

**Connection Flow**:
1. User specifies federation gateway URL
2. Connect to federation gateway (NOT trust.hypermesh.online)
3. Submit federation join request
4. Gateway validates (may require approval)
5. Receive federation-signed certificate
6. Certificate only valid within federation
7. Connect to other federation members
8. Trust limited to federation boundary

**Implementation Requirements**:
- Each federation has its own CA/gateway
- Certificates scoped to federation
- No cross-federation trust by default
- Federation gateway URL user-configurable
- Example: `bank.internal`, `hospital.federation`, `government.fed`

### 1.4 Public Network Trust Model

**Core Principle**: Global CA with blockchain-registered certificates.

**Trust Architecture**:
```rust
struct PublicTrust {
    // Global trust anchor
    global_ca: "trust.hypermesh.online",

    // Blockchain-registered certificate
    blockchain_certificate: BlockchainCert,

    // Full Proof of State validation
    proof_requirements: AllFourProofs,

    // DNS-as-Asset registration
    dns_asset: AssetId,

    // Maximum transparency
    validation_method: FullBlockchainValidation,
}
```

**Connection Flow**:
1. Connect to trust.hypermesh.online
2. Submit Proof of State (all 4 proofs)
3. Receive blockchain-registered certificate
4. Register DNS-as-Asset on blockchain
5. Certificate globally verifiable
6. Enable CAESAR rewards
7. Full network participation

**Implementation Requirements**:
- Only THIS mode uses trust.hypermesh.online
- Requires full blockchain validation
- DNS registration as blockchain asset
- Maximum rewards and visibility
- Full audit trail on blockchain

## 2. Multi-Network Connection Management

### 2.1 Connection Context Architecture

```rust
/// Each network maintains completely isolated context
pub struct NetworkConnection {
    /// Network type determines trust model
    network_type: NetworkType,

    /// Trust anchor (None for Anonymous/P2P)
    trust_anchor: Option<TrustAnchor>,

    /// Network-specific certificate
    certificate: Option<NetworkCertificate>,

    /// Isolated STOQ transport instance
    stoq_transport: StoqTransport,

    /// Network-specific privacy settings
    privacy_config: PrivacyConfig,

    /// Isolated connection pool
    connection_pool: ConnectionPool,

    /// Network-specific asset visibility
    visible_assets: HashSet<AssetId>,
}

/// Node manages multiple network connections
pub struct MultiNetworkNode {
    /// All network connections (isolated)
    connections: HashMap<NetworkId, NetworkConnection>,

    /// Packet filter preventing cross-network leakage
    packet_filter: IsolationFilter,

    /// Per-network resource allocation
    resource_allocation: HashMap<NetworkId, ResourceQuota>,
}
```

### 2.2 Isolation Requirements

**Critical**: Zero packet leakage between networks.

```rust
/// Isolation filter prevents any cross-network communication
pub struct IsolationFilter {
    /// Track packet origin network
    packet_origin: HashMap<PacketId, NetworkId>,

    /// Enforce strict isolation
    pub fn validate_packet(&self, packet: &Packet) -> Result<()> {
        let origin_network = self.packet_origin.get(&packet.id)?;
        let destination_network = packet.destination_network;

        if origin_network != destination_network {
            return Err(IsolationViolation);
        }

        Ok(())
    }
}
```

### 2.3 Asset Visibility Control

```rust
/// User controls which assets are visible to which networks
pub struct AssetVisibilityControl {
    /// Asset -> Networks mapping
    asset_visibility: HashMap<AssetId, HashSet<NetworkId>>,

    /// Default visibility for new assets
    default_visibility: VisibilityPolicy,

    pub fn set_asset_visibility(
        &mut self,
        asset: AssetId,
        networks: HashSet<NetworkId>
    ) {
        self.asset_visibility.insert(asset, networks);
    }
}
```

## 3. Bootstrap Flow Per Network Type

### 3.1 Anonymous Network Bootstrap

```rust
async fn bootstrap_anonymous() -> Result<NetworkConnection> {
    // Step 1: Generate ephemeral keys (no certificate)
    let ephemeral_key = generate_ephemeral_key();

    // Step 2: Create STOQ listener in ephemeral mode
    let stoq = StoqTransport::new_ephemeral(ephemeral_key);

    // Step 3: Configure for anonymous mode
    stoq.set_mode(StoqMode::Anonymous);
    stoq.disable_identity_tracking();
    stoq.disable_certificate_validation();

    // Step 4: Start accepting connections
    stoq.listen_ephemeral().await?;

    // Step 5: No peer database, no persistence
    // All connections are ephemeral

    Ok(NetworkConnection {
        network_type: NetworkType::Anonymous,
        trust_anchor: None,
        certificate: None,
        stoq_transport: stoq,
        // ...
    })
}
```

### 3.2 P2P Network Bootstrap

```rust
async fn bootstrap_p2p(peer_addresses: Vec<String>) -> Result<NetworkConnection> {
    // Step 1: Generate self-signed certificate
    let self_signed = generate_self_signed_cert();

    // Step 2: Create STOQ with P2P mode
    let stoq = StoqTransport::new_p2p(self_signed.clone());

    // Step 3: Connect to specified peers
    for peer_addr in peer_addresses {
        // User-initiated connection
        let peer_conn = stoq.connect_peer(&peer_addr).await?;

        // Exchange certificates
        let peer_cert = peer_conn.exchange_certificate(self_signed.clone()).await?;

        // User manually accepts/rejects
        if user_accepts_peer_cert(&peer_cert) {
            stoq.add_trusted_peer(peer_addr, peer_cert);
        }
    }

    // Step 4: Maintain peer-specific trust

    Ok(NetworkConnection {
        network_type: NetworkType::P2P,
        trust_anchor: None,
        certificate: Some(self_signed),
        stoq_transport: stoq,
        // ...
    })
}
```

### 3.3 Federated Network Bootstrap

```rust
async fn bootstrap_federated(federation_gateway: String) -> Result<NetworkConnection> {
    // Step 1: Generate self-signed for initial connection
    let self_signed = generate_self_signed_cert();

    // Step 2: Connect to federation gateway (NOT trust.hypermesh.online!)
    let gateway_conn = connect_to_gateway(&federation_gateway).await?;

    // Step 3: Request federation membership
    let join_request = FederationJoinRequest {
        node_id: local_node_id(),
        requested_role: NetworkRole::Member,
        credentials: user_federation_credentials(),
    };

    // Step 4: Gateway validates and issues certificate
    let federation_cert = gateway_conn.request_certificate(join_request).await?;

    // Step 5: Create STOQ with federation certificate
    let stoq = StoqTransport::new_federated(federation_cert.clone());

    // Step 6: Connect to other federation members
    let federation_members = gateway_conn.get_member_list().await?;
    for member in federation_members {
        stoq.connect_federation_member(member).await?;
    }

    Ok(NetworkConnection {
        network_type: NetworkType::Federated,
        trust_anchor: Some(TrustAnchor::Federation(federation_gateway)),
        certificate: Some(federation_cert),
        stoq_transport: stoq,
        // ...
    })
}
```

### 3.4 Public Network Bootstrap

```rust
async fn bootstrap_public() -> Result<NetworkConnection> {
    // Step 1: Generate self-signed for initial connection
    let self_signed = generate_self_signed_cert();

    // Step 2: Connect to global CA (ONLY for public mode!)
    let ca_conn = connect_to_ca("trust.hypermesh.online").await?;

    // Step 3: Submit Proof of State
    let proof_of_state = generate_proof_of_state()?;

    // Step 4: Request blockchain-registered certificate
    let blockchain_cert = ca_conn.request_blockchain_cert(proof_of_state).await?;

    // Step 5: Register DNS-as-Asset
    let dns_asset = register_dns_asset(desired_name).await?;

    // Step 6: Create STOQ with blockchain certificate
    let stoq = StoqTransport::new_public(blockchain_cert.clone());

    // Step 7: Enable CAESAR rewards
    enable_caesar_rewards(&blockchain_cert).await?;

    Ok(NetworkConnection {
        network_type: NetworkType::Public,
        trust_anchor: Some(TrustAnchor::Global("trust.hypermesh.online")),
        certificate: Some(blockchain_cert),
        stoq_transport: stoq,
        // ...
    })
}
```

## 4. Quality Gates & Metrics

### QG1: Self-Sufficient Bootstrap ✓
- [ ] Node starts without network connectivity
- [ ] Self-signed certificate created locally
- [ ] Localhost DNS resolution functional
- [ ] Unique genesis block created
- [ ] No external dependencies required

**Test**:
```bash
# Disconnect network
sudo ip link set eth0 down
# Start node
./blockmatrix --bootstrap
# Verify: Node running, localhost cert created, DNS resolves localhost
```

### QG2: Privacy Mode Isolation ✓
- [ ] Anonymous connections never leak identity
- [ ] P2P connections don't share peer lists
- [ ] Federated networks isolated from each other
- [ ] Public network can be completely disabled

**Test**:
```rust
#[test]
async fn test_privacy_isolation() {
    let node = MultiNetworkNode::new();

    // Join anonymous network
    node.join_network(NetworkType::Anonymous).await?;

    // Verify no certificate requests
    assert!(node.certificate_requests().is_empty());

    // Verify ephemeral keys destroyed on disconnect
    let conn = node.get_connection(NetworkType::Anonymous);
    conn.disconnect().await;
    assert!(conn.session_keys().is_empty());
}
```

### QG3: Multi-Network Simultaneous Operation ✓
- [ ] Single node connected to all 4 types simultaneously
- [ ] Each network has isolated STOQ connection
- [ ] Certificate validation scoped per network
- [ ] No cross-network state leakage

**Test**:
```rust
#[test]
async fn test_multi_network_simultaneous() {
    let node = MultiNetworkNode::new();

    // Join all 4 network types
    node.join_network(NetworkType::Anonymous).await?;
    node.join_network(NetworkType::P2P).await?;
    node.join_network(NetworkType::Federated).await?;
    node.join_network(NetworkType::Public).await?;

    // Verify 4 isolated connections
    assert_eq!(node.active_networks().len(), 4);

    // Verify isolation
    for (net1, net2) in node.network_pairs() {
        assert!(!can_communicate(net1, net2));
    }
}
```

### QG4: Trust Model Correctness ✓
- [ ] Anonymous: No cert validation, no signing
- [ ] P2P: Direct peer exchange only
- [ ] Federated: Federation gateway CA only
- [ ] Public: trust.hypermesh.online CA only

**Test Matrix**:
| Network Type | Certificate Source | Trust Anchor | Validation |
|-------------|-------------------|--------------|------------|
| Anonymous | None | None | Disabled |
| P2P | Self-signed | None | Manual peer |
| Federated | Federation gateway | gateway.federation.example | Federation scope |
| Public | trust.hypermesh.online | Global CA | Blockchain |

### QG5: User Control ✓
- [ ] User can disable public network entirely
- [ ] User specifies federation gateway URL
- [ ] User accepts/rejects P2P peers
- [ ] User controls asset sharing per network

**Test**:
```rust
#[test]
async fn test_user_control() {
    let config = NodeConfig {
        disable_public: true,
        federation_gateway: Some("gateway.mycompany.internal"),
        p2p_peers: vec!["peer1.local", "peer2.local"],
        asset_visibility: AssetVisibilityPolicy::Explicit,
    };

    let node = MultiNetworkNode::with_config(config);

    // Verify public disabled
    assert!(node.join_network(NetworkType::Public).is_err());

    // Verify federation uses custom gateway
    node.join_network(NetworkType::Federated).await?;
    assert_eq!(node.federation_gateway(), "gateway.mycompany.internal");
}
```

### QG6: Network Transition ✓
- [ ] Can transition Private → any tier
- [ ] Can downgrade Public → Private
- [ ] Can connect/disconnect independently
- [ ] No disruption to other networks

**Test**:
```rust
#[test]
async fn test_network_transitions() {
    let node = MultiNetworkNode::new();

    // Start with private
    node.set_mode(NetworkType::Private);

    // Transition to public
    node.join_network(NetworkType::Public).await?;

    // Add anonymous (shouldn't affect public)
    node.join_network(NetworkType::Anonymous).await?;

    // Verify both active
    assert!(node.is_connected(NetworkType::Public));
    assert!(node.is_connected(NetworkType::Anonymous));

    // Disconnect public
    node.leave_network(NetworkType::Public).await?;

    // Anonymous still active
    assert!(node.is_connected(NetworkType::Anonymous));
}
```

### QG7: Certificate Lifecycle ✓
- [ ] Self-signed cert never leaves localhost
- [ ] Anonymous uses ephemeral certs
- [ ] P2P certs exchanged out-of-band
- [ ] Federated certs scoped to federation
- [ ] Public certs blockchain-registered

**Test**:
```rust
#[test]
async fn test_certificate_lifecycle() {
    // Anonymous: ephemeral
    let anon_conn = bootstrap_anonymous().await?;
    assert!(anon_conn.certificate.is_none());

    // P2P: self-signed exchange
    let p2p_conn = bootstrap_p2p(vec!["peer.local"]).await?;
    assert!(p2p_conn.certificate.unwrap().is_self_signed());

    // Federated: gateway-issued
    let fed_conn = bootstrap_federated("gateway.fed").await?;
    assert_eq!(fed_conn.certificate.unwrap().issuer(), "gateway.fed");

    // Public: blockchain-registered
    let pub_conn = bootstrap_public().await?;
    assert!(pub_conn.certificate.unwrap().is_blockchain_registered());
}
```

### QG8: Data Isolation ✓
- [ ] Assets shared to Anonymous are truly anonymous
- [ ] P2P assets only visible to specific peer
- [ ] Federated assets contained within federation
- [ ] Public assets globally discoverable
- [ ] User controls which networks see which assets

**Test**:
```rust
#[test]
async fn test_asset_isolation() {
    let node = MultiNetworkNode::new();
    let asset = create_test_asset();

    // Share to anonymous only
    node.share_asset(asset.id, vec![NetworkType::Anonymous]).await?;

    // Verify not visible in other networks
    assert!(!node.is_asset_visible(asset.id, NetworkType::P2P));
    assert!(!node.is_asset_visible(asset.id, NetworkType::Federated));
    assert!(!node.is_asset_visible(asset.id, NetworkType::Public));
}
```

## 5. Implementation Gaps Analysis

### 5.1 Current State Problems

**Critical Issue**: STOQ's certificate manager ALWAYS tries to connect to a single CA regardless of privacy mode:

```rust
// WRONG - Current implementation
pub fn production(node_id: String, common_name: String, ipv6_addresses: Vec<Ipv6Addr>) -> Self {
    Self {
        mode: CertificateMode::TrustChainProduction,
        trustchain_endpoint: Some("quic://[::1]:8443".to_string()), // ALWAYS same CA!
    }
}
```

**What's Missing**:
1. No network-type-aware certificate handling
2. No federation gateway support
3. No P2P certificate exchange
4. No anonymous/ephemeral mode
5. Single trust anchor hardcoded

### 5.2 Required Refactoring

**Certificate Manager Refactor**:
```rust
// CORRECT - Network-aware implementation needed
pub enum CertificateStrategy {
    Anonymous,                          // No certificates
    P2P(PeerCertificateExchange),      // Direct exchange
    Federated(FederationGateway),      // Federation-specific CA
    Public(GlobalCA),                   // trust.hypermesh.online
}

pub struct NetworkAwareCertificateManager {
    strategies: HashMap<NetworkType, CertificateStrategy>,

    pub async fn get_certificate(&self, network: NetworkType) -> Result<Option<Certificate>> {
        match self.strategies.get(&network) {
            Some(CertificateStrategy::Anonymous) => Ok(None),
            Some(CertificateStrategy::P2P(exchange)) => exchange.get_peer_cert().await,
            Some(CertificateStrategy::Federated(gateway)) => gateway.request_cert().await,
            Some(CertificateStrategy::Public(ca)) => ca.request_blockchain_cert().await,
            None => Err(NetworkNotConfigured),
        }
    }
}
```

### 5.3 File Structure Gaps

**Missing Components**:
```
/blockmatrix/src/network/
  mod.rs                    ✓ Exists (basic)
  multi_network.rs          ✗ Missing - Need multi-network manager
  trust_models/
    anonymous.rs            ✗ Missing - Anonymous trust model
    p2p.rs                 ✗ Missing - P2P trust model
    federated.rs           ✗ Missing - Federation trust model
    public.rs              ✗ Missing - Public trust model
  isolation/
    packet_filter.rs       ✗ Missing - Cross-network isolation
    connection_pools.rs    ✗ Missing - Per-network pools
  certificate/
    exchange.rs            ✗ Missing - P2P cert exchange
    federation.rs          ✗ Missing - Federation gateway client
```

## 6. Proposed Architecture

### 6.1 File Structure

```
/blockmatrix/src/network/
  mod.rs                    # NetworkManager facade
  multi_network.rs          # Multi-network coordinator

  trust/
    mod.rs                  # Trust model traits
    anonymous.rs            # Anonymous network handler
    p2p.rs                  # P2P network handler
    federated.rs            # Federated network handler
    public.rs               # Public network handler

  isolation/
    mod.rs                  # Isolation manager
    packet_filter.rs        # Packet-level isolation
    connection_pool.rs      # Per-network STOQ pools
    resource_quota.rs       # Per-network resource limits

  certificate/
    mod.rs                  # Certificate strategy trait
    ephemeral.rs           # Ephemeral key management
    peer_exchange.rs       # P2P certificate exchange
    federation_client.rs   # Federation gateway client
    blockchain_ca.rs       # Public CA client
```

### 6.2 Core Interfaces

```rust
/// Network handler trait - implemented by each network type
#[async_trait]
pub trait NetworkHandler: Send + Sync {
    /// Bootstrap the network connection
    async fn bootstrap(&self, config: NetworkConfig) -> Result<NetworkConnection>;

    /// Connect to the network
    async fn connect(&self) -> Result<()>;

    /// Validate a peer in this network's context
    async fn validate_peer(&self, peer: &PeerInfo) -> Result<bool>;

    /// Handle asset request with network-specific rules
    async fn handle_asset_request(&self, request: AssetRequest) -> Result<AssetResponse>;

    /// Disconnect from network
    async fn disconnect(&self) -> Result<()>;
}

/// Certificate strategy trait
#[async_trait]
pub trait CertificateStrategy: Send + Sync {
    /// Get certificate for this network (None for Anonymous)
    async fn get_certificate(&self) -> Result<Option<Certificate>>;

    /// Validate peer certificate in network context
    async fn validate_certificate(&self, cert: &Certificate) -> Result<bool>;

    /// Exchange certificates (P2P only)
    async fn exchange_certificates(&self, peer: &PeerInfo) -> Result<()>;
}

/// Isolation manager trait
#[async_trait]
pub trait IsolationManager: Send + Sync {
    /// Validate packet doesn't cross network boundary
    async fn validate_packet(&self, packet: &Packet) -> Result<()>;

    /// Get isolated connection pool for network
    async fn get_connection_pool(&self, network: NetworkId) -> Result<ConnectionPool>;

    /// Check for isolation violations
    async fn check_violations(&self) -> Vec<IsolationViolation>;
}
```

### 6.3 Multi-Network Coordinator

```rust
/// Central coordinator for multi-network participation
pub struct MultiNetworkCoordinator {
    /// Network handlers by type
    handlers: HashMap<NetworkType, Box<dyn NetworkHandler>>,

    /// Active network connections
    connections: Arc<RwLock<HashMap<NetworkId, NetworkConnection>>>,

    /// Isolation manager
    isolation: Arc<dyn IsolationManager>,

    /// Certificate strategies
    cert_strategies: HashMap<NetworkType, Box<dyn CertificateStrategy>>,

    /// Asset visibility controller
    asset_visibility: Arc<RwLock<AssetVisibilityControl>>,
}

impl MultiNetworkCoordinator {
    /// Join a network with specified type
    pub async fn join_network(
        &self,
        network_type: NetworkType,
        config: NetworkConfig,
    ) -> Result<NetworkId> {
        // Get appropriate handler
        let handler = self.handlers.get(&network_type)
            .ok_or_else(|| anyhow!("Unknown network type"))?;

        // Bootstrap with network-specific logic
        let connection = handler.bootstrap(config).await?;

        // Store isolated connection
        let network_id = connection.network_id();
        self.connections.write().await.insert(network_id, connection);

        // Configure isolation
        self.isolation.configure_network(network_id, network_type).await?;

        Ok(network_id)
    }

    /// Leave a network
    pub async fn leave_network(&self, network_id: NetworkId) -> Result<()> {
        // Get connection
        let connection = self.connections.write().await.remove(&network_id)
            .ok_or_else(|| anyhow!("Network not found"))?;

        // Disconnect gracefully
        connection.disconnect().await?;

        // Remove isolation configuration
        self.isolation.remove_network(network_id).await?;

        Ok(())
    }
}
```

## 7. Migration Plan

### Phase 1: Foundation (Week 1)
1. Create network trust model implementations
2. Implement isolation manager
3. Add per-network certificate strategies
4. Create multi-network coordinator

### Phase 2: Integration (Week 2)
1. Integrate with existing STOQ transport
2. Update bootstrap process for network awareness
3. Implement packet-level isolation
4. Add federation gateway support

### Phase 3: Testing (Week 3)
1. Implement all quality gates
2. Multi-network stress testing
3. Isolation violation testing
4. Certificate lifecycle testing

### Phase 4: Production (Week 4)
1. Deploy federation gateway examples
2. Documentation and guides
3. Migration tooling for existing nodes
4. Performance optimization

## 8. Critical Design Decisions

### Decision 1: Trust Anchor Separation
**Decision**: Each network type has its own trust anchor (or none).
**Rationale**: Prevents trust leakage and maintains isolation.

### Decision 2: No Cross-Network Bridging
**Decision**: Networks are completely isolated, no packet routing between them.
**Rationale**: Security and privacy guarantee.

### Decision 3: User-Controlled Federation Gateway
**Decision**: Federation gateway URL is user-specified, not hardcoded.
**Rationale**: Allows private federations without central control.

### Decision 4: Ephemeral Anonymous Mode
**Decision**: Anonymous mode uses ephemeral keys, no certificates.
**Rationale**: True anonymity requires no persistent identity.

### Decision 5: P2P Manual Trust
**Decision**: P2P requires manual certificate acceptance.
**Rationale**: No automatic trust transitivity in P2P networks.

## 9. Security Considerations

### Isolation Guarantees
- Packet-level filtering prevents cross-network communication
- Separate STOQ transport instances per network
- No shared memory between network handlers
- Audit logging for isolation violations

### Certificate Security
- Anonymous: No certificates to compromise
- P2P: User controls trust decisions
- Federated: Trust limited to federation boundary
- Public: Blockchain provides audit trail

### Attack Vectors Mitigated
- Cross-network data leakage: Prevented by isolation
- Certificate confusion: Network-scoped validation
- Trust elevation: No automatic trust transitivity
- Privacy degradation: User controls network participation

## 10. Conclusion

This architecture provides a complete solution for multi-network trust with proper isolation. The key insight is that **trust anchors must be network-specific**, not universal. The current implementation's attempt to use `trust.hypermesh.online` for all modes is fundamentally wrong and must be refactored.

**Next Steps**:
1. Refactor STOQ certificate manager for network awareness
2. Implement network-specific trust models
3. Add isolation manager with packet filtering
4. Create federation gateway support
5. Test with all quality gates

This architecture ensures that a single node can participate in multiple networks simultaneously while maintaining complete isolation and network-specific trust models.