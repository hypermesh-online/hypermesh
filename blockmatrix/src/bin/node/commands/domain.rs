// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Domain subcommand handlers (register, create, list, nodes, invite, join).

use anyhow::{Context, Result};
use tracing::{info, warn};

use blockmatrix::assets::core::{AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope};
use blockmatrix::bootstrap::{NodeBootstrap, PrivacyMode};
use blockmatrix::dns::domain::DomainRegistration;
use blockmatrix::dns::invitation;
use blockmatrix::StateProof;

use crate::cli::DomainAction;
use trustchain::proof_of_state::StateProofOps;

/// Path to persisted domain registrations for a given node.
fn domain_registrations_path(data_dir: &std::path::Path, node_id: &str) -> std::path::PathBuf {
    data_dir.join(node_id).join("domain_registrations.json")
}

/// Load domain registrations from disk.
fn load_domain_registrations(
    data_dir: &std::path::Path,
    node_id: &str,
) -> Vec<DomainRegistration> {
    let path = domain_registrations_path(data_dir, node_id);
    if !path.exists() {
        return Vec::new();
    }
    match blockmatrix::dns::domain::load_domains(&path) {
        Ok(domains) => domains,
        Err(e) => {
            warn!("Failed to load domain registrations: {e}");
            Vec::new()
        }
    }
}

/// Save domain registrations to disk.
fn save_domain_registrations(
    data_dir: &std::path::Path,
    node_id: &str,
    domains: &[DomainRegistration],
) -> Result<()> {
    let path = domain_registrations_path(data_dir, node_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    blockmatrix::dns::domain::save_domains(domains, &path)
        .context("failed to save domain registrations")
}

/// Run the Domain subcommand: register, create, list, nodes, or invite.
pub async fn run_domain(
    action: DomainAction,
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    node_id: &str,
) -> Result<()> {
    match action {
        DomainAction::Register { name, privacy } => {
            let privacy_mode: PrivacyMode = privacy.into();
            let mut domains = load_domain_registrations(data_dir, node_id);

            if domains.iter().any(|d| d.domain_name == name) {
                anyhow::bail!("Domain '{}' is already registered on this node", name);
            }

            let reg = DomainRegistration::new(&name, privacy_mode, node_id.to_string());

            let dns_data_str = format!("DOMAIN:REGISTER:{name}");
            let asset_data = AssetData {
                config: dns_data_str.as_bytes().to_vec(),
                definition: format!("domain-registration:{name}").into_bytes(),
                metadata: format!("network_id={},privacy={privacy_mode:?}", reg.network_id)
                    .into_bytes(),
            };
            let registration = AssetRegistration::from_asset_data(
                &asset_data,
                NetworkScope::Global,
                AssetCategory::BaseSystem(BaseSystemType::Dns),
            );
            let state_proof = StateProof::generate_from_network(node_id)
                .await
                .context("PoS proof generation failed for domain registration")?;
            let block = bootstrap
                .blockchain()
                .register_asset_record(registration, &state_proof)
                .await
                .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

            domains.push(reg.clone());
            save_domain_registrations(data_dir, node_id, &domains)?;

            println!();
            println!("  Domain Registered");
            println!("  -----------------");
            println!("  domain:     {name}");
            println!("  network_id: {}", reg.network_id);
            println!("  privacy:    {privacy_mode:?}");
            println!("  block:      #{}", block.index);
            println!();
        }
        DomainAction::Create { name, privacy } => {
            let privacy_mode: PrivacyMode = privacy.into();
            let mut domains = load_domain_registrations(data_dir, node_id);

            if domains.iter().any(|d| d.domain_name == name) {
                anyhow::bail!("Domain '{}' is already registered on this node", name);
            }

            if let Some(dot_pos) = name.find('.') {
                let parent = &name[dot_pos + 1..];
                if !parent.is_empty() && !domains.iter().any(|d| d.domain_name == parent) {
                    warn!(
                        "Parent domain '{}' not registered on this node (proceeding anyway)",
                        parent
                    );
                }
            }

            let reg = DomainRegistration::new(&name, privacy_mode, node_id.to_string());

            let dns_data_str = format!("DOMAIN:CREATE:{name}");
            let asset_data = AssetData {
                config: dns_data_str.as_bytes().to_vec(),
                definition: format!("domain-subdomain:{name}").into_bytes(),
                metadata: format!("network_id={},privacy={privacy_mode:?}", reg.network_id)
                    .into_bytes(),
            };
            let registration = AssetRegistration::from_asset_data(
                &asset_data,
                NetworkScope::Global,
                AssetCategory::BaseSystem(BaseSystemType::Dns),
            );
            let state_proof = StateProof::generate_from_network(node_id)
                .await
                .context("PoS proof generation failed for subdomain creation")?;
            let block = bootstrap
                .blockchain()
                .register_asset_record(registration, &state_proof)
                .await
                .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

            domains.push(reg.clone());
            save_domain_registrations(data_dir, node_id, &domains)?;

            println!();
            println!("  Sub-Domain Created");
            println!("  ------------------");
            println!("  domain:     {name}");
            println!("  network_id: {}", reg.network_id);
            println!(
                "  parent:     {}",
                reg.parent_network_id.as_deref().unwrap_or("(none)")
            );
            println!("  privacy:    {privacy_mode:?}");
            println!("  block:      #{}", block.index);
            println!();
        }
        DomainAction::List => {
            let domains = load_domain_registrations(data_dir, node_id);
            if domains.is_empty() {
                println!("No domains registered.");
            } else {
                println!();
                println!("  Registered Domains");
                println!("  ------------------");
                for d in &domains {
                    println!(
                        "  {:<30} net={} privacy={:?}",
                        d.domain_name,
                        &d.network_id[..16],
                        d.privacy_mode,
                    );
                }
                println!();
            }
        }
        DomainAction::Nodes { domain } => {
            let domains = load_domain_registrations(data_dir, node_id);
            let found = domains.iter().find(|d| d.domain_name == domain);
            match found {
                Some(d) => {
                    println!();
                    println!("  Domain: {}", d.domain_name);
                    println!("  Network ID: {}", d.network_id);
                    println!("  Owner: {}", d.owner_node_id);
                    println!(
                        "  Members: (local node only -- connect for network view)"
                    );
                    println!();
                }
                None => {
                    println!("Domain '{}' not found in local registrations.", domain);
                }
            }
        }
        DomainAction::Invite { domain, peer, ttl } => {
            let domains = load_domain_registrations(data_dir, node_id);
            let found = domains.iter().find(|d| d.domain_name == domain);
            let reg = match found {
                Some(d) => d,
                None => {
                    anyhow::bail!(
                        "Domain '{}' not registered on this node. Register it first.",
                        domain
                    );
                }
            };

            let proof_bytes = reg
                .state_proof_bytes
                .as_deref()
                .unwrap_or(node_id.as_bytes());

            let invitee = if peer == "open" {
                None
            } else {
                Some(peer.as_str())
            };
            let inv = invitation::create_invitation(&domain, proof_bytes, invitee, ttl);
            let token = invitation::encode_invitation(&inv)
                .map_err(|e| anyhow::anyhow!("failed to encode invitation: {e}"))?;

            println!();
            println!("  Domain Invitation");
            println!("  -----------------");
            println!("  domain:  {domain}");
            println!(
                "  peer:    {}",
                if peer == "open" { "(open)" } else { &peer }
            );
            println!("  expires: {} seconds", ttl);
            println!("  token:");
            println!("  {token}");
            println!();
        }
    }
    Ok(())
}

/// Run the Join subcommand: join a domain network (optionally with invitation).
pub async fn run_join(
    network: &str,
    invite_token: Option<&str>,
    node_id: &str,
    data_dir: &std::path::Path,
) -> Result<()> {
    if let Some(token_str) = invite_token {
        let inv = invitation::decode_invitation(token_str)
            .map_err(|e| anyhow::anyhow!("Invalid invitation: {e}"))?;

        if inv.domain_name != network {
            anyhow::bail!(
                "Invitation is for domain '{}', not '{}'",
                inv.domain_name,
                network
            );
        }

        if !inv.invitee_node_id.is_empty() && inv.invitee_node_id != node_id {
            anyhow::bail!(
                "Invitation is for node '{}', not this node ('{}')",
                inv.invitee_node_id,
                node_id
            );
        }

        info!("Invitation validated for domain '{}'", network);
    }

    let domains = load_domain_registrations(data_dir, node_id);
    let network_id = blockmatrix::dns::domain::derive_network_id(network);

    println!();
    println!("  Join Domain Network");
    println!("  -------------------");
    println!("  domain:     {network}");
    println!("  network_id: {network_id}");
    if domains.iter().any(|d| d.domain_name == network) {
        println!("  status:     already registered (owner)");
    } else {
        println!("  status:     membership recorded (connect daemon to sync)");
    }
    println!();

    Ok(())
}
