// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CLI command types for matrix topology queries, node management, and asset operations
//!
//! These are pure data types representing parsed CLI commands. No framework
//! dependency (e.g., clap) is used -- a future binary crate will parse CLI
//! arguments and convert them into these types.

use hypermesh_lib::BlockchainScope;

// ---------------------------------------------------------------------------
// Top-level command
// ---------------------------------------------------------------------------

/// Top-level CLI command dispatched to sub-command handlers.
#[derive(Debug, Clone, PartialEq)]
pub enum CliCommand {
    /// Matrix topology queries (neighbors, routing, paths).
    Topology(TopologyCommand),
    /// Node management (status, listing, registration).
    Node(NodeCommand),
    /// Asset operations (info, transfer, pipeline).
    Asset(AssetCommand),
}

// ---------------------------------------------------------------------------
// Topology commands
// ---------------------------------------------------------------------------

/// Commands for querying the Block-MATRIX topology.
#[derive(Debug, Clone, PartialEq)]
pub enum TopologyCommand {
    /// Find nodes within a radius of a matrix position.
    QueryNeighbors { x: i64, y: i64, z: i64, radius: f64 },
    /// Calculate the routing cost between two positions.
    RoutingCost {
        from_x: i64,
        from_y: i64,
        from_z: i64,
        to_x: i64,
        to_y: i64,
        to_z: i64,
    },
    /// Show the routing path (intermediate hops) between two positions.
    ShowPath {
        from_x: i64,
        from_y: i64,
        from_z: i64,
        to_x: i64,
        to_y: i64,
        to_z: i64,
    },
    /// Display matrix dimensions and node count.
    MatrixInfo,
}

// ---------------------------------------------------------------------------
// Node commands
// ---------------------------------------------------------------------------

/// Commands for managing nodes in the matrix.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeCommand {
    /// Show the status of a specific node.
    Status { node_id: String },
    /// List known nodes, optionally filtered by blockchain scope.
    List { scope: Option<BlockchainScope> },
    /// Register a new node at a matrix position.
    Register {
        x: i64,
        y: i64,
        z: i64,
        scope: BlockchainScope,
    },
}

// ---------------------------------------------------------------------------
// Asset commands
// ---------------------------------------------------------------------------

/// Commands for asset operations (info, transfers, pipeline).
#[derive(Debug, Clone, PartialEq)]
pub enum AssetCommand {
    /// Show details for a specific asset.
    Info { asset_id: String },
    /// Initiate a cross-scope asset transfer.
    Transfer {
        asset_id: String,
        from_scope: BlockchainScope,
        to_scope: BlockchainScope,
    },
    /// Run a pipeline action on a file path.
    Pipeline {
        action: PipelineAction,
        path: String,
    },
}

// ---------------------------------------------------------------------------
// Pipeline action
// ---------------------------------------------------------------------------

/// Supported asset pipeline actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineAction {
    /// Brotli compression (stage 1).
    Compress,
    /// Kyber-1024 encryption (stage 2).
    Encrypt,
    /// Reed-Solomon erasure coding (stage 3).
    Shard,
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Parse a string into a `BlockchainScope`.
///
/// Accepts case-insensitive `"device"` and `"network"`.
///
/// # Errors
///
/// Returns an error message if the string does not match a known scope.
pub fn parse_scope(s: &str) -> Result<BlockchainScope, String> {
    match s.to_lowercase().as_str() {
        "device" => Ok(BlockchainScope::Device),
        "network" => Ok(BlockchainScope::Network),
        other => Err(format!(
            "Unknown blockchain scope '{other}'. Expected 'device' or 'network'.",
        )),
    }
}

/// Parse a string into a `PipelineAction`.
///
/// Accepts case-insensitive `"compress"`, `"encrypt"`, and `"shard"`.
///
/// # Errors
///
/// Returns an error message if the string does not match a known action.
pub fn parse_pipeline_action(s: &str) -> Result<PipelineAction, String> {
    match s.to_lowercase().as_str() {
        "compress" => Ok(PipelineAction::Compress),
        "encrypt" => Ok(PipelineAction::Encrypt),
        "shard" => Ok(PipelineAction::Shard),
        other => Err(format!(
            "Unknown pipeline action '{other}'. Expected 'compress', 'encrypt', or 'shard'.",
        )),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scope_device() {
        assert_eq!(parse_scope("device"), Ok(BlockchainScope::Device));
        assert_eq!(parse_scope("Device"), Ok(BlockchainScope::Device));
        assert_eq!(parse_scope("DEVICE"), Ok(BlockchainScope::Device));
    }

    #[test]
    fn test_parse_scope_network() {
        assert_eq!(parse_scope("network"), Ok(BlockchainScope::Network));
        assert_eq!(parse_scope("Network"), Ok(BlockchainScope::Network));
    }

    #[test]
    fn test_parse_scope_invalid() {
        let err = parse_scope("public").unwrap_err();
        assert!(err.contains("Unknown blockchain scope"));
        assert!(err.contains("public"));
    }

    #[test]
    fn test_parse_pipeline_action() {
        assert_eq!(
            parse_pipeline_action("compress"),
            Ok(PipelineAction::Compress)
        );
        assert_eq!(
            parse_pipeline_action("Encrypt"),
            Ok(PipelineAction::Encrypt)
        );
        assert_eq!(parse_pipeline_action("SHARD"), Ok(PipelineAction::Shard));
    }

    #[test]
    fn test_parse_pipeline_action_invalid() {
        let err = parse_pipeline_action("distribute").unwrap_err();
        assert!(err.contains("Unknown pipeline action"));
    }

    #[test]
    fn test_cli_command_variants() {
        let topo = CliCommand::Topology(TopologyCommand::MatrixInfo);
        let node = CliCommand::Node(NodeCommand::Status {
            node_id: "n1".into(),
        });
        let asset = CliCommand::Asset(AssetCommand::Info {
            asset_id: "a1".into(),
        });

        // Verify Debug works (no panic)
        let _ = format!("{topo:?}");
        let _ = format!("{node:?}");
        let _ = format!("{asset:?}");
    }
}
