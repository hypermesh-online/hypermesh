// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Configuration loading, merging, and management subcommands.

use anyhow::{Context, Result};

use blockmatrix::ipc;

use crate::cli::{Cli, ConfigCommand};

/// Load config from `--config` path or the default location.
pub(crate) fn load_config(cli: &Cli) -> ipc::HypermeshConfig {
    match &cli.config {
        Some(path) => ipc::HypermeshConfig::load_from(path),
        None => ipc::HypermeshConfig::load(),
    }
}

/// Parse a string value as JSON; fall back to a JSON string if it fails.
pub(crate) fn parse_config_value(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

pub(crate) fn merge_config_into_cli(cli: &mut Cli) {
    let config = load_config(cli);
    if cli.coord_x == 0 && config.node.coord_x != 0 {
        cli.coord_x = config.node.coord_x;
    }
    if cli.coord_y == 0 && config.node.coord_y != 0 {
        cli.coord_y = config.node.coord_y;
    }
    if cli.coord_z == 0 && config.node.coord_z != 0 {
        cli.coord_z = config.node.coord_z;
    }
    if cli.stoq_port == 9292 && config.network.stoq_port != 9292 {
        cli.stoq_port = config.network.stoq_port;
    }
    if cli.data_dir == "~/.blockmatrix" && config.node.data_dir != "~/.blockmatrix" {
        cli.data_dir = config.node.data_dir.clone();
    }
    if cli.bootstrap.is_empty() && !config.network.bootstrap_nodes.is_empty() {
        cli.bootstrap = config.network.bootstrap_nodes.clone();
    }
    if !cli.reflector && config.network.reflector {
        cli.reflector = true;
    }
}

pub(crate) fn handle_config(action: &ConfigCommand, cli: &Cli) -> Result<()> {
    match action {
        ConfigCommand::Show => {
            let config = load_config(cli);
            let output = serde_json::to_string_pretty(&config)
                .context("failed to serialize config")?;
            println!("{output}");
        }
        ConfigCommand::Get { key } => {
            let config = load_config(cli);
            let value =
                serde_json::to_value(&config).context("failed to serialize config")?;
            match ipc::config::get_dotpath(&value, key) {
                Some(v) => {
                    let output =
                        serde_json::to_string_pretty(v).context("failed to format value")?;
                    println!("{output}");
                }
                None => {
                    eprintln!("Key not found: {key}");
                    std::process::exit(1);
                }
            }
        }
        ConfigCommand::Set { key, value } => {
            let mut config = load_config(cli);
            let mut json_value =
                serde_json::to_value(&config).context("failed to serialize config")?;
            let parsed = parse_config_value(value);
            ipc::config::set_dotpath(&mut json_value, key, parsed)
                .map_err(|e| anyhow::anyhow!("failed to set key: {e}"))?;
            config = serde_json::from_value(json_value)
                .context("invalid config after update")?;
            match &cli.config {
                Some(path) => config
                    .save_to(path)
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
                None => config.save().map_err(|e| anyhow::anyhow!("{e}"))?,
            }
            println!("Set {key} = {value}");
        }
        ConfigCommand::Init => {
            let config = ipc::HypermeshConfig::default();
            let path = match &cli.config {
                Some(p) => p.clone(),
                None => ipc::HypermeshConfig::default_path(),
            };
            config
                .save_to(&path)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("Created {}", path.display());
        }
    }
    Ok(())
}

pub(crate) fn handle_destroy(chaotic: bool, cli: &Cli) -> Result<()> {
    let config = load_config(cli);
    let data_dir_str = if config.node.data_dir != "~/.blockmatrix" {
        config.node.data_dir.clone()
    } else {
        cli.data_dir.clone()
    };
    let data_dir = if data_dir_str.starts_with('~') {
        dirs::home_dir()
            .context("could not determine home directory")?
            .join(&data_dir_str[2..])
    } else {
        std::path::PathBuf::from(&data_dir_str)
    };

    if !data_dir.exists() {
        eprintln!("Nothing to destroy: {} does not exist", data_dir.display());
        return Ok(());
    }

    // D5: node state may live under a legacy coordinate key (`node_{x}_{y}_{z}`)
    // OR the identity key (device-id hex, a dir carrying a `blockchain/`
    // sub-dir), and the FALCON keypair lives at the coordinate-independent
    // `data_dir/identity`. Destroy recognises all three so it works across the
    // migration window.
    let mut node_dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&data_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let is_legacy = name.starts_with("node_");
            let is_identity = name == "identity";
            let is_state_dir = path.join("blockchain").is_dir();
            if is_legacy || is_identity || is_state_dir {
                node_dirs.push(path);
            }
        }
    }

    if node_dirs.is_empty() {
        eprintln!(
            "Nothing to destroy: no node data found in {}",
            data_dir.display()
        );
        return Ok(());
    }

    eprintln!("Found {} node(s) to destroy:", node_dirs.len());
    for d in &node_dirs {
        eprintln!("  {}", d.display());
    }

    if !chaotic {
        eprintln!("\nType 'yes' to confirm:");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim() != "yes" {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    for d in &node_dirs {
        std::fs::remove_dir_all(d)
            .context(format!("failed to remove {}", d.display()))?;
        println!("Destroyed {}", d.display());
    }

    cleanup_sockets();

    Ok(())
}

fn cleanup_sockets() {
    if let Ok(sock) = std::env::var("HYPERMESH_SOCK") {
        if std::path::Path::new(&sock).exists() {
            std::fs::remove_file(&sock).ok();
            println!("Removed socket {sock}");
        }
    }
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let sock_dir = std::path::PathBuf::from(runtime_dir).join("hypermesh");
        if sock_dir.exists() {
            std::fs::remove_dir_all(&sock_dir).ok();
            println!("Removed {}", sock_dir.display());
        }
    }
    if let Some(home) = dirs::home_dir() {
        let sock = home.join(".hypermesh").join("ctl.sock");
        if sock.exists() {
            std::fs::remove_file(&sock).ok();
            println!("Removed {}", sock.display());
        }
        let old_identity = home.join(".hypermesh").join("identity");
        if old_identity.exists() {
            std::fs::remove_dir_all(&old_identity).ok();
            println!("Cleaned legacy identity at ~/.hypermesh/identity/");
        }
    }
}
