// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Dashboard command dispatch -- deploy, list, info, init subcommands.

use anyhow::{Context, Result};
use tracing::info;

use blockmatrix::bootstrap::NodeBootstrap;
use blockmatrix::ipc;

use crate::cli::DashboardAction;

pub(crate) async fn dispatch_dashboard(
    action: DashboardAction,
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    nid: &str,
) -> Result<()> {
    match action {
        DashboardAction::Deploy { path } => {
            deploy_dashboard(&path, bootstrap, data_dir, nid).await?;
        }
        DashboardAction::List => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                match client
                    .call_ok("dashboard.list", serde_json::json!({}))
                    .await
                {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => eprintln!("Error: {e}"),
                }
            } else {
                println!(
                    "No dashboards registered yet. \
                     Deploy with: hypermesh dashboard deploy <path>"
                );
            }
        }
        DashboardAction::Info { name } => {
            let client = ipc::IpcClient::new();
            if client.is_daemon_running().await {
                match client
                    .call_ok("dashboard.info", serde_json::json!({"name": name}))
                    .await
                {
                    Ok(resp) => println!(
                        "{}",
                        serde_json::to_string_pretty(&resp).unwrap_or_default()
                    ),
                    Err(e) => eprintln!("Error: {e}"),
                }
            } else {
                println!("Dashboard '{}': not found", name);
            }
        }
        DashboardAction::Init { name } => {
            init_dashboard(name)?;
        }
    }
    Ok(())
}

async fn deploy_dashboard(
    path: &std::path::Path,
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    nid: &str,
) -> Result<()> {
    let manifest_path = path.join("dashboard.toml");
    if !manifest_path.exists() {
        eprintln!("No dashboard.toml found in {}", path.display());
        std::process::exit(1);
    }
    let toml_str = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
    let manifest = blockmatrix::dashboard::parse_manifest(&toml_str)
        .map_err(|e| anyhow::anyhow!(e))?;
    if let Err(errors) =
        blockmatrix::dashboard::validate_manifest(&manifest, path)
    {
        for e in &errors {
            eprintln!("Validation error: {e}");
        }
        std::process::exit(1);
    }
    info!(
        "Dashboard '{}' v{} validated",
        manifest.dashboard.name, manifest.dashboard.version
    );
    info!("Domain: {}", manifest.dashboard.domain);

    let files = blockmatrix::dashboard::deploy::collect_dashboard_files(
        path,
        &manifest.access,
    )
    .with_context(|| "failed to collect dashboard files")?;

    if files.is_empty() {
        eprintln!("No files found in dashboard scope directories");
        std::process::exit(1);
    }

    let bundle = blockmatrix::dashboard::deploy::bundle_files(&files);

    let client = ipc::IpcClient::new();
    if client.is_daemon_running().await {
        deploy_via_daemon(&client, &manifest, &toml_str, &files).await?;
    } else {
        deploy_standalone(
            &manifest, &toml_str, &bundle, bootstrap, data_dir, nid, &files,
        )
        .await?;
    }
    Ok(())
}

async fn deploy_via_daemon(
    client: &ipc::IpcClient,
    manifest: &blockmatrix::dashboard::DashboardManifest,
    toml_str: &str,
    files: &std::collections::BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    use base64::Engine as _;
    let files_json: serde_json::Value = files
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                serde_json::Value::String(
                    base64::engine::general_purpose::STANDARD.encode(v),
                ),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    match client
        .call_ok(
            "dashboard.deploy",
            serde_json::json!({
                "name": manifest.dashboard.name,
                "manifest_toml": toml_str,
                "files": files_json,
            }),
        )
        .await
    {
        Ok(resp) => println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_default()
        ),
        Err(e) => {
            eprintln!("Deploy via daemon failed: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn deploy_standalone(
    manifest: &blockmatrix::dashboard::DashboardManifest,
    toml_str: &str,
    bundle: &[u8],
    bootstrap: &NodeBootstrap,
    data_dir: &std::path::Path,
    nid: &str,
    files: &std::collections::BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    use blockmatrix::assets::core::{
        AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope,
    };
    use blockmatrix::StateProof;

    let asset_data = AssetData {
        config: format!("DASHBOARD:DEPLOY:{}", manifest.dashboard.name)
            .into_bytes(),
        definition: bundle.to_vec(),
        metadata: toml_str.as_bytes().to_vec(),
    };
    let registration = AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Dashboard),
    );
    let content_hash = registration.content_hash;
    let state_proof = StateProof::generate_from_network(nid)
        .await
        .context("PoS proof generation failed for dashboard deploy")?;
    let block = bootstrap
        .blockchain()
        .register_asset_record(registration, &state_proof)
        .await
        .map_err(|e| anyhow::anyhow!("blockchain write failed: {e}"))?;

    blockmatrix::dashboard::deploy::store_dashboard_bundle(
        data_dir,
        &content_hash,
        toml_str,
        bundle,
    )
    .with_context(|| "failed to store dashboard bundle")?;

    println!();
    println!("  Dashboard Deployed");
    println!("  ------------------");
    println!("  name:    {}", manifest.dashboard.name);
    println!("  version: {}", manifest.dashboard.version);
    println!("  domain:  {}", manifest.dashboard.domain);
    println!("  hash:    {}", hex::encode(content_hash));
    println!("  block:   #{}", block.index);
    println!("  files:   {}", files.len());
    println!();
    Ok(())
}

fn init_dashboard(name: Option<String>) -> Result<()> {
    let project_name = name.unwrap_or_else(|| "my-dashboard".to_string());
    info!("Scaffolding dashboard project: {}", project_name);

    let dir = std::path::PathBuf::from(&project_name);
    std::fs::create_dir_all(dir.join("dist/public"))?;
    std::fs::create_dir_all(dir.join("dist/private"))?;

    let manifest_toml =
        blockmatrix::dashboard::scaffold_manifest(&project_name);
    std::fs::write(dir.join("dashboard.toml"), &manifest_toml)?;

    std::fs::write(
        dir.join("dist/public/index.html"),
        blockmatrix::dashboard::scaffold_html(&project_name, "public"),
    )?;
    std::fs::write(
        dir.join("dist/private/index.html"),
        blockmatrix::dashboard::scaffold_html(&project_name, "private"),
    )?;

    println!("Created dashboard project at ./{project_name}/");
    println!("  dashboard.toml");
    println!("  dist/public/index.html");
    println!("  dist/private/index.html");
    println!(
        "\nDeploy with: hypermesh dashboard deploy ./{project_name}/"
    );
    Ok(())
}
