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
    let addr = parse_target(target)?;
    let coord = (cli.coord_x, cli.coord_y, cli.coord_z);

    // Resolve identity directory for loading/creating FALCON keypair
    let data_dir = resolve_data_dir(&cli.data_dir)?;
    let nid = blockmatrix::bootstrap::node_id(
        &blockmatrix::matrix::coordinate::MatrixCoordinate::new(coord.0, coord.1, coord.2)?,
    );

    let identity_dir = data_dir.join(&nid).join("identity");
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
/// Supports:
/// - `host:port`
/// - `[ipv6]:port`
/// - `host` (default port 9292)
/// - IPv4 addresses (mapped to IPv6)
fn parse_target(target: &str) -> Result<SocketAddr> {
    // Try direct parse first (handles `[::1]:9292`, `1.2.3.4:9292`, etc.)
    if let Ok(sa) = target.parse::<SocketAddr>() {
        return Ok(sa);
    }

    // Try as `host:port` via DNS resolution
    if target.contains(':') && !target.starts_with('[') {
        // Could be ipv6 without brackets or host:port
        // Try host:port resolution
        if let Ok(mut addrs) = target.to_socket_addrs() {
            if let Some(sa) = addrs.next() {
                return Ok(sa);
            }
        }
        // Maybe bare IPv6 address without port
        let with_port = format!("[{}]:{}", target, DEFAULT_PORT);
        if let Ok(sa) = with_port.parse::<SocketAddr>() {
            return Ok(sa);
        }
    }

    // Try as bare host or IPv4 without port
    let with_port = format!("{}:{}", target, DEFAULT_PORT);
    if let Ok(sa) = with_port.parse::<SocketAddr>() {
        return Ok(sa);
    }

    // DNS resolution with default port
    if let Ok(mut addrs) = with_port.to_socket_addrs() {
        if let Some(sa) = addrs.next() {
            return Ok(sa);
        }
    }

    Err(anyhow!("cannot resolve target address: {}", target))
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
