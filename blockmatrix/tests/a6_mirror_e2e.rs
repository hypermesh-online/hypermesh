// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! A6.4/A6.6 — multi-node mirror end-to-end (torrent model, 3 nodes A→B→C).
//!
//! This test drives the REAL `hypermesh` binary as three separate
//! subprocesses over their IPC control sockets to prove the full
//! consumer-becomes-provider ("everyone is a mirror") flow.
//!
//! # A6.6 model (interest-scoped registration delivery)
//!
//! A6.1 made the publish path register an asset's shards on-chain
//! (`StoragePointer::Sharded` via `add_block` in `ipc/handlers/store.rs`), so
//! the PUBLISHER's `authorizes_shard` is true and its serve gate opens.
//!
//! A6.6 removed the fetcher's dependency on whole-chain propagation: when A
//! serves a shard it attaches THAT asset's on-chain registration
//! (`BlockAssetEntry`) in the SHARD_FETCH response envelope
//! (`shard_len(4)+shard+registration_json`). B re-validates the delivered
//! registration (zero-trust: content-binding + shard-coverage + `add_block`'s
//! `state_proof.validate()` + head-linkage) and RE-ANCHORS it as a FRESH block
//! on B's OWN chain — so B's `authorizes_shard(shard)` flips false→true and B's
//! re-announce gate opens, WITHOUT B ever syncing A's chain. Block propagation
//! is deliberately unnecessary for mirroring: the registration travels WITH the
//! shard, to exactly the node that touched the vector.
//!
//! # Flow proven
//! - A publishes a payload (`store`) → shards registered on A's chain
//!   (`registered_block >= 1`, `shard_hashes` returned).
//! - A serves its own shards locally (`shard.fetch` → `source == "local"`).
//! - B bootstraps to A, completes the bilateral PoS handshake, then DIRECTLY
//!   fetches each shard. The fetch delivers the registration; B re-anchors it on
//!   its own chain and RE-ANNOUNCES itself as a provider (`source == "network"`
//!   AND `announce_targets > 0`). B is now a live mirror — this single assertion
//!   exercises the WHOLE A6.6 mechanism (fetch→deliver→re-anchor→re-announce).
//!   No chain-height convergence is required or waited for.
//! - C bootstraps to B ONLY and fetches from the network; the provider it
//!   pulls from is B (the new mirror), proving consumer→provider propagation.
//! - Negative: a fetch for an unregistered shard id is refused, proving the
//!   serve gate still holds.
//!
//! # Gating
//! Skipped by default. Runs only when `HM_RUN_SUBPROCESS_HARNESS=1` is set AND
//! a `hypermesh` release binary is locatable. The test fn is also `#[ignore]`
//! so `cargo test` never runs it without `--ignored`.
//!
//! To run manually:
//! ```bash
//! cargo build --release -p blockmatrix --bin hypermesh
//! HM_RUN_SUBPROCESS_HARNESS=1 \
//!   cargo test -p blockmatrix --test a6_mirror_e2e -- --ignored --nocapture
//! ```

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

// ─── Tunables ──────────────────────────────────────────────────────────────

/// Overall per-wait ceiling for node readiness / sync polling.
const READY_TIMEOUT: Duration = Duration::from_secs(60);
/// Interval between poll attempts.
const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// Network-fetch retry budget for "B became a mirror" (~12 × 5s = 60s).
const FETCH_RETRIES: usize = 12;
const FETCH_RETRY_INTERVAL: Duration = Duration::from_secs(5);
/// Per-IPC-call timeout.
const IPC_CALL_TIMEOUT: Duration = Duration::from_secs(10);
/// Grace period for a child to exit gracefully after an IPC `shutdown` before
/// `Node::drop` SIGKILLs it. Best-effort — see the teardown note in the test.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

// ─── Node handle ───────────────────────────────────────────────────────────

/// A spawned `hypermesh` daemon subprocess with its own socket + data dir.
struct Node {
    name: &'static str,
    child: Child,
    sock: PathBuf,
    stdout_log: PathBuf,
    stderr_log: PathBuf,
}

impl Node {
    /// Dump this node's captured stdout/stderr to the test output (for
    /// debugging a failed poll or assertion).
    fn dump_logs(&self) {
        eprintln!("──── node {} stdout ({}) ────", self.name, self.stdout_log.display());
        eprintln!("{}", read_to_string_lossy(&self.stdout_log));
        eprintln!("──── node {} stderr ({}) ────", self.name, self.stderr_log.display());
        eprintln!("{}", read_to_string_lossy(&self.stderr_log));
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        // Best-effort cleanup so a panicking test never orphans daemons.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_to_string_lossy(path: &Path) -> String {
    let mut s = String::new();
    if let Ok(mut f) = std::fs::File::open(path) {
        let _ = f.read_to_string(&mut s);
    }
    s
}

// ─── Binary + gating ───────────────────────────────────────────────────────

/// Locate the `hypermesh` binary, preferring an explicit override, then the
/// cargo-provided path, then conventional workspace release locations.
fn locate_binary() -> Option<PathBuf> {
    // 1. Operator override.
    if let Ok(p) = std::env::var("HYPERMESH_BIN") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    // 2. Cargo sets CARGO_BIN_EXE_<name> for integration tests of the package
    //    that defines the `[[bin]]` — blockmatrix defines `hypermesh`.
    if let Some(p) = option_env!("CARGO_BIN_EXE_hypermesh") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    // 3. Conventional workspace locations (relative to the package manifest).
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("../target/release/hypermesh"),
        manifest.join("target/release/hypermesh"),
        PathBuf::from("target/release/hypermesh"),
    ];
    candidates.into_iter().find(|c| c.exists())
}

/// Returns `Some(bin)` when the harness is enabled and a binary exists, else
/// prints a skip message and returns `None`.
fn preflight() -> Option<PathBuf> {
    if std::env::var("HM_RUN_SUBPROCESS_HARNESS").ok().as_deref() != Some("1") {
        eprintln!(
            "skipping A6.4 mirror E2E — set HM_RUN_SUBPROCESS_HARNESS=1 to enable"
        );
        return None;
    }
    match locate_binary() {
        Some(bin) => {
            eprintln!("A6.4 mirror E2E using binary: {}", bin.display());
            Some(bin)
        }
        None => {
            eprintln!(
                "skipping A6.4 mirror E2E — no hypermesh binary found. Build with \
                 `cargo build --release -p blockmatrix --bin hypermesh` or set HYPERMESH_BIN."
            );
            None
        }
    }
}

// ─── Subprocess spawn ──────────────────────────────────────────────────────

/// Spawn one `hypermesh connect public --foreground` daemon.
///
/// Root flags precede the `connect` subcommand. Every node gets its OWN
/// `HYPERMESH_SOCK` (IPC isolation) + `--data-dir` (chain/identity isolation)
/// and shares the `--network-id` (required for block sync). Bootstrap peers are
/// literal loopback `SocketAddr`s (no DNS resolution in the CLI).
fn spawn_node(
    bin: &Path,
    name: &'static str,
    workdir: &Path,
    stoq_port: u16,
    network_id: &str,
    bootstrap: &[String],
) -> Node {
    let data_dir = workdir.join(format!("{name}-data"));
    let sock = workdir.join(format!("{name}.sock"));
    let stdout_log = workdir.join(format!("{name}.out.log"));
    let stderr_log = workdir.join(format!("{name}.err.log"));
    std::fs::create_dir_all(&data_dir).expect("test: create node data dir");

    let out = std::fs::File::create(&stdout_log).expect("test: create stdout log");
    let err = std::fs::File::create(&stderr_log).expect("test: create stderr log");

    let mut cmd = Command::new(bin);
    cmd.arg("--privacy")
        .arg("public")
        .arg("-s")
        .arg(stoq_port.to_string())
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--network-id")
        .arg(network_id);
    for b in bootstrap {
        cmd.arg("-b").arg(b);
    }
    cmd.arg("connect").arg("public").arg("--foreground");

    cmd.env("HYPERMESH_SOCK", &sock)
        // Isolate any XDG-derived fallbacks so nothing collides across nodes.
        .env("XDG_RUNTIME_DIR", workdir.join(format!("{name}-xdg")))
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err));

    let child = cmd.spawn().expect("test: spawn hypermesh node");
    Node {
        name,
        child,
        sock,
        stdout_log,
        stderr_log,
    }
}

// ─── Minimal IPC client (newline-delimited JSON-RPC 2.0 over Unix socket) ────

/// One JSON-RPC call against a daemon socket: connect, write `<json>\n`, read
/// one response line. Omits `protocol_version` (server treats absent as a
/// pre-J.1 client and accepts it), so this stays decoupled from the daemon's
/// exact version. Returns the parsed response object (`{result|error, ...}`).
async fn ipc_call(sock: &Path, method: &str, params: Value) -> Result<Value, String> {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1u64,
    });
    let mut bytes = serde_json::to_vec(&request).map_err(|e| format!("serialize: {e}"))?;
    bytes.push(b'\n');

    let fut = async {
        let stream = UnixStream::connect(sock)
            .await
            .map_err(|e| format!("connect {}: {e}", sock.display()))?;
        let (reader, mut writer) = stream.into_split();
        writer
            .write_all(&bytes)
            .await
            .map_err(|e| format!("write: {e}"))?;
        let mut buf = BufReader::new(reader);
        let mut line = String::new();
        buf.read_line(&mut line)
            .await
            .map_err(|e| format!("read: {e}"))?;
        serde_json::from_str::<Value>(line.trim()).map_err(|e| format!("parse: {e}"))
    };

    match tokio::time::timeout(IPC_CALL_TIMEOUT, fut).await {
        Ok(res) => res,
        Err(_) => Err(format!("ipc_call {method} timed out")),
    }
}

/// Call and require a JSON-RPC success, returning the `result` value.
async fn ipc_ok(sock: &Path, method: &str, params: Value) -> Result<Value, String> {
    let resp = ipc_call(sock, method, params).await?;
    if let Some(err) = resp.get("error") {
        if !err.is_null() {
            return Err(format!("RPC error on {method}: {err}"));
        }
    }
    resp.get("result")
        .cloned()
        .ok_or_else(|| format!("{method}: response missing result: {resp}"))
}

// ─── Polling ───────────────────────────────────────────────────────────────

/// Poll an async predicate until it returns `Ok(Some(T))`, bounded by
/// `timeout`. On timeout, dumps `node`'s logs and returns `Err`.
async fn poll_until<T, F, Fut>(
    label: &str,
    node: &Node,
    timeout: Duration,
    mut f: F,
) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<T>>,
{
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(v) = f().await {
            return Ok(v);
        }
        if Instant::now() >= deadline {
            eprintln!("poll_until('{label}') timed out after {timeout:?}");
            node.dump_logs();
            return Err(format!("timeout waiting for: {label}"));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Wait until a node answers `ping` with `"pong"`.
async fn wait_ready(node: &Node) -> Result<(), String> {
    poll_until("daemon ready (ping→pong)", node, READY_TIMEOUT, || async {
        match ipc_call(&node.sock, "ping", serde_json::json!(null)).await {
            Ok(v) if v.get("result") == Some(&serde_json::json!("pong")) => Some(()),
            _ => None,
        }
    })
    .await
}

/// Read a node's connected-peer NETWORK node_ids via `network.peers`.
///
/// These are the real handshake-derived node_ids (BLAKE3(FALCON pubkey) hex) —
/// the SAME identity a `shard.fetch` result reports in its `peer` field. NOTE:
/// `status.node_id` returns a coordinate LABEL (e.g. `node_0_0_0`), not this
/// hashed id, so it must NOT be used to match a fetch's `peer`.
async fn network_peer_ids(node: &Node) -> Vec<String> {
    ipc_ok(&node.sock, "network.peers", serde_json::json!({}))
        .await
        .ok()
        .and_then(|r| r.get("peers").and_then(Value::as_array).cloned())
        .map(|peers| {
            peers
                .iter()
                .filter_map(|p| p.get("node_id").and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Read a node's connected-peer count via `status`.
async fn peer_count(node: &Node) -> u64 {
    ipc_ok(&node.sock, "status", serde_json::json!({}))
        .await
        .ok()
        .and_then(|r| r.get("peers").and_then(Value::as_u64))
        .unwrap_or(0)
}

// ─── The test ──────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "subprocess mirror E2E; run with HM_RUN_SUBPROCESS_HARNESS=1 -- --ignored"]
async fn a6_mirror_torrent_flow_three_nodes() {
    let Some(bin) = preflight() else {
        return;
    };

    let workdir = tempfile::TempDir::new().expect("test: workdir");
    let wd = workdir.path();
    let network_id = "a6-mirror-e2e-net";

    // Distinct loopback STOQ ports; bootstrap addrs are literal (no DNS).
    let (port_a, port_b, port_c) = (9401u16, 9402u16, 9403u16);
    let addr_a = format!("[::1]:{port_a}");
    let addr_b = format!("[::1]:{port_b}");

    // ── 1. Spawn the full A→B→C topology BEFORE publishing. ──
    //
    // Live-propagation ordering matters: a node pushes a newly-created block to
    // its CURRENTLY-connected peers on the next sync tick. If A publishes before
    // any peer is connected, A advances its propagation watermark with no target
    // and never re-pushes, and the reflector PULL path is a weak fallback. The
    // realistic torrent shape — seed up, peers join, THEN publish — has A deliver
    // the registration block live to B, and B forward it to C. So: bring the mesh
    // fully up first, then store on A.
    let node_a = spawn_node(&bin, "A", wd, port_a, network_id, &[]);
    wait_ready(&node_a).await.expect("test: A did not become ready");

    let node_b = spawn_node(&bin, "B", wd, port_b, network_id, &[addr_a.clone()]);
    wait_ready(&node_b).await.expect("test: B did not become ready");

    let node_c = spawn_node(&bin, "C", wd, port_c, network_id, &[addr_b.clone()]);
    wait_ready(&node_c).await.expect("test: C did not become ready");

    // ── 2. Wait for the mesh to fully connect: A↔B and B↔C. ──
    // A sees B (inbound), B sees A + C, C sees B. A's push target is B; B forwards
    // to C. Require A>=1 so A has a live peer to receive the registration block.
    poll_until("A sees a peer (B inbound)", &node_a, READY_TIMEOUT, || async {
        (peer_count(&node_a).await >= 1).then_some(())
    })
    .await
    .expect("test: A never saw B connect inbound");
    poll_until("B sees A and C", &node_b, READY_TIMEOUT, || async {
        (peer_count(&node_b).await >= 2).then_some(())
    })
    .await
    .expect("test: B never connected to both A and C");
    poll_until("C sees B as peer", &node_c, READY_TIMEOUT, || async {
        (peer_count(&node_c).await >= 1).then_some(())
    })
    .await
    .expect("test: C never connected to B");
    eprintln!("mesh formed: A↔B↔C");

    // ── 3. A publishes → shards registered on-chain, block pushed to the mesh. ──
    let payload_path = wd.join("payload.bin");
    // Enough bytes to drive real Reed-Solomon sharding, with a marker.
    let payload = b"HYPERMESH-A6.4-MIRROR-E2E-".repeat(4096);
    std::fs::write(&payload_path, &payload).expect("test: write payload");

    let store_res = ipc_ok(
        &node_a.sock,
        "store",
        serde_json::json!({ "path": payload_path.to_string_lossy() }),
    )
    .await
    .unwrap_or_else(|e| {
        node_a.dump_logs();
        panic!("test: A store failed: {e}");
    });

    let asset_id = store_res
        .get("asset_id")
        .and_then(Value::as_str)
        .expect("test: store result has asset_id")
        .to_string();
    let registered_block = store_res
        .get("registered_block")
        .and_then(Value::as_u64)
        .expect("test: store result has registered_block");
    assert!(
        registered_block >= 1,
        "A6.1: publish must register shards on-chain (block index >= 1), got {registered_block}",
    );
    let shard_hashes: Vec<String> = store_res
        .get("shard_hashes")
        .and_then(Value::as_array)
        .expect("test: store result has shard_hashes")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    assert!(
        !shard_hashes.is_empty(),
        "asset {asset_id} must produce at least one shard",
    );
    eprintln!(
        "A stored asset {} → {} shard(s), registered in block {}",
        &asset_id[..16.min(asset_id.len())],
        shard_hashes.len(),
        registered_block,
    );

    // ── 4. A serves its own shards locally. ──
    for h in &shard_hashes {
        let res = ipc_ok(&node_a.sock, "shard.fetch", serde_json::json!({ "shard_id": h }))
            .await
            .unwrap_or_else(|e| {
                node_a.dump_logs();
                panic!("test: A shard.fetch({h}) failed: {e}");
            });
        assert_eq!(
            res.get("source").and_then(Value::as_str),
            Some("local"),
            "A must serve its own shard {h} locally",
        );
    }

    // ── 5. B fetches each shard from the network AND becomes a mirror. ──
    //
    // A6.6: B does NOT wait for chain-height convergence — it never syncs A's
    // chain. B connects (peer present, asserted in step 2), then DIRECTLY
    // fetches. The fetch response carries A's on-chain registration for the
    // shard; B re-validates + re-anchors it on its OWN chain, flipping its
    // re-announce gate open. So the assertion below — source flips to "network"
    // AND announce_targets>0 — proves the ENTIRE A6.6 path in one shot:
    // fetch delivers registration → B re-anchors → B re-announces itself.
    // Retry-bounded because provider discovery / connect settle asynchronously.
    for h in &shard_hashes {
        let mut became_mirror = false;
        for attempt in 0..FETCH_RETRIES {
            match ipc_call(&node_b.sock, "shard.fetch", serde_json::json!({ "shard_id": h })).await {
                Ok(resp) => {
                    if let Some(result) = resp.get("result") {
                        let source = result.get("source").and_then(Value::as_str);
                        let announce = result
                            .get("announce_targets")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        if source == Some("network") && announce > 0 {
                            became_mirror = true;
                            break;
                        }
                    }
                }
                Err(e) => eprintln!("B shard.fetch({h}) attempt {attempt}: {e}"),
            }
            tokio::time::sleep(FETCH_RETRY_INTERVAL).await;
        }
        if !became_mirror {
            node_a.dump_logs();
            node_b.dump_logs();
            panic!(
                "B never became a mirror for shard {h}: expected source=network AND \
                 announce_targets>0 within {FETCH_RETRIES} retries",
            );
        }
    }
    eprintln!("B is a live mirror (announce_targets>0 on every shard)");

    // ── 6. C (bootstrapped to B ONLY) pulls from the new mirror B. ──
    //
    // A6.6: C also does not wait for chain-height convergence. B is now an
    // authoritative provider (it re-anchored A's registration during step 5),
    // so C — connected to B alone (asserted in step 2) — fetches directly and B
    // serves. C's only known provider is B, so a network fetch must succeed and
    // name B as the source.
    //
    // Identity check: C's ONE connected peer (via `network.peers`) is B, keyed
    // by the real handshake node_id — the same id a `shard.fetch` result reports
    // in its `peer` field (an 8-hex prefix). We assert the fetch's `peer` is a
    // prefix of that id, which genuinely proves "C pulled from mirror B".
    let c_peers = network_peer_ids(&node_c).await;
    assert_eq!(
        c_peers.len(),
        1,
        "C must have exactly one connected peer (B); got {c_peers:?}",
    );
    let b_net_id = c_peers[0].clone();
    let b_prefix = b_net_id[..8.min(b_net_id.len())].to_string();
    for h in &shard_hashes {
        let mut ok = false;
        for attempt in 0..FETCH_RETRIES {
            match ipc_call(&node_c.sock, "shard.fetch", serde_json::json!({ "shard_id": h })).await {
                Ok(resp) => {
                    if let Some(result) = resp.get("result") {
                        if result.get("source").and_then(Value::as_str) == Some("network") {
                            let peer = result
                                .get("peer")
                                .and_then(Value::as_str)
                                .unwrap_or_default();
                            assert_eq!(
                                peer, b_prefix,
                                "C must pull shard {h} from the new mirror B ({b_prefix}), got '{peer}'",
                            );
                            ok = true;
                            break;
                        }
                    }
                }
                Err(e) => eprintln!("C shard.fetch({h}) attempt {attempt}: {e}"),
            }
            tokio::time::sleep(FETCH_RETRY_INTERVAL).await;
        }
        if !ok {
            node_b.dump_logs();
            node_c.dump_logs();
            panic!("C never fetched shard {h} from the network (mirror B)");
        }
    }
    eprintln!("C fetched every shard from mirror B — consumer→provider propagation proven");

    // ── 7. NEGATIVE: an unregistered shard id is refused (serve gate holds). ──
    let bogus = hex::encode([0x99u8; 32]);
    let neg = ipc_call(&node_a.sock, "shard.fetch", serde_json::json!({ "shard_id": bogus }))
        .await
        .expect("test: negative shard.fetch should return a response, not a transport error");
    let refused = neg
        .get("error")
        .map(|e| !e.is_null())
        .unwrap_or(false)
        || neg.get("result").is_none();
    assert!(
        refused,
        "an unregistered shard must be refused (not-found), got: {neg}",
    );

    // ── 8. Teardown: request graceful IPC shutdown (leaf-first C → B → A). ──
    //
    // This is BEST-EFFORT and NOT a mirror-flow assertion. A separate, A6.6-
    // unrelated shutdown-lifecycle gap exists: a node that still has ACTIVE PEER
    // CONNECTIONS does not always exit promptly on IPC `shutdown` (a lone node
    // exits in ~0.5s; a connected node's runtime drop can stall on a peer I/O
    // task). That is a production connection-teardown concern outside A6.6's
    // scope (which touches only chain/shard/handler code, never the node
    // lifecycle). `Node::drop` force-kills (SIGKILL) every child regardless, so
    // no daemon is ever orphaned. We therefore log graceful-exit latency and
    // WARN on a slow exit rather than fail the mirror-flow test on an unrelated
    // gap — the flow itself is already fully proven by steps 1–7 above.
    for node in [&node_c, &node_b, &node_a] {
        let _ = ipc_call(&node.sock, "shutdown", serde_json::json!({})).await;
        let t0 = Instant::now();
        if wait_for_exit(node, SHUTDOWN_GRACE) {
            eprintln!("node {} exited gracefully in {:?}", node.name, t0.elapsed());
        } else {
            eprintln!(
                "WARNING: node {} did not exit within {SHUTDOWN_GRACE:?} of IPC shutdown \
                 (known connected-node shutdown-lifecycle gap, unrelated to A6.6); \
                 Drop will SIGKILL it.",
                node.name,
            );
        }
    }
    eprintln!("A6.4/A6.6 mirror torrent flow complete: A→B→C mirror path validated.");
}

/// Poll `try_wait` until the child exits, bounded by `grace`.
fn wait_for_exit(node: &Node, grace: Duration) -> bool {
    // `Node.child` is behind `&Node`; use a short spin on try_wait via a raw
    // pointer is not needed — we take a mutable borrow through the process
    // handle by re-opening it. Instead poll the OS directly with the pid.
    let deadline = Instant::now() + grace;
    let pid = node.child.id();
    loop {
        if !pid_alive(pid) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// True while a process with `pid` exists (POSIX `kill(pid, 0)`).
fn pid_alive(pid: u32) -> bool {
    // SAFETY: signal 0 performs existence/permission check only, no delivery.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
