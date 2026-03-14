# Phase 2: Domains & Network Naming — Complete Implementation Plan

## 1. Overview

Phase 2 transforms HyperMesh domains from flat DNS records into first-class blockchain assets that simultaneously create Network-scope blockchains. The core architectural insight is that **a domain IS a network**. Registering `persist.hypermesh` creates a Network-scope chain identified by `BLAKE3("persist.hypermesh")`; creating subdomain `home.persist.hypermesh` creates a nested Network chain under its parent. Joining a domain means joining its network chain via `SyncManager::join_network()`.

This phase builds on Phase 1 (daemon/client IPC) and integrates with the existing DNS module at `/home/persist/hypermesh/core/blockmatrix/src/dns/`, the sync manager at `/home/persist/hypermesh/core/blockmatrix/src/blockchain/sync_manager.rs`, the gateway domain router at `/home/persist/hypermesh/core/gateway/src/domain_router.rs`, and the node binary at `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs`.

### Architecture Diagram

```
                    ┌──────────────────────────────────────────┐
                    │        Global Chain (Device scope)        │
                    │   "persist.hypermesh" registered here     │
                    │   as BaseSystemType::Dns asset            │
                    │   config = DOMAIN:REGISTER:persist...     │
                    └──────────────────┬───────────────────────┘
                                       │ network_id = hex(BLAKE3("persist.hypermesh")[..16])
                    ┌──────────────────▼───────────────────────┐
                    │   persist.hypermesh Network Chain          │
                    │   "home.persist.hypermesh" registered     │
                    │   here as sub-domain asset                │
                    │   Members: owner-node, invited peers      │
                    └──────────────────┬───────────────────────┘
                                       │ network_id = hex(BLAKE3("home.persist.hypermesh")[..16])
                    ┌──────────────────▼───────────────────────┐
                    │   home.persist.hypermesh Network Chain     │
                    │   DNS records: nas → fd00::10             │
                    │   Members: node-A, node-B                 │
                    └──────────────────────────────────────────┘

  Resolution Walk (right-to-left):
  "nas.home.persist.hypermesh"
    1. Cache check
    2. Local chain exact match
    3. Query pool for BLAKE3("home.persist.hypermesh") → finds "nas" record
    4. (Fallback) Query pool for BLAKE3("persist.hypermesh")
    5. (Fallback) Query global public pool

  CLI Flow:
  ┌──────────────┐  IPC  ┌────────────────────┐  SyncManager  ┌────────────────┐
  │ hypermesh     │──────→│  daemon             │──────────────→│ Network Chain   │
  │  domain       │       │  DomainNetworkMgr   │               │ (per domain)    │
  │  register/    │       │  DnsResolver         │               │                 │
  │  create/      │       │  DnsRegistrar        │               │                 │
  │  invite       │       │                      │               │                 │
  └──────────────┘       └────────────────────┘               └────────────────┘

  Invitation Flow:
  Owner: domain invite home.persist.hypermesh --peer <node_id>
    → DomainInvitation { token = BLAKE3-HMAC(owner_proof, "home.persist.hypermesh:node_id:expiry") }
    → base64url-encoded string

  Peer: connect --network home.persist.hypermesh --invite <token>
    → decode token → verify HMAC + expiry → join_domain() → connect to reflectors
```

### Key Design Decisions

1. **Domain chain ID derivation**: `BLAKE3(domain_name.as_bytes())` produces a 32-byte hash. The first 16 bytes are hex-encoded to form a 32-character `network_id` string for `SyncManager::join_network()`. This is deterministic: any node can compute the chain ID from the domain name alone without network access.

2. **No new `BaseSystemType` variant**: The existing `Dns` variant (at `/home/persist/hypermesh/core/blockmatrix/src/assets/core/asset_id.rs` line 86) already covers domain registration. A new `DomainRegistration` struct carries the extra metadata (parent chain, privacy mode, network chain ID). This avoids a breaking change across `SystemAssetKind` (in `lib/src/asset.rs`), `BaseSystemType`, and all bidirectional `From` implementations documented in MEMORY.md.

3. **Backward compatibility**: Existing `DnsRecord`, `DnsRegistration`, pools, and cache continue to work unchanged. The `DomainRegistration` is a new struct layered on top, registered as a `BaseSystemType::Dns` asset with a specific `AssetData.config` prefix (`DOMAIN:REGISTER:...`) distinguishable from plain DNS records (`DNS:REGISTER:...`).

4. **Invitation tokens**: BLAKE3-HMAC over `(domain_name, invitee_node_id, expiry_timestamp)` keyed by the domain owner's PoS stake proof bytes. Compact (32 bytes + metadata), verifiable without blockchain lookup, time-bounded.

5. **SyncManager integration**: `SyncManager::join_network()` at `/home/persist/hypermesh/core/blockmatrix/src/blockchain/sync_manager.rs` line 216 is synchronous and takes `(network_id: String, privacy_mode: PrivacyMode, now_unix_secs: u64)`. The `DomainNetworkManager` wraps this behind an async-friendly interface that derives the network_id from the domain name.

---

## 2. Sprint 1: Domain Asset Type & Registration (~300 lines)

**Goal**: Domains become first-class blockchain assets. Registering a domain creates a derived Network-scope chain ID and a corresponding DNS pool.

### 2.1 Create `domain.rs` module — `DomainRegistration` struct (~60 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/domain.rs` (new)

This file introduces the core type that links a domain name to a Network-scope blockchain.

**Type definitions:**

```rust
/// A domain registered as a blockchain asset that creates a Network-scope chain.
///
/// Each domain maps 1:1 to a Network chain identified by `network_id`.
/// Sub-domains reference their parent's `network_id` via `parent_network_id`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainRegistration {
    /// Parsed domain (from dns/mod.rs Domain type)
    pub domain: Domain,
    /// Full 32-byte BLAKE3 hash of domain name
    pub chain_id: [u8; 32],
    /// Hex-encoded first 16 bytes of chain_id — used as SyncManager network_id
    pub network_id: String,
    /// Parent domain's network_id (None for top-level domains)
    pub parent_network_id: Option<String>,
    /// Privacy mode for this domain's network
    pub privacy_mode: PrivacyMode,
    /// Node ID of the registering owner
    pub owner_node_id: String,
    /// Registration timestamp
    pub created_at: SystemTime,
    /// Serialized PoS proof used during registration
    pub state_proof_bytes: Option<Vec<u8>>,
}
```

**Functions:**

- `pub fn derive_chain_id(domain_name: &str) -> [u8; 32]` — Returns `*blake3::hash(domain_name.as_bytes()).as_bytes()`. Pure function, no side effects.
- `pub fn derive_network_id(domain_name: &str) -> String` — Returns `hex::encode(&derive_chain_id(domain_name)[..16])`. Produces a 32-character hex string suitable for `SyncManager::join_network()`.
- `impl DomainRegistration` with constructor `fn new(domain: Domain, privacy_mode: PrivacyMode, owner_node_id: String) -> Self` that:
  - Computes `chain_id` and `network_id` from `domain.full`
  - Computes `parent_network_id` by calling `domain.parent()` and, if `Some`, calling `derive_network_id(parent.full)`
  - Sets `created_at` to `SystemTime::now()`
  - Leaves `state_proof_bytes` as `None` (set later during registration)

**Dependencies**: `blake3` (already in workspace), `hex` (already in workspace via various crates), `serde`, `PrivacyMode` from `crate::bootstrap`.

**Line estimate**: ~60 lines including imports and constructor.

### 2.2 Add domain registration method to `DnsRegistrar` (~80 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/registration.rs` (modify)

The existing `DnsRegistrar` at line 47 has `pool_manager`, `validator`, `blockchain`, and `registrations` fields. This step adds domain-specific registration.

**New field:**

```rust
/// Active domain registrations (domain_name -> DomainRegistration)
domain_registrations: Arc<RwLock<HashMap<String, DomainRegistration>>>,
```

Initialize in `DnsRegistrar::new()` as `Arc::new(RwLock::new(HashMap::new()))`.

**New methods:**

- `pub async fn register_domain(&self, domain: Domain, privacy_mode: PrivacyMode, owner_node_id: String, proof: StateProof) -> DnsResult<DomainRegistration>`:
  1. Validate domain name length (max 63 chars per component, max 253 total)
  2. Check domain not already registered: `domain_registrations.read().await.contains_key(&domain.full)`
  3. If domain has a parent (`domain.parent().is_some()`), verify parent is registered locally (lookup in `domain_registrations`)
  4. Call `self.validator.validate_registration(&domain, &proof).await?` (existing validation)
  5. Create `DomainRegistration::new(domain.clone(), privacy_mode, owner_node_id)`
  6. Set `state_proof_bytes` from `proof.to_bytes()`
  7. Register to blockchain via existing `register_to_blockchain()` with `AssetData.config = format!("DOMAIN:REGISTER:{}:{}:{}", domain.full, privacy_mode, registration.network_id)`
  8. Create DNS pool for this domain: `self.pool_manager.create_domain_pool(&registration.network_id, pool_visibility).await?`
  9. Store in `domain_registrations`
  10. Return the `DomainRegistration`

- `pub async fn get_domain(&self, domain_name: &str) -> Option<DomainRegistration>` — Read from `domain_registrations`.
- `pub async fn list_domains(&self) -> Vec<DomainRegistration>` — Collect all values.

**Determining pool visibility from privacy mode:**

```rust
let pool_visibility = match privacy_mode {
    pm if pm == PrivacyMode::PUBLIC => PoolVisibility::Public,
    pm if pm == PrivacyMode::ANONYMOUS => PoolVisibility::FullyFederated,
    _ => PoolVisibility::NetworkRestricted,
};
```

### 2.3 Register the `domain` module (~10 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/mod.rs` (modify)

At line 21, among the existing `pub mod` declarations:

- Add `pub mod domain;`

In the re-export section (around line 29-36):

- Add `pub use domain::{DomainRegistration, derive_chain_id, derive_network_id};`

In the `DnsError` enum (line 44), add a new variant:

```rust
#[error("Domain not registered: {domain}")]
DomainNotRegistered { domain: String },
```

Also add:

```rust
#[error("Parent domain not registered: {parent}")]
ParentDomainRequired { parent: String },

#[error("Domain already registered: {domain}")]
DomainAlreadyRegistered { domain: String },
```

### 2.4 Domain pool creation in `DnsPoolManager` (~40 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/pools.rs` (modify)

Add method to `DnsPoolManager` (after `register_federated` around line 165):

```rust
/// Create a DNS pool for a registered domain.
///
/// This is called automatically during domain registration. The pool is
/// keyed by the domain's network_id (derived from BLAKE3 of domain name).
pub async fn create_domain_pool(
    &self,
    network_id: &str,
    visibility: PoolVisibility,
) -> DnsResult<()> {
    let mut pools = self.federated_pools.write().await;
    if pools.contains_key(network_id) {
        return Ok(()); // Pool already exists (idempotent)
    }

    let pool = Arc::new(DnsPool::new(
        format!("domain-{network_id}"),
        DnsPoolType::Federated {
            network_id: network_id.to_string(),
        },
        visibility,
    ));

    pools.insert(network_id.to_string(), pool);
    info!("Created domain DNS pool: {}", network_id);
    Ok(())
}
```

Also add a lookup method:

```rust
/// Check if a pool exists for a given network_id.
pub async fn has_pool(&self, network_id: &str) -> bool {
    self.federated_pools.read().await.contains_key(network_id)
}
```

### 2.5 Persistence for domain registrations (~50 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/domain.rs` (append)

Follow the existing pattern of `dns_records.json` used in the node binary. Add:

```rust
use std::path::Path;

/// Persist domain registrations to a JSON file.
pub fn save_domains(domains: &[DomainRegistration], path: &Path) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(domains)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load domain registrations from a JSON file.
pub fn load_domains(path: &Path) -> anyhow::Result<Vec<DomainRegistration>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let json = std::fs::read_to_string(path)?;
    let domains: Vec<DomainRegistration> = serde_json::from_str(&json)?;
    Ok(domains)
}
```

The node binary will call `save_domains()` after each `register_domain()` and `load_domains()` on startup, storing to `{data_dir}/domain_registrations.json`.

### 2.6 Unit tests (~60 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/domain.rs` (tests module)

```
#[cfg(test)]
mod tests {
    // test_derive_chain_id_deterministic:
    //   Call derive_chain_id("persist.hypermesh") twice, assert equal.
    //   Call with different domain, assert different.

    // test_derive_network_id_format:
    //   Assert length == 32 (16 bytes hex-encoded).
    //   Assert all chars are hex digits.

    // test_parent_chain_derivation:
    //   DomainRegistration::new for "home.persist.hypermesh":
    //     parent_network_id == Some(derive_network_id("persist.hypermesh"))
    //   DomainRegistration::new for "persist.hypermesh":
    //     parent_network_id == Some(derive_network_id("hypermesh"))
    //   DomainRegistration::new for "hypermesh":
    //     parent_network_id == None

    // test_domain_registration_serde_roundtrip:
    //   Create DomainRegistration, serialize to JSON, deserialize, assert equal fields.

    // test_domain_persistence_roundtrip:
    //   Create vec of DomainRegistrations, save_domains to tempfile, load_domains, assert equal.

    // test_domain_registration_via_registrar:
    //   Create DnsRegistrar with mock validator (strict_mode=false),
    //   Call register_domain, assert returns Active-equivalent DomainRegistration,
    //   Assert pool was created via pool_manager.has_pool().
}
```

### Sprint 1 Test Plan

| # | Test Name | Description | Type |
|---|-----------|-------------|------|
| 1 | `test_derive_chain_id_deterministic` | Same domain always produces same BLAKE3 hash; different domains differ | Unit |
| 2 | `test_derive_network_id_format` | Hex string is exactly 32 characters, all hex digits | Unit |
| 3 | `test_parent_chain_derivation` | `home.persist.hypermesh` parent is `persist.hypermesh`; top-level has `None` | Unit |
| 4 | `test_domain_registration_serde_roundtrip` | JSON serialize/deserialize preserves all fields | Unit |
| 5 | `test_domain_persistence_roundtrip` | `save_domains` then `load_domains` returns identical data | Unit |
| 6 | `test_domain_registration_via_registrar` | `register_domain()` creates pool and stores registration | Unit |
| 7 | `test_duplicate_domain_rejected` | Second `register_domain()` for same name returns `DomainAlreadyRegistered` | Unit |
| 8 | `test_subdomain_requires_parent` | Registering `sub.parent` when `parent` not registered returns `ParentDomainRequired` | Unit |

### Sprint 1 Quality Gates

- All existing DNS tests pass unchanged (run `cargo test -p blockmatrix dns`)
- `cargo check -p blockmatrix` compiles with zero errors
- No new `BaseSystemType` or `SystemAssetKind` variants introduced
- `DomainRegistration` implements `Serialize + Deserialize + Clone + Debug`
- `derive_chain_id` and `derive_network_id` are pure functions (no I/O, no side effects)
- No `.unwrap()` in production code (per pre-commit hook at `.git/hooks/pre-commit`)
- Domain name length validation added (max 63 per component, max 253 total)

---

## 3. Sprint 2: Hierarchical Resolver & Network Join (~350 lines)

**Goal**: Names resolve across the domain hierarchy by walking parent chains. Joining a domain joins its Network-scope chain via `SyncManager`.

### 3.1 Hierarchical resolution in `DnsResolver` (~100 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/resolver.rs` (modify)

**New field** on `DnsResolver` (line 67):

```rust
/// Domain registry for hierarchical resolution
domain_registry: Option<Arc<RwLock<HashMap<String, DomainRegistration>>>>,
```

**New builder method:**

```rust
pub fn with_domain_registry(
    mut self,
    registry: Arc<RwLock<HashMap<String, DomainRegistration>>>,
) -> Self {
    self.domain_registry = Some(registry);
    self
}
```

**New method** `resolve_hierarchical`:

```rust
/// Resolve a domain by walking the hierarchy right-to-left.
///
/// For "nas.home.persist.hypermesh":
///   1. Check local cache (existing behavior)
///   2. Find authoritative network: walk parents until a registered domain is found
///      - "home.persist.hypermesh" → network_id exists? Query its pool for "nas.home.persist.hypermesh"
///      - "persist.hypermesh" → network_id exists? Query its pool
///      - "hypermesh" → network_id exists? Query its pool
///   3. Fall through to public pool (existing behavior)
async fn resolve_hierarchical(&self, query: &DnsQuery) -> DnsResult<Vec<DnsRecord>> {
    // Walk domain components to find authoritative network
    let mut current = query.domain.clone();

    loop {
        if let Some(parent) = current.parent() {
            let parent_network_id = derive_network_id(&parent.full);

            // Try querying the parent's pool for the original full domain name
            match self.pool_manager.query_federated(&parent_network_id, &query.domain.full).await {
                Ok(records) if !records.is_empty() => return Ok(records),
                _ => {
                    // Not found in this pool, walk up
                    current = parent;
                    continue;
                }
            }
        } else {
            // Reached root, try public pool
            break;
        }
    }

    // Fall through to public resolution
    self.resolve_public(query).await
}
```

**Modify existing `resolve()` method** (line 100): After the cache check and before the tier-based dispatch, insert a check:

```rust
// For multi-component domains, attempt hierarchical resolution
if query.domain.subdomains.len() >= 1 && self.domain_registry.is_some() {
    match self.resolve_hierarchical(&query).await {
        Ok(records) if !records.is_empty() => {
            // Cache and return
            if let Some(first) = records.first() {
                self.cache.set(&query.domain.full, &query.record_type, records.clone(), first.ttl).await?;
            }
            return Ok(DnsResponse {
                domain: query.domain.clone(),
                tier: DnsResolutionTier::Federated {
                    network_id: "hierarchical".to_string(),
                },
                records,
                timestamp: SystemTime::now(),
                from_cache: false,
            });
        }
        _ => {
            // Fall through to existing tier-based resolution
        }
    }
}
```

**Helper method:**

```rust
/// Find the network_id of the most specific registered domain that is
/// an ancestor of the given domain.
fn find_authoritative_network_id(&self, domain: &Domain) -> Option<String> {
    // ... walks domain.parent() chain, checks domain_registry for each
}
```

### 3.2 Update `DnsResolutionTier` for hierarchical results (~15 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/resolver.rs` (modify)

Add a variant to `DnsResolutionTier` (line 24):

```rust
/// Hierarchical resolution through domain chain walk
Hierarchical {
    /// The authoritative domain whose pool answered the query
    authoritative_domain: String,
    /// That domain's network_id
    network_id: String,
},
```

Update the hierarchical resolution code to use this variant instead of the placeholder `Federated { network_id: "hierarchical" }`.

### 3.3 `DomainNetworkManager` — bridge between DNS and SyncManager (~80 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/domain.rs` (append)

This struct provides the high-level "join a domain" operation that combines domain lookup with `SyncManager::join_network()`.

```rust
/// Manages the relationship between domain registrations and network membership.
///
/// Bridges the DNS layer (domain names) with the sync layer (network IDs).
pub struct DomainNetworkManager {
    /// Reference to the SyncManager for network join/leave
    sync_manager: Arc<RwLock<SyncManager>>,
    /// Domain registry for looking up domain -> network_id mapping
    domain_registry: Arc<RwLock<HashMap<String, DomainRegistration>>>,
}
```

**Methods:**

- `pub fn new(sync_manager, domain_registry) -> Self`

- `pub async fn join_domain(&self, domain_name: &str) -> Result<NetworkMembership, String>`:
  1. Read `domain_registry`, find `DomainRegistration` for `domain_name`
  2. If not found, try computing `derive_network_id(domain_name)` and join with default PRIVATE privacy (for domains discovered via remote resolution, not yet in local registry)
  3. Extract `network_id` and `privacy_mode`
  4. Get current unix timestamp
  5. Call `self.sync_manager.write().await.join_network(network_id, privacy_mode, now_secs)?`
  6. Return a clone of the resulting `NetworkMembership`

- `pub async fn leave_domain(&self, domain_name: &str) -> Result<(), String>`:
  1. Compute `network_id = derive_network_id(domain_name)`
  2. Call `self.sync_manager.write().await.leave_network(&network_id)?`

- `pub async fn is_domain_member(&self, domain_name: &str) -> bool`:
  1. Compute `network_id`
  2. Call `self.sync_manager.read().await.is_member(&network_id)`

- `pub async fn domain_sync_state(&self, domain_name: &str) -> Option<SyncState>`:
  1. Compute `network_id`
  2. Call `self.sync_manager.read().await.sync_state(&network_id).cloned()`

### 3.4 Bootstrap peer discovery for domain networks (~60 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/domain.rs` (append)

Add to `DomainNetworkManager`:

```rust
/// Discover reflector/bootstrap peers for a domain's network.
///
/// Resolves the domain name to find SRV or AAAA records that represent
/// nodes already participating in the domain's Network chain.
pub async fn discover_domain_peers(
    &self,
    domain_name: &str,
    resolver: &DnsResolver,
) -> Result<Vec<SocketAddr>, DnsError> {
    let domain = Domain::parse(domain_name)?;
    let network_id = derive_network_id(domain_name);

    // Strategy 1: Look for SRV records in the domain's pool
    let srv_query = DnsQuery {
        domain: domain.clone(),
        record_type: DnsRecordType::SRV,
        requester_network: Some(network_id.clone()),
        proof: None,
        timestamp: SystemTime::now(),
    };

    if let Ok(response) = resolver.resolve(srv_query).await {
        let addrs: Vec<SocketAddr> = response.records.iter().filter_map(|r| {
            if let DnsRecordData::SRV { port, target, .. } = &r.data {
                // target is an IPv6 address string for HyperMesh
                target.parse::<Ipv6Addr>().ok().map(|ip| {
                    SocketAddr::new(std::net::IpAddr::V6(ip), *port)
                })
            } else {
                None
            }
        }).collect();

        if !addrs.is_empty() {
            return Ok(addrs);
        }
    }

    // Strategy 2: Look for AAAA records (assume default STOQ port 9292)
    let aaaa_query = DnsQuery {
        domain,
        record_type: DnsRecordType::AAAA,
        requester_network: Some(network_id),
        proof: None,
        timestamp: SystemTime::now(),
    };

    if let Ok(response) = resolver.resolve(aaaa_query).await {
        let addrs: Vec<SocketAddr> = response.records.iter().filter_map(|r| {
            if let DnsRecordData::AAAA(ip) = &r.data {
                Some(SocketAddr::new(std::net::IpAddr::V6(*ip), 9292))
            } else {
                None
            }
        }).collect();

        return Ok(addrs);
    }

    Ok(Vec::new()) // No peers found; domain may not have active nodes yet
}
```

### 3.5 Integration with `NetworkManager` (~30 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/network/mod.rs` (modify)

Add method to `NetworkManager` (after `join_network()` around line 194):

```rust
/// Connect to peers in a domain's network.
///
/// Iterates the provided peer addresses and attempts STOQ connections
/// to each. This is used after `DomainNetworkManager::join_domain()` to
/// establish actual transport connections for chain synchronization.
pub async fn connect_to_domain_network(
    &self,
    domain_name: &str,
    peers: Vec<SocketAddr>,
) -> Result<Vec<String>> {
    info!(
        "Connecting to domain network '{}' via {} peer(s)",
        domain_name,
        peers.len()
    );

    let mut connected_ids = Vec::new();
    for addr in peers {
        match self.connect_to_peer(addr).await {
            Ok(node_id) => {
                info!("Connected to domain '{}' peer: {} ({})", domain_name, addr, node_id);
                connected_ids.push(node_id);
            }
            Err(e) => {
                warn!("Failed to connect to domain '{}' peer {}: {}", domain_name, addr, e);
            }
        }
    }

    if connected_ids.is_empty() {
        warn!("No peers reachable for domain '{}' — operating in offline mode", domain_name);
    }

    Ok(connected_ids)
}
```

### 3.6 Unit and integration tests (~65 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/resolver.rs` (append to existing tests module)

```
// test_hierarchical_resolve_local_first:
//   Setup: register "home.persist.hypermesh" pool with "nas.home.persist.hypermesh" AAAA record.
//   Query "nas.home.persist.hypermesh" — should find it in home pool.

// test_hierarchical_resolve_walks_parents:
//   Setup: register "persist.hypermesh" pool with "nas.home.persist.hypermesh" record.
//   Do NOT create "home.persist.hypermesh" pool.
//   Query should walk up and find it in persist pool.

// test_hierarchical_resolve_falls_to_global:
//   Setup: register "nas.home.persist.hypermesh" in public pool only.
//   Query should eventually fall to public pool.

// test_flat_domain_unaffected:
//   Setup: register "nike" in public pool.
//   Query "nike" — should use existing Public tier, not hierarchical.
```

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/domain.rs` (append to tests module)

```
// test_join_domain_creates_sync_membership:
//   Create SyncManager, DomainNetworkManager.
//   Register "home.persist.hypermesh" domain.
//   Call join_domain("home.persist.hypermesh").
//   Assert sync_manager.is_member(derive_network_id("home.persist.hypermesh")).

// test_join_domain_correct_network_id:
//   After join_domain, the NetworkMembership.network_id matches derive_network_id().

// test_leave_domain_removes_membership:
//   join_domain, then leave_domain. Assert is_member returns false.

// test_domain_sync_state_starts_discovering:
//   After join_domain, domain_sync_state returns Some(SyncState::Discovering).

// test_discover_domain_peers_from_srv:
//   Register SRV records in domain pool, call discover_domain_peers, assert addresses returned.
```

### Sprint 2 Test Plan

| # | Test Name | Description | Type |
|---|-----------|-------------|------|
| 1 | `test_hierarchical_resolve_local_first` | Multi-component domain found in its own pool | Unit |
| 2 | `test_hierarchical_resolve_walks_parents` | Record not in child pool, found by walking to parent | Unit |
| 3 | `test_hierarchical_resolve_falls_to_global` | No domain pools registered, falls through to public | Unit |
| 4 | `test_flat_domain_unaffected` | Single-component domain still uses Public tier | Unit |
| 5 | `test_join_domain_creates_sync_membership` | `join_domain()` calls `SyncManager::join_network()` | Unit |
| 6 | `test_join_domain_correct_network_id` | Membership uses `derive_network_id(domain_name)` | Unit |
| 7 | `test_leave_domain_removes_membership` | After `leave_domain()`, `is_member()` is false | Unit |
| 8 | `test_domain_sync_state_starts_discovering` | Initial sync state is `Discovering` | Unit |
| 9 | `test_discover_domain_peers_from_srv` | SRV records in pool resolve to `SocketAddr` list | Unit |
| 10 | `test_discover_domain_peers_aaaa_fallback` | AAAA records used when no SRV records exist | Unit |

### Sprint 2 Quality Gates

- All Sprint 1 tests pass
- All existing DNS tests pass (flat domain resolution unchanged)
- Hierarchical resolution only activates when `domain_registry` is set (opt-in)
- `SyncManager::join_network()` called with deterministic `network_id` derived from domain name
- No circular dependency between `dns` and `network` modules: `dns/domain.rs` imports from `blockchain/sync_manager.rs`, not from `network/mod.rs` directly. The `NetworkManager::connect_to_domain_network()` is called by the node binary, not by dns code
- No `.unwrap()` in production code

---

## 4. Sprint 3: CLI, Gateway Routing & Invitations (~250 lines)

**Goal**: Full CLI surface for domain management, cross-domain gateway resolution, and an invitation system for private domain networks.

### 4.1 CLI command types for domains (~50 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (modify)

Add to the `Commands` enum (after `Dns` at line 146):

```rust
/// Domain operations — register, create sub-domains, invite peers
Domain {
    #[clap(subcommand)]
    action: DomainAction,
},

/// Connect to a domain's network
Connect {
    /// Domain name to connect to (e.g., "home.persist.hypermesh")
    network: String,
    /// Invitation token (required for private domains)
    #[clap(long)]
    invite: Option<String>,
},
```

Define `DomainAction` enum (after `DnsAction` around line 166):

```rust
#[derive(Subcommand, Debug)]
enum DomainAction {
    /// Register a top-level domain on the global chain
    Register {
        /// Domain name (e.g., "persist.hypermesh")
        name: String,
        /// Privacy mode for the domain's network
        #[clap(long, value_enum, default_value = "private")]
        privacy: PrivacyModeArg,
    },
    /// Create a sub-domain under an existing domain
    Create {
        /// Full sub-domain name (e.g., "home.persist.hypermesh")
        name: String,
        /// Privacy mode for the sub-domain's network
        #[clap(long, value_enum, default_value = "private")]
        privacy: PrivacyModeArg,
    },
    /// List all registered domains on this node
    List,
    /// Show nodes connected to a domain's network
    Nodes {
        /// Domain name
        domain: String,
    },
    /// Generate an invitation token for a peer to join a domain
    Invite {
        /// Domain name
        domain: String,
        /// Node ID of the peer to invite
        #[clap(long)]
        peer: String,
        /// Token validity in seconds (default: 1 hour)
        #[clap(long, default_value = "3600")]
        ttl: u64,
    },
}
```

### 4.2 CLI execution logic (~60 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (modify, in the main match arms)

In the existing `match cli.command` block, add arms:

```rust
Some(Commands::Domain { action }) => {
    match action {
        DomainAction::Register { name, privacy } => {
            // 1. Parse domain
            // 2. Generate PoS proof (same pattern as DNS register)
            // 3. Call registrar.register_domain(domain, privacy.into(), node_id, proof)
            // 4. Save domain_registrations.json
            // 5. Print: "Domain '{name}' registered — network_id: {network_id}"
        }
        DomainAction::Create { name, privacy } => {
            // Same as Register but:
            // 1. Verify parent domain exists in local registry
            // 2. Register on parent's chain (requires membership)
            // If parent not in registry, error: "Parent domain not registered locally"
        }
        DomainAction::List => {
            // Call registrar.list_domains()
            // Print table: Name | Network ID | Privacy | Parent | Created
        }
        DomainAction::Nodes { domain } => {
            // Compute network_id from domain
            // Query sync_manager for membership status
            // Query network_manager for connected nodes in that network
            // Print table: Node ID | Address | Sync State
        }
        DomainAction::Invite { domain, peer, ttl } => {
            // 1. Look up domain registration
            // 2. Create invitation token
            // 3. Print base64url token string
        }
    }
}

Some(Commands::Connect { network, invite }) => {
    // 1. If invite provided: decode and verify invitation token
    // 2. Call domain_network_manager.join_domain(&network)
    // 3. Call domain_network_manager.discover_domain_peers(&network, &resolver)
    // 4. Call network_manager.connect_to_domain_network(&network, peers)
    // 5. Print: "Joined domain '{network}' — connected to {n} peer(s)"
}
```

### 4.3 Invitation token system (~60 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/invitation.rs` (new)

```rust
use blake3;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// An invitation granting a peer permission to join a domain's network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainInvitation {
    /// Domain name the invitation is for
    pub domain_name: String,
    /// Network ID (derived from domain, included for convenience)
    pub network_id: String,
    /// Specific invitee node ID (empty string = open invitation)
    pub invitee_node_id: String,
    /// Unix timestamp when invitation expires
    pub expires_at: u64,
    /// BLAKE3-HMAC token
    pub token: [u8; 32],
}
```

**Functions:**

```rust
/// Create an invitation for a peer to join a domain's network.
///
/// The token is a BLAKE3 keyed hash (HMAC-like) using the first 32 bytes
/// of the domain owner's PoS stake proof as the key.
pub fn create_invitation(
    domain_name: &str,
    owner_proof_bytes: &[u8],
    invitee_node_id: Option<&str>,
    ttl_secs: u64,
) -> DomainInvitation {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let expires_at = now + ttl_secs;
    let invitee = invitee_node_id.unwrap_or("");
    let network_id = super::domain::derive_network_id(domain_name);

    // Build the key: first 32 bytes of owner proof (pad with zeros if shorter)
    let mut key = [0u8; 32];
    let copy_len = owner_proof_bytes.len().min(32);
    key[..copy_len].copy_from_slice(&owner_proof_bytes[..copy_len]);

    // HMAC payload
    let payload = format!("{domain_name}:{invitee}:{expires_at}");
    let token = *blake3::keyed_hash(&key, payload.as_bytes()).as_bytes();

    DomainInvitation {
        domain_name: domain_name.to_string(),
        network_id,
        invitee_node_id: invitee.to_string(),
        expires_at,
        token,
    }
}

/// Verify an invitation token against the domain owner's proof.
pub fn verify_invitation(
    invitation: &DomainInvitation,
    owner_proof_bytes: &[u8],
) -> bool {
    // Check expiry
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now > invitation.expires_at {
        return false;
    }

    // Recompute token
    let mut key = [0u8; 32];
    let copy_len = owner_proof_bytes.len().min(32);
    key[..copy_len].copy_from_slice(&owner_proof_bytes[..copy_len]);

    let payload = format!(
        "{}:{}:{}",
        invitation.domain_name, invitation.invitee_node_id, invitation.expires_at
    );
    let expected = *blake3::keyed_hash(&key, payload.as_bytes()).as_bytes();

    // Constant-time comparison
    invitation.token == expected
}

/// Encode an invitation as a base64url string for CLI transport.
pub fn encode_invitation(invitation: &DomainInvitation) -> Result<String, serde_json::Error> {
    let json = serde_json::to_vec(invitation)?;
    Ok(base64_url_encode(&json))
}

/// Decode an invitation from a base64url string.
pub fn decode_invitation(token_str: &str) -> Result<DomainInvitation, String> {
    let bytes = base64_url_decode(token_str)
        .map_err(|e| format!("Invalid invitation token encoding: {e}"))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| format!("Invalid invitation token format: {e}"))
}

// Simple base64url helpers (no padding, URL-safe alphabet)
fn base64_url_encode(data: &[u8]) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    URL_SAFE_NO_PAD.encode(data)
}

fn base64_url_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    URL_SAFE_NO_PAD.decode(s)
}
```

**Note**: The `base64` crate is already a dependency in the workspace (used by STOQ and TrustChain). Verify with `cargo metadata` or `Cargo.toml`.

### 4.4 Gateway DNS routing enhancement (~40 lines)

**File**: `/home/persist/hypermesh/core/gateway/src/domain_router.rs` (modify)

Add methods to `DomainRouter` (after `add_route` at line 69):

```rust
/// Register a route for a domain's network chain.
///
/// DNS queries for `*.domain_name` or `domain_name` itself will be
/// forwarded to the given reflector address for resolution.
pub fn add_domain_chain_route(&self, domain_name: &str, reflector_addr: SocketAddr) {
    let route = DomainRoute {
        domain: domain_name.to_string(),
        backend_addr: reflector_addr,
        backend_name: format!("domain-chain:{domain_name}"),
    };
    self.exact_routes.insert(domain_name.to_string(), route);
    debug!(
        domain = %domain_name,
        reflector = %reflector_addr,
        "registered domain chain route"
    );
}

/// Route a DNS query to the correct chain's reflector.
///
/// Walks the domain hierarchy: exact match, then parent domains, then wildcards.
/// Returns the reflector address that can answer queries for this domain's chain.
pub fn route_dns_query(&self, domain: &str) -> Option<SocketAddr> {
    // 1. Exact match
    if let Some(route) = self.exact_routes.get(domain) {
        self.stats.exact_hits.fetch_add(1, Ordering::Relaxed);
        return Some(route.backend_addr);
    }

    // 2. Walk parent domains
    let parts: Vec<&str> = domain.split('.').collect();
    for i in 1..parts.len() {
        let parent = parts[i..].join(".");
        if let Some(route) = self.exact_routes.get(&parent) {
            self.stats.exact_hits.fetch_add(1, Ordering::Relaxed);
            return Some(route.backend_addr);
        }
    }

    // 3. Fall through to wildcard matching (existing behavior via resolve())
    let resolved = self.resolve(domain);
    resolved.map(|r| r.backend_addr)
}
```

### 4.5 Register invitation module (~5 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/mod.rs` (modify)

Add to module declarations:

```rust
pub mod invitation;
```

Add to re-exports:

```rust
pub use invitation::{
    DomainInvitation, create_invitation, verify_invitation,
    encode_invitation, decode_invitation,
};
```

### 4.6 Tests (~25 lines)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/dns/invitation.rs` (tests module)

```
#[cfg(test)]
mod tests {
    // test_invitation_create_and_verify:
    //   Create invitation with known owner proof. Verify returns true.

    // test_invitation_expired:
    //   Create invitation with ttl_secs = 0.
    //   Sleep 10ms. Verify returns false.

    // test_invitation_wrong_key:
    //   Create with proof A. Verify with proof B. Returns false.

    // test_invitation_encode_decode_roundtrip:
    //   Create invitation, encode, decode, compare all fields.

    // test_invitation_open_vs_targeted:
    //   Open invitation (no invitee): invitee_node_id is "".
    //   Targeted invitation: invitee_node_id is specific ID.
    //   Both verify correctly with matching proof.
}
```

**File**: `/home/persist/hypermesh/core/gateway/src/domain_router.rs` (append to existing tests)

```
// test_domain_chain_route_exact:
//   add_domain_chain_route("persist.hypermesh", addr).
//   route_dns_query("persist.hypermesh") returns Some(addr).

// test_domain_chain_route_parent_walk:
//   add_domain_chain_route("persist.hypermesh", addr).
//   route_dns_query("nas.home.persist.hypermesh") returns Some(addr) via parent walk.

// test_domain_chain_route_no_interference:
//   Existing exact route for "trust.hypermesh.online" still works.
//   Domain chain route for "persist.hypermesh" does not interfere.
```

### Sprint 3 Test Plan

| # | Test Name | Description | Type |
|---|-----------|-------------|------|
| 1 | `test_invitation_create_and_verify` | Token created and verified with same proof | Unit |
| 2 | `test_invitation_expired` | Expired token returns false on verify | Unit |
| 3 | `test_invitation_wrong_key` | Token verified with wrong proof returns false | Unit |
| 4 | `test_invitation_encode_decode_roundtrip` | Base64url encode/decode preserves all fields | Unit |
| 5 | `test_invitation_open_vs_targeted` | Both open and targeted invitations verify correctly | Unit |
| 6 | `test_domain_chain_route_exact` | Exact domain match routes to reflector | Unit |
| 7 | `test_domain_chain_route_parent_walk` | Sub-domain query walks up to registered parent | Unit |
| 8 | `test_domain_chain_route_no_interference` | Existing gateway routes unaffected | Unit |
| 9 | `test_cli_domain_register_parses` | Clap parses `domain register persist.hypermesh --privacy public` | Unit |
| 10 | `test_cli_connect_parses` | Clap parses `connect home.persist.hypermesh --invite <token>` | Unit |

### Sprint 3 Quality Gates

- All Sprint 1 and Sprint 2 tests pass
- All existing gateway tests pass (194 unit tests, per MEMORY.md)
- Invitation token verification is constant-time (BLAKE3 comparison)
- Invitation tokens have a maximum TTL (suggest: 7 days = 604800 seconds)
- CLI commands do not conflict with existing `Start`, `Status`, `SetPrivacy`, `Store`, `Fetch`, `Dns` subcommands
- `base64` crate dependency exists in workspace (verify before implementation)
- No `.unwrap()` in production code
- All new files have the standard copyright header

---

## 5. Cross-Cutting Concerns

### 5.1 Backward Compatibility

| Component | Impact | Mitigation |
|-----------|--------|------------|
| `DnsRecord` | None | Unchanged |
| `DnsRegistration` | New field `domain_registrations` on `DnsRegistrar` | Field initialized as empty HashMap; existing `register_public`/`register_federated` untouched |
| `DnsPoolManager` | New method `create_domain_pool` | Additive; existing pools unaffected |
| `DnsResolver` | New `domain_registry` field | Optional (`Option`); when `None`, hierarchical resolution skipped entirely |
| `DnsCache` | None | Cache keys include full domain name; hierarchical results cached normally |
| `DnsValidator` | None | Existing `validate_registration()` reused for domain registration |
| `DnsResolutionTier` | New `Hierarchical` variant | Additive enum variant; existing match arms unaffected |
| `SyncManager` | None | Existing `join_network()` called with domain-derived network_id |
| `NetworkManager` | New method `connect_to_domain_network` | Additive; delegates to existing `connect_to_peer()` |
| `DomainRouter` (gateway) | New methods | Additive; existing exact/wildcard routing unchanged |
| `BaseSystemType` | None | Existing `Dns` variant used; no new variants |
| `CliCommand` (commands.rs) | Not modified | CLI additions are in `node.rs` binary, not the library command types |
| Node binary | New `Domain` and `Connect` subcommands | Additive to `Commands` enum |

### 5.2 Security Considerations

**Domain Registration Authorization:**
- Top-level domains (e.g., `persist.hypermesh`) require PoS validation (all four proofs: PoSpace, PoStake, PoWork, PoTime) against the local Device chain via existing `DnsValidator::validate_registration()`
- Sub-domains (e.g., `home.persist.hypermesh`) additionally require the registering node to be a member of the parent domain's Network chain (checked via `SyncManager::is_member()`)
- The `DnsRegistrar::register_domain()` enforces parent registration exists locally before allowing sub-domain creation

**Invitation Token Security:**
- Keying material: first 32 bytes of the domain owner's PoS stake proof (serialized). This is not a secret per se, but it is unique to the owner and not publicly broadcast
- BLAKE3 keyed hash provides HMAC-equivalent security (128-bit collision resistance)
- Time-bounded: every invitation has an explicit `expires_at` unix timestamp
- Constant-time verification: the `==` on `[u8; 32]` in Rust performs byte-by-byte comparison (acceptable for 32-byte arrays; if timing attacks are a concern in production, use `subtle::ConstantTimeEq`)
- No nonce/replay protection in alpha: a valid unexpired token can be reused. Production should add a nonce stored in the domain's chain to prevent replay
- Maximum TTL recommendation: 604800 seconds (7 days). The CLI defaults to 3600 (1 hour)

**Domain Name Validation:**
- Max 63 characters per component (matching DNS RFC 1035 label limits)
- Max 253 characters total (matching DNS RFC 1035 name limits)
- Components must be non-empty (existing `Domain::parse()` checks this)
- Reserved names that cannot be registered as top-level domains: `hypermesh`, `caesar`, `trust`, `assets`, `catalog` (these are TrustChain service domains, per `/home/persist/hypermesh/core/blockmatrix/src/dns/trustchain.rs` line 102-105)
- Add validation in Sprint 1's `register_domain()` method

**Network Isolation:**
- Each domain's DNS pool is a separate federated pool keyed by `network_id`
- Pool access requires matching `network_id` in the query's `requester_network` field
- The existing `DnsPoolManager::can_access()` method at `/home/persist/hypermesh/core/blockmatrix/src/dns/pools.rs` line 194 enforces this boundary
- Cross-domain queries walk the hierarchy through the resolver, not through direct pool access

### 5.3 Dependencies and Sequencing

```
Phase 1 (daemon/client IPC) ← PREREQUISITE
    │
    ▼
Sprint 1: Domain Asset Type & Registration
    │  New files: domain.rs
    │  Modified: registration.rs, pools.rs, mod.rs
    │  ~300 lines
    │
    ▼
Sprint 2: Hierarchical Resolver & Network Join
    │  Modified: resolver.rs, domain.rs (DomainNetworkManager), network/mod.rs
    │  ~350 lines
    │  Depends on Sprint 1: DomainRegistration type, derive_network_id()
    │
    ▼
Sprint 3: CLI, Gateway Routing & Invitations
    │  New files: invitation.rs
    │  Modified: node.rs (CLI), gateway/domain_router.rs, mod.rs
    │  ~250 lines
    │  Depends on Sprint 2: DomainNetworkManager, hierarchical resolver
```

Sprint 1 is fully self-contained and can be implemented and tested independently. Sprint 2 imports `DomainRegistration` and `derive_network_id` from Sprint 1. Sprint 3 imports `DomainNetworkManager` from Sprint 2 and `create_invitation`/`verify_invitation` from its own new module.

### 5.4 Potential Challenges and Mitigations

| Challenge | Risk | Mitigation |
|-----------|------|------------|
| **SyncManager is synchronous** | `join_network()` at line 216 of `sync_manager.rs` is `&mut self`, not async. The node binary runs in a tokio runtime. | `SyncManager` is already wrapped in `Arc<RwLock<>>` in the node binary (see imports at line 38). Use `sync_manager.write().await.join_network(...)`. The `RwLock` is tokio's. |
| **Parent chain verification is local-only in Sprint 1** | A node could register a sub-domain without the parent existing on any Network chain | Acceptable for alpha. Sprint 1 checks local `domain_registrations` HashMap. True cross-node verification requires Sprint 2's sync integration, where the parent's Network chain is queried for the domain asset. |
| **Domain name collisions across nodes** | Two nodes register the same top-level domain simultaneously | First to write a valid block wins (blockchain ordering). In Sprint 1 (single-node), this cannot happen. With Network scope sync (future), the Network chain's bilateral PoS verification resolves conflicts by block ordering. |
| **Gateway crate depends on `DomainRouter` without blockmatrix types** | The gateway crate at `/home/persist/hypermesh/core/gateway/` does not depend on blockmatrix. Adding domain chain routing must not create a dependency. | Sprint 3's `add_domain_chain_route()` takes primitive types (`&str`, `SocketAddr`), not blockmatrix types. The node binary calls the gateway with computed values. No new crate dependency needed. |
| **`base64` crate availability** | Invitation encoding requires `base64`. | Already in workspace: used by `stoq` (QUIC handshake) and `trustchain` (certificate encoding). Verify with `grep -r 'base64' Cargo.toml` before implementation. |
| **Hierarchical resolution performance** | Walking 4+ levels of parent domains could be slow with network round-trips | Sprint 2's implementation queries local pools only (in-memory `HashMap` lookups). Network-level resolution (querying remote reflectors) is a future optimization. The DNS cache also prevents repeated walks for the same domain. |
| **`DnsResolutionTier::Hierarchical` variant breaks exhaustive matches** | Adding a new variant to the existing enum could break match expressions in other code | Grep for `DnsResolutionTier` match usage before implementation. The enum is only matched in `resolver.rs` internally and in test assertions. |

### 5.5 New File Summary

| Sprint | File | Lines | Purpose |
|--------|------|-------|---------|
| 1 | `blockmatrix/src/dns/domain.rs` | ~170 | `DomainRegistration`, `derive_chain_id`, `derive_network_id`, persistence, `DomainNetworkManager` (Sprint 2 addition), tests |
| 3 | `blockmatrix/src/dns/invitation.rs` | ~80 | `DomainInvitation`, `create_invitation`, `verify_invitation`, `encode_invitation`, `decode_invitation`, tests |

### 5.6 Modified File Summary

| Sprint | File | Changes | Lines Added |
|--------|------|---------|-------------|
| 1 | `blockmatrix/src/dns/mod.rs` | Add `domain` module, re-exports, 3 error variants | ~10 |
| 1 | `blockmatrix/src/dns/registration.rs` | Add `domain_registrations` field, `register_domain()`, `get_domain()`, `list_domains()` | ~80 |
| 1 | `blockmatrix/src/dns/pools.rs` | Add `create_domain_pool()`, `has_pool()` | ~40 |
| 2 | `blockmatrix/src/dns/resolver.rs` | Add `domain_registry` field, `with_domain_registry()`, `resolve_hierarchical()`, `Hierarchical` tier variant | ~115 |
| 2 | `blockmatrix/src/network/mod.rs` | Add `connect_to_domain_network()` | ~30 |
| 3 | `blockmatrix/src/dns/mod.rs` | Add `invitation` module and re-exports | ~5 |
| 3 | `blockmatrix/src/bin/node.rs` | Add `Domain` and `Connect` subcommands, `DomainAction` enum, execution logic | ~110 |
| 3 | `gateway/src/domain_router.rs` | Add `add_domain_chain_route()`, `route_dns_query()` | ~40 |

**Total estimated new/modified lines**: ~680 across 3 sprints.

### 5.7 Target Experience Validation

After all 3 sprints, the following CLI flows work:

```bash
# Sprint 1: Register a top-level domain (writes to local Device chain)
hypermesh domain register persist.hypermesh --privacy public
# Output: Domain 'persist.hypermesh' registered — network_id: a3b7c9d2e1f04a5b...

# Sprint 1: Create a sub-domain (writes to parent domain's chain)
hypermesh domain create home.persist.hypermesh --privacy private
# Output: Domain 'home.persist.hypermesh' created — network_id: 7f2a8b4c...
#         Parent: persist.hypermesh (a3b7c9d2e1f04a5b...)

# Sprint 2: Join a domain's network
hypermesh connect home.persist.hypermesh
# Output: Joined domain 'home.persist.hypermesh' — connected to 2 peer(s)
#         Sync state: Discovering

# Sprint 1 (existing DNS register, now into domain pool):
hypermesh dns register nas --addr fd00::10
# (Registers in home.persist.hypermesh's pool if currently connected)

# Sprint 2: Resolve across domains
hypermesh dns resolve nas.home.persist.hypermesh
# Output: nas.home.persist.hypermesh → fd00::10 (via Hierarchical: home.persist.hypermesh)

# Sprint 3: Generate invitation
hypermesh domain invite home.persist.hypermesh --peer node-abc123
# Output: Invitation token (valid 1h):
#         eyJkb21haW5fbmFtZSI6ImhvbWUucGVyc2lzdC5oeXBlcm1lc2gi...

# Sprint 3: Peer uses invitation to join
hypermesh connect home.persist.hypermesh --invite eyJkb21haW5fbmFtZSI6...
# Output: Invitation verified. Joined domain 'home.persist.hypermesh' — connected to 3 peer(s)

# Sprint 1: List domains
hypermesh domain list
# Output:
# Domain                      | Network ID       | Privacy | Parent
# persist.hypermesh           | a3b7c9d2e1f04a5b | public  | —
# home.persist.hypermesh      | 7f2a8b4c01239def | private | persist.hypermesh

# Sprint 3: Show domain peers
hypermesh domain nodes home.persist.hypermesh
# Output:
# Node ID    | Address           | Sync State
# node-abc1  | [fd00::10]:9292   | Synchronized (height: 42)
# node-def2  | [fd00::20]:9292   | Syncing (75%, 2 peers)
```

---

### Critical Files for Implementation

- `/home/persist/hypermesh/core/blockmatrix/src/dns/registration.rs` — Core change: add `register_domain()` method that creates `DomainRegistration`, validates PoS, writes blockchain asset, and creates federated DNS pool. This is the central integration point between domains and the existing DNS/blockchain infrastructure.
- `/home/persist/hypermesh/core/blockmatrix/src/dns/resolver.rs` — Hierarchical resolution: add `resolve_hierarchical()` that walks domain components right-to-left querying parent pools. Critical for cross-domain name resolution.
- `/home/persist/hypermesh/core/blockmatrix/src/blockchain/sync_manager.rs` — Integration target: `join_network()` at line 216 is the existing entry point that `DomainNetworkManager` calls with domain-derived `network_id`. Understanding its `NetworkMembership`, `SyncState`, and `SyncConfig` types is essential.
- `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` — CLI entry point: add `Domain` and `Connect` subcommands (extends existing `Commands` enum at line 113), wire execution through to `DnsRegistrar`, `DomainNetworkManager`, and `NetworkManager`.
- `/home/persist/hypermesh/core/blockmatrix/src/dns/mod.rs` — Module hub: register new `domain` and `invitation` submodules, add error variants to `DnsError`, manage re-exports. The existing `Domain` struct (line 81) with `parse()`, `parent()`, `is_public()`, `is_federated()` is the foundation for all domain operations.