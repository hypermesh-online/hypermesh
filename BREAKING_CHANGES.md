# Breaking Changes Analysis

## 1. API Breaking Changes

### Language Runtime APIs
**Impact: HIGH** - Affects all consumers of VM execution

#### Before (with Julia)
```rust
pub enum LanguageRuntime {
    Julia(JuliaVM),
    Python(PythonVM),
    Rust(RustVM),
    // ...
}
```

#### After (without Julia)
```rust
pub enum LanguageRuntime {
    Python(PythonVM),
    Rust(RustVM),
    // Julia removed
}
```

**Affected Files**:
- `/blockmatrix/src/catalog/vm/mod.rs` - Exports removed
- `/blockmatrix/src/catalog/vm/languages/mod.rs` - Factory pattern broken
- `/catalog/src/validation/mod.rs` - Validator exports removed

**Migration Required**:
```rust
// Old code
let runtime = LanguageRuntime::create("julia", config)?;

// New code - will fail
// Must check for Julia and provide alternative
match language {
    "julia" => return Err("Julia support removed, use Python"),
    _ => LanguageRuntime::create(language, config)?
}
```

### DNS System Changes
**Impact: CRITICAL** - Core bootstrap mechanism

#### Deprecated API
```rust
// dns_over_quic.rs - DEPRECATED
pub struct DnsOverQuicClient {
    server_id: String,
}

// dns_over_stoq.rs - NEW
pub struct DnsOverStoqClient {
    stoq_client: Arc<StoqClient>,
    matrix_position: MatrixCoordinate,
}
```

**Breaking Changes**:
1. Constructor signature completely different
2. Resolution methods take different parameters
3. Certificate validation flow changed
4. No backward compatibility layer

**Affected Components**:
- All DNS resolution calls
- Certificate generation flows
- Bootstrap sequences
- TrustChain integration

### Asset System APIs
**Impact: HIGH** - Multiple conflicting implementations

#### Current Confusion
```rust
// blockmatrix/src/assets/
pub struct AssetId(pub String);

// catalog/src/assets.rs
pub struct AssetId {
    id: Uuid,
    network_scope: String,
}

// caesar/src/assets/
pub struct AssetId {
    chain_id: u64,
    asset_hash: [u8; 32],
}
```

**After Consolidation**:
- Single AssetId type
- Breaking all existing asset references
- Database migration required

## 2. Configuration Breaking Changes

### Transport Parameters
**Impact: MEDIUM** - STOQ configuration

#### Old Configuration
```toml
[transport]
enable_rsa = true
rsa_key_size = 2048
```

#### New Configuration
```toml
[transport]
enable_falcon = true
falcon_variant = "falcon-1024"
# RSA removed
```

**Files Affected**:
- `/stoq/config/*.toml`
- `/trustchain/config/production.toml`
- All deployment configurations

### Privacy Tier Configuration
**Impact: LOW** - New mandatory fields

#### Now Required
```toml
[privacy]
tier = "public"  # NEW: mandatory
asset_privacy = "private"  # NEW: separate from network
proxy_enabled = true  # NEW: NAT configuration
```

## 3. Database Schema Changes

### Asset Storage
**Impact: CRITICAL** - Data migration required

#### Old Schema
```sql
CREATE TABLE assets (
    id TEXT PRIMARY KEY,
    data BLOB
);
```

#### New Schema
```sql
CREATE TABLE assets (
    id TEXT PRIMARY KEY,
    network_scope TEXT NOT NULL,  -- NEW
    content_hash TEXT NOT NULL,   -- NEW
    matrix_position TEXT,          -- NEW
    consensus_proofs JSONB,        -- NEW
    data BLOB
);
```

**Migration Required**:
- Add new columns with defaults
- Backfill content hashes
- Update all queries
- Rebuild indexes

## 4. Network Protocol Changes

### STOQ Protocol
**Impact: HIGH** - Incompatible with old clients

#### Protocol Version Change
```rust
// Old
const PROTOCOL_VERSION: u32 = 1;

// New
const PROTOCOL_VERSION: u32 = 2;
// Includes PoS validation, matrix addressing
```

**Breaking Changes**:
1. New mandatory headers
2. Different handshake sequence
3. Matrix position requirements
4. PoS token validation

**No Backward Compatibility**: V1 clients cannot connect to V2 servers

## 5. Certificate Changes

### From RSA to FALCON-1024
**Impact: CRITICAL** - Security infrastructure

#### Certificate Generation
```rust
// OLD: RSA
let key = RsaKeyPair::generate(2048)?;

// NEW: FALCON
let key = FalconEngine::generate_keypair(FalconVariant::Falcon1024)?;
```

**Incompatibilities**:
- Existing RSA certs invalid
- CA chain must be regenerated
- Client verification fails
- TLS handshake changes

**Required Actions**:
1. Regenerate all certificates
2. Update all trust stores
3. Implement dual-mode temporarily
4. Plan cert rotation strategy

## 6. Import Path Changes

### Module Reorganization
**Impact: MEDIUM** - All imports break

#### Examples
```rust
// OLD
use blockmatrix::julia::JuliaVM;
use blockmatrix::dns::dns_over_quic;
use catalog::assets::AssetId;

// NEW
// use blockmatrix::julia::JuliaVM;  // REMOVED
use blockmatrix::dns::dns_over_stoq;  // MOVED
use blockmatrix::assets::AssetId;     // CONSOLIDATED
```

## 7. CLI Changes (Future)

### Command Structure
**Impact: LOW** - CLI not yet implemented

When implemented, will need to handle:
- Removed Julia commands
- Changed asset commands
- New matrix topology commands

## Migration Strategy

### Phase 1: Deprecation Warnings
```rust
#[deprecated(since = "2.0.0", note = "Use Python adapter instead")]
pub struct JuliaAdapter;

#[deprecated(since = "2.0.0", note = "Use dns_over_stoq")]
pub mod dns_over_quic;
```

### Phase 2: Compatibility Layer
```rust
// Temporary shim
pub mod compat {
    pub fn migrate_asset_id(old: OldAssetId) -> NewAssetId {
        // Migration logic
    }
}
```

### Phase 3: Version Bump
- Current: 1.0.0
- After cleanup: 2.0.0
- SemVer MAJOR bump required

### Phase 4: Migration Guide
Required documentation:
1. API migration examples
2. Configuration updates
3. Database migration scripts
4. Certificate rotation guide

## Risk Matrix

| Component | Breaking Changes | Risk | Mitigation |
|-----------|-----------------|------|------------|
| Julia Removal | 10+ APIs | LOW | Few users |
| DNS Consolidation | All DNS calls | HIGH | Gradual migration |
| Asset System | All asset refs | HIGH | Compatibility layer |
| Crypto Migration | All certs | CRITICAL | Dual-mode operation |
| Protocol Version | All connections | HIGH | Version negotiation |
| Database Schema | All queries | CRITICAL | Migration scripts |

## Recommendations

### Must Do Before Cleanup
1. **Version all APIs** - Add version headers
2. **Create migration tools** - Automated updates
3. **Document everything** - Migration guides
4. **Test compatibility** - Old client vs new server
5. **Plan rollback** - Emergency procedures

### Should Not Remove Yet
1. RSA support (need dual-mode)
2. Old DNS implementation (gradual migration)
3. Asset compatibility layer (6-month deprecation)

### Can Remove Now
1. Julia modules (isolated, low impact)
2. Archived documentation (duplicates)
3. Deprecated test code (already disabled)