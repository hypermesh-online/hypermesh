// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS subcommand handlers.

use anyhow::{Context, Result};
use tracing::{info, warn};

use blockmatrix::assets::core::{AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope};
use blockmatrix::bootstrap::NodeBootstrap;
use blockmatrix::StateProof;

use crate::cli::DnsAction;

/// Path to the persisted DNS records file for a given node data directory.
fn dns_records_path(data_dir: &std::path::Path, node_id: &str) -> std::path::PathBuf {
    data_dir.join(node_id).join("dns_records.json")
}

/// Load persisted DNS records from disk and register them into the resolver.
pub async fn load_persisted_dns(
    dns: &blockmatrix::bootstrap::DnsResolver,
    data_dir: &std::path::Path,
    node_id: &str,
) {
    let path = dns_records_path(data_dir, node_id);
    if !path.exists() {
        return;
    }
    match std::fs::read_to_string(&path) {
        Ok(json) => {
            if let Ok(records) =
                serde_json::from_str::<std::collections::HashMap<String, String>>(&json)
            {
                let mut count = 0u64;
                for (name, addr_str) in &records {
                    if let Ok(addr) = addr_str.parse::<std::net::IpAddr>() {
                        dns.register(name.clone(), addr).await;
                        count += 1;
                    }
                }
                if count > 0 {
                    info!("Loaded {count} persisted DNS record(s) from disk");
                }
            }
        }
        Err(e) => {
            warn!("Failed to read DNS records from {}: {e}", path.display());
        }
    }
}

/// Scan the blockchain for DNS-typed block entries and register them
/// in the local resolver.
pub async fn extract_dns_from_blockchain(
    dns: &blockmatrix::bootstrap::DnsResolver,
    bootstrap: &NodeBootstrap,
) {
    let chain = bootstrap.blockchain().get_chain().await;
    let mut count = 0u64;

    for block in &chain {
        let entries = blockmatrix::dns::extract_dns_entries_from_block(block);
        for (domain_name, ip_addr) in entries {
            dns.register(domain_name, ip_addr).await;
            count += 1;
        }
    }

    if count > 0 {
        info!("Extracted {count} DNS record(s) from blockchain");
    }
}

/// Persist a single DNS record by updating the on-disk JSON file.
pub(crate) fn persist_dns_record(
    data_dir: &std::path::Path,
    node_id: &str,
    name: &str,
    addr: std::net::IpAddr,
) -> Result<()> {
    let path = dns_records_path(data_dir, node_id);
    let mut records: std::collections::HashMap<String, String> = if path.exists() {
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&json).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };
    records.insert(name.to_string(), addr.to_string());
    let json = serde_json::to_string_pretty(&records)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, json)?;
    Ok(())
}

/// Run the Dns subcommand: register, resolve, or list DNS names.
pub async fn run_dns(
    action: DnsAction,
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    node_id: &str,
) -> Result<()> {
    match action {
        DnsAction::Register { name, addr } => {
            let target_addr: std::net::IpAddr = if let Some(ref a) = addr {
                a.parse()
                    .with_context(|| format!("invalid IPv6 address: {a}"))?
            } else {
                std::net::IpAddr::from(std::net::Ipv6Addr::LOCALHOST)
            };

            bootstrap.dns().register(name.clone(), target_addr).await;
            persist_dns_record(data_dir, node_id, &name, target_addr)?;

            let bc = bootstrap.blockchain();
            let ipv6_addr = match target_addr {
                std::net::IpAddr::V6(v6) => v6,
                std::net::IpAddr::V4(v4) => v4.to_ipv6_mapped(),
            };
            let dns_entry = blockmatrix::dns::DnsBlockEntry {
                domain_name: name.clone(),
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
            let state_proof = StateProof::generate_from_network(node_id)
                .await
                .context("PoS proof generation failed for DNS registration")?;
            let block = bc
                .register_dns_asset(registration, &state_proof, dns_bytes)
                .await
                .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

            info!(
                "DNS block #{} stored locally (propagation deferred to next node start)",
                block.index,
            );

            println!();
            println!("  DNS Registered");
            println!("  --------------");
            println!("  name:  {name}");
            println!("  addr:  {target_addr}");
            println!("  block: {}", block.hash);
            println!("  chain: height {}", bc.get_height().await);
            println!();
        }
        DnsAction::Resolve { name } => {
            match bootstrap.dns().resolve(&name).await {
                Some(addr) => println!("{name} -> {addr}"),
                None => println!("{name}: not found"),
            }
        }
        DnsAction::List => {
            let records = bootstrap.dns().all_records().await;
            if records.is_empty() {
                println!("No DNS records registered.");
            } else {
                println!();
                println!("  DNS Records");
                println!("  -----------");
                let mut sorted: Vec<_> = records.into_iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                for (name, addr) in sorted {
                    println!("  {name:<20} -> {addr}");
                }
                println!();
            }
        }
    }
    Ok(())
}
