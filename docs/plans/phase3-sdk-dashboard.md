# Phase 3: Dashboard SDK & UI Platform -- Complete Implementation Plan

## Overview

Phase 3 builds the developer-facing SDK layer and dashboard asset system for HyperMesh. It connects the existing gateway (HTTP/3 on port 8443), catalog (package registry with STOQ API), and node binary (clap CLI with Start/Status/Store/Fetch/DNS commands) into a cohesive platform where dashboards are first-class assets served through scope-aware routing.

### Architecture Diagram

```
                         Developer Workflow
                         ==================

  hypermesh dashboard init    hypermesh dashboard deploy ./dist
         |                              |
         v                              v
  scaffold generator          dashboard.toml parser
  (Sprint 4)                  (Sprint 2)
                                        |
                                        v
                              Asset Pipeline
                              (Compress -> Encrypt -> Shard -> Distribute)
                                        |
                                        v
                              Catalog Registration
                              (AssetKind::User + Dashboard type)
                                        |
         +-----------+------------------+--------------------+
         v           v                  v                    v
      BlockMatrix   ShardStore    DNS Registration    Gateway Serving
      (blockchain)  (local/dist)  (domain -> IPv6)    (scope-aware)

                         Runtime Request Flow
                         ====================

  Browser --HTTP/3--> Gateway (:8443)
                         |
                    +----+----+
                    | AuthMgr |-->AuthResult (Anonymous/Authenticated/BootstrapRequired)
                    +----+----+
                         |
                    +----+----------+
                    | DomainRouter  |--> resolve "persist.hypermesh" -> DashboardRoute
                    +----+----------+
                         |
                    +----+--------------+
                    | DashboardServer   |--> scope = match auth_result {
                    | (Sprint 3)        |       Anonymous => "public/"
                    +----+--------------+       Authenticated => "private/"
                         |                      Owner => "admin/"
                         v                   }
                    Asset Cache --> Pipeline Reconstruction (on miss)
                         |
                         v
                    HTTP/3 Response (static files + correct Content-Type)

                         SDK Architecture
                         ================

  TypeScript SDK --+
  Go SDK ----------+--> Unix Socket --> JSON-RPC --> Daemon (node.rs)
  Python SDK ------+                                    |
  C FFI -----------+                                    +-- status()
                                                        +-- dns.register()
  Rust SDK -------------> Unix Socket (local)           +-- asset.store()
                     or   STOQ (remote)                 +-- network.peers()
                                                        +-- blockchain.height()
```

**Prerequisite assumption**: Phase 1 delivers a daemon process with JSON-RPC over Unix socket at `~/.hypermesh/hypermesh.sock`. This plan references that IPC layer as an input dependency. If Phase 1 is not yet complete, Sprint 1 must be sequenced after it.

---

## Sprint 1: Rust SDK Crate (~400 lines)

**Goal**: New `hypermesh-sdk` crate wrapping IPC (local) and STOQ (remote) into a typed Rust API.

### Step 1.1: Create crate skeleton

Create `/home/persist/hypermesh/core/hypermesh-sdk/Cargo.toml`:
- Add `"hypermesh-sdk"` to workspace members in `/home/persist/hypermesh/core/Cargo.toml` (line 12, after `"engauge"`)
- Dependencies: `hypermesh-lib` (workspace), `tokio` (workspace), `serde` (workspace), `serde_json` (workspace), `thiserror` (workspace), `tracing` (workspace), `anyhow` (workspace)
- Optional dependency on `stoq` (feature-gated `remote` feature) for STOQ transport mode

**Estimated lines**: ~30 (Cargo.toml)

### Step 1.2: Connection modes and error types

Create `/home/persist/hypermesh/core/hypermesh-sdk/src/lib.rs`:
```rust
pub mod client;
pub mod api;
pub mod error;

pub use client::{HyperMeshClient, ConnectionMode};
pub use error::SdkError;
```

Create `/home/persist/hypermesh/core/hypermesh-sdk/src/error.rs` (~40 lines):

The `SdkError` enum covers five failure categories:
- `Connection(String)` -- socket or STOQ connection failures
- `Ipc(String)` -- Unix socket I/O errors
- `Rpc { code: i32, message: String }` -- JSON-RPC error responses from the daemon
- `Serialization(String)` -- JSON encoding/decoding failures
- `Timeout(std::time::Duration)` -- request timeout exceeded
- `NotConnected` -- call attempted before `connect()`

All variants derive `thiserror::Error` with Display implementations.

### Step 1.3: Client connection management

Create `/home/persist/hypermesh/core/hypermesh-sdk/src/client.rs` (~120 lines):

Key types:
```rust
pub enum ConnectionMode {
    /// Local Unix socket (default: ~/.hypermesh/hypermesh.sock)
    Local { socket_path: Option<PathBuf> },
    /// Remote STOQ connection (requires `remote` feature)
    #[cfg(feature = "remote")]
    Remote { endpoint: String, pos_token: Option<String> },
}

pub struct HyperMeshClient {
    mode: ConnectionMode,
    inner: ClientInner,
}
```

`ClientInner` is an enum wrapping either a `tokio::net::UnixStream` (with newline-delimited JSON framing) or a `StoqApiClient` from the existing `/home/persist/hypermesh/core/stoq/src/api/mod.rs` (line 257).

Methods:
- `async fn connect(mode: ConnectionMode) -> Result<Self, SdkError>` -- establish connection
- `async fn connect_local() -> Result<Self, SdkError>` -- convenience for default socket path `~/.hypermesh/hypermesh.sock`
- `fn is_connected(&self) -> bool`
- `async fn disconnect(&mut self)`
- `async fn raw_call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, SdkError>` -- low-level JSON-RPC call

The JSON-RPC framing follows the same wire format as Phase 1's IPC: newline-delimited JSON with `{"jsonrpc":"2.0","id":N,"method":"...","params":{...}}` request and `{"jsonrpc":"2.0","id":N,"result":...}` or `{"jsonrpc":"2.0","id":N,"error":{"code":...,"message":...}}` response. Request IDs auto-increment via an `AtomicU64`.

### Step 1.4: API modules

Create `/home/persist/hypermesh/core/hypermesh-sdk/src/api/mod.rs` with six submodules. Each API group is a zero-cost wrapper struct `XxxApi<'a>` holding `&'a HyperMeshClient` and delegating to `raw_call`.

**`api/node.rs`** (~40 lines):
- `async fn status(&self) -> Result<NodeStatus, SdkError>` -- calls JSON-RPC method `node.status`
- `async fn connect_network(&self, mode: &str) -> Result<(), SdkError>` -- calls `node.connect`
- `async fn disconnect_network(&self) -> Result<(), SdkError>` -- calls `node.disconnect`

Return type `NodeStatus` contains: `node_id: String`, `privacy_mode: String`, `chain_height: u64`, `peer_count: usize`, `uptime_secs: u64`.

**`api/dns.rs`** (~35 lines):
- `async fn register(&self, name: &str, addr: &str) -> Result<DnsRecord, SdkError>` -- calls `dns.register`
- `async fn resolve(&self, name: &str) -> Result<Option<String>, SdkError>` -- calls `dns.resolve`
- `async fn list(&self) -> Result<Vec<DnsRecord>, SdkError>` -- calls `dns.list`

These methods mirror the existing `DnsAction` enum in `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (lines 149-166).

**`api/asset.rs`** (~40 lines):
- `async fn store(&self, data: &[u8], metadata: AssetMetadata) -> Result<AssetInfo, SdkError>` -- calls `asset.store`
- `async fn fetch(&self, asset_id: &str) -> Result<Vec<u8>, SdkError>` -- calls `asset.fetch`
- `async fn list(&self) -> Result<Vec<AssetInfo>, SdkError>` -- calls `asset.list`
- `async fn info(&self, asset_id: &str) -> Result<AssetInfo, SdkError>` -- calls `asset.info`

These mirror the existing `Store` and `Fetch` commands in node.rs (lines 127-140).

**`api/network.rs`** (~30 lines):
- `async fn peers(&self) -> Result<Vec<PeerInfo>, SdkError>` -- calls `network.peers`
- `async fn connect_peer(&self, addr: &str) -> Result<(), SdkError>` -- calls `network.connect_peer`
- `async fn disconnect_peer(&self, node_id: &str) -> Result<(), SdkError>` -- calls `network.disconnect_peer`

**`api/blockchain.rs`** (~30 lines):
- `async fn height(&self) -> Result<u64, SdkError>` -- calls `blockchain.height`
- `async fn block(&self, index: u64) -> Result<BlockInfo, SdkError>` -- calls `blockchain.block`
- `async fn validate(&self) -> Result<ValidationResult, SdkError>` -- calls `blockchain.validate`

**`api/topology.rs`** (~25 lines):
- `async fn neighbors(&self) -> Result<Vec<NeighborInfo>, SdkError>` -- calls `topology.neighbors`
- `async fn route(&self, target: &str) -> Result<RouteInfo, SdkError>` -- calls `topology.route`
- `async fn info(&self) -> Result<TopologyInfo, SdkError>` -- calls `topology.info`

**Estimated lines**: ~200 across all API modules.

### Step 1.5: Ergonomic access

Add accessor methods to `HyperMeshClient` (~30 lines):
```rust
pub fn node(&self) -> api::NodeApi<'_> { ... }
pub fn dns(&self) -> api::DnsApi<'_> { ... }
pub fn asset(&self) -> api::AssetApi<'_> { ... }
pub fn network(&self) -> api::NetworkApi<'_> { ... }
pub fn blockchain(&self) -> api::BlockchainApi<'_> { ... }
pub fn topology(&self) -> api::TopologyApi<'_> { ... }
```

Usage: `let status = client.node().status().await?;`

### Test Plan (Sprint 1)
- **SdkError formatting**: 5 tests verifying Display output for each variant
- **JSON-RPC serialization**: 4 tests verifying request construction and response parsing (success, error, malformed, timeout)
- **API method tests** (mock socket server): 18 tests total (3 per module) -- create a mock Unix socket server that returns canned JSON-RPC responses, verify the SDK deserializes them into the correct typed structs
- **Integration test**: 1 test starting the real daemon (if Phase 1 available), connecting the SDK, calling `status()`

**Total estimated tests**: ~28

### Quality Gates
- `cargo check -p hypermesh-sdk` compiles with zero warnings
- All 28 tests pass
- `SdkError` covers all failure modes without any catch-all variants
- No `unwrap()` in production code (only in test code)
- Every public type derives `Debug` and is `Send + Sync`
- API method signatures match 1:1 with daemon's IPC method names

---

## Sprint 2: Dashboard Asset Type & Registration (~300 lines)

**Goal**: Dashboards registered in Catalog with manifest format, CLI commands for management.

### Step 2.1: Add Dashboard to Catalog type registry

Modify `/home/persist/hypermesh/core/catalog/src/registry/` to register `"Dashboard"` as a new `AssetTypeDefinition` in `CatalogRegistry`.

Dashboard is a user-defined type registered at bootstrap, not a `SystemAssetKind` variant. The `SystemAssetKind` enum in `/home/persist/hypermesh/core/lib/src/asset.rs` (lines 59-71) has 10 variants for system resources (Cpu, Gpu, Memory, Storage, Network, Container, Economic, Blockchain, Dns, Transmission). Dashboards are Catalog packages, not system resources, so they use `AssetKind::User(UserAssetKind { type_name: "Dashboard", type_hash })` from line 30 of the same file.

The `CatalogRegistry` already supports type definitions via `AssetTypeDefinition`. Dashboard type schema: requires `name`, `version`, `description`, `domain`; optional `access.public`, `access.private`, `access.admin`.

**Estimated lines**: ~40

### Step 2.2: Dashboard manifest parser

Create `/home/persist/hypermesh/core/catalog/src/assets/dashboard.rs`:

Types:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardManifest {
    pub dashboard: DashboardMeta,
    pub access: DashboardAccess,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAccess {
    pub public: Option<String>,   // relative path to public/ directory
    pub private: Option<String>,  // relative path to private/ directory
    pub admin: Option<String>,    // relative path to admin/ directory
}
```

Functions:
- `fn parse_manifest(toml_str: &str) -> Result<DashboardManifest>` -- parse `dashboard.toml` using the `toml` crate (workspace dependency, line 93 of workspace Cargo.toml)
- `fn validate_manifest(manifest: &DashboardManifest, base_dir: &Path) -> Result<Vec<String>>` -- checks: directories referenced by `access` fields exist on disk, version is valid semver, domain follows HyperMesh naming rules (alphanumeric + hyphens, no dots except `.hypermesh`), at least one access scope defined

Wire this into `catalog/src/assets/mod.rs` (currently at `/home/persist/hypermesh/core/catalog/src/assets/mod.rs` lines 1-17) by adding `pub mod dashboard;`.

**Estimated lines**: ~80

### Step 2.3: Dashboard registration pipeline

Create `/home/persist/hypermesh/core/catalog/src/assets/dashboard_pipeline.rs`:

This module orchestrates the full registration flow:

```rust
pub struct DashboardRegistration {
    pub asset_id: String,
    pub domain: String,
    pub shard_map: ShardMapSummary,
    pub content_hash: ContentHash,
}

pub async fn register_dashboard(
    manifest: &DashboardManifest,
    dist_dir: &Path,
    catalog: &Catalog,
) -> Result<DashboardRegistration>
```

Steps:
1. **Validate** manifest + directory structure via `validate_manifest()`
2. **Archive**: Tar the dist directory into a single blob (using `tar` crate, workspace dependency line 136)
3. **Pipeline**: Feed the tarball through the existing `AssetPipeline` from `/home/persist/hypermesh/core/blockmatrix/src/assets/pipeline/` -- Brotli compress, Kyber-1024 encrypt (whole blob), Reed-Solomon 10+4 shard
4. **Store**: Persist shards via `ShardStore` from `/home/persist/hypermesh/core/blockmatrix/src/network/shard_store.rs`
5. **Register**: Create an `AssetPackage` with type "Dashboard" and register in Catalog via `catalog.publish_asset()`
6. **DNS**: Register the domain (`manifest.dashboard.domain`) pointing to the node's IPv6 address, following the existing DNS registration pattern in node.rs (lines 149-166)

The shard map (asset_id, shard hashes, decryption key, metadata) is persisted to `~/.hypermesh/shard_maps/` following the existing `ShardMap` pattern in node.rs (lines 267-276).

**Estimated lines**: ~100

### Step 2.4: CLI commands for dashboard management

Extend the existing `Commands` enum in `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (line 113):

Add a new variant:
```rust
/// Dashboard operations
Dashboard {
    #[clap(subcommand)]
    action: DashboardAction,
},
```

Add a new subcommand enum:
```rust
#[derive(Subcommand, Debug)]
enum DashboardAction {
    /// Initialize a new dashboard project
    Init { name: Option<String> },
    /// Deploy a dashboard from dist directory
    Deploy { path: PathBuf },
    /// List registered dashboards
    List,
    /// Remove a registered dashboard
    Remove { name: String },
    /// Show dashboard info
    Info { name: String },
}
```

Command handlers:
- `Deploy`: reads `dashboard.toml` from the given path, calls `register_dashboard()` from Step 2.3
- `List`: queries the local blockchain for Dashboard-type assets, outputs a table
- `Remove`: unregisters the dashboard from Catalog and removes the DNS entry
- `Info`: shows dashboard metadata, domain, shard count, content hash
- `Init`: delegates to scaffold generator (Sprint 4, stubbed here)

These commands are also exposed via IPC as `dashboard.init`, `dashboard.deploy`, `dashboard.list`, `dashboard.remove`, `dashboard.info` -- matching the SDK's method naming convention.

**Estimated lines**: ~80

### Test Plan (Sprint 2)
- **Manifest parsing**: valid TOML with all fields (1 test), valid TOML with optional fields omitted (1 test), minimal valid manifest (1 test)
- **Manifest validation errors**: missing required field `name` (1 test), missing `version` (1 test), missing `domain` (1 test), invalid version format (1 test)
- **Directory validation**: referenced directory does not exist (1 test), valid structure passes (1 test)
- **Dashboard registration**: mock pipeline, verify `AssetPackage` creation with correct type (1 test), verify DNS registration is triggered (1 test)
- **Dashboard type in CatalogRegistry**: register type, search for it, confirm it appears in results (2 tests)
- **CLI command parsing**: each `DashboardAction` variant parses correctly from args (5 tests)

**Total estimated tests**: ~18

### Quality Gates
- `cargo check -p catalog -p blockmatrix` compiles with zero warnings
- All 18 tests pass
- `DashboardManifest` roundtrips through TOML serialization (`toml::to_string` then `toml::from_str` produces identical struct)
- Dashboard type discoverable via `CatalogRegistry::search` with query "Dashboard"
- No hardcoded paths -- all use `data_dir` from CLI args or `~/.hypermesh/` default
- Shard map file created at expected location after successful deploy

---

## Sprint 3: Scope-Aware Gateway Serving (~350 lines)

**Goal**: Gateway serves dashboard content with different directories based on PoS auth level.

### Step 3.1: Dashboard asset server module

Create `/home/persist/hypermesh/core/gateway/src/dashboard_server.rs` (~150 lines):

This module resolves a domain to a dashboard asset, reconstructs content from the pipeline cache, and serves static files with correct Content-Type headers.

```rust
pub struct DashboardServer {
    /// Cache: domain -> DashboardCache
    cache: Arc<DashMap<String, DashboardCache>>,
    /// Cache TTL
    cache_ttl: Duration,
    /// Stats
    stats: Arc<DashboardServerStats>,
}

struct DashboardCache {
    /// Files per scope: scope_name -> (relative_path -> CachedFile)
    scopes: HashMap<String, HashMap<String, CachedFile>>,
    /// Owner identity for admin scope determination
    owner_identity: String,
    loaded_at: Instant,
}

struct CachedFile {
    content: Bytes,
    content_type: String,
}

struct DashboardServerStats {
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    scope_public: AtomicU64,
    scope_private: AtomicU64,
    scope_admin: AtomicU64,
    not_found: AtomicU64,
}
```

This follows the atomic stats pattern used throughout the gateway crate -- see `ScopeRouterStats` in `/home/persist/hypermesh/core/gateway/src/scope_router.rs` (lines 64-73) and `DomainRouterStats` in `/home/persist/hypermesh/core/gateway/src/domain_router.rs` (lines 29-33).

Methods:
- `fn new(cache_ttl: Duration) -> Self`
- `async fn serve(&self, domain: &str, path: &str, auth: &AuthResult) -> Result<Response<Bytes>>` -- main entry point
- `async fn load_dashboard(&self, domain: &str) -> Result<DashboardCache>` -- reconstruct from pipeline (fetch shard map, unshard, decrypt, decompress, untar, index files per scope)
- `fn invalidate(&self, domain: &str)` -- remove from cache (called on deploy)
- `fn stats(&self) -> DashboardServerStatsSnapshot`

### Step 3.2: Auth-level to scope mapping (~30 lines)

The scope determination uses the existing `AuthResult` enum from `/home/persist/hypermesh/core/gateway/src/auth.rs` (lines 24-42):

```rust
enum DashboardScope {
    Public,   // AuthResult::Anonymous or AuthResult::BootstrapRequired or AuthResult::Rejected
    Private,  // AuthResult::Authenticated where identity != owner
    Admin,    // AuthResult::Authenticated where identity == owner
}

fn determine_scope(auth: &AuthResult, owner_identity: &str) -> DashboardScope {
    match auth {
        AuthResult::Anonymous
        | AuthResult::BootstrapRequired
        | AuthResult::Rejected { .. } => DashboardScope::Public,
        AuthResult::Authenticated { identity, .. } => {
            if identity == owner_identity {
                DashboardScope::Admin
            } else {
                DashboardScope::Private
            }
        }
    }
}
```

This maps directly to the three directory scopes from `dashboard.toml`: `access.public`, `access.private`, `access.admin`.

### Step 3.3: Integrate into gateway router (~50 lines)

Modify `/home/persist/hypermesh/core/gateway/src/router.rs`. The existing `GatewayRouter::route` method (line 147) handles request routing. Currently, the `select_backend` method (line 229) matches on path prefixes (`/api/v1/trustchain`, `/api/v1/blockmatrix`, etc.) and returns an error for unknown paths (line 269).

Add dashboard serving as a **pre-check** before backend selection:

```rust
// In GatewayRouter::route(), after CORS preflight check (line 166)
// but before select_backend (line 174):
if let Some(dashboard_response) = self.try_serve_dashboard(&path, &req).await {
    let mut response = dashboard_response;
    self.cors.apply_cors(&mut response);
    logger.log_response(&response);
    return Ok(response);
}
```

The `try_serve_dashboard` method:
1. Extracts the Host header or SNI from the request
2. Checks `DomainRouter` (at `/home/persist/hypermesh/core/gateway/src/domain_router.rs`) for a dashboard route
3. If found, authenticates via `AuthManager` (at `/home/persist/hypermesh/core/gateway/src/auth.rs`)
4. Delegates to `DashboardServer::serve()`
5. Returns `None` if the domain is not a dashboard (falls through to API proxy)

Add `DashboardServer` as a field on `GatewayRouter` (initialized in `GatewayRouter::new` at line 51).

### Step 3.4: Content-Type detection and fallback (~40 lines)

Content-Type mapping in `DashboardServer`:
```rust
fn detect_content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "application/javascript",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("wasm") => "application/wasm",
        Some("map") => "application/json",
        _ => "application/octet-stream",
    }
}
```

Fallback behavior:
1. If the requested scope directory does not exist in the dashboard, fall back to the next lower scope (admin -> private -> public). This means a dashboard with only `public/` still works for authenticated users.
2. If the file is not found in the scope directory, serve `index.html` from that scope (SPA routing support -- essential for React Router used in `/home/persist/hypermesh/core/ui/frontend/package.json` line 42).
3. If no `index.html` exists in any applicable scope, return a 404 JSON response.
4. Path traversal prevention: reject any path containing `..` segments before lookup.

### Step 3.5: Cache invalidation (~80 lines)

Cache management:
- Each `DashboardCache` entry has a configurable TTL (default 300 seconds)
- On `dashboard deploy`, the node binary sends an invalidation signal to the gateway (via internal channel or IPC notification). The `DashboardServer::invalidate(domain)` method removes the cached entry.
- A background `tokio::spawn` task sweeps expired entries every 60 seconds
- Cache size is bounded: if more than 100 dashboards are cached, evict the least recently used entries

Wire the module into `/home/persist/hypermesh/core/gateway/src/lib.rs` by adding `pub mod dashboard_server;`.

### Test Plan (Sprint 3)
- **Scope determination**: 5 tests covering Anonymous->Public, BootstrapRequired->Public, Authenticated non-owner->Private, Authenticated owner->Admin, Rejected->Public
- **Content-Type detection**: 8 tests covering html, js, css, json, png, svg, wasm, unknown extension
- **Cache behavior**: cold load triggers pipeline reconstruction (1 test), warm hit returns cached content (1 test), TTL expiry triggers reload (1 test)
- **Fallback logic**: missing scope falls back to lower scope (1 test), SPA routing serves index.html for unknown paths (1 test), 404 for completely missing content (1 test)
- **Path traversal**: request with `../` segments is rejected (1 test)
- **Integration**: mock dashboard with 3 scope directories, verify correct content served per auth level (3 tests)

**Total estimated tests**: ~22

### Quality Gates
- `cargo check -p gateway` compiles with zero warnings
- All 22 tests pass (existing 194 gateway tests must also still pass)
- Dashboard serving does not break existing API proxy routing -- requests to `/api/v1/*` paths still reach backend services
- Cache eviction works correctly (no memory leak from stale dashboards)
- All responses include correct `Content-Type` and `Cache-Control: public, max-age=300` headers
- Path traversal attacks are blocked

---

## Sprint 4: Default Dashboard & CLI (~350 lines)

**Goal**: "Hello, HyperMesh!" page auto-deployed on onboarding; CLI scaffolding for developers.

### Step 4.1: Default dashboard content

Create a new module: `/home/persist/hypermesh/core/blockmatrix/src/dashboard/mod.rs`:
```rust
pub mod default;
pub mod scaffold;
```

Create `/home/persist/hypermesh/core/blockmatrix/src/dashboard/default.rs` with embedded HTML:
```rust
pub const DEFAULT_PUBLIC_HTML: &str = include_str!("default_content/public.html");
pub const DEFAULT_PRIVATE_HTML: &str = include_str!("default_content/private.html");
pub const DEFAULT_ADMIN_HTML: &str = include_str!("default_content/admin.html");
```

The three HTML files (~80 lines each) are stored at `/home/persist/hypermesh/core/blockmatrix/src/dashboard/default_content/`:

**`public.html`**: Static welcome page. No JavaScript, no live data (anonymous visitors see this). Contains:
- HyperMesh branding and logo
- Node's public domain name
- Brief description of what HyperMesh is
- Link to documentation
- Bootstrap instructions ("Connect with STOQ to access the full dashboard")

**`private.html`**: Live dashboard using `fetch()` against the gateway's API endpoints. On page load, uses the session's Bearer token (from cookie or URL parameter set during bootstrap) to call:
- `/api/v1/blockmatrix/status` -- displays chain height, peer count, uptime
- `/api/v1/blockmatrix/dns/list` -- displays registered domains
- `/api/v1/blockmatrix/assets` -- displays stored assets with sizes
- `/api/v1/caesar/status` -- displays Caesar balance summary
- `/api/v1/engauge/metrics` -- displays engagement metrics

Uses vanilla JavaScript (no framework dependency) for maximum portability. Refreshes data every 30 seconds.

**`admin.html`**: Extends private dashboard with write operations:
- Privacy mode switcher (calls `/api/v1/blockmatrix/privacy`)
- Peer connect/disconnect controls
- Dashboard management (list/remove registered dashboards)
- DNS registration form
- Asset upload form
- Log viewer (streams from `/api/v1/engauge/logs`)

**Estimated lines**: ~240 (3 HTML files)

### Step 4.2: Auto-registration on `connect public`

Modify the privacy mode transition flow in `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs`. After the existing `SetPrivacy` command handler successfully transitions to Public mode, add:

```rust
// After privacy mode transition to Public succeeds:
if !dashboard_already_registered(&persistence, &data_dir).await? {
    info!("Registering default dashboard...");
    register_default_dashboard(&blockchain, &dns_manager, &data_dir, &node_domain).await?;
    info!("Default dashboard live at http3://{}.hypermesh", node_domain);
}
```

The `register_default_dashboard` function (~50 lines):
1. Creates a temp directory structure: `public/index.html`, `private/index.html`, `admin/index.html` using the embedded constants from Step 4.1
2. Creates a `DashboardManifest` with `name = "default"`, `domain = "{node_name}.hypermesh"`, version `"0.1.0"`
3. Calls `register_dashboard()` from Sprint 2 Step 2.3
4. Writes a marker file (`~/.hypermesh/default_dashboard_registered`) to prevent re-registration
5. Cleans up the temp directory

The `dashboard_already_registered` function checks for the marker file.

**Estimated lines**: ~50

### Step 4.3: Scaffold generator

Create `/home/persist/hypermesh/core/blockmatrix/src/dashboard/scaffold.rs` (~100 lines):

`hypermesh dashboard init [name]` generates the following directory structure:
```
my-dashboard/
  dashboard.toml          # Pre-filled manifest with name and domain
  dist/
    public/
      index.html          # Minimal public welcome page
    private/
      index.html          # SDK-connected page with status display
    admin/
      index.html          # Admin page with management controls
  package.json            # If TypeScript SDK is desired
  tsconfig.json           # TypeScript configuration
  README.md               # Getting started guide
```

The scaffold uses simple string interpolation (replacing `{{name}}` and `{{domain}}` placeholders). The `handlebars` crate is available as a workspace dependency (line 144 of workspace Cargo.toml) but simple `str::replace` is sufficient for the 6 template files.

The generated `dashboard.toml`:
```toml
[dashboard]
name = "{{name}}"
version = "0.1.0"
description = "My HyperMesh Dashboard"
domain = "{{name}}.hypermesh"

[access]
public = "dist/public/"
private = "dist/private/"
admin = "dist/admin/"

[dependencies]
hypermesh-sdk = "^0.1"
```

### Step 4.4: Local development server

Add `Dev` variant to `DashboardAction` in node.rs:
```rust
/// Serve dashboard locally for development (no pipeline registration)
Dev {
    /// Path to dashboard project directory
    path: PathBuf,
    /// Port to serve on (default: 3000)
    #[clap(short, long, default_value = "3000")]
    port: u16,
},
```

Implementation (~60 lines): Uses the gateway's `DashboardServer` in a lightweight mode -- reads files directly from disk instead of from the asset pipeline. Serves on `http://[::1]:{port}` with the same scope-aware logic. The `auth` parameter defaults to `AuthResult::Authenticated { identity: "owner", .. }` so the developer sees the admin view.

Feature-gated behind `#[cfg(feature = "development")]` to avoid including a development HTTP server in production builds.

### Test Plan (Sprint 4)
- **Default HTML content**: public.html contains "HyperMesh" (1 test), private.html contains fetch() calls (1 test), admin.html contains form elements (1 test)
- **Auto-registration**: mock blockchain+DNS, verify `register_dashboard` called with correct manifest (1 test), verify DNS registration with node domain (1 test)
- **Auto-registration skip**: marker file exists, verify `register_dashboard` is NOT called (1 test)
- **Scaffold generator**: verify directory structure matches expected (1 test), verify `dashboard.toml` is valid TOML and parses correctly (1 test), verify name substitution works (1 test)
- **Idempotency**: run auto-registration twice, verify only one dashboard exists (1 test)

**Total estimated tests**: ~10

### Quality Gates
- `cargo check -p blockmatrix` compiles with zero warnings
- All 10 tests pass
- Default dashboard renders in a browser (manual verification with `hypermesh dashboard dev`)
- Private HTML successfully fetches live data from gateway API when authenticated
- Admin HTML includes all management controls listed in Step 4.1
- Scaffold generates a valid project that can be deployed with `hypermesh dashboard deploy`
- Auto-registration is idempotent -- running twice does not create duplicate dashboards or DNS entries

---

## Sprint 5: Multi-Language SDKs (~1500 lines total)

**Goal**: TypeScript, Go, Python, and C FFI bindings -- all using the same JSON-RPC protocol over Unix socket.

### Step 5.1: TypeScript SDK (`@hypermesh/sdk`, ~500 lines)

Create `/home/persist/hypermesh/core/hypermesh-sdk/ts/`:

**`package.json`** (~20 lines):
```json
{
  "name": "@hypermesh/sdk",
  "version": "0.1.0",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": { "build": "tsc", "test": "vitest run" },
  "devDependencies": { "typescript": "^5.9", "vitest": "^2.1" }
}
```

**`src/client.ts`** (~120 lines):
- `HyperMeshClient` class with static `connect(socketPath?: string)` factory
- Uses Node.js `net.createConnection()` for Unix socket
- JSON-RPC framing: newline-delimited JSON (read until `\n`, parse as JSON)
- Request ID auto-increment
- Promise-based with configurable timeout (default 30s)
- Exposes `raw_call(method: string, params: object): Promise<any>` for extensibility

**`src/browser-client.ts`** (~80 lines):
- `BrowserClient` class for use inside dashboard HTML
- Uses `fetch()` against gateway HTTP/3 endpoints instead of Unix socket
- Same API surface as `HyperMeshClient` but routes through `/api/v1/*`
- Automatically includes Bearer token from session storage

This addresses the fact that browser-based dashboards cannot use Unix sockets. The `private.html` and `admin.html` from Sprint 4 use this browser client.

**`src/api/node.ts`** (~40 lines): `status()`, `connect()`, `disconnect()`
**`src/api/dns.ts`** (~35 lines): `register()`, `resolve()`, `list()`
**`src/api/asset.ts`** (~40 lines): `store()`, `fetch()`, `list()`, `info()`
**`src/api/network.ts`** (~30 lines): `peers()`, `connectPeer()`, `disconnectPeer()`
**`src/api/blockchain.ts`** (~30 lines): `height()`, `block()`, `validate()`
**`src/api/topology.ts`** (~25 lines): `neighbors()`, `route()`, `info()`

**`src/types.ts`** (~80 lines): TypeScript interfaces matching all Rust SDK return types:
```typescript
interface NodeStatus {
  node_id: string;
  privacy_mode: string;
  chain_height: number;
  peer_count: number;
  uptime_secs: number;
}
// ... etc for DnsRecord, AssetInfo, PeerInfo, BlockInfo, etc.
```

**`src/error.ts`** (~30 lines): `SdkError` class with typed `code` field
**`src/index.ts`** (~20 lines): Re-exports `HyperMeshClient`, `BrowserClient`, all types

**`src/example/dashboard-component.tsx`** (~50 lines): React component demonstrating SDK usage with `@tanstack/react-query` (already a dependency in the UI frontend at line 37 of `/home/persist/hypermesh/core/ui/frontend/package.json`):
```tsx
import { useQuery } from '@tanstack/react-query';
import { HyperMesh } from '@hypermesh/sdk';

export function NodeStatus() {
  const { data } = useQuery({
    queryKey: ['node-status'],
    queryFn: async () => {
      const hm = await HyperMesh.connect();
      return hm.node.status();
    },
    refetchInterval: 5000,
  });
  return <div>Chain Height: {data?.chain_height ?? '...'}</div>;
}
```

### Step 5.2: Go SDK (`sdk-go`, ~400 lines)

Create `/home/persist/hypermesh/core/hypermesh-sdk/go/`:

**`go.mod`** (~5 lines): `module github.com/hypermesh-online/sdk-go`, Go 1.22+
**`client.go`** (~100 lines):
- `type Client struct` with `net.Conn` for Unix socket
- `func Connect(socketPath string) (*Client, error)` -- `net.Dial("unix", socketPath)`
- `func (c *Client) Close() error`
- `func (c *Client) call(ctx context.Context, method string, params interface{}) (json.RawMessage, error)` -- JSON-RPC with context-based cancellation and timeout
- Request ID via `atomic.AddInt64`

**`api.go`** (~150 lines): All API methods organized as method groups:
```go
func (c *Client) Status(ctx context.Context) (*NodeStatus, error)
func (c *Client) DnsRegister(ctx context.Context, name, addr string) (*DnsRecord, error)
func (c *Client) DnsResolve(ctx context.Context, name string) (string, error)
func (c *Client) DnsList(ctx context.Context) ([]DnsRecord, error)
func (c *Client) AssetStore(ctx context.Context, data []byte, meta AssetMetadata) (*AssetInfo, error)
// ... etc
```

**`types.go`** (~80 lines): Go structs with `json` tags matching the JSON-RPC response shapes
**`error.go`** (~30 lines): `type SdkError struct { Code int; Message string }` implementing `error` interface
**`client_test.go`** (~40 lines): Tests using a mock Unix socket listener

### Step 5.3: Python SDK (`hypermesh`, ~300 lines)

Create `/home/persist/hypermesh/core/hypermesh-sdk/python/`:

**`pyproject.toml`** (~15 lines): Package metadata, Python >= 3.10
**`hypermesh/client.py`** (~80 lines):
- `class HyperMeshClient` with sync and async modes
- Sync: `socket.socket(AF_UNIX)` with `socket.connect()`
- Async: `asyncio.open_unix_connection()`
- JSON-RPC framing with `json.dumps()` + `\n` delimiter
- Request ID counter

**`hypermesh/api.py`** (~100 lines): All API methods as instance methods:
```python
async def status(self) -> NodeStatus: ...
async def dns_register(self, name: str, addr: str) -> DnsRecord: ...
async def dns_resolve(self, name: str) -> Optional[str]: ...
# ... etc
```

**`hypermesh/types.py`** (~60 lines): `@dataclass` types with type hints:
```python
@dataclass
class NodeStatus:
    node_id: str
    privacy_mode: str
    chain_height: int
    peer_count: int
    uptime_secs: int
```

**`hypermesh/error.py`** (~20 lines): `class SdkError(Exception)` with `code` and `message` attributes
**`hypermesh/__init__.py`** (~10 lines): Re-exports
**`tests/test_client.py`** (~30 lines): Tests using `unittest.mock` for the socket

### Step 5.4: C FFI (`libhypermesh`, ~300 lines)

Add to the existing Rust SDK crate at `/home/persist/hypermesh/core/hypermesh-sdk/`, feature-gated:

**`Cargo.toml` additions**:
```toml
[lib]
crate-type = ["lib", "cdylib"]

[features]
default = []
remote = ["stoq"]
ffi = []

[build-dependencies]
cbindgen = "0.26"
```

**`src/ffi.rs`** (~200 lines):

Opaque handle pattern:
```rust
/// Opaque client handle for C consumers
pub struct HmClient {
    runtime: tokio::runtime::Runtime,
    client: HyperMeshClient,
}

#[repr(C)]
pub struct HmNodeStatus {
    pub chain_height: u64,
    pub peer_count: u64,
    pub uptime_secs: u64,
    pub node_id: [u8; 65],      // hex string + null terminator
    pub privacy_mode: [u8; 16], // "Anonymous\0", "Private\0", or "Public\0"
}

#[repr(C)]
pub enum HmError {
    Ok = 0,
    Connection = 1,
    Rpc = 2,
    Serialization = 3,
    Timeout = 4,
    NotConnected = 5,
    NullPointer = 6,
    BufferTooSmall = 7,
}
```

Exported functions:
```rust
#[no_mangle]
pub extern "C" fn hm_connect(socket_path: *const c_char) -> *mut HmClient { ... }

#[no_mangle]
pub extern "C" fn hm_status(client: *mut HmClient, out: *mut HmNodeStatus) -> HmError { ... }

#[no_mangle]
pub extern "C" fn hm_dns_register(
    client: *mut HmClient,
    name: *const c_char,
    addr: *const c_char,
    out_buf: *mut u8,
    buf_len: usize,
) -> HmError { ... }

#[no_mangle]
pub extern "C" fn hm_disconnect(client: *mut HmClient) { ... }

#[no_mangle]
pub extern "C" fn hm_free(client: *mut HmClient) { ... }
```

Each function validates pointers (null check returns `HmError::NullPointer`), creates a tokio runtime block for async operations, and writes results to caller-provided buffers.

**`cbindgen.toml`** (~15 lines): Configuration for header generation
**`build.rs`** (~20 lines): Runs cbindgen to generate `/home/persist/hypermesh/core/hypermesh-sdk/include/libhypermesh.h`
**`examples/c_example.c`** (~60 lines): Demonstrates connect, status, disconnect with error checking

### Cross-Language Consistency Requirements

All five SDKs (Rust, TypeScript, Go, Python, C) must adhere to:

1. **Identical JSON-RPC method names**: `node.status`, `dns.register`, `dns.resolve`, `dns.list`, `asset.store`, `asset.fetch`, `asset.list`, `asset.info`, `network.peers`, `network.connect_peer`, `network.disconnect_peer`, `blockchain.height`, `blockchain.block`, `blockchain.validate`, `topology.neighbors`, `topology.route`, `topology.info` (17 methods total)

2. **Identical JSON parameter keys**: All SDKs serialize the same field names in JSON. For example, `dns.register` always sends `{"name": "...", "addr": "..."}`.

3. **Identical JSON response shapes**: Return type field names are consistent across all languages. The JSON wire format is the single source of truth; each language maps to its idiomatic types.

4. **Naming conventions per language**:
   | Language | Method style | Type style | Module style |
   |----------|-------------|------------|-------------|
   | Rust | `snake_case` | `PascalCase` | `snake_case` |
   | TypeScript | `camelCase` | `PascalCase` | `camelCase` |
   | Go | `PascalCase` (exported) | `PascalCase` | package name |
   | Python | `snake_case` | `PascalCase` (dataclass) | `snake_case` |
   | C | `hm_snake_case` | `HmPascalCase` | flat namespace |

5. **Same error categories**: Connection, RPC (with numeric code + message string), Serialization, Timeout, NotConnected

6. **Same default socket path**: `~/.hypermesh/hypermesh.sock` in all SDKs. Overridable via `HYPERMESH_SOCKET` environment variable.

### Test Plan (Sprint 5)
- **TypeScript**: 12 tests -- client connection (2), each API group returns correct types (6), error handling for RPC errors (2), timeout behavior (1), BrowserClient fetch mock (1)
- **Go**: 10 tests -- client connect/close (2), each API group (5), context cancellation (1), error types (2)
- **Python**: 10 tests -- sync client (2), async client (2), each API group (4), error handling (2)
- **C FFI**: 6 tests -- connect (1), status (1), disconnect (1), null pointer safety (1), buffer overflow returns BufferTooSmall (1), free after disconnect (1)

**Total estimated tests**: ~38

### Quality Gates
- **TypeScript**: `npm run build` succeeds with zero TypeScript errors, `npm test` passes, no `any` types in public API
- **Go**: `go build ./...` succeeds, `go test ./...` passes, `go vet ./...` clean
- **Python**: `pytest` passes, all public functions have type hints, `mypy` clean
- **C FFI**: `cbindgen` generates valid header, example program compiles with `gcc`, links against `libhypermesh.so`, runs without segfault, valgrind reports zero leaks
- **Cross-language**: All five SDKs produce byte-identical JSON-RPC request messages for the same operation (verified by a cross-language integration test that captures wire output)

---

## How the Default Dashboard Dogfoods the SDK

The default dashboard (Sprint 4) is the primary integration test for the entire Phase 3 stack:

1. **`public/index.html`** validates:
   - Anonymous scope routing works (gateway serves public directory when no auth header present)
   - Static content serving (Content-Type detection, cache headers)
   - Domain routing (DomainRouter resolves `{node}.hypermesh` to the dashboard)

2. **`private/index.html`** validates:
   - Authenticated scope routing works (Bearer token -> AuthResult::Authenticated -> private directory)
   - TypeScript BrowserClient works (fetch() against gateway API endpoints)
   - SDK method coverage: `status()`, `dns.list()`, `asset.list()`, `blockchain.height()` all return real data
   - Error handling: if a backend is down, the dashboard degrades gracefully

3. **`admin/index.html`** validates:
   - Owner scope routing works (owner identity match -> admin directory)
   - Write operations through SDK: `dns.register()`, `network.connect_peer()`
   - Dashboard management: listing and removing dashboards works from the admin UI
   - Full round-trip: admin deploys a new dashboard -> it appears in the dashboard list -> it is accessible at its domain

This means every sprint's deliverables are validated through the default dashboard: Rust SDK (Sprint 1) provides the API contract, Catalog registration (Sprint 2) makes the dashboard a real asset, Gateway serving (Sprint 3) delivers it to the browser, and the TypeScript SDK (Sprint 5) powers the live data display.

---

## Security Model

### PoS Validation for All SDK Operations

1. **Local mode (Unix socket)**: The daemon process runs on the local machine with the node's identity. Local SDK calls are implicitly authorized as the node owner. The Unix socket is protected by filesystem permissions (`0600`, owner-only read/write). No PoS token is needed for local calls because physical access to the socket IS the authorization.

2. **Remote mode (STOQ)**: Every STOQ connection carries a PoS token in the handshake, as implemented in the node binary's bootstrap flow (`/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs`). The SDK's `ConnectionMode::Remote` requires a `pos_token` parameter. The daemon validates this token bilaterally (four proofs: PoSpace, PoStake, PoWork, PoTime) before accepting any RPC calls. Failed validation results in connection termination.

3. **Dashboard serving (HTTP/3)**: The gateway's `AuthManager` at `/home/persist/hypermesh/core/gateway/src/auth.rs` validates Bearer tokens against the session store and bootstrap handler. The `DashboardServer` maps `AuthResult` to scopes. No private or admin content is ever served to unauthenticated clients. The `AuthResult::Rejected` variant falls back to public scope, never to private.

4. **Dashboard deployment**: The `dashboard.deploy` IPC method is only available via local Unix socket (node owner). Remote deployment over STOQ requires valid PoS with the owner's identity. There is no HTTP/3 endpoint for deployment.

5. **Cross-language SDKs**: All non-Rust SDKs connect exclusively via Unix socket (local-only by default). The socket path is not exposed to the network. Remote access requires the Rust SDK's `remote` feature with STOQ transport and PoS authentication.

### Threat Mitigations

| Threat | Mitigation |
|--------|-----------|
| Unauthorized dashboard content access | Scope-aware serving: `AuthResult` determines which directory is served; public/private/admin boundaries enforced at gateway level |
| Dashboard content tampering | Content stored through asset pipeline with BLAKE3 content hashing; shards are integrity-verified on reconstruction |
| SDK credential theft | Unix socket permissions (`0600`); PoS tokens for remote; no passwords or long-lived secrets stored by SDK |
| Cache poisoning | Cache keyed by `(domain, scope)`; TTL-based expiry; explicit invalidation on deploy; no user-controlled cache keys |
| Path traversal | `DashboardServer` rejects any path containing `..` before file lookup; all paths resolved within the scope directory |
| Denial of service via dashboard upload | Pipeline handles compression/encryption/sharding with bounded memory; maximum dashboard size enforced at manifest validation |
| Cross-scope information leakage | Each scope has a separate file set; the DashboardServer never merges files across scopes; fallback only goes down (admin->private->public), never up |

---

## Dependency Graph & Sequencing

```
Phase 1 (IPC/daemon) ----------------------+
Phase 2 (DNS/domains) ---------------------+|
                                            ||
Sprint 1 (Rust SDK) <---------------------+| depends on Phase 1 IPC
    |                                       |
Sprint 2 (Dashboard Asset) <---------------+ depends on Phase 2 DNS
    |
Sprint 3 (Gateway Serving) <-- Sprint 2 (needs registered dashboards to serve)
    |
Sprint 4 (Default Dashboard + CLI) <-- Sprint 3 (needs serving infrastructure)
                                    <-- Sprint 1 (default dashboard uses SDK)
    |
Sprint 5 (Multi-Language SDKs) <-- Sprint 1 (follows same API contract)
                                   # Sprint 5 is independent of Sprints 2-4
                                   # Can be parallelized with Sprints 3-4
```

**Parallelization opportunity**: Sprint 5 (multi-language SDKs) depends only on Sprint 1 (the Rust SDK establishes the API contract and JSON-RPC method names). Once Sprint 1 is complete, Sprint 5 can run in parallel with Sprints 3 and 4 by assigning to a separate developer or agent.

**Critical path**: Sprint 1 -> Sprint 2 -> Sprint 3 -> Sprint 4

---

## Potential Challenges

1. **Phase 1 dependency**: The Rust SDK (Sprint 1) assumes a daemon with JSON-RPC over Unix socket exists from Phase 1. If Phase 1 is not complete, Sprint 1 must either stub the IPC layer with mock responses or be deferred until Phase 1 delivers. The stub approach is preferable because it allows Sprint 2-5 to proceed on schedule.

2. **Pipeline reconstruction latency**: Serving dashboard files from the asset pipeline (decompress + decrypt + unshard) on every HTTP/3 request would add hundreds of milliseconds of latency. The cache layer in Sprint 3 mitigates this for warm requests, but the first request per domain incurs cold-start latency. Mitigation: pre-load all registered dashboards into cache on gateway startup by scanning the catalog for Dashboard-type assets.

3. **Gateway crate structure**: The gateway's `main.rs` at `/home/persist/hypermesh/core/gateway/src/main.rs` uses `h3` and `h3-quinn` for HTTP/3. The library modules (`auth.rs`, `domain_router.rs`, `scope_router.rs`, `inbound.rs`, `router.rs`) are clean, well-tested, and follow a consistent pattern with `DashMap` + atomic stats. Adding `DashboardServer` requires it to integrate with `GatewayRouter` which currently only proxies to backend services via `select_backend`. The cleanest approach is a pre-routing check: try dashboard serving first, fall through to API proxy if the domain is not a dashboard.

4. **Content embedding**: The default dashboard HTML (Sprint 4) is embedded via `include_str!()` which means changes require recompilation. For the alpha phase this is acceptable. In a future phase, the default dashboard could be loaded from a template directory at runtime (read from `~/.hypermesh/templates/`).

5. **TypeScript SDK in browser vs Node.js**: The Unix socket approach works for Node.js but not for browser clients. Sprint 5 addresses this with a `BrowserClient` class that uses `fetch()` against the gateway's HTTP/3 API. The Sprint 4 default dashboard's `private.html` and `admin.html` must use this browser client, not the Node.js socket client. This split must be documented clearly in the SDK's README.

6. **C FFI tokio runtime**: The C FFI wrapper creates a tokio runtime per client handle (since C callers do not have an async runtime). This means each `hm_connect()` call spins up a runtime thread pool. For single-client usage this is fine, but multiple simultaneous clients from C would waste resources. Document that C consumers should create one client and share it.

---

## Summary of Deliverables by Sprint

| Sprint | New Files | Modified Files | Lines | Tests |
|--------|-----------|---------------|-------|-------|
| 1: Rust SDK | `hypermesh-sdk/` (8 files) | `Cargo.toml` (workspace members) | ~400 | 28 |
| 2: Dashboard Asset | `catalog/src/assets/dashboard.rs`, `dashboard_pipeline.rs` | `catalog/src/assets/mod.rs`, `blockmatrix/src/bin/node.rs` | ~300 | 18 |
| 3: Gateway Serving | `gateway/src/dashboard_server.rs` | `gateway/src/router.rs`, `gateway/src/lib.rs` | ~350 | 22 |
| 4: Default Dashboard | `blockmatrix/src/dashboard/` (6 files) | `blockmatrix/src/bin/node.rs`, `blockmatrix/src/lib.rs` | ~350 | 10 |
| 5: Multi-Language SDKs | `hypermesh-sdk/ts/` (12 files), `go/` (5 files), `python/` (6 files), `src/ffi.rs` | `hypermesh-sdk/Cargo.toml` | ~1500 | 38 |
| **Total** | **~37 new files** | **~8 modified files** | **~2900** | **116** |

---

### Critical Files for Implementation
- `/home/persist/hypermesh/core/gateway/src/auth.rs` - Contains the `AuthResult` enum (lines 24-42) that drives scope-aware dashboard serving; Sprint 3 maps these variants to Public/Private/Admin scopes
- `/home/persist/hypermesh/core/gateway/src/domain_router.rs` - Contains `DomainRouter` with exact and wildcard matching (lines 41-142); Sprint 3 adds dashboard domain routes here
- `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` - Contains the CLI `Commands` enum (lines 113-147) and `DnsAction` subcommand (lines 149-166); Sprint 2 adds `Dashboard` subcommand, Sprint 4 adds auto-registration in the `SetPrivacy` handler
- `/home/persist/hypermesh/core/stoq/src/api/mod.rs` - Contains the STOQ API framework (`ApiHandler` trait, `ApiRequest`/`ApiResponse` types, `StoqApiClient` at line 257) that defines the RPC pattern the Rust SDK wraps for remote mode
- `/home/persist/hypermesh/core/catalog/src/assets/mod.rs` - Module root for catalog asset types (lines 1-17); Sprint 2 adds `pub mod dashboard;` here and the `DashboardManifest`/`DashboardAccess` types that define the `dashboard.toml` format