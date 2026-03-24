// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Command dispatch -- routes CLI subcommands to their handlers.

use anyhow::Result;
use tracing::{info, warn};

use blockmatrix::bootstrap::NodeBootstrap;
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use blockmatrix::persistence::PersistenceManager;
use blockmatrix::ipc;

use crate::cli::{
    CaesarAction, CatalogAction, Cli, Commands, DnsAction, DomainAction,
    EngaugeAction, GatewayAction, ShareAction, TrustchainAction,
};
use crate::commands::connect::service_ipc_call;
use crate::commands::domain::{run_domain, run_join};
use crate::commands::store::{run_fetch, run_store};

#[allow(clippy::too_many_lines)]
pub(crate) async fn dispatch_command(
    mut cli: Cli,
    coord: MatrixCoordinate,
    nid: &str,
    data_dir: &std::path::Path,
    bootstrap: &NodeBootstrap,
    persistence: std::sync::Arc<PersistenceManager>,
) -> Result<()> {
    match cli.command {
        Some(Commands::Start) => {
            eprintln!(
                "'start' is deprecated. Use 'hypermesh connect public --foreground' instead."
            );
            eprintln!("Running as 'connect public --foreground'...");
            crate::commands::connect::run_connect(
                &cli, coord, nid, data_dir, bootstrap, persistence,
            )
            .await?;
        }
        Some(Commands::Connect { privacy, .. }) => {
            cli.privacy = privacy;
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                println!("Daemon already running.");
                return Ok(());
            }
            crate::commands::connect::run_connect(
                &cli, coord, nid, data_dir, bootstrap, persistence,
            )
            .await?;
        }
        Some(Commands::Disconnect) => {
            let client = ipc::IpcClient::new();
            if !client.is_daemon_running().await {
                eprintln!("No daemon running.");
                std::process::exit(1);
            }
            match client.call("shutdown", serde_json::json!({})).await {
                Ok(_) => println!("Daemon shutting down."),
                Err(e) => eprintln!("Failed to send shutdown: {e}"),
            }
        }
        Some(Commands::Status) => {
            dispatch_status(bootstrap, cli.json).await;
        }
        Some(Commands::SetPrivacy { mode }) => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                let mode_str = format!("{mode:?}");
                match client
                    .call_ok("set_privacy", serde_json::json!({"mode": mode_str}))
                    .await
                {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => eprintln!("Error: {e}"),
                }
            } else {
                let new_mode = mode.into();
                info!("Transitioning to {:?} mode...", new_mode);
                bootstrap.set_privacy_mode(new_mode).await?;
                info!("Privacy mode updated successfully");
            }
        }
        Some(Commands::Store { path }) => {
            dispatch_store(path).await?;
        }
        Some(Commands::Fetch { asset_id, output }) => {
            dispatch_fetch(asset_id, output).await?;
        }
        Some(Commands::Dns { action }) => {
            dispatch_dns(action, bootstrap, data_dir, nid).await?;
        }
        Some(Commands::Domain { action }) => {
            dispatch_domain(action, bootstrap, data_dir, nid).await?;
        }
        Some(Commands::Join { network, invite }) => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                match client
                    .call_ok(
                        "domain.join",
                        serde_json::json!({
                            "domain": network,
                            "invite": invite,
                        }),
                    )
                    .await
                {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => eprintln!("Error: {e}"),
                }
            } else {
                run_join(&network, invite.as_deref(), nid, data_dir).await?;
            }
        }
        Some(Commands::Config { .. }) => {
            unreachable!("config commands handled before bootstrap");
        }
        Some(Commands::Dashboard { action }) => {
            crate::dispatch_dashboard::dispatch_dashboard(action, bootstrap, data_dir, nid).await?;
        }
        Some(Commands::Caesar { action }) => {
            dispatch_caesar(action, cli.json).await?;
        }
        Some(Commands::Trustchain { action }) => {
            dispatch_trustchain(action, cli.json).await?;
        }
        Some(Commands::Engauge { action }) => {
            dispatch_engauge(action, cli.json).await?;
        }
        Some(Commands::Catalog { action }) => {
            dispatch_catalog(action, cli.json).await?;
        }
        Some(Commands::Gateway { action }) => {
            dispatch_gateway(action, cli.json).await?;
        }
        Some(Commands::Share { action }) => {
            dispatch_share(action, cli.json).await?;
        }
        Some(Commands::Destroy { .. }) => {
            unreachable!("destroy handled before bootstrap");
        }
        Some(Commands::Ping { .. }) => {
            unreachable!("ping handled before bootstrap");
        }
        None => {
            info!("Node initialized successfully. Use 'connect' to run or 'status' to check.");
        }
    }
    Ok(())
}

async fn dispatch_status(bootstrap: &NodeBootstrap, json: bool) {
    let client = ipc::IpcClient::new();
    if client.is_daemon_running().await {
        match client.call_ok("status", serde_json::json!({})).await {
            Ok(resp) => println!(
                "{}",
                serde_json::to_string_pretty(&resp).unwrap_or_default()
            ),
            Err(e) => eprintln!("Error: {e}"),
        }
    } else if json {
        let height = bootstrap.blockchain().get_height().await;
        let privacy = format!("{:?}", bootstrap.privacy_mode().await);
        let status = serde_json::json!({
            "online": false,
            "genesis": bootstrap.genesis_block().hash,
            "chain_height": height,
            "privacy_mode": privacy,
            "self_sufficient": true,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&status).unwrap_or_default()
        );
    } else {
        info!("Node Status (offline):");
        info!("  Genesis: {}", bootstrap.genesis_block().hash);
        info!(
            "  Blockchain height: {}",
            bootstrap.blockchain().get_height().await
        );
        info!("  Privacy mode: {:?}", bootstrap.privacy_mode().await);
        info!("  Self-sufficient: yes");
        eprintln!("No daemon running. Start with: hypermesh connect public");
    }
}

async fn dispatch_store(path: std::path::PathBuf) -> Result<()> {
    let client = ipc::IpcClient::new();
    if client.is_daemon_running().await {
        let path_str = path.display().to_string();
        match client
            .call_ok("store", serde_json::json!({"path": path_str}))
            .await
        {
            Ok(resp) => println!(
                "{}",
                serde_json::to_string_pretty(&resp).unwrap_or_default()
            ),
            Err(e) => {
                warn!("IPC store failed ({e}), falling back to standalone");
                run_store(path, None).await?;
            }
        }
    } else {
        run_store(path, None).await?;
    }
    Ok(())
}

async fn dispatch_fetch(
    asset_id: String,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    let client = ipc::IpcClient::new();
    if client.is_daemon_running().await {
        match client
            .call_ok(
                "fetch",
                serde_json::json!({
                    "asset_id": asset_id,
                    "output": output.as_ref().map(|p| p.display().to_string()),
                }),
            )
            .await
        {
            Ok(resp) => println!(
                "{}",
                serde_json::to_string_pretty(&resp).unwrap_or_default()
            ),
            Err(e) => {
                warn!("IPC fetch failed ({e}), falling back to standalone");
                run_fetch(asset_id, output).await?;
            }
        }
    } else {
        run_fetch(asset_id, output).await?;
    }
    Ok(())
}

async fn dispatch_dns(
    action: DnsAction,
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    nid: &str,
) -> Result<()> {
    let client = ipc::IpcClient::new();
    if client.is_daemon_running().await {
        let result = match &action {
            DnsAction::Register { name, addr } => {
                client
                    .call_ok(
                        "dns.register",
                        serde_json::json!({"name": name, "addr": addr}),
                    )
                    .await
            }
            DnsAction::Resolve { name } => {
                client
                    .call_ok("dns.resolve", serde_json::json!({"name": name}))
                    .await
            }
            DnsAction::List => {
                client
                    .call_ok("dns.list", serde_json::json!({}))
                    .await
            }
        };
        match result {
            Ok(resp) => println!(
                "{}",
                serde_json::to_string_pretty(&resp).unwrap_or_default()
            ),
            Err(e) => eprintln!("Error: {e}"),
        }
    } else {
        crate::commands::dns::run_dns(action, bootstrap, data_dir, nid).await?;
    }
    Ok(())
}

async fn dispatch_domain(
    action: DomainAction,
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    nid: &str,
) -> Result<()> {
    let client = ipc::IpcClient::new();
    if client.is_daemon_running().await {
        let result = match &action {
            DomainAction::Register { name, privacy } => {
                client
                    .call_ok(
                        "domain.register",
                        serde_json::json!({
                            "name": name,
                            "privacy": format!("{privacy:?}"),
                        }),
                    )
                    .await
            }
            DomainAction::List => {
                client
                    .call_ok("domain.list", serde_json::json!({}))
                    .await
            }
            _ => {
                run_domain(action, bootstrap, data_dir, nid).await?;
                return Ok(());
            }
        };
        match result {
            Ok(resp) => println!(
                "{}",
                serde_json::to_string_pretty(&resp).unwrap_or_default()
            ),
            Err(e) => {
                warn!("IPC domain call failed ({e}), falling back to offline");
                run_domain(action, bootstrap, data_dir, nid).await?;
            }
        }
    } else {
        run_domain(action, bootstrap, data_dir, nid).await?;
    }
    Ok(())
}

async fn dispatch_caesar(action: CaesarAction, json: bool) -> Result<()> {
    match action {
        CaesarAction::Wallet => {
            service_ipc_call("caesar.wallet", serde_json::json!({}), json).await?;
        }
        CaesarAction::Balance => {
            service_ipc_call("caesar.balance", serde_json::json!({}), json).await?;
        }
        CaesarAction::Transactions { limit } => {
            service_ipc_call(
                "caesar.transactions",
                serde_json::json!({"limit": limit}),
                json,
            )
            .await?;
        }
        CaesarAction::Rewards => {
            service_ipc_call("caesar.rewards", serde_json::json!({}), json).await?;
        }
        CaesarAction::Route {
            destination,
            amount,
        } => {
            service_ipc_call(
                "caesar.route_packet",
                serde_json::json!({
                    "destination": destination,
                    "amount_grams": amount,
                }),
                json,
            )
            .await?;
        }
        CaesarAction::Governor => {
            service_ipc_call("caesar.governor_params", serde_json::json!({}), json).await?;
        }
    }
    Ok(())
}

async fn dispatch_trustchain(action: TrustchainAction, json: bool) -> Result<()> {
    match action {
        TrustchainAction::Certs => {
            service_ipc_call("trustchain.certificates", serde_json::json!({}), json)
                .await?;
        }
        TrustchainAction::Issue { subject, scope } => {
            service_ipc_call(
                "trustchain.issue",
                serde_json::json!({"subject": subject, "scope": scope}),
                json,
            )
            .await?;
        }
        TrustchainAction::Validate { cert_path } => {
            service_ipc_call(
                "trustchain.validate",
                serde_json::json!({"cert_pem": cert_path}),
                json,
            )
            .await?;
        }
        TrustchainAction::Revoke { cert_id } => {
            service_ipc_call(
                "trustchain.revoke",
                serde_json::json!({"cert_id": cert_id}),
                json,
            )
            .await?;
        }
        TrustchainAction::Zones => {
            service_ipc_call("trustchain.dns_zones", serde_json::json!({}), json).await?;
        }
    }
    Ok(())
}

async fn dispatch_engauge(action: EngaugeAction, json: bool) -> Result<()> {
    match action {
        EngaugeAction::Capacity => {
            service_ipc_call("engauge.capacity", serde_json::json!({}), json).await?;
        }
        EngaugeAction::Traffic => {
            service_ipc_call("engauge.traffic", serde_json::json!({}), json).await?;
        }
        EngaugeAction::Marketplace => {
            service_ipc_call("engauge.marketplace", serde_json::json!({}), json).await?;
        }
        EngaugeAction::Metrics => {
            service_ipc_call("engauge.node_metrics", serde_json::json!({}), json).await?;
        }
        EngaugeAction::Leases => {
            service_ipc_call("engauge.leases", serde_json::json!({}), json).await?;
        }
    }
    Ok(())
}

async fn dispatch_catalog(action: CatalogAction, json: bool) -> Result<()> {
    match action {
        CatalogAction::Browse { query, page } => {
            service_ipc_call(
                "catalog.browse",
                serde_json::json!({"query": query, "page": page}),
                json,
            )
            .await?;
        }
        CatalogAction::Search { query } => {
            service_ipc_call(
                "catalog.search",
                serde_json::json!({"query": query}),
                json,
            )
            .await?;
        }
        CatalogAction::Info { name } => {
            service_ipc_call(
                "catalog.package_info",
                serde_json::json!({"name": name}),
                json,
            )
            .await?;
        }
        CatalogAction::Stats => {
            service_ipc_call("catalog.registry_stats", serde_json::json!({}), json)
                .await?;
        }
    }
    Ok(())
}

async fn dispatch_gateway(action: GatewayAction, json: bool) -> Result<()> {
    match action {
        GatewayAction::Transfer { asset_id, from, to } => {
            service_ipc_call(
                "gateway.transfer",
                serde_json::json!({
                    "asset_id": asset_id,
                    "source_scope": from,
                    "target_scope": to,
                }),
                json,
            )
            .await?;
        }
        GatewayAction::Status { transfer_id } => {
            service_ipc_call(
                "gateway.status",
                serde_json::json!({"transfer_id": transfer_id}),
                json,
            )
            .await?;
        }
        GatewayAction::List => {
            service_ipc_call("gateway.list", serde_json::json!({}), json).await?;
        }
    }
    Ok(())
}

async fn dispatch_share(action: ShareAction, json: bool) -> Result<()> {
    match action {
        ShareAction::Send { asset_id, with } => {
            service_ipc_call(
                "share.send",
                serde_json::json!({
                    "asset_id": asset_id,
                    "recipient": with,
                }),
                json,
            )
            .await?;
        }
        ShareAction::Inbox { limit } => {
            service_ipc_call(
                "share.inbox",
                serde_json::json!({"limit": limit}),
                json,
            )
            .await?;
        }
        ShareAction::Accept { invite_id } => {
            service_ipc_call(
                "share.accept",
                serde_json::json!({"invite_id": invite_id}),
                json,
            )
            .await?;
        }
        ShareAction::Reject { invite_id } => {
            service_ipc_call(
                "share.reject",
                serde_json::json!({"invite_id": invite_id}),
                json,
            )
            .await?;
        }
        ShareAction::Pubkey => {
            service_ipc_call("identity.pubkey", serde_json::json!({}), json).await?;
        }
        ShareAction::PeerPubkey { node_id } => {
            service_ipc_call(
                "peer.pubkey",
                serde_json::json!({"node_id": node_id}),
                json,
            )
            .await?;
        }
    }
    Ok(())
}
