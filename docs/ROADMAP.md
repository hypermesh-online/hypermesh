<!-- Copyright (c) 2026 Hypermesh Foundation. All rights reserved. -->

# HyperMesh Roadmap: CLI, Domains, and SDK Platform

Three phases that build on each other. Phase 1 is the foundation — everything else depends on it.

---

## Phase 1: CLI Architecture & Bootstrap Experience

**Goal**: A complete, intuitive CLI that talks to a running daemon — not one that restarts the node every time.

### The Problem

Today, every `hypermesh` CLI invocation creates a **new node instance**. Running `hypermesh status` while the systemd service is active spawns a second process that competes for the same data directory. There is no daemon/client separation. The `blockmatrix/src/cli/` library has a full `CommandExecutor` with topology, node, and asset commands that are **never wired to the binary**.

### Architecture: Daemon + Client

```
hypermesh start   →  Daemon process (long-lived, owns blockchain, network, STOQ)
                      ├── Unix socket listener at /run/hypermesh/ctl.sock
                      ├── STOQ listener on configured port
                      └── Periodic sync, block propagation, peer management

hypermesh <cmd>   →  Client process (short-lived, connects to daemon via Unix socket)
                      ├── Sends JSON-RPC request
                      ├── Receives response
                      └── Exits
```

The daemon exposes a local Unix socket (`/run/hypermesh/ctl.sock` or `~/.hypermesh/ctl.sock` for non-root). All CLI commands except `start` become thin clients that serialize a request, send it to the socket, and print the response.

### Simplified Bootstrap

```bash
# Current (verbose, requires knowing IPv6 address):
hypermesh --privacy public --bootstrap "[2600:1900:4001:cf7::]:9292" start

# New (intuitive defaults):
hypermesh connect public              # Join public mesh (resolves trust.hypermesh.online)
hypermesh connect private             # Localhost only (no network)
hypermesh connect anonymous           # Join mesh, untracked

# Override bootstrap endpoint:
hypermesh connect public --bootstrap "[fd00::1]:9292"

# Connect to a named network:
hypermesh connect public --network home.persist.hypermesh

# Start as reflector:
hypermesh connect public --reflector

# All other options still available:
hypermesh connect public --port 9300 --coord 10,20,5 --data-dir /var/lib/hypermesh
```

`connect` replaces `start` as the primary command. It:
1. Starts the daemon if not already running
2. Bootstraps the blockchain (or resumes from disk)
3. Connects to the mesh (unless private)
4. Prints connection status and exits (daemon keeps running)

`hypermesh disconnect` gracefully shuts down the daemon.

### Complete CLI Surface

```
hypermesh connect <mode>          Start daemon and join mesh
  public                          Full mesh participation (default bootstrap: trust.hypermesh.online)
  private                         Localhost only, no network
  anonymous                       Mesh participation, untracked
  --bootstrap <addr>              Override bootstrap peer(s)
  --network <name>                Connect to named network
  --reflector                     Run as public relay
  --port <n>                      STOQ port (default: 9292)
  --coord <x,y,z>                 Matrix coordinates (default: 0,0,0)
  --data-dir <path>               Persistence directory (default: ~/.hypermesh)

hypermesh disconnect              Gracefully stop the daemon

hypermesh status                  Node status summary
  --json                          Machine-readable output

hypermesh network                 Network operations
  peers                           List connected peers
  ping <peer_id>                  Ping a specific peer
  connect <addr>                  Connect to a specific peer
  disconnect <peer_id>            Disconnect from peer

hypermesh blockchain              Blockchain operations
  height                          Current chain height
  block <index|hash>              Show block details
  validate                        Verify chain integrity
  stats                           Chain statistics

hypermesh dns                     DNS operations
  register <name> [--addr <ip>]   Register name on blockchain
  resolve <name>                  Resolve a name
  list                            List registered names

hypermesh asset                   Asset operations
  list                            List registered assets
  info <id>                       Show asset details
  store <path>                    Store file through pipeline
  fetch <id> [--output <path>]    Fetch and reconstruct asset

hypermesh topology                Matrix topology
  neighbors [--radius <n>]        Find neighbors
  route <target_coord>            Show routing path
  info                            Matrix position and stats

hypermesh privacy                 Privacy management
  status                          Current privacy mode
  set <mode>                      Change privacy mode

hypermesh config                  Configuration
  get <key>                       Get config value
  set <key> <value>               Set config value
  show                            Show all configuration

hypermesh logs                    View daemon logs
  --follow                        Stream logs
  --level <level>                 Filter by level
```

### Implementation Plan

| Step | What | How | Lines |
|------|------|-----|-------|
| 1.1 | **IPC Server** | Unix socket listener in daemon, JSON-RPC protocol | ~200 |
| 1.2 | **IPC Client** | CLI client that connects to socket, sends command, prints result | ~150 |
| 1.3 | **Daemon lifecycle** | `connect` starts daemon (fork+detach or systemd), `disconnect` stops it | ~100 |
| 1.4 | **Wire existing commands** | Route `dns`, `store`, `fetch`, `status` through IPC | ~100 |
| 1.5 | **New query commands** | `blockchain`, `network`, `topology`, `asset` — wire CommandExecutor | ~200 |
| 1.6 | **`connect` command** | Replace `start`, add trust.hypermesh.online default, `--network` flag | ~80 |
| 1.7 | **`--coord x,y,z` shorthand** | Parse comma-separated instead of three flags | ~20 |
| 1.8 | **Config system** | TOML config at `~/.hypermesh/config.toml`, CLI overrides | ~150 |
| 1.9 | **Completions + man pages** | clap `generate` for bash/zsh/fish, man page generation | ~50 |
| | **Total** | | **~1050** |

### Config File (`~/.hypermesh/config.toml`)

```toml
[node]
privacy = "public"
coordinates = [0, 0, 0]
port = 9292
data_dir = "~/.hypermesh"
reflector = false

[network]
bootstrap = ["trust.hypermesh.online:9292"]

[logging]
level = "info"
```

### Quality Gates

- Every command works against running daemon (no second-process spawn)
- `hypermesh connect public` works with zero flags on a fresh install
- Tab completion works in bash/zsh/fish
- `--json` flag on every query command for scripting
- Daemon startup < 3 seconds, command response < 100ms

---

## Phase 2: Domains & Network Naming

**Goal**: Hierarchical domain naming that mirrors federation nesting, intuitive enough that `home.persist.hypermesh` just works.

### The Model

DNS names in HyperMesh are blockchain assets. Domain hierarchy maps to network scope:

```
hypermesh                         Global root (trust.hypermesh.online)
├── persist.hypermesh             User's top-level domain
│   ├── home.persist.hypermesh    Home mesh (private network)
│   │   ├── nas.home.persist      NAS device
│   │   └── desktop.home.persist  Desktop
│   ├── work.persist.hypermesh    Work mesh (federated)
│   └── public.persist.hypermesh  Public-facing services
├── google.hypermesh              Another org's domain
│   ├── cloud.google.hypermesh    Their public cloud
│   └── internal.google.hypermesh Their internal (not visible to you)
```

**Domains ARE networks.** Creating a domain creates a Network scope blockchain. Nodes that join the domain join that network's chain. Sub-domains are nested networks (same protocol, recursive).

### CLI Integration

```bash
# Register a top-level domain (requires PoS on global chain)
hypermesh domain register persist.hypermesh

# Create a sub-network (home mesh)
hypermesh domain create home.persist.hypermesh --privacy private

# Join a network by domain name
hypermesh connect public --network home.persist.hypermesh

# Register a device name within a domain
hypermesh dns register nas --domain home.persist.hypermesh --addr fd00::10

# Resolve across domains
hypermesh dns resolve nas.home.persist.hypermesh

# List domains you own
hypermesh domain list

# List nodes in a domain
hypermesh domain nodes home.persist.hypermesh

# Invite a device to your domain
hypermesh domain invite home.persist.hypermesh --peer <node_id>
```

### Architecture

| Component | What | Where |
|-----------|------|-------|
| **Domain Registry** | Maps domain names to Network scope chain IDs | Block in global chain (PoS asset) |
| **Hierarchical DNS** | Resolve `x.y.z` by walking chain: z → y.z → x.y.z | `blockmatrix/src/dns/` |
| **Network-as-Domain** | `domain create` = `SyncManager::join_network()` + DNS asset | `blockmatrix/src/network/` |
| **Cross-domain resolution** | Gateway routes DNS queries to appropriate chain | `gateway/src/domain_router.rs` |
| **Domain invite** | Signed invitation token (PoS) that grants join permission | New in `blockmatrix/src/dns/` |

### Implementation Plan

| Step | What | Lines |
|------|------|-------|
| 2.1 | **Domain asset type** | Add `Domain` to BaseSystemType, registration creates Network scope chain | ~100 |
| 2.2 | **Hierarchical resolver** | Walk domain components right-to-left, query appropriate chain | ~150 |
| 2.3 | **`domain` CLI commands** | register, create, list, nodes, invite | ~200 |
| 2.4 | **`--network` in connect** | Resolve domain name → bootstrap address → join | ~80 |
| 2.5 | **Cross-domain gateway routing** | Gateway forwards DNS queries to correct chain's reflectors | ~150 |
| 2.6 | **Domain invitation system** | Signed tokens for join authorization | ~120 |
| | **Total** | **~800** |

### Quality Gates

- `hypermesh domain create home.persist.hypermesh` takes < 5 seconds
- Joining by domain name works: `hypermesh connect public --network home.persist.hypermesh`
- Sub-domain nesting works to arbitrary depth
- Cross-domain resolution works through gateway
- Private domains are invisible to non-members

---

## Phase 3: Dashboard SDK & UI Platform

**Goal**: Users build their own UIs using a multi-language HyperMesh SDK, register them as discoverable assets, and serve them scope-aware (public vs private views on the same endpoint).

### The Problem (All 3a-3g)

Users need to:
- Build UIs that talk to HyperMesh (3e: SDK in Rust/TS/Go/Python/C/C#)
- Register UIs as discoverable assets (3b, 3d: via Catalog)
- Serve different views for public vs private access (3a, 3c: scope-aware)
- Go from SDK → code → register → deploy with minimal friction (3f: pipeline)
- See a default "Hello, HyperMesh!" page on onboarding (3g: default dashboard)

### Architecture

```
SDK (multi-language)          HyperMesh Node              User's Browser
┌──────────────┐             ┌──────────────┐             ┌──────────┐
│ TypeScript   │             │              │   HTTP/3    │          │
│ Go           │──STOQ──────▶│  Gateway     │◀────────────│ Browser  │
│ Rust         │  (native)   │  ├─ scope    │             │          │
│ Python       │             │  │  router   │             └──────────┘
│ C/C++        │             │  ├─ domain   │
│ C#           │             │  │  router   │
└──────────────┘             │  └─ asset    │
                             │     server   │
                             │              │
                             │  Catalog     │
                             │  ├─ register │
                             │  └─ discover │
                             └──────────────┘
```

### Dashboard as Asset

A dashboard is a registered asset with:
- **AssetType**: `UserDefined` with Catalog type registration
- **Content**: Static files (HTML/JS/CSS) stored through the asset pipeline (compressed, encrypted, sharded)
- **Scope config**: Declares which views are public vs private
- **Metadata**: Name, version, description, author, icon, access rules

```toml
# dashboard.toml — Dashboard asset manifest
[dashboard]
name = "my-app"
version = "0.1.0"
description = "My HyperMesh Dashboard"
domain = "persist.hypermesh"

[access]
public = "dist/public/"       # Served to anonymous visitors
private = "dist/private/"     # Served to authenticated mesh members
admin = "dist/admin/"         # Served to node owner only

[dependencies]
hypermesh-sdk = "^0.1"
```

### Scope-Aware Serving

When a request arrives at the gateway:

```
External request (HTTP/3)
  → Gateway checks PoS auth header
  → No auth: serve dashboard's "public" directory
  → Valid PoS (mesh member): serve "private" directory
  → Node owner: serve "admin" directory
```

This means the same URL (`https://persist.hypermesh.online/`) shows:
- **Public visitor**: Marketing page, public metrics, registration prompt
- **Mesh member**: Network dashboard, peer status, shared resources
- **Owner**: Full admin panel, configuration, private data

### Multi-Language SDK

The SDK exposes HyperMesh operations through STOQ:

**Core SDK (Rust)** — `hypermesh-sdk` crate:
```rust
pub struct HyperMeshClient {
    // Connects to local daemon via Unix socket or remote via STOQ
}

impl HyperMeshClient {
    pub async fn connect(endpoint: &str) -> Result<Self>;
    pub async fn status() -> Result<NodeStatus>;
    pub async fn store(data: &[u8]) -> Result<AssetId>;
    pub async fn fetch(id: &AssetId) -> Result<Vec<u8>>;
    pub async fn dns_register(name: &str, addr: Ipv6Addr) -> Result<()>;
    pub async fn dns_resolve(name: &str) -> Result<Ipv6Addr>;
    pub async fn peers() -> Result<Vec<PeerInfo>>;
    pub async fn blockchain_height() -> Result<u64>;
    // ... maps 1:1 to CLI commands
}
```

**Language bindings** via the IPC socket (Phase 1):

| Language | Approach | Package |
|----------|----------|---------|
| **Rust** | Native crate | `hypermesh-sdk` on crates.io |
| **TypeScript/JS** | Unix socket + JSON-RPC | `@hypermesh/sdk` on npm |
| **Go** | Unix socket + JSON-RPC | `github.com/hypermesh-online/sdk-go` |
| **Python** | Unix socket + JSON-RPC | `hypermesh` on PyPI |
| **C/C++** | Rust FFI (cbindgen) | `libhypermesh.h` + `.so`/`.dylib` |
| **C#** | Rust FFI (P/Invoke) or Unix socket | `HyperMesh.Sdk` on NuGet |

The JSON-RPC over Unix socket approach (from Phase 1's daemon architecture) means **every language gets SDK support for free** — they just need a Unix socket client and JSON serialization, which every language has.

For remote access (game engines, cloud services), the SDK can also connect via STOQ directly, authenticated by PoS.

### Default Dashboard (3g)

On first `hypermesh connect public`, the node registers a default dashboard asset:

```
Hello, HyperMesh!

You have registered the public domain ${domain_name} @ ${ipv6_address}
and are running ${node_count} node(s)!

You have registered ${catalog_count} catalog package(s) and
${caesar_enabled ? "have" : "have not"} set up Caesar network support!

${engauge_metrics_summary}

[Configure Your Node]  [Browse Catalog]  [View Network]
```

This is itself a dashboard asset registered in Catalog — dogfooding the platform. Users can replace it with their own dashboard or extend it.

### Implementation Plan

| Step | What | Lines |
|------|------|-------|
| 3.1 | **`hypermesh-sdk` Rust crate** | Client library wrapping IPC socket, async API | ~400 |
| 3.2 | **Dashboard asset type** | Catalog type registration for dashboards, manifest format | ~200 |
| 3.3 | **Scope-aware gateway serving** | Route to public/private/admin dirs based on auth | ~200 |
| 3.4 | **`hypermesh dashboard` CLI** | `deploy`, `list`, `remove` commands | ~150 |
| 3.5 | **Default "Hello, HyperMesh!" dashboard** | Static HTML + live data from SDK | ~300 |
| 3.6 | **TypeScript SDK** | `@hypermesh/sdk` npm package (Unix socket + JSON-RPC) | ~500 |
| 3.7 | **Go SDK** | `sdk-go` (Unix socket + JSON-RPC) | ~400 |
| 3.8 | **Python SDK** | `hypermesh` PyPI package | ~300 |
| 3.9 | **C FFI bindings** | cbindgen from Rust SDK, header + shared lib | ~200 |
| 3.10 | **Dashboard registration pipeline** | `hypermesh dashboard deploy ./dist` → pipeline → register | ~150 |
| 3.11 | **Catalog dashboard discovery** | Browse/search dashboards, respect scope visibility | ~100 |
| | **Total** | **~2900** |

### Quality Gates

- `hypermesh dashboard deploy ./dist` registers a working dashboard in < 10 seconds
- Same URL shows different content based on auth (public/private/admin)
- TypeScript SDK: `npm install @hypermesh/sdk` + 5 lines of code to connect
- Default dashboard shows live node metrics on first connect
- Google-scale scenario: org dashboard with department sub-dashboards, all scope-isolated
- SDK operations are PoS-validated (no unauthenticated access to private resources)

---

## Dependency Graph

```
Phase 1: CLI Architecture
  ├── 1.1-1.3: Daemon/IPC (foundation for EVERYTHING)
  ├── 1.4-1.5: Wire commands (unlocks all CLI)
  ├── 1.6-1.7: Simplified bootstrap (user experience)
  └── 1.8-1.9: Config + completions (polish)
         │
         ▼
Phase 2: Domains & Naming
  ├── 2.1-2.2: Domain model + resolver (builds on blockchain + DNS)
  ├── 2.3-2.4: CLI + connect integration (builds on Phase 1 CLI)
  └── 2.5-2.6: Gateway routing + invites (builds on gateway + PoS)
         │
         ▼
Phase 3: SDK & Dashboard Platform
  ├── 3.1: Rust SDK (wraps Phase 1 IPC socket)
  ├── 3.2-3.3: Dashboard assets + scope serving (builds on Phase 2 domains)
  ├── 3.4-3.5: CLI + default dashboard (builds on SDK)
  ├── 3.6-3.9: Multi-language SDKs (wraps Rust SDK or IPC directly)
  └── 3.10-3.11: Pipeline + discovery (builds on Catalog)
```

## Estimated Scope

| Phase | New Code | Modifications | Total |
|-------|----------|---------------|-------|
| Phase 1 | ~800 | ~250 | ~1050 |
| Phase 2 | ~600 | ~200 | ~800 |
| Phase 3 | ~2500 | ~400 | ~2900 |
| **Total** | **~3900** | **~850** | **~4750** |
