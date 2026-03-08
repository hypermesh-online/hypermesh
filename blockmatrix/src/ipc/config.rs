// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! TOML-based configuration for HyperMesh nodes.
//!
//! Config file lives at `~/.hypermesh/config.toml` by default. Every field
//! has a sensible default so the file can be absent or partial.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn default_data_dir() -> String {
    "~/.blockmatrix".to_string()
}
fn default_stoq_port() -> u16 {
    9292
}
fn default_privacy() -> String {
    "public".to_string()
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_mode() -> String {
    "auto".to_string()
}
fn default_gateway_url() -> String {
    "stoq://trust.hypermesh.online".to_string()
}

/// Connection mode for service communication.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionMode {
    /// Try FFI -> IPC -> STOQ in order.
    Auto,
    /// Direct library linking.
    Ffi,
    /// Unix socket JSON-RPC to daemon.
    Ipc,
    /// STOQ protocol API.
    Stoq,
}

impl ConnectionMode {
    /// Parse a connection mode from a string. Unknown values default to `Auto`.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ffi" => Self::Ffi,
            "ipc" => Self::Ipc,
            "stoq" => Self::Stoq,
            // Legacy: treat "http" as "stoq" for backward compatibility
            "http" => Self::Stoq,
            _ => Self::Auto,
        }
    }
}

/// Service endpoint and connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    /// Connection mode: "auto" | "ffi" | "ipc" | "stoq".
    /// auto = try FFI -> IPC -> STOQ in order.
    #[serde(default = "default_mode")]
    pub mode: String,

    /// Gateway unified entry point (default: stoq://trust.hypermesh.online).
    #[serde(default = "default_gateway_url")]
    pub gateway_url: String,
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            gateway_url: default_gateway_url(),
        }
    }
}

impl ServicesConfig {
    /// Parse the `mode` field into a [`ConnectionMode`] enum.
    pub fn connection_mode(&self) -> ConnectionMode {
        ConnectionMode::from_str(&self.mode)
    }
}

/// Top-level HyperMesh configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypermeshConfig {
    #[serde(default)]
    pub node: NodeConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub services: ServicesConfig,
}

/// Node identity and data directory settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    #[serde(default)]
    pub coord_x: i64,
    #[serde(default)]
    pub coord_y: i64,
    #[serde(default)]
    pub coord_z: i64,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

/// Network and STOQ transport settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_stoq_port")]
    pub stoq_port: u16,
    #[serde(default)]
    pub bootstrap_nodes: Vec<String>,
    #[serde(default = "default_privacy")]
    pub privacy: String,
    #[serde(default)]
    pub reflector: bool,
}

/// Logging settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for HypermeshConfig {
    fn default() -> Self {
        Self {
            node: NodeConfig::default(),
            network: NetworkConfig::default(),
            logging: LoggingConfig::default(),
            services: ServicesConfig::default(),
        }
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            coord_x: 0,
            coord_y: 0,
            coord_z: 0,
            data_dir: default_data_dir(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            stoq_port: default_stoq_port(),
            bootstrap_nodes: Vec::new(),
            privacy: default_privacy(),
            reflector: false,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl HypermeshConfig {
    /// Default config file path: `~/.hypermesh/config.toml`.
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".hypermesh")
            .join("config.toml")
    }

    /// Load config from the default path. Returns defaults if the file is
    /// missing or unreadable.
    pub fn load() -> Self {
        Self::load_from(&Self::default_path())
    }

    /// Load config from a specific path. Returns defaults if the file is
    /// missing or unreadable.
    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save config to the default path, creating parent directories as needed.
    pub fn save(&self) -> Result<(), String> {
        self.save_to(&Self::default_path())
    }

    /// Save config to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create config directory: {e}"))?;
        }
        let contents =
            toml::to_string_pretty(self).map_err(|e| format!("failed to serialize config: {e}"))?;
        std::fs::write(path, contents)
            .map_err(|e| format!("failed to write config file: {e}"))?;
        Ok(())
    }
}

/// Navigate a `serde_json::Value` using a dot-separated key path.
///
/// Returns `None` if any segment is missing.
pub fn get_dotpath<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

/// Set a value at a dot-separated key path in a `serde_json::Value`.
///
/// Creates intermediate objects as needed. Returns `Err` if a non-terminal
/// segment is not an object.
pub fn set_dotpath(
    root: &mut serde_json::Value,
    path: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    let segments: Vec<&str> = path.split('.').collect();
    if segments.is_empty() {
        return Err("empty key path".to_string());
    }
    let mut current = root;
    for segment in &segments[..segments.len() - 1] {
        if !current.is_object() {
            return Err(format!("'{segment}' is not an object"));
        }
        current = current
            .as_object_mut()
            .ok_or_else(|| format!("'{segment}' is not an object"))?
            .entry(*segment)
            .or_insert_with(|| serde_json::json!({}));
    }
    let last = segments[segments.len() - 1];
    let obj = current
        .as_object_mut()
        .ok_or_else(|| format!("parent of '{last}' is not an object"))?;
    obj.insert(last.to_string(), value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = HypermeshConfig::default();
        assert_eq!(config.node.coord_x, 0);
        assert_eq!(config.node.data_dir, "~/.blockmatrix");
        assert_eq!(config.network.stoq_port, 9292);
        assert_eq!(config.network.privacy, "public");
        assert!(!config.network.reflector);
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.services.mode, "auto");
        assert_eq!(config.services.gateway_url, "stoq://trust.hypermesh.online");
        assert_eq!(config.services.connection_mode(), ConnectionMode::Auto);
    }

    #[test]
    fn test_load_missing_file_returns_defaults() {
        let config = HypermeshConfig::load_from(Path::new("/tmp/nonexistent_hypermesh_config.toml"));
        assert_eq!(config.network.stoq_port, 9292);
        assert_eq!(config.node.data_dir, "~/.blockmatrix");
    }

    #[test]
    fn test_save_reload_roundtrip() {
        let dir = std::env::temp_dir().join("hypermesh_config_test");
        let path = dir.join("config.toml");

        let mut config = HypermeshConfig::default();
        config.node.coord_x = 42;
        config.network.stoq_port = 9999;
        config.network.bootstrap_nodes = vec!["[::1]:9292".to_string()];
        config.save_to(&path).expect("test: save config");

        let loaded = HypermeshConfig::load_from(&path);
        assert_eq!(loaded.node.coord_x, 42);
        assert_eq!(loaded.network.stoq_port, 9999);
        assert_eq!(loaded.network.bootstrap_nodes, vec!["[::1]:9292"]);
        // Unset fields get defaults
        assert_eq!(loaded.logging.level, "info");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r#"
[node]
coord_x = 10

[network]
reflector = true
"#;
        let config: HypermeshConfig =
            toml::from_str(toml_str).expect("test: parse partial toml");
        assert_eq!(config.node.coord_x, 10);
        assert_eq!(config.node.coord_y, 0); // default
        assert_eq!(config.node.data_dir, "~/.blockmatrix"); // default
        assert!(config.network.reflector);
        assert_eq!(config.network.stoq_port, 9292); // default
    }

    #[test]
    fn test_dotpath_navigation() {
        let config = HypermeshConfig::default();
        let value = serde_json::to_value(&config).expect("test: serialize");

        assert_eq!(
            get_dotpath(&value, "network.stoq_port"),
            Some(&serde_json::json!(9292)),
        );
        assert_eq!(
            get_dotpath(&value, "node.data_dir"),
            Some(&serde_json::json!("~/.blockmatrix")),
        );
        assert!(get_dotpath(&value, "nonexistent.key").is_none());
    }

    #[test]
    fn test_set_dotpath() {
        let mut value = serde_json::json!({"node": {"coord_x": 0}});
        set_dotpath(&mut value, "node.coord_x", serde_json::json!(99))
            .expect("test: set dotpath");
        assert_eq!(value["node"]["coord_x"], 99);
    }

    #[test]
    fn test_connection_mode_parsing() {
        assert_eq!(ConnectionMode::from_str("ffi"), ConnectionMode::Ffi);
        assert_eq!(ConnectionMode::from_str("FFI"), ConnectionMode::Ffi);
        assert_eq!(ConnectionMode::from_str("ipc"), ConnectionMode::Ipc);
        assert_eq!(ConnectionMode::from_str("IPC"), ConnectionMode::Ipc);
        assert_eq!(ConnectionMode::from_str("stoq"), ConnectionMode::Stoq);
        assert_eq!(ConnectionMode::from_str("STOQ"), ConnectionMode::Stoq);
        // Legacy: "http" maps to Stoq
        assert_eq!(ConnectionMode::from_str("http"), ConnectionMode::Stoq);
        assert_eq!(ConnectionMode::from_str("auto"), ConnectionMode::Auto);
        assert_eq!(ConnectionMode::from_str("unknown"), ConnectionMode::Auto);
        assert_eq!(ConnectionMode::from_str(""), ConnectionMode::Auto);
    }

    #[test]
    fn test_services_config_connection_mode() {
        let mut svc = ServicesConfig::default();
        assert_eq!(svc.connection_mode(), ConnectionMode::Auto);

        svc.mode = "ffi".to_string();
        assert_eq!(svc.connection_mode(), ConnectionMode::Ffi);

        svc.mode = "ipc".to_string();
        assert_eq!(svc.connection_mode(), ConnectionMode::Ipc);

        svc.mode = "stoq".to_string();
        assert_eq!(svc.connection_mode(), ConnectionMode::Stoq);
    }

    #[test]
    fn test_parse_services_toml() {
        let toml_str = r#"
[services]
mode = "stoq"
gateway_url = "stoq://my-gateway.local"
"#;
        let config: HypermeshConfig =
            toml::from_str(toml_str).expect("test: parse services toml");
        assert_eq!(config.services.mode, "stoq");
        assert_eq!(config.services.gateway_url, "stoq://my-gateway.local");
        assert_eq!(config.services.connection_mode(), ConnectionMode::Stoq);
    }

    #[test]
    fn test_dotpath_services() {
        let config = HypermeshConfig::default();
        let value = serde_json::to_value(&config).expect("test: serialize");

        assert_eq!(
            get_dotpath(&value, "services.mode"),
            Some(&serde_json::json!("auto")),
        );
        assert_eq!(
            get_dotpath(&value, "services.gateway_url"),
            Some(&serde_json::json!("stoq://trust.hypermesh.online")),
        );
    }

    #[test]
    fn test_set_dotpath_services() {
        let config = HypermeshConfig::default();
        let mut value = serde_json::to_value(&config).expect("test: serialize");

        set_dotpath(
            &mut value,
            "services.mode",
            serde_json::json!("ipc"),
        )
        .expect("test: set services.mode");
        assert_eq!(value["services"]["mode"], "ipc");

        set_dotpath(
            &mut value,
            "services.gateway_url",
            serde_json::json!("stoq://remote:9292"),
        )
        .expect("test: set services.gateway_url");
        assert_eq!(value["services"]["gateway_url"], "stoq://remote:9292");
    }

    #[test]
    fn test_services_roundtrip() {
        let dir = std::env::temp_dir().join("hypermesh_config_svc_test");
        let path = dir.join("config.toml");

        let mut config = HypermeshConfig::default();
        config.services.mode = "ipc".to_string();
        config.services.gateway_url = "stoq://my-gateway.local".to_string();
        config.save_to(&path).expect("test: save config");

        let loaded = HypermeshConfig::load_from(&path);
        assert_eq!(loaded.services.mode, "ipc");
        assert_eq!(loaded.services.gateway_url, "stoq://my-gateway.local");
        assert_eq!(loaded.services.connection_mode(), ConnectionMode::Ipc);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
