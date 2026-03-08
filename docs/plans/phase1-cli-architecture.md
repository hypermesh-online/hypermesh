# Phase 1: CLI Architecture & Bootstrap Experience -- Implementation Plan

## Overview

### Problem

Every CLI invocation of `hypermesh` (e.g., `hypermesh status`, `hypermesh dns list`) performs the **full bootstrap sequence**: genesis block creation or recovery, hardware assessment, certificate loading, DNS initialization, and blockchain persistence setup. This takes seconds per invocation and means there is no persistent daemon process. The `start` subcommand runs the node inline and blocks until Ctrl+C; all other commands are one-shot operations that bootstrap a fresh node context, execute, and exit.

There is no IPC mechanism. The library-level CLI (`blockmatrix/src/cli/`) with its `CommandExecutor`, `CliCommand` enum, and `CliOutput` formatting is completely disconnected from the binary at `blockmatrix/src/bin/node.rs`. The extension CLI at `blockmatrix/src/extensions/cli/` is likewise unwired.

### Solution

Introduce a **daemon/client architecture** with Unix domain socket IPC using JSON-RPC 2.0:

```
                    +-----------------+
                    |  hypermesh CLI  |
                    | (thin client)   |
                    +--------+--------+
                             |
                    JSON-RPC over Unix socket
                    ($XDG_RUNTIME_DIR/hypermesh/ctl.sock)
                             |
                    +--------+--------+
                    |  hypermesh      |
                    |  daemon         |
                    |  (node.rs)      |
                    +--------+--------+
                             |
            +----------------+----------------+
            |                |                |
    NodeBootstrap    NetworkManager    NodeBlockchain
    (genesis/cert)   (STOQ/peers)     (blocks/assets)
```

**Daemon** (`hypermesh connect`): Runs the full bootstrap, starts the IPC server on a Unix socket, then enters the existing event loop (STOQ listener, sync, discovery). Holds `Arc` references to all live subsystems.

**Client** (all other commands): Connects to the Unix socket, sends a JSON-RPC request, prints the response, exits. If the daemon is not running, prints an error with instructions.

### Scope

4 sprints, approximately 1,300 lines of new code and 200 lines of modifications. No new crate dependencies required -- `tokio` (already `full` features, includes `tokio::net::UnixListener`/`UnixStream`), `serde`/`serde_json` (already present), and `dirs` (already v5.0) cover all needs.

---

## Sprint 1: IPC Foundation (~350 lines new)

**Goal**: Create a JSON-RPC 2.0 protocol layer over Unix domain sockets. The daemon can listen, clients can connect, and a request/response round-trip works with a `ping` method.

### Step 1.1: Create IPC module root

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/mod.rs` (~20 lines)

- Declare submodules: `pub mod protocol;`, `pub mod server;`, `pub mod client;`, `pub mod handler;`
- Re-export primary types: `IpcServer`, `IpcClient`, `RpcRequest`, `RpcResponse`, `RequestHandler`

### Step 1.2: Register the module in lib.rs

**File**: `/home/persist/hypermesh/core/blockmatrix/src/lib.rs` (~1 line, insert near line 270)

- Add `pub mod ipc;` alongside the existing `pub mod cli;`

### Step 1.3: Define JSON-RPC protocol types

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/protocol.rs` (~80 lines)

Types to create:

```rust
/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,         // Always "2.0"
    pub method: String,          // e.g., "status", "dns.register"
    pub params: serde_json::Value,  // {} for no params
    pub id: u64,
}

/// JSON-RPC 2.0 success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: u64,
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// Standard error codes
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;
pub const DAEMON_NOT_RUNNING: i64 = -32000;

/// Determine the socket path. Precedence:
/// 1. $HYPERMESH_SOCK (if set)
/// 2. $XDG_RUNTIME_DIR/hypermesh/ctl.sock
/// 3. ~/.hypermesh/ctl.sock (fallback)
pub fn socket_path() -> PathBuf { ... }

/// Determine the PID file path (sibling to socket).
pub fn pid_file_path() -> PathBuf { ... }
```

Helper functions:
- `RpcResponse::success(id, result)` -- construct success response
- `RpcResponse::error(id, code, message)` -- construct error response
- `RpcRequest::new(method, params)` -- construct request with auto-incrementing id

Wire protocol: Each message is a newline-delimited JSON line (`serde_json` line). The framing is: `<json>\n`. This avoids length-prefix complexity and works with `tokio::io::BufReader::read_line`.

### Step 1.4: Implement IPC server

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/server.rs` (~120 lines)

```rust
/// Unix socket IPC server for the daemon.
pub struct IpcServer {
    socket_path: PathBuf,
    handler: Arc<RequestHandler>,
    shutdown: tokio::sync::watch::Sender<bool>,
}

impl IpcServer {
    pub fn new(handler: Arc<RequestHandler>) -> Result<Self> { ... }

    /// Start listening. Spawns a tokio task per connection.
    /// Returns when shutdown signal is received.
    pub async fn run(&self) -> Result<()> {
        // 1. Create parent directory if needed
        // 2. Remove stale socket file if exists
        // 3. Bind UnixListener
        // 4. Write PID file
        // 5. Loop: accept connection, spawn handle_connection
        // 6. On shutdown: remove socket file, remove PID file
    }

    /// Signal the server to stop.
    pub fn shutdown(&self) { ... }

    /// Handle a single client connection (read lines, dispatch, write responses).
    async fn handle_connection(
        stream: tokio::net::UnixStream,
        handler: Arc<RequestHandler>,
    ) { ... }
}
```

Key implementation details:
- `tokio::net::UnixListener::bind(socket_path)` for the listener
- Each connection handled via `tokio::io::BufReader` reading newline-delimited JSON
- The handler is `Arc<RequestHandler>` so it can be shared across connections
- Graceful shutdown via `tokio::sync::watch` channel
- Socket file cleanup in a `Drop` impl or explicit shutdown method
- PID file at `socket_path.with_extension("pid")`

### Step 1.5: Implement IPC client

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/client.rs` (~70 lines)

```rust
/// Unix socket IPC client for CLI commands.
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    pub fn new() -> Self {
        Self { socket_path: socket_path() }
    }

    /// Check if daemon is running (socket exists and responds to ping).
    pub async fn is_daemon_running(&self) -> bool { ... }

    /// Send a request and wait for response. Timeout after 30 seconds.
    pub async fn call(&self, method: &str, params: serde_json::Value) -> Result<RpcResponse> {
        // 1. Connect to Unix socket
        // 2. Write request as JSON line
        // 3. Read response line
        // 4. Deserialize RpcResponse
    }

    /// Convenience: call and extract result, or return formatted error.
    pub async fn call_ok(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> { ... }
}

/// Check if daemon is running without creating a full client.
pub fn daemon_running() -> bool {
    socket_path().exists()
}
```

### Step 1.6: Implement request handler / dispatcher

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/handler.rs` (~60 lines)

```rust
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for an async handler function.
pub type HandlerFn = Arc<
    dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, RpcError>> + Send>>
    + Send
    + Sync,
>;

/// Routes JSON-RPC method names to handler functions.
pub struct RequestHandler {
    handlers: HashMap<String, HandlerFn>,
}

impl RequestHandler {
    pub fn new() -> Self { ... }

    /// Register a method handler.
    pub fn register(&mut self, method: &str, handler: HandlerFn) { ... }

    /// Dispatch a request to the appropriate handler.
    pub async fn dispatch(&self, request: RpcRequest) -> RpcResponse {
        match self.handlers.get(&request.method) {
            Some(handler) => {
                match handler(request.params).await {
                    Ok(result) => RpcResponse::success(request.id, result),
                    Err(rpc_err) => RpcResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(rpc_err),
                        id: request.id,
                    },
                }
            }
            None => RpcResponse::error(
                request.id,
                METHOD_NOT_FOUND,
                format!("Unknown method: {}", request.method),
            ),
        }
    }
}
```

### Tests (Sprint 1)

- **Unit tests in `protocol.rs`** (~5 tests): Serialize/deserialize `RpcRequest`, `RpcResponse`, `RpcError`. Test `socket_path()` resolution with/without env vars. Test helper constructors.
- **Unit tests in `handler.rs`** (~3 tests): Register handler, dispatch known method, dispatch unknown method returns `METHOD_NOT_FOUND`.
- **Integration test in `server.rs`** (~3 tests): Start server in background tokio task, connect client, send `ping` request, verify `pong` response. Test shutdown. Test stale socket cleanup.
- **Expected test count**: ~11 tests

### Quality Gates (Sprint 1)

1. `cargo check -p blockmatrix` passes with zero new warnings
2. `cargo test -p blockmatrix -- ipc` -- all 11 tests pass
3. JSON-RPC round-trip works: client sends `{"jsonrpc":"2.0","method":"ping","params":{},"id":1}`, server responds `{"jsonrpc":"2.0","result":"pong","id":1}`
4. Socket file is cleaned up on server shutdown
5. Client returns clear error when daemon is not running
6. No `.unwrap()` or `panic!()` in production code (pre-commit hook enforced)

---

## Sprint 2: Daemon Lifecycle & `connect`/`disconnect` Commands (~300 lines new, ~200 lines modified)

**Goal**: `hypermesh connect public` starts the daemon with IPC server, `hypermesh disconnect` stops it gracefully. PID file prevents duplicate daemons. All existing bootstrap logic preserved.

### Step 2.1: Refactor clap `Commands` enum

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (modify lines 113-148)

Replace the current `Commands` enum:

```rust
#[derive(Subcommand, Debug)]
enum Commands {
    /// Connect to the mesh (starts daemon if not running)
    Connect {
        /// Privacy mode for this connection
        #[clap(value_enum, default_value = "public")]
        mode: PrivacyModeArg,
        /// Run in foreground (don't detach)
        #[clap(long)]
        foreground: bool,
    },
    /// Disconnect from the mesh (stops daemon)
    Disconnect,
    /// Show node status
    Status,
    /// Transition to different privacy mode
    SetPrivacy {
        #[clap(value_enum)]
        mode: PrivacyModeArg,
    },
    /// Store a file as a distributed asset
    Store {
        path: std::path::PathBuf,
    },
    /// Fetch a distributed asset
    Fetch {
        asset_id: String,
        #[clap(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// DNS operations
    Dns {
        #[clap(subcommand)]
        action: DnsAction,
    },
    // --- DEPRECATED (hidden, shows migration message) ---
    /// [DEPRECATED] Use 'connect' instead
    #[clap(hide = true)]
    Start,
}
```

Changes:
- `Start` becomes hidden and prints deprecation message pointing to `connect`
- New `Connect` with `mode` (defaults to `public`) and `--foreground`
- New `Disconnect` that sends shutdown via IPC

### Step 2.2: Create shared daemon state struct

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/state.rs` (~50 lines, new file)

Add `pub mod state;` to `ipc/mod.rs`.

```rust
/// Shared state accessible to IPC handlers.
/// Holds Arc references to all daemon subsystems.
pub struct DaemonState {
    pub bootstrap: Arc<NodeBootstrap>,
    pub blockchain: Arc<NodeBlockchain>,
    pub persistence: Arc<PersistenceManager>,
    pub network: Option<Arc<NetworkManager>>,
    pub coordinate: MatrixCoordinate,
    pub node_id: String,
    pub data_dir: PathBuf,
    pub privacy_mode: PrivacyModeArg,
    pub started_at: std::time::Instant,
    pub shutdown_tx: tokio::sync::watch::Sender<bool>,
    pub executor: Arc<tokio::sync::Mutex<CommandExecutor>>,
}
```

This struct is constructed once during daemon startup and passed to `RequestHandler` closures via `Arc<DaemonState>`.

### Step 2.3: Wire `connect` command into the daemon startup path

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (modify `main()`, lines 992-1333)

The existing `Some(Commands::Start)` block (lines 993-1303, ~310 lines) becomes the body of `Some(Commands::Connect { mode, foreground })`:

1. Check if daemon already running: `if IpcClient::new().is_daemon_running().await { eprintln!("Daemon already running"); return Ok(()); }`
2. Set privacy mode from `mode` arg (replaces reading from `cli.privacy`)
3. All existing STOQ/network/sync setup remains unchanged
4. **After** network initialization, **before** the Ctrl+C wait:
   - Build `DaemonState` from all the live `Arc` references
   - Build `RequestHandler`, register `ping` handler (from Sprint 1)
   - Create `IpcServer`, spawn `server.run()` as a tokio task
5. Replace `tokio::signal::ctrl_c().await?` with: `tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = shutdown_rx.changed() => {} }` so both Ctrl+C and IPC `disconnect` can trigger shutdown
6. On shutdown: call `ipc_server.shutdown()`, then existing persistence flush

**Estimated modifications**: ~50 lines inserted into the existing Start block, ~20 lines restructured.

### Step 2.4: Implement `disconnect` command

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (add to match block)

```rust
Some(Commands::Disconnect) => {
    let client = IpcClient::new();
    if !client.is_daemon_running().await {
        eprintln!("No daemon running.");
        std::process::exit(1);
    }
    match client.call("shutdown", serde_json::json!({})).await {
        Ok(_) => println!("Daemon shutting down."),
        Err(e) => eprintln!("Failed to send shutdown: {e}"),
    }
}
```

The `shutdown` IPC method handler (registered in the `RequestHandler`) signals the server's `watch` channel, which triggers the `tokio::select!` in the daemon to break out of the wait loop.

### Step 2.5: Route `Status` command through IPC client

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (modify `Status` match arm)

For Sprint 2, only `Status` is converted to IPC-first. Other commands remain as-is (Sprint 3 handles the full conversion). The `Status` match arm becomes:

```rust
Some(Commands::Status) => {
    let client = IpcClient::new();
    if client.is_daemon_running().await {
        // Query daemon for live status
        let resp = client.call_ok("status", serde_json::json!({})).await?;
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        // Fallback: show bootstrap-only status (existing behavior)
        info!("Node Status (offline):");
        info!("  Genesis: {}", bootstrap.genesis_block().hash);
        info!("  Blockchain height: {}", bootstrap.blockchain().get_height().await);
        info!("  Privacy mode: {:?}", bootstrap.privacy_mode().await);
        info!("  Self-sufficient: yes");
    }
}
```

### Step 2.6: Register the `status` and `shutdown` IPC handlers

Register during daemon startup (step 2.3), inside the `Connect` block:

```rust
let state = Arc::new(daemon_state);
let mut handler = RequestHandler::new();

// ping
handler.register("ping", Arc::new(|_| Box::pin(async { Ok(json!("pong")) })));

// shutdown
let shutdown_tx_clone = shutdown_tx.clone();
handler.register("shutdown", Arc::new(move |_| {
    let tx = shutdown_tx_clone.clone();
    Box::pin(async move {
        let _ = tx.send(true);
        Ok(json!({"status": "shutting_down"}))
    })
}));

// status
let state_status = state.clone();
handler.register("status", Arc::new(move |_| {
    let s = state_status.clone();
    Box::pin(async move {
        let height = s.blockchain.get_height().await;
        let peer_count = match &s.network {
            Some(n) => n.get_node_count().await,
            None => 0,
        };
        Ok(json!({
            "node_id": s.node_id,
            "coordinate": format!("({},{},{})", s.coordinate.x, s.coordinate.y, s.coordinate.z),
            "chain_height": height,
            "privacy_mode": format!("{:?}", s.privacy_mode),
            "peers": peer_count,
            "uptime_secs": s.started_at.elapsed().as_secs(),
        }))
    })
}));
```

### Tests (Sprint 2)

- **DaemonState construction test** (~2 tests): Build a `DaemonState` with mock data, verify all fields accessible.
- **Connect/Disconnect integration test** (~3 tests): Start daemon in foreground mode in a tokio task, verify socket file created, send `status` via client, send `disconnect`, verify clean shutdown and socket removal.
- **Deprecation test** (~1 test): Running with `Start` subcommand prints migration message.
- **Expected test count**: ~6 tests

### Quality Gates (Sprint 2)

1. `hypermesh connect public --foreground` starts daemon, prints PID, creates socket
2. `hypermesh status` (in another terminal) returns JSON with chain_height, peers, uptime
3. `hypermesh disconnect` stops daemon, removes socket and PID file
4. Double `connect` prints "Daemon already running" and exits cleanly
5. `disconnect` with no daemon prints "No daemon running" and exits with code 1
6. `hypermesh start` prints deprecation message directing to `connect`
7. All existing tests still pass (`cargo test -p blockmatrix`)

---

## Sprint 3: Wire All Commands Through IPC (~400 lines modified)

**Goal**: Every CLI command routes through IPC when the daemon is running. The library `CommandExecutor` and extension CLI are wired into the handler dispatch.

### Step 3.1: Create handler registration module

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/handlers/mod.rs` (~30 lines, new file)

Create a handlers subdirectory to organize handler registration. This replaces inline handler registration in node.rs.

```rust
pub mod dns;
pub mod store;
pub mod network;
pub mod blockchain;
pub mod topology;
pub mod asset;

/// Register all IPC handlers onto the given RequestHandler.
pub fn register_all(handler: &mut RequestHandler, state: Arc<DaemonState>) {
    // Core
    register_ping(handler);
    register_shutdown(handler, state.shutdown_tx.clone());
    register_status(handler, state.clone());

    // DNS
    dns::register(handler, state.clone());

    // Store/Fetch
    store::register(handler, state.clone());

    // Network
    network::register(handler, state.clone());

    // Blockchain
    blockchain::register(handler, state.clone());

    // Topology (delegates to CommandExecutor)
    topology::register(handler, state.clone());

    // Asset (delegates to CommandExecutor)
    asset::register(handler, state.clone());
}
```

Update `ipc/mod.rs` to add `pub mod handlers;`.

### Step 3.2: DNS handlers

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/handlers/dns.rs` (~60 lines)

Methods:
- `dns.register` -- params: `{ "name": "foo", "addr": "::1" }` -- accesses `DaemonState.bootstrap.dns()` to register in local resolver, writes DNS asset to blockchain via `DaemonState.blockchain`, persists to disk via the same logic currently in `run_dns(DnsAction::Register)` (lines 730-780 of node.rs)
- `dns.resolve` -- params: `{ "name": "foo" }` -- calls `bootstrap.dns().resolve()`, returns address or null
- `dns.list` -- no params -- calls `bootstrap.dns().all_records()`, returns sorted name/addr pairs

Each handler accesses `DaemonState.bootstrap` to get the `DnsResolver` and `NodeBlockchain`.

### Step 3.3: Store/Fetch handlers

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/handlers/store.rs` (~50 lines)

Methods:
- `store` -- params: `{ "path": "/path/to/file" }` -- calls existing `run_store()` function with `ShardDistributionCtx` constructed from `DaemonState.network` and shard transport. Returns asset_id and shard map summary.
- `fetch` -- params: `{ "asset_id": "abc", "output": "/path/to/output" }` -- calls existing `run_fetch()`. Returns reconstruction status and byte count.

Note: `run_store` and `run_fetch` are currently free functions in node.rs. They should be extracted to a shared module (or left in node.rs and called from the handler closures which have access to the binary's scope). Since handlers are registered in the binary's `main()`, they can capture references to these functions directly.

### Step 3.4: Network handlers

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/handlers/network.rs` (~40 lines)

Methods:
- `network.peers` -- returns list of connected nodes from `NetworkManager::get_connected_nodes()`, formatted as JSON array with node_id, coordinate, address
- `network.connect` -- params: `{ "addr": "[::1]:9292" }` -- calls `NetworkManager::connect_to_peer()`, returns success/error

### Step 3.5: Blockchain handlers

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/handlers/blockchain.rs` (~50 lines)

Methods:
- `blockchain.height` -- returns `blockchain.get_height().await` as integer
- `blockchain.block` -- params: `{ "index": 5 }` or `{ "hash": "abc..." }` -- loads from `PersistenceManager::load_block()` using `BlockQuery::ByIndex` or `BlockQuery::ByHash`, returns block fields as JSON
- `blockchain.validate` -- iterates chain via `blockchain.get_chain().await`, verifies hash linkage, returns validation result

### Step 3.6: Topology and Asset handlers (wire existing CommandExecutor)

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/handlers/topology.rs` (~40 lines)
**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/handlers/asset.rs` (~40 lines)

These translate IPC params into `CliCommand` variants and execute via `CommandExecutor`:

Topology methods:
- `topology.neighbors` -- params: `{ "x": 0, "y": 0, "z": 0, "radius": 10.0 }` -- builds `TopologyCommand::QueryNeighbors`, locks executor mutex, calls `executor.execute()`, serializes `CliOutput` to JSON via its new `Serialize` impl
- `topology.info` -- builds `TopologyCommand::MatrixInfo`, same pattern
- `topology.routing_cost` -- params with `from_x/y/z` and `to_x/y/z` -- builds `TopologyCommand::RoutingCost`
- `topology.path` -- same coordinate pairs -- builds `TopologyCommand::ShowPath`

Asset methods:
- `asset.info` -- params: `{ "asset_id": "abc" }` -- builds `AssetCommand::Info`
- `asset.transfer` -- params with `asset_id`, `from_scope`, `to_scope` -- builds `AssetCommand::Transfer`
- `asset.list` -- lists assets from blockchain (not via CommandExecutor, since that uses in-memory registry; instead query blockchain asset records directly)

**Integration point**: The `CommandExecutor` currently uses an in-memory `HashMap<String, NodeRecord>`. In daemon context, the executor is initialized once (inside `DaemonState`) and reused across IPC calls. Since `CommandExecutor::execute` takes `&mut self`, it is wrapped in `Arc<tokio::sync::Mutex<CommandExecutor>>` inside `DaemonState`.

### Step 3.7: Convert all binary command match arms to IPC-first

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (modify match arms at lines 1304-1333)

Pattern for each command:

```rust
Some(Commands::SetPrivacy { mode }) => {
    let client = IpcClient::new();
    if client.is_daemon_running().await {
        let resp = client.call_ok(
            "set_privacy",
            serde_json::json!({"mode": format!("{:?}", mode)}),
        ).await?;
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        eprintln!("Error: No daemon running. Start with 'hypermesh connect'.");
        std::process::exit(1);
    }
}
```

Commands that require a running daemon: `SetPrivacy`, `Dns *`.

**Store and Fetch** preserve standalone fallback: if daemon is running, route through IPC (gets network distribution). If not, print a warning and run standalone (existing behavior with `dist_ctx = None`).

```rust
Some(Commands::Store { path }) => {
    let client = IpcClient::new();
    if client.is_daemon_running().await {
        let resp = client.call_ok(
            "store",
            serde_json::json!({"path": path.to_string_lossy()}),
        ).await?;
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        warn!("No daemon running. Storing locally without network distribution.");
        run_store(path, None).await?;
    }
}
```

### Step 3.8: Add `CliOutput` JSON serialization

**File**: `/home/persist/hypermesh/core/blockmatrix/src/cli/output.rs` (~15 lines modified)

Add `Serialize` derive to `CliOutput`, `CliTable`, and `CliError` so they can be returned over IPC. Currently these types derive `Debug, Clone, PartialEq` -- add `serde::Serialize`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum CliOutput { ... }

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CliTable { ... }

#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize)]
pub enum CliError { ... }
```

This enables topology/asset handlers to return `serde_json::to_value(&cli_output)` directly.

### Tests (Sprint 3)

- **Handler unit tests** (~12 tests, 2 per handler file): Each handler file tests request/response for its methods with a mock `DaemonState`.
- **Integration test** (~4 tests): Start daemon, exercise `dns.register` + `dns.resolve` round-trip, `blockchain.height`, `topology.info`, `store` + `fetch` round-trip.
- **Fallback test** (~2 tests): `store`/`fetch` work without daemon (standalone mode preserved).
- **Expected test count**: ~18 tests

### Quality Gates (Sprint 3)

1. All 17 IPC methods respond correctly (table below)
2. `hypermesh dns register foo` (with daemon) writes to blockchain and returns confirmation
3. `hypermesh dns list` returns records from running daemon
4. `hypermesh store /path/to/file` routes through daemon (gets network distribution)
5. Existing `CommandExecutor` tests still pass (no regressions in `blockmatrix::cli`)
6. `CliOutput` serializes to JSON correctly
7. Error responses use proper JSON-RPC error codes

**Complete IPC method table**:

| Method | Params | Source |
|--------|--------|--------|
| `ping` | `{}` | Built-in |
| `shutdown` | `{}` | Built-in |
| `status` | `{}` | `DaemonState` fields |
| `set_privacy` | `{"mode":"public"}` | `NodeBootstrap::set_privacy_mode` |
| `dns.register` | `{"name":"foo","addr":"::1"}` | `DnsResolver` + blockchain |
| `dns.resolve` | `{"name":"foo"}` | `DnsResolver` |
| `dns.list` | `{}` | `DnsResolver` |
| `store` | `{"path":"/file"}` | `AssetPipeline` + network |
| `fetch` | `{"asset_id":"abc","output":"/out"}` | `AssetPipeline` |
| `network.peers` | `{}` | `NetworkManager` |
| `network.connect` | `{"addr":"[::1]:9292"}` | `NetworkManager` |
| `blockchain.height` | `{}` | `NodeBlockchain` |
| `blockchain.block` | `{"index":5}` or `{"hash":"abc"}` | `PersistenceManager` |
| `blockchain.validate` | `{}` | `NodeBlockchain` |
| `topology.neighbors` | `{"x":0,"y":0,"z":0,"radius":10}` | `CommandExecutor` |
| `topology.info` | `{}` | `CommandExecutor` |
| `asset.info` | `{"asset_id":"abc"}` | `CommandExecutor` |

---

## Sprint 4: Config System + Polish (~250 lines new)

**Goal**: TOML config file at `~/.hypermesh/config.toml`, `--json` flag on all query commands, shell completions.

### Step 4.1: Create config module

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/config.rs` (~100 lines, new file)

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypermeshConfig {
    #[serde(default)]
    pub node: NodeConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Matrix coordinate X
    #[serde(default)]
    pub coord_x: i64,
    /// Matrix coordinate Y
    #[serde(default)]
    pub coord_y: i64,
    /// Matrix coordinate Z
    #[serde(default)]
    pub coord_z: i64,
    /// Data directory
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// STOQ port
    #[serde(default = "default_stoq_port")]
    pub stoq_port: u16,
    /// Bootstrap nodes
    #[serde(default)]
    pub bootstrap_nodes: Vec<String>,
    /// Default privacy mode
    #[serde(default = "default_privacy")]
    pub privacy: String,
    /// Run as reflector
    #[serde(default)]
    pub reflector: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for HypermeshConfig { ... }
impl Default for NodeConfig { ... }
impl Default for NetworkConfig { ... }
impl Default for LoggingConfig { ... }

impl HypermeshConfig {
    /// Load from default path (~/.hypermesh/config.toml).
    /// Returns default config if file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::default_path();
        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load from a specific path.
    pub fn load_from(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save to default path.
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    /// Config file path.
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".hypermesh")
            .join("config.toml")
    }
}

fn default_data_dir() -> String { "~/.blockmatrix".into() }
fn default_stoq_port() -> u16 { 9292 }
fn default_privacy() -> String { "public".into() }
fn default_log_level() -> String { "info".into() }
```

Update `ipc/mod.rs` to add `pub mod config;`.

### Step 4.2: Integrate config with CLI parsing

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (modify `main()`, ~30 lines inserted after line 814)

After `Cli::parse()`, load config and merge:

```rust
let cli = Cli::parse();

// Load config file (CLI flags override config values)
let config = match &cli.config {
    Some(path) => HypermeshConfig::load_from(path)?,
    None => HypermeshConfig::load()?,
};

// CLI flags override config file values (0 = not specified for coords)
let coord_x = if cli.coord_x != 0 { cli.coord_x } else { config.node.coord_x };
let coord_y = if cli.coord_y != 0 { cli.coord_y } else { config.node.coord_y };
let coord_z = if cli.coord_z != 0 { cli.coord_z } else { config.node.coord_z };
let stoq_port = if cli.stoq_port != 9292 { cli.stoq_port } else { config.network.stoq_port };
let data_dir_str = if cli.data_dir != "~/.blockmatrix" {
    cli.data_dir.clone()
} else {
    config.node.data_dir.clone()
};
let bootstrap_nodes = if !cli.bootstrap.is_empty() {
    cli.bootstrap.clone()
} else {
    config.network.bootstrap_nodes.clone()
};
let reflector = cli.reflector || config.network.reflector;
```

### Step 4.3: Add `config` IPC handlers and CLI subcommand

**File**: `/home/persist/hypermesh/core/blockmatrix/src/ipc/handlers/config.rs` (~40 lines, new handler file)

Methods:
- `config.show` -- returns current config as JSON
- `config.get` -- params: `{ "key": "network.stoq_port" }` -- dot-path traversal into config struct, returns specific value
- `config.set` -- params: `{ "key": "network.stoq_port", "value": "9292" }` -- updates in-memory config, writes to disk via `HypermeshConfig::save()`

Update `handlers/mod.rs` to add `pub mod config;` and wire registration.

Add `Config` subcommand to the `Commands` enum in `node.rs`:

```rust
/// Manage configuration
Config {
    #[clap(subcommand)]
    action: ConfigCommand,
},
```

```rust
#[derive(Subcommand, Debug)]
enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Get a config value by dot-path key
    Get { key: String },
    /// Set a config value by dot-path key
    Set { key: String, value: String },
}
```

`Config` commands route through IPC if daemon is running (to show live config), or read/write the file directly if not.

### Step 4.4: Add `--json` flag to all query commands

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (~20 lines)

Add a global `--json` flag to the `Cli` struct:

```rust
/// Output in JSON format
#[clap(long, global = true)]
json: bool,
```

In each command handler, when `cli.json` is true:
- If the response is already JSON (from IPC `call_ok`), print the raw `serde_json::Value` without `info!()` formatting
- If formatting locally (offline mode), wrap output in `CliOutput::Json` variant

### Step 4.5: Shell completions

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (~15 lines)

Add a hidden `Completions` command:

```rust
/// Generate shell completions
#[clap(hide = true)]
Completions {
    /// Shell to generate for
    #[clap(value_enum)]
    shell: clap_complete::Shell,
},
```

Handler:

```rust
Some(Commands::Completions { shell }) => {
    clap_complete::generate(
        shell,
        &mut Cli::command(),
        "hypermesh",
        &mut std::io::stdout(),
    );
}
```

Note: `clap_complete` is a lightweight crate. Check if `clap` workspace dependency already enables the `complete` feature. If not available, add `clap_complete = "4"` to `blockmatrix/Cargo.toml` dev-dependencies, or defer shell completions entirely. The core IPC architecture does not depend on this.

### Step 4.6: Add `--config` CLI flag

**File**: `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` (~5 lines added to `Cli` struct)

```rust
/// Path to config file (default: ~/.hypermesh/config.toml)
#[clap(long, global = true)]
config: Option<std::path::PathBuf>,
```

### Tests (Sprint 4)

- **Config loading tests** (~5 tests): Load from default path (missing file returns defaults), load from explicit path, parse all sections, CLI override precedence, save and reload round-trip.
- **Config IPC tests** (~3 tests): `config.show` returns valid JSON matching struct, `config.get` returns specific key, `config.set` persists change and subsequent `config.get` reflects it.
- **JSON output tests** (~2 tests): `--json` flag on `status` and `dns list` returns valid JSON (parseable by `serde_json::from_str`).
- **Expected test count**: ~10 tests

### Quality Gates (Sprint 4)

1. `~/.hypermesh/config.toml` is loaded on startup; missing file uses defaults without error
2. CLI flags override config file values (verify with explicit test)
3. `hypermesh config show` prints current config as JSON (via IPC if daemon running, from file if not)
4. `hypermesh config set network.stoq_port 9292` writes change to disk
5. `hypermesh status --json` prints machine-parsable JSON
6. `hypermesh completions bash` outputs valid bash completion script (if `clap_complete` available)
7. All previous tests still pass

---

## Migration Notes

### For users currently running `hypermesh start`

1. `hypermesh start` will print a deprecation warning: `"'start' is deprecated, use 'hypermesh connect public --foreground' instead"`
2. The `start` command will continue to work identically for one release cycle (it internally delegates to `connect public --foreground`)
3. All existing CLI flags (`-x`, `-y`, `-z`, `-p`, `-b`, `-s`, `--reflector`, `--data-dir`) remain and work identically
4. `hypermesh store` and `hypermesh fetch` continue to work without a daemon (standalone mode preserved), but gain network distribution when daemon is running

### Behavioral changes

| Before | After |
|--------|-------|
| `hypermesh status` boots full node, takes seconds | `hypermesh status` sends IPC request, returns in milliseconds |
| `hypermesh dns list` boots full node | `hypermesh dns list` sends IPC request |
| `hypermesh store file.bin` has no network | `hypermesh store file.bin` distributes to peers if daemon running |
| No way to check if node is running | `hypermesh status` returns immediately with live state |
| Every command pays bootstrap cost | Only `connect` pays bootstrap cost; all other commands are thin clients |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **Unix socket permissions** on multi-user systems | Medium | Low | Socket created in `$XDG_RUNTIME_DIR` (user-private). Fallback to `~/.hypermesh/` which is 0700. |
| **Stale socket file** after crash | High | Medium | On server start, check if PID file exists and process is alive (`kill(pid, 0)`). If PID dead, remove stale socket. On client connect failure to existing socket, suggest removal. |
| **Concurrent IPC requests** cause data races | Low | High | `DaemonState` uses `Arc` for all subsystems. `CommandExecutor` wrapped in `tokio::sync::Mutex`. All blockchain operations already use `Arc<NodeBlockchain>` with internal `RwLock`. |
| **Large responses** (e.g., full blockchain dump) | Low | Medium | IPC responses are newline-delimited JSON. Large responses stream fine over Unix sockets. Add `max_response_size` limit (10MB default). For very large queries, implement pagination. |
| **Breaking change** for scripts using `hypermesh start` | Medium | Medium | `start` command preserved as hidden alias. Deprecation warning for 1 release cycle. |
| **Config file conflicts** with existing `--data-dir` flag | Low | Low | Config file only sets defaults. CLI flags always win. Document precedence: CLI > env > config > defaults. |
| **macOS compatibility** (no `$XDG_RUNTIME_DIR`) | Medium | Low | Three-tier fallback: `$HYPERMESH_SOCK` > `$XDG_RUNTIME_DIR/hypermesh/ctl.sock` > `~/.hypermesh/ctl.sock`. macOS hits the `~/.hypermesh/` fallback. |
| **`clap_complete` dependency** | Low | Low | If not available as workspace dep, defer shell completions to a follow-up. Core IPC architecture is independent. |

---

## Summary: New Files

| File | Sprint | Lines (est.) | Purpose |
|------|--------|-------------|---------|
| `blockmatrix/src/ipc/mod.rs` | 1 | 20 | Module root + re-exports |
| `blockmatrix/src/ipc/protocol.rs` | 1 | 80 | JSON-RPC types, socket path resolution |
| `blockmatrix/src/ipc/server.rs` | 1 | 120 | Unix socket listener with graceful shutdown |
| `blockmatrix/src/ipc/client.rs` | 1 | 70 | Unix socket client with timeout |
| `blockmatrix/src/ipc/handler.rs` | 1 | 60 | Request dispatcher (method name to handler fn) |
| `blockmatrix/src/ipc/state.rs` | 2 | 50 | Shared daemon state struct |
| `blockmatrix/src/ipc/handlers/mod.rs` | 3 | 30 | Handler registration orchestrator |
| `blockmatrix/src/ipc/handlers/dns.rs` | 3 | 60 | DNS method handlers |
| `blockmatrix/src/ipc/handlers/store.rs` | 3 | 50 | Store/Fetch method handlers |
| `blockmatrix/src/ipc/handlers/network.rs` | 3 | 40 | Network method handlers |
| `blockmatrix/src/ipc/handlers/blockchain.rs` | 3 | 50 | Blockchain method handlers |
| `blockmatrix/src/ipc/handlers/topology.rs` | 3 | 40 | Topology handlers (via CommandExecutor) |
| `blockmatrix/src/ipc/handlers/asset.rs` | 3 | 40 | Asset handlers (via CommandExecutor) |
| `blockmatrix/src/ipc/config.rs` | 4 | 100 | TOML config loading/saving |
| `blockmatrix/src/ipc/handlers/config.rs` | 4 | 40 | Config IPC handlers |

**Total new**: ~850 lines across 15 files
**Total modified**: ~200 lines in `node.rs`, ~15 lines in `output.rs`, ~1 line in `lib.rs`
**Total tests**: ~45 new tests across 4 sprints

---

### Critical Files for Implementation

- `/home/persist/hypermesh/core/blockmatrix/src/bin/node.rs` - Primary binary requiring refactor: clap Commands enum (lines 113-148), daemon lifecycle in the Start block (lines 993-1303), all command match arms (lines 1304-1333). This is the single most complex file to modify.
- `/home/persist/hypermesh/core/blockmatrix/src/cli/output.rs` - Must add Serialize derives to CliOutput, CliTable, and CliError (lines 18-155) so IPC handlers can serialize structured CLI results to JSON for transport over the socket.
- `/home/persist/hypermesh/core/blockmatrix/src/bootstrap/mod.rs` - NodeBootstrap struct (line 169) holds all subsystem references (blockchain, dns, cert, privacy_mode). DaemonState wraps these via Arc for IPC handler access. Understanding this struct's public API is essential for handler implementation.
- `/home/persist/hypermesh/core/blockmatrix/src/cli/executor.rs` - CommandExecutor (line 45) with topology/node/asset execution logic. Will be wrapped in `Arc<Mutex<>>` inside DaemonState and invoked by topology and asset IPC handlers to avoid reimplementing command logic.
- `/home/persist/hypermesh/core/blockmatrix/src/network/mod.rs` - NetworkManager (line 67) with public async methods for peer listing (`get_connected_nodes`), peer connection (`connect_to_peer`), and node counting (`get_node_count`). Used directly by network IPC handlers and for building status responses.