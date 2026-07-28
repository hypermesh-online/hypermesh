// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! `hypermesh ping` subcommand -- STOQ handshake + RTT measurement.

use anyhow::{anyhow, Context, Result};
use std::net::{Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Instant;

use crate::cli::Cli;

/// Default STOQ port when not specified in the target address.
const DEFAULT_PORT: u16 = 9292;

/// Run the ping subcommand: connect via STOQ, perform bilateral PoS
/// handshake, measure RTT, print results.
pub async fn run_ping(target: &str, count: u32, cli: &Cli) -> Result<()> {
    let addr = parse_target(target).await?;
    let coord = (cli.coord_x, cli.coord_y, cli.coord_z);

    // Resolve identity directory for loading/creating FALCON keypair.
    // D5: identity lives at the coordinate-independent `data_dir/identity`;
    // adopt a legacy coord-keyed copy so an un-migrated install still signs
    // with its own keypair rather than an ephemeral one.
    let data_dir = resolve_data_dir(&cli.data_dir)?;
    let legacy_key = blockmatrix::bootstrap::node_id(
        &blockmatrix::matrix::coordinate::MatrixCoordinate::new(coord.0, coord.1, coord.2)?,
    );
    let identity_dir = blockmatrix::bootstrap::adopt_legacy_identity(&data_dir, &legacy_key)
        .unwrap_or_else(|_| blockmatrix::bootstrap::identity_dir(&data_dir));
    let falcon_identity =
        blockmatrix::identity::FalconIdentity::load_or_create(&identity_dir)
            .unwrap_or_else(|_| blockmatrix::identity::FalconIdentity::generate());

    let signer: Arc<dyn hypermesh_lib::NodeSigner> = Arc::new(falcon_identity);
    let proof_provider: Arc<dyn hypermesh_lib::StateProofProvider> = Arc::new(
        blockmatrix::proof_of_state::BlockMatrixProofProvider::new(
            signer.node_id().to_string(),
            signer.clone(),
        ),
    );

    // Create a lightweight STOQ transport (ephemeral, OS-assigned port)
    let stoq_config = stoq::TransportConfig {
        port: 0, // OS-assigned
        bind_address: Ipv6Addr::UNSPECIFIED,
        ..stoq::TransportConfig::default()
    };
    let transport = Arc::new(
        stoq::StoqTransport::new_for_network(
            stoq_config,
            stoq::transport::NetworkType::P2P,
        )
        .await
        .context("failed to initialize STOQ transport")?,
    );

    println!("PING {} via STOQ (FALCON-1024 bilateral PoS)", addr);

    let mut sent: u32 = 0;
    let mut received: u32 = 0;
    let mut rtts: Vec<f64> = Vec::with_capacity(count as usize);

    for seq in 1..=count {
        sent += 1;
        match ping_once(&transport, &addr, coord, signer.as_ref(), proof_provider.as_ref()).await {
            Ok(result) => {
                received += 1;
                let ms = result.rtt.as_secs_f64() * 1000.0;
                rtts.push(ms);
                println!(
                    "PONG from {} at ({},{},{}) -- {:.2}ms (proof={}B, pubkey={}B) seq={}",
                    &result.peer_node_id[..12.min(result.peer_node_id.len())],
                    result.peer_coord.0,
                    result.peer_coord.1,
                    result.peer_coord.2,
                    ms,
                    result.proof_len,
                    result.pubkey_len,
                    seq,
                );
            }
            Err(e) => {
                eprintln!("TIMEOUT/ERROR seq={}: {}", seq, e);
            }
        }

        // Small delay between pings (except after last)
        if seq < count {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    // Summary
    let loss = if sent > 0 {
        ((sent - received) as f64 / sent as f64) * 100.0
    } else {
        0.0
    };

    println!();
    println!("--- {} ping statistics ---", target);
    println!(
        "{} sent, {} received, {:.0}% loss",
        sent, received, loss,
    );

    if !rtts.is_empty() {
        let min = rtts.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = rtts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg = rtts.iter().sum::<f64>() / rtts.len() as f64;
        println!("rtt min/avg/max = {:.2}/{:.2}/{:.2} ms", min, avg, max);
    }

    Ok(())
}

/// Result from a single ping attempt.
struct PingResult {
    rtt: std::time::Duration,
    peer_node_id: String,
    peer_coord: (i64, i64, i64),
    proof_len: usize,
    pubkey_len: usize,
}

/// Execute a single STOQ connect + bilateral PoS handshake.
async fn ping_once(
    transport: &Arc<stoq::StoqTransport>,
    addr: &SocketAddr,
    local_coord: (i64, i64, i64),
    signer: &dyn hypermesh_lib::NodeSigner,
    proof_provider: &dyn hypermesh_lib::StateProofProvider,
) -> Result<PingResult> {
    let start = Instant::now();

    let ipv6 = match addr {
        SocketAddr::V6(v6) => *v6.ip(),
        SocketAddr::V4(v4) => v4.ip().to_ipv6_mapped(),
    };

    let endpoint = stoq::Endpoint::new(ipv6, addr.port());
    let connection = transport.connect(&endpoint).await
        .context("STOQ connect failed")?;

    // Open stream and write handshake discriminator
    let mut stream = connection.open_stream().await
        .context("failed to open stream")?;
    stream
        .write_discriminator(blockmatrix::network::CONN_TYPE_HANDSHAKE)
        .await
        .context("failed to write discriminator")?;

    let result = stoq::initiate_handshake_on_stream(
        &mut stream,
        signer,
        proof_provider,
        local_coord,
    )
    .await
    .context("bilateral PoS handshake failed")?;

    let _ = stream.finish_send();
    let rtt = start.elapsed();

    Ok(PingResult {
        rtt,
        peer_node_id: result.peer_node_id,
        peer_coord: result.peer_coordinate,
        proof_len: result.peer_proof.len(),
        pubkey_len: result.peer_pubkey.len(),
    })
}

/// Parse a target string into a `SocketAddr`.
///
/// Resolution order:
/// 1. Direct IP:port parse (`[::1]:9292`, `1.2.3.4:9292`)
/// 2. HyperMesh DNS — well-known services (`trust`, `caesar`, `catalog`, etc.)
///    and user-registered names from persisted DNS records
/// 3. System DNS fallback — clearnet hostnames (`trust.hypermesh.online`)
async fn parse_target(target: &str) -> Result<SocketAddr> {
    // Try direct parse first (handles `[::1]:9292`, `1.2.3.4:9292`, etc.)
    if let Ok(sa) = target.parse::<SocketAddr>() {
        return Ok(sa);
    }

    // Split target into (name, optional port)
    let (name, port) = split_host_port(target);

    // --- HyperMesh DNS: decentralized namespace first ---
    if let Some(sa) = resolve_hypermesh_dns(name, port).await {
        tracing::info!("Resolved '{}' via HyperMesh DNS → {}", name, sa);
        return Ok(sa);
    }

    // --- System DNS fallback ---
    let with_port = format!("{}:{}", name, port.unwrap_or(DEFAULT_PORT));

    // Try direct IP:port parse (bare IPv4/IPv6 without port handled here)
    if let Ok(sa) = with_port.parse::<SocketAddr>() {
        return Ok(sa);
    }

    // Try as `[ipv6]:port`
    if name.contains(':') {
        let bracketed = format!("[{}]:{}", name, port.unwrap_or(DEFAULT_PORT));
        if let Ok(sa) = bracketed.parse::<SocketAddr>() {
            return Ok(sa);
        }
    }

    // System DNS resolution
    if let Ok(mut addrs) = with_port.to_socket_addrs() {
        if let Some(sa) = addrs.next() {
            return Ok(sa);
        }
    }

    Err(anyhow!("cannot resolve target address: {}", target))
}

/// Split a target into (hostname, optional port).
fn split_host_port(target: &str) -> (&str, Option<u16>) {
    // `[ipv6]:port`
    if target.starts_with('[') {
        if let Some(bracket_end) = target.find(']') {
            let host = &target[1..bracket_end];
            let port = target[bracket_end + 1..]
                .strip_prefix(':')
                .and_then(|p| p.parse().ok());
            return (host, port);
        }
    }

    // `host:port` — but NOT bare IPv6 (which has multiple colons)
    if let Some(colon_pos) = target.rfind(':') {
        let before = &target[..colon_pos];
        let after = &target[colon_pos + 1..];
        // Only treat as host:port if the part after last colon is a valid port number
        // (avoids misinterpreting `2600:1900::` as host=`2600:1900:` port=`:`)
        if let Ok(port) = after.parse::<u16>() {
            if !before.contains(':') || before.starts_with('[') {
                return (before, Some(port));
            }
        }
    }

    (target, None)
}

/// Resolve a name via HyperMesh's decentralized DNS.
///
/// Resolution layers (in order):
/// 1. If daemon is running: query IPC `dns.resolve` for full blockchain DNS
///    (includes peer-synced records, user-registered names, network DNS)
/// 2. Well-known local services (trust, caesar, catalog, ngauge, blockmatrix)
/// 3. Returns None → caller falls through to system DNS
async fn resolve_hypermesh_dns(name: &str, explicit_port: Option<u16>) -> Option<SocketAddr> {
    // Query running daemon for full DNS (blockchain + peer-synced + local services).
    // The daemon's DNS resolver has the complete picture: local services point to
    // ::1, but network-registered names point to their actual node addresses.
    // If no daemon is running, HyperMesh DNS is unavailable — fall through to
    // system DNS so that `trust.hypermesh.online` still resolves via clearnet.
    resolve_via_ipc(name, explicit_port).await
}

/// Try to resolve a name via IPC to the running daemon.
/// Returns None if daemon isn't running or resolution fails.
async fn resolve_via_ipc(name: &str, explicit_port: Option<u16>) -> Option<SocketAddr> {
    let client = blockmatrix::ipc::IpcClient::new();
    if !client.is_daemon_running().await {
        return None;
    }
    match client
        .call_ok("dns.resolve", serde_json::json!({"name": name}))
        .await
    {
        Ok(resp) => {
            // Response format: {"name": "trust", "address": "::1"}
            let addr_str = resp.get("address")?.as_str()?;
            let ip: std::net::IpAddr = addr_str.parse().ok()?;
            // Use explicit port, or well-known service port, or default STOQ port
            let port = explicit_port.unwrap_or_else(|| {
                blockmatrix::bootstrap::LOCAL_SERVICES
                    .iter()
                    .find(|s| s.name == name)
                    .map(|s| s.port)
                    .unwrap_or(DEFAULT_PORT)
            });
            Some(SocketAddr::new(ip, port))
        }
        Err(_) => None,
    }
}

/// Expand `~` and resolve the data directory path.
fn resolve_data_dir(data_dir: &str) -> Result<std::path::PathBuf> {
    if data_dir.starts_with('~') {
        let home = dirs::home_dir().context("could not determine home directory")?;
        Ok(home.join(&data_dir[2..]))
    } else {
        Ok(std::path::PathBuf::from(data_dir))
    }
}
