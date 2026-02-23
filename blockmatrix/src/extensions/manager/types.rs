// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Extension manager types - configuration, state, health, metrics

use std::collections::HashSet;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::assets::core::PrivacyMode;
use crate::extensions::{
    ExtensionCapability, ExtensionMetadata, ResourceLimits,
};

/// Extension manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManagerConfig {
    /// Extension directory paths
    pub extension_dirs: Vec<PathBuf>,

    /// Auto-load extensions on startup
    pub auto_load: bool,

    /// Verify extension signatures
    pub verify_signatures: bool,

    /// Maximum extensions to load
    pub max_extensions: usize,

    /// Enable extension hot-reload
    pub hot_reload: bool,

    /// Extension timeout for operations
    pub operation_timeout: std::time::Duration,

    /// Global resource limits
    pub global_limits: ResourceLimits,

    /// Allowed capabilities for extensions
    pub allowed_capabilities: HashSet<ExtensionCapability>,

    /// Extension marketplace URL
    pub marketplace_url: Option<String>,

    /// Enable extension sandboxing
    pub enable_sandboxing: bool,

    /// Extension cache directory
    pub cache_dir: PathBuf,
}

impl Default for ExtensionManagerConfig {
    fn default() -> Self {
        Self {
            extension_dirs: vec![
                PathBuf::from("./extensions"),
                PathBuf::from("/usr/local/hypermesh/extensions"),
                PathBuf::from("~/.hypermesh/extensions"),
            ],
            auto_load: true,
            verify_signatures: true,
            max_extensions: 100,
            hot_reload: false,
            operation_timeout: std::time::Duration::from_secs(30),
            global_limits: ResourceLimits::default(),
            allowed_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::VMExecution,
                ExtensionCapability::ContainerManagement,
                ExtensionCapability::NetworkAccess,
                ExtensionCapability::ConsensusAccess,
                ExtensionCapability::TransportAccess,
                ExtensionCapability::MonitoringAccess,
            ]),
            marketplace_url: Some("https://marketplace.hypermesh.online".to_string()),
            enable_sandboxing: true,
            cache_dir: PathBuf::from("~/.hypermesh/extension-cache"),
        }
    }
}

/// Extension state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStateInfo {
    /// Extension ID
    pub id: String,

    /// Current state
    pub state: ExtensionState,

    /// Health status
    pub health: ExtensionHealth,

    /// Load timestamp
    pub loaded_at: std::time::SystemTime,

    /// Last activity timestamp
    pub last_activity: std::time::SystemTime,

    /// Request count
    pub request_count: u64,

    /// Error count
    pub error_count: u64,

    /// Resource usage
    pub resource_usage: ResourceUsage,
}

/// Extension state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionState {
    /// Extension is loading
    Loading,

    /// Extension is active and running
    Active,

    /// Extension is paused
    Paused,

    /// Extension is unloading
    Unloading,

    /// Extension has errored
    Error(String),
}

/// Extension health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionHealth {
    /// Extension is healthy
    Healthy,

    /// Extension is degraded
    Degraded(String),

    /// Extension is unhealthy
    Unhealthy(String),
}

/// Resource usage tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_percent: f32,

    /// Memory usage in bytes
    pub memory_bytes: u64,

    /// Network bandwidth in bytes/sec
    pub network_bandwidth: u64,

    /// Storage usage in bytes
    pub storage_bytes: u64,

    /// Active operation count
    pub active_operations: usize,
}

/// Extension metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionMetrics {
    /// Total extensions loaded
    pub total_loaded: usize,

    /// Total extensions failed
    pub total_failed: usize,

    /// Total requests processed
    pub total_requests: u64,

    /// Total errors
    pub total_errors: u64,

    /// Average request duration
    pub avg_request_duration: std::time::Duration,

    /// Peak memory usage
    pub peak_memory: u64,

    /// Peak CPU usage
    pub peak_cpu: f32,
}

/// Extension operation context
pub struct ExtensionContext {
    /// Extension ID
    pub extension_id: String,

    /// Request ID
    pub request_id: String,

    /// Operation timeout
    pub timeout: std::time::Duration,

    /// Granted capabilities
    pub capabilities: HashSet<ExtensionCapability>,

    /// Privacy level
    pub privacy_level: PrivacyMode,
}

/// Extension information combining metadata and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    /// Extension metadata
    pub metadata: ExtensionMetadata,

    /// Current state information
    pub state: ExtensionStateInfo,
}

/// Extension manifest for file-based loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ExtensionManifest {
    /// Extension ID
    pub id: String,

    /// Extension name
    pub name: String,

    /// Extension version
    pub version: String,

    /// Entry point (binary or module path)
    pub entry_point: String,

    /// Extension type (native, wasm, script)
    pub extension_type: String,

    /// Required capabilities
    pub capabilities: Vec<String>,

    /// Configuration schema
    pub config_schema: Option<serde_json::Value>,
}
