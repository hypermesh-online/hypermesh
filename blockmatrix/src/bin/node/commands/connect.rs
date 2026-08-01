// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Connect/disconnect subcommand handler -- starts the node daemon.

use anyhow::{Context, Result};
use tracing::{info, warn};

use blockmatrix::assets::core::{AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope};
use blockmatrix::bootstrap::{NodeBootstrap, PrivacyMode};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::network::shard_store::ShardStore;
use blockmatrix::network::shard_transport::StoqShardTransport;
use blockmatrix::network::NetworkManager;
use blockmatrix::persistence::PersistenceManager;
use blockmatrix::ipc;

use crate::cli::Cli;

use super::services::register_default_dashboard;

/// Call a service method via IPC and print the result.
pub async fn service_ipc_call(
    method: &str,
    params: serde_json::Value,
    json_output: bool,
) -> Result<()> {
    let client = ipc::IpcClient::new();
    if !client.is_daemon_running().await {
        if json_output {
            let err = serde_json::json!({
                "error": "daemon_offline",
                "message": format!("Service method '{}' requires a running daemon", method),
                "hint": "Start with: hypermesh connect public",
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&err).unwrap_or_default()
            );
        } else {
            eprintln!(
                "Service '{}' requires a running daemon.\nStart with: hypermesh connect public",
                method.split('.').next().unwrap_or(method),
            );
        }
        return Ok(());
    }
    match client.call_ok(method, params).await {
        Ok(resp) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&resp).unwrap_or_default()
            );
        }
        Err(e) => {
            if json_output {
                let err = serde_json::json!({
                    "error": "service_error",
                    "method": method,
                    "message": format!("{e}"),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&err).unwrap_or_default()
                );
            } else {
                eprintln!("Error calling {method}: {e}");
            }
        }
    }
    Ok(())
}

/// Run the connect/start flow: initialize STOQ, network, sync loops, IPC server,
/// then wait for Ctrl+C or IPC shutdown.
pub async fn run_connect(
    cli: &Cli,
    coord: MatrixCoordinate,
    nid: &str,
    data_dir: &std::path::Path,
    bootstrap: &NodeBootstrap,
    persistence: std::sync::Arc<PersistenceManager>,
) -> Result<()> {
    info!("Starting node services...");

    let target_mode = cli.privacy.into();
    if bootstrap.privacy_mode().await != target_mode {
        bootstrap.set_privacy_mode(target_mode).await?;
    }

    let mut network_ref: Option<std::sync::Arc<NetworkManager>> = None;
    let mut shard_store_ref: Option<std::sync::Arc<ShardStore>> = None;
    let mut shard_transport_ref: Option<std::sync::Arc<StoqShardTransport>> = None;
    let mut shard_location_index_ref: Option<
        std::sync::Arc<blockmatrix::network::swarm_provider::ShardLocationIndex>,
    > = None;
    let mut consumer_provider_manager_ref: Option<
        std::sync::Arc<blockmatrix::network::consumer_provider::ConsumerProviderManager>,
    > = None;
    #[cfg(feature = "intelligence")]
    let mut ngauge_bits: Option<NGaugeBits> = None;

    let privacy_mode = bootstrap.privacy_mode().await;
    let has_bootstrap_peers = !cli.bootstrap.is_empty();

    // P3 (F5): a single shared InboxStore for received share invitations. The
    // SAME Arc is handed to the network `PeerContext` (so TAG_SHARE_INVITE
    // deliveries land here) and to the daemon state (so `share.inbox` /
    // `share.accept` read the same store).
    let share_inbox_store = std::sync::Arc::new(
        blockmatrix::sharing::inbox::InboxStore::new(Some(data_dir.join(nid).join("inbox"))),
    );

    if privacy_mode != PrivacyMode::PRIVATE || has_bootstrap_peers {
        let result = super::node_bringup::start_network(
            cli,
            coord,
            nid,
            data_dir,
            bootstrap,
            privacy_mode,
            has_bootstrap_peers,
            share_inbox_store.clone(),
        )
        .await?;
        network_ref = Some(result.network);
        shard_store_ref = Some(result.shard_store);
        shard_transport_ref = Some(result.shard_transport);
        shard_location_index_ref = Some(result.shard_location_index);
        consumer_provider_manager_ref = Some(result.consumer_provider_manager);
        #[cfg(feature = "intelligence")]
        {
            ngauge_bits = Some(NGaugeBits {
                demand_tracker: result.swarm_demand_tracker,
                analytics: result.ngauge_analytics,
            });
        }
    }

    // --- IPC Server Setup ---
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    let daemon_shard_store = shard_store_ref.unwrap_or_else(|| {
        std::sync::Arc::new(ShardStore::new_with_dir(
            data_dir.join(nid).join("shards"),
        ))
    });

    #[cfg(feature = "caesar")]
    let caesar_instance = {
        match caesar::CaesarProtocol::new(caesar::CaesarConfig::default()).await {
            Ok(protocol) => {
                info!("Caesar EVP protocol initialized");
                Some(std::sync::Arc::new(tokio::sync::RwLock::new(protocol)))
            }
            Err(e) => {
                warn!("Caesar init failed, caesar.* IPC will report unavailable: {e}");
                None
            }
        }
    };

    let daemon_state = std::sync::Arc::new(ipc::DaemonState {
        blockchain: bootstrap.blockchain().clone(),
        persistence: persistence.clone(),
        network: network_ref,
        shard_store: daemon_shard_store,
        shard_transport: shard_transport_ref,
        coordinate: coord,
        node_id: nid.to_string(),
        data_dir: data_dir.to_path_buf(),
        privacy_mode: format!("{:?}", bootstrap.privacy_mode().await),
        started_at: std::time::Instant::now(),
        shutdown_tx: shutdown_tx.clone(),
        dns_resolver: bootstrap.dns().clone(),
        shard_location_index: shard_location_index_ref,
        consumer_provider_manager: consumer_provider_manager_ref,
        #[cfg(feature = "caesar")]
        caesar: caesar_instance,
        #[cfg(feature = "intelligence")]
        ngauge_bridge: ngauge_bits.map(|eb| {
            std::sync::Arc::new(blockmatrix::intelligence::ngauge_bridge::NGaugeBridge::new(
                eb.demand_tracker,
                eb.analytics,
                hypermesh_lib::MatrixPosition {
                    x: coord.x as f64,
                    y: coord.y as f64,
                    z: coord.z as f64,
                },
            ))
        }),
        // Phase F.1: not yet wired — these become Some(...) when the
        // node actually joins a federation.  Until then,
        // `trustchain.request_cert` falls through to local self-signing.
        #[cfg(feature = "intelligence")]
        federation_manager: None,
        #[cfg(feature = "intelligence")]
        threshold_coordinator: None,

        transfer_coordinator: None,
            foundation_signing_key: None,
            dns_registrar: None,
            release_feed_subscriber: None,
        receipt_validator: std::sync::Arc::new(
            blockmatrix::assets::cross_chain::CrossChainReceiptValidator::new(),
        ),
        // Phase K.1: alpha-default inert — operators opt in by setting
        // these fields after constructing the daemon state when they
        // want capability-token auth or light-mode header sync.
        capability_token_issuer: None,
        revocation_registry: std::sync::Arc::new(
            blockmatrix::auth::RevocationRegistry::new(),
        ),
        light_sync_manager: None,
        // Phase M.4.5c.1: the adapter that bridges
        // `catalog::CatalogRegistry` into this trait object lives in the
        // `catalog` crate (see `catalog::wire_catalog_registry`). The
        // blockmatrix bin cannot depend on the catalog crate without
        // re-introducing the `catalog -> blockmatrix` dependency cycle,
        // so the wiring is done from a downstream binary that depends on
        // both crates. Until then, `catalog.dependencies` and
        // `catalog.search` IPC handlers return alpha-honest empty
        // responses ("status":"alpha","note":"catalog registry not
        // wired").
        catalog_registry: None,
        // P3 (F5): share the SAME inbox Arc the network PeerContext uses so
        // TAG_SHARE_INVITE deliveries are visible to `share.inbox`/`accept`.
        inbox_store: Some(share_inbox_store.clone()),
    });

    // Phase I.1: rebuild the cross-chain receipt index from any
    // receipts already on the local chain. This makes restart-recovery
    // deterministic — receipts written before a daemon restart remain
    // queryable via `chain.lookup_cross_receipt`.
    {
        let count = daemon_state
            .receipt_validator
            .rebuild_from_chain(&daemon_state.blockchain)
            .await;
        if count > 0 {
            tracing::info!(
                "CrossChainReceiptValidator: indexed {} receipt(s) from persisted chain",
                count
            );
        }
    }

    let mut handler = ipc::RequestHandler::new();
    ipc::register_all(&mut handler, daemon_state.clone());

    let handler = std::sync::Arc::new(handler);

    let ipc_server = match ipc::IpcServer::new(handler.clone()) {
        Ok(server) => {
            let server = std::sync::Arc::new(server);
            let server_run = server.clone();
            tokio::spawn(async move {
                if let Err(e) = server_run.run().await {
                    warn!("IPC server error: {}", e);
                }
            });
            info!("IPC server started");
            Some(server)
        }
        Err(e) => {
            warn!("Failed to start IPC server: {e}");
            None
        }
    };

    info!("API access available via IPC (Unix socket). STOQ API bridge deferred until protocol discriminator is implemented.");

    register_default_dashboard(bootstrap, data_dir, nid, coord).await;

    // --- DNS self-registration from --name flag ---
    if let Some(ref name) = cli.name {
        if let Err(e) = register_node_dns_name(name, bootstrap, data_dir, nid, coord).await {
            warn!("DNS self-registration failed for '{}': {}", name, e);
        }
    }

    info!("Node running in {:?} mode", bootstrap.privacy_mode().await);
    info!("Press Ctrl+C to stop");

    #[cfg(unix)]
    let mut sigterm = match tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    ) {
        Ok(sig) => Some(sig),
        Err(e) => {
            warn!("Failed to install SIGTERM handler: {}", e);
            None
        }
    };

    #[cfg(unix)]
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(e) = result {
                warn!("Failed to listen for Ctrl+C: {}", e);
            }
            info!("SIGINT received, shutting down...");
        }
        _ = async {
            match sigterm.as_mut() {
                Some(sig) => { sig.recv().await; }
                None => std::future::pending::<()>().await,
            }
        } => {
            info!("SIGTERM received, shutting down...");
        }
        _ = shutdown_rx.changed() => {
            info!("Shutdown requested via IPC");
        }
    }

    #[cfg(not(unix))]
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(e) = result {
                warn!("Failed to listen for Ctrl+C: {}", e);
            }
            info!("Ctrl+C received, shutting down...");
        }
        _ = shutdown_rx.changed() => {
            info!("Shutdown requested via IPC");
        }
    }

    // Close the STOQ transport FIRST so the accept loop + every per-peer
    // message loop unpark from quinn and return, dropping their connections
    // and the driver. Without this, a node with active peers hangs until the
    // QUIC idle-timeout (~30s) instead of exiting promptly.
    if let Some(network) = daemon_state.network.as_ref() {
        network.shutdown().await;
    }

    if let Some(server) = ipc_server {
        server.shutdown();
    }

    info!("Shutting down -- flushing persistence...");
    if let Err(e) = persistence.flush().await {
        warn!("Persistence flush error: {}", e);
    }
    if let Err(e) = persistence.shutdown().await {
        warn!("Persistence shutdown error: {}", e);
    }
    info!("Persistence flushed, shutdown complete.");

    Ok(())
}

/// NGauge handles carried back from `start_network` when the `intelligence`
/// feature is enabled.
#[cfg(feature = "intelligence")]
struct NGaugeBits {
    demand_tracker: std::sync::Arc<blockmatrix::network::SwarmDemandTracker>,
    analytics: std::sync::Arc<std::sync::Mutex<ngauge::SwarmAnalytics>>,
}

/// Register a DNS name for this node on the local blockchain.
///
/// This replicates the `dns register` command logic so that nodes can
/// self-register their name at boot via `--name`.
async fn register_node_dns_name(
    name: &str,
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    node_id: &str,
    coord: MatrixCoordinate,
) -> Result<()> {
    // TODO: Use the node's peer-facing address once NAT detection is implemented.
    // For now, register with ::1 -- peers discover the real address from the
    // STOQ connection's remote address, not from the DNS record's stored IP.
    let target_addr = std::net::IpAddr::from(std::net::Ipv6Addr::LOCALHOST);

    // Register in the in-memory resolver
    bootstrap.dns().register(name.to_string(), target_addr).await;

    // Persist to disk so the record survives restarts
    super::dns::persist_dns_record(data_dir, node_id, name, target_addr)?;

    // Write a blockchain DNS asset so it propagates to peers
    let bc = bootstrap.blockchain();
    let ipv6_addr = match target_addr {
        std::net::IpAddr::V6(v6) => v6,
        std::net::IpAddr::V4(v4) => v4.to_ipv6_mapped(),
    };
    let dns_entry = blockmatrix::dns::DnsBlockEntry {
        domain_name: name.to_string(),
        record_type: blockmatrix::dns::DnsRecordType::AAAA,
        record_data: blockmatrix::dns::DnsRecordData::AAAA(ipv6_addr),
        ttl: 300,
        owner: node_id.to_string(),
        grant_signature: None,
    };
    let dns_bytes = serde_json::to_vec(&dns_entry)
        .context("failed to serialize DNS entry")?;

    let asset_data = AssetData {
        config: name.as_bytes().to_vec(),
        definition: dns_bytes.clone(),
        metadata: Vec::new(),
    };
    let registration = AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Dns),
    );
    let state_proof = crate::hardware::build_hardware_state_proof(node_id, coord);
    let block = bc
        .register_dns_asset(registration, &state_proof, dns_bytes)
        .await
        .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

    info!(
        "DNS self-registered '{}' -> {} (block #{})",
        name, target_addr, block.index,
    );

    Ok(())
}
