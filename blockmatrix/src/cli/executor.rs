// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Command execution logic for CLI commands
//!
//! `CommandExecutor` holds references to the matrix foundation components and
//! translates each `CliCommand` into operations on the underlying subsystems.
//! Topology commands delegate to tensor routing functions, node commands
//! operate on `MatrixCoordinate`, and asset commands use gateway transfer types.

use std::collections::HashMap;

use hypermesh_lib::BlockchainScope;

use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::neighbors::find_neighbors;
use crate::matrix::tensor::routing::{
    calculate_routing_path, calculate_routing_vector, score_route_quality,
};

use super::commands::{
    AssetCommand, CliCommand, NodeCommand, PipelineAction, TopologyCommand,
};
use super::output::{CliError, CliOutput, CliTable};

// ---------------------------------------------------------------------------
// Node registry (in-memory)
// ---------------------------------------------------------------------------

/// Minimal in-memory record for a registered node.
#[derive(Debug, Clone)]
struct NodeRecord {
    node_id: String,
    position: MatrixCoordinate,
    scope: BlockchainScope,
}

// ---------------------------------------------------------------------------
// CommandExecutor
// ---------------------------------------------------------------------------

/// Executes parsed CLI commands against matrix subsystems.
///
/// Maintains an in-memory node registry for demonstration and testing purposes.
/// A production implementation would back this with the persistence layer.
pub struct CommandExecutor {
    /// Registered nodes keyed by node ID.
    nodes: HashMap<String, NodeRecord>,
    /// Auto-incrementing counter for generated node IDs.
    next_node_id: u64,
}

impl CommandExecutor {
    /// Create a new executor with an empty node registry.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            next_node_id: 1,
        }
    }

    /// Execute a CLI command and return structured output.
    pub fn execute(&mut self, command: CliCommand) -> Result<CliOutput, CliError> {
        match command {
            CliCommand::Topology(cmd) => self.execute_topology(cmd),
            CliCommand::Node(cmd) => self.execute_node(cmd),
            CliCommand::Asset(cmd) => self.execute_asset(cmd),
        }
    }

    // -----------------------------------------------------------------------
    // Topology
    // -----------------------------------------------------------------------

    fn execute_topology(&self, cmd: TopologyCommand) -> Result<CliOutput, CliError> {
        match cmd {
            TopologyCommand::QueryNeighbors { x, y, z, radius } => {
                self.topology_query_neighbors(x, y, z, radius)
            }
            TopologyCommand::RoutingCost {
                from_x, from_y, from_z,
                to_x, to_y, to_z,
            } => self.topology_routing_cost(from_x, from_y, from_z, to_x, to_y, to_z),
            TopologyCommand::ShowPath {
                from_x, from_y, from_z,
                to_x, to_y, to_z,
            } => self.topology_show_path(from_x, from_y, from_z, to_x, to_y, to_z),
            TopologyCommand::MatrixInfo => self.topology_matrix_info(),
        }
    }

    fn topology_query_neighbors(
        &self,
        x: i64,
        y: i64,
        z: i64,
        radius: f64,
    ) -> Result<CliOutput, CliError> {
        if radius < 0.0 {
            return Err(CliError::InvalidArgument(
                "Radius must be non-negative".into(),
            ));
        }

        let center = make_coordinate(x, y, z)?;
        let candidates: Vec<MatrixCoordinate> =
            self.nodes.values().map(|n| n.position).collect();
        let neighbors = find_neighbors(&center, &candidates, radius);

        let mut table = CliTable::new(vec![
            "Node".into(),
            "Position".into(),
            "Distance".into(),
        ]);

        for pos in &neighbors {
            if let Some(record) = self.find_node_at(pos) {
                let dist = center.euclidean_distance(pos);
                table.add_row(vec![
                    record.node_id.clone(),
                    format!("{}", pos),
                    format!("{:.2}", dist),
                ]).map_err(|e| CliError::ExecutionFailed(e.to_string()))?;
            }
        }

        Ok(CliOutput::Table(table))
    }

    fn topology_routing_cost(
        &self,
        from_x: i64,
        from_y: i64,
        from_z: i64,
        to_x: i64,
        to_y: i64,
        to_z: i64,
    ) -> Result<CliOutput, CliError> {
        let from = make_coordinate(from_x, from_y, from_z)?;
        let to = make_coordinate(to_x, to_y, to_z)?;

        let euclidean = from.euclidean_distance(&to);
        let manhattan = from.manhattan_distance(&to);

        let direction = calculate_routing_vector(&from, &to);
        let path = calculate_routing_path(&from, &to, 50.0);
        let quality = score_route_quality(&path, 50.0);

        let text = format!(
            "Routing cost from {} to {}:\n  Euclidean distance: {:.2}\n  Manhattan distance: {}\n  Direction vector:   ({:.4}, {:.4}, {:.4})\n  Path hops:          {}\n  Route quality:      {:.2}",
            from, to,
            euclidean,
            manhattan,
            direction.x, direction.y, direction.z,
            path.len(),
            quality,
        );
        Ok(CliOutput::Text(text))
    }

    fn topology_show_path(
        &self,
        from_x: i64,
        from_y: i64,
        from_z: i64,
        to_x: i64,
        to_y: i64,
        to_z: i64,
    ) -> Result<CliOutput, CliError> {
        let from = make_coordinate(from_x, from_y, from_z)?;
        let to = make_coordinate(to_x, to_y, to_z)?;

        let path = calculate_routing_path(&from, &to, 50.0);

        let mut table = CliTable::new(vec![
            "Hop".into(),
            "Position".into(),
            "Hop Distance".into(),
        ]);

        for (i, coord) in path.iter().enumerate() {
            let hop_dist = if i == 0 {
                0.0
            } else {
                path[i - 1].euclidean_distance(coord)
            };
            table.add_row(vec![
                format!("{}", i),
                format!("{}", coord),
                format!("{:.2}", hop_dist),
            ]).map_err(|e| CliError::ExecutionFailed(e.to_string()))?;
        }

        Ok(CliOutput::Table(table))
    }

    fn topology_matrix_info(&self) -> Result<CliOutput, CliError> {
        let node_count = self.nodes.len();

        let (min_x, max_x, min_y, max_y, min_z, max_z) = if self.nodes.is_empty() {
            (0, 0, 0, 0, 0, 0)
        } else {
            self.nodes.values().fold(
                (i64::MAX, i64::MIN, i64::MAX, i64::MIN, i64::MAX, i64::MIN),
                |(mnx, mxx, mny, mxy, mnz, mxz), n| {
                    (
                        mnx.min(n.position.x), mxx.max(n.position.x),
                        mny.min(n.position.y), mxy.max(n.position.y),
                        mnz.min(n.position.z), mxz.max(n.position.z),
                    )
                },
            )
        };

        let text = format!(
            "Matrix Info:\n  Total nodes: {}\n  X range:     [{}, {}]\n  Y range:     [{}, {}]\n  Z range:     [{}, {}]",
            node_count, min_x, max_x, min_y, max_y, min_z, max_z,
        );
        Ok(CliOutput::Text(text))
    }

    // -----------------------------------------------------------------------
    // Node
    // -----------------------------------------------------------------------

    fn execute_node(&mut self, cmd: NodeCommand) -> Result<CliOutput, CliError> {
        match cmd {
            NodeCommand::Status { node_id } => self.node_status(&node_id),
            NodeCommand::List { scope } => self.node_list(scope),
            NodeCommand::Register { x, y, z, scope } => {
                self.node_register(x, y, z, scope)
            }
        }
    }

    fn node_status(&self, node_id: &str) -> Result<CliOutput, CliError> {
        let record = self
            .nodes
            .get(node_id)
            .ok_or_else(|| CliError::NotFound(format!("Node '{}'", node_id)))?;

        let text = format!(
            "Node: {}\n  Position: {}\n  Scope:    {}",
            record.node_id, record.position, record.scope,
        );
        Ok(CliOutput::Text(text))
    }

    fn node_list(&self, scope: Option<BlockchainScope>) -> Result<CliOutput, CliError> {
        let mut table = CliTable::new(vec![
            "Node ID".into(),
            "Position".into(),
            "Scope".into(),
        ]);

        let mut entries: Vec<&NodeRecord> = self
            .nodes
            .values()
            .filter(|n| scope.map_or(true, |s| n.scope == s))
            .collect();
        entries.sort_by(|a, b| a.node_id.cmp(&b.node_id));

        for record in entries {
            table.add_row(vec![
                record.node_id.clone(),
                format!("{}", record.position),
                format!("{}", record.scope),
            ]).map_err(|e| CliError::ExecutionFailed(e.to_string()))?;
        }

        Ok(CliOutput::Table(table))
    }

    fn node_register(
        &mut self,
        x: i64,
        y: i64,
        z: i64,
        scope: BlockchainScope,
    ) -> Result<CliOutput, CliError> {
        let position = make_coordinate(x, y, z)?;

        let node_id = format!("node-{}", self.next_node_id);
        self.next_node_id += 1;

        self.nodes.insert(
            node_id.clone(),
            NodeRecord {
                node_id: node_id.clone(),
                position,
                scope,
            },
        );

        let text = format!(
            "Registered node '{}' at {} (scope: {})",
            node_id, position, scope,
        );
        Ok(CliOutput::Text(text))
    }

    // -----------------------------------------------------------------------
    // Asset
    // -----------------------------------------------------------------------

    fn execute_asset(&self, cmd: AssetCommand) -> Result<CliOutput, CliError> {
        match cmd {
            AssetCommand::Info { asset_id } => self.asset_info(&asset_id),
            AssetCommand::Transfer {
                asset_id,
                from_scope,
                to_scope,
            } => self.asset_transfer(&asset_id, from_scope, to_scope),
            AssetCommand::Pipeline { action, path } => {
                self.asset_pipeline(action, &path)
            }
        }
    }

    fn asset_info(&self, asset_id: &str) -> Result<CliOutput, CliError> {
        if asset_id.is_empty() {
            return Err(CliError::InvalidArgument(
                "Asset ID must not be empty".into(),
            ));
        }

        // In a real implementation this would query the AssetManager.
        // For now return a structured placeholder showing the query was valid.
        let text = format!(
            "Asset: {}\n  Status: not found in local registry\n  Hint:   use 'asset transfer' to move assets between scopes",
            asset_id,
        );
        Ok(CliOutput::Text(text))
    }

    fn asset_transfer(
        &self,
        asset_id: &str,
        from_scope: BlockchainScope,
        to_scope: BlockchainScope,
    ) -> Result<CliOutput, CliError> {
        if asset_id.is_empty() {
            return Err(CliError::InvalidArgument(
                "Asset ID must not be empty".into(),
            ));
        }

        if from_scope == to_scope {
            return Err(CliError::InvalidArgument(format!(
                "Source and target scopes are identical: {}",
                from_scope,
            )));
        }

        let text = format!(
            "Transfer queued:\n  Asset:      {}\n  From scope: {}\n  To scope:   {}\n  Status:     Pending",
            asset_id, from_scope, to_scope,
        );
        Ok(CliOutput::Text(text))
    }

    fn asset_pipeline(
        &self,
        action: PipelineAction,
        path: &str,
    ) -> Result<CliOutput, CliError> {
        if path.is_empty() {
            return Err(CliError::InvalidArgument(
                "Path must not be empty".into(),
            ));
        }

        let action_name = match action {
            PipelineAction::Compress => "Compress (Brotli)",
            PipelineAction::Encrypt => "Encrypt (Kyber-1024)",
            PipelineAction::Shard => "Shard (Reed-Solomon)",
        };

        let text = format!(
            "Pipeline action queued:\n  Action: {}\n  Path:   {}",
            action_name, path,
        );
        Ok(CliOutput::Text(text))
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Find the first node at a given matrix position.
    fn find_node_at(&self, pos: &MatrixCoordinate) -> Option<&NodeRecord> {
        self.nodes.values().find(|n| n.position == *pos)
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Free-standing helpers
// ---------------------------------------------------------------------------

/// Validate and construct a `MatrixCoordinate`, mapping errors to `CliError`.
fn make_coordinate(x: i64, y: i64, z: i64) -> Result<MatrixCoordinate, CliError> {
    MatrixCoordinate::new(x, y, z).map_err(|e| CliError::InvalidArgument(e.to_string()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::*;

    // -- Topology tests -------------------------------------------------------

    #[test]
    fn test_topology_query_neighbors_returns_results() {
        let mut exec = CommandExecutor::new();

        // Register nodes
        exec.execute(CliCommand::Node(NodeCommand::Register {
            x: 5, y: 5, z: 5,
            scope: BlockchainScope::Device,
        })).expect("test: register node");
        exec.execute(CliCommand::Node(NodeCommand::Register {
            x: 100, y: 100, z: 100,
            scope: BlockchainScope::Device,
        })).expect("test: register far node");

        let result = exec.execute(CliCommand::Topology(
            TopologyCommand::QueryNeighbors { x: 0, y: 0, z: 0, radius: 20.0 },
        )).expect("test: query neighbors");

        match result {
            CliOutput::Table(table) => {
                assert_eq!(table.row_count(), 1, "Only the close node should match");
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_topology_query_neighbors_negative_radius() {
        let exec = CommandExecutor::new();
        let result = exec.execute_topology(TopologyCommand::QueryNeighbors {
            x: 0, y: 0, z: 0, radius: -1.0,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_topology_routing_cost() {
        let exec = CommandExecutor::new();
        let result = exec.execute_topology(TopologyCommand::RoutingCost {
            from_x: 0, from_y: 0, from_z: 0,
            to_x: 100, to_y: 0, to_z: 0,
        }).expect("test: routing cost");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("Euclidean distance: 100.00"));
                assert!(text.contains("Manhattan distance: 100"));
                assert!(text.contains("Route quality"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_topology_show_path() {
        let exec = CommandExecutor::new();
        let result = exec.execute_topology(TopologyCommand::ShowPath {
            from_x: 0, from_y: 0, from_z: 0,
            to_x: 200, to_y: 0, to_z: 0,
        }).expect("test: show path");

        match result {
            CliOutput::Table(table) => {
                assert!(table.row_count() >= 3, "Should have multiple hops for distance 200");
                assert_eq!(table.headers[0], "Hop");
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_topology_matrix_info() {
        let mut exec = CommandExecutor::new();
        exec.execute(CliCommand::Node(NodeCommand::Register {
            x: -10, y: 20, z: 0,
            scope: BlockchainScope::Device,
        })).expect("test: register");

        let result = exec.execute_topology(TopologyCommand::MatrixInfo)
            .expect("test: matrix info");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("Total nodes: 1"));
                assert!(text.contains("-10"));
                assert!(text.contains("20"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    // -- Node tests -----------------------------------------------------------

    #[test]
    fn test_node_list_empty() {
        let mut exec = CommandExecutor::new();
        let result = exec.execute(CliCommand::Node(NodeCommand::List { scope: None }))
            .expect("test: list nodes");

        match result {
            CliOutput::Table(table) => {
                assert_eq!(table.row_count(), 0);
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_node_list_populated() {
        let mut exec = CommandExecutor::new();
        exec.execute(CliCommand::Node(NodeCommand::Register {
            x: 1, y: 2, z: 3,
            scope: BlockchainScope::Device,
        })).expect("test: register");
        exec.execute(CliCommand::Node(NodeCommand::Register {
            x: 4, y: 5, z: 6,
            scope: BlockchainScope::Network,
        })).expect("test: register");

        let result = exec.execute(CliCommand::Node(NodeCommand::List { scope: None }))
            .expect("test: list all");
        match &result {
            CliOutput::Table(table) => assert_eq!(table.row_count(), 2),
            other => unreachable!("test: expected Table, got {:?}", other),
        }

        // Filter by scope
        let result = exec.execute(CliCommand::Node(NodeCommand::List {
            scope: Some(BlockchainScope::Device),
        })).expect("test: list filtered");
        match result {
            CliOutput::Table(table) => assert_eq!(table.row_count(), 1),
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_node_register() {
        let mut exec = CommandExecutor::new();
        let result = exec.execute(CliCommand::Node(NodeCommand::Register {
            x: 10, y: 20, z: 30,
            scope: BlockchainScope::Device,
        })).expect("test: register node");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("Registered node"));
                assert!(text.contains("(10,20,30)"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_node_status_found() {
        let mut exec = CommandExecutor::new();
        exec.execute(CliCommand::Node(NodeCommand::Register {
            x: 0, y: 0, z: 0,
            scope: BlockchainScope::Device,
        })).expect("test: register");

        let result = exec.execute(CliCommand::Node(NodeCommand::Status {
            node_id: "node-1".into(),
        })).expect("test: status");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("node-1"));
                assert!(text.contains("Device"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_node_status_not_found() {
        let exec = CommandExecutor::new();
        let result = exec.node_status("nonexistent");
        assert!(matches!(result, Err(CliError::NotFound(_))));
    }

    // -- Asset tests ----------------------------------------------------------

    #[test]
    fn test_asset_transfer_valid_scopes() {
        let exec = CommandExecutor::new();
        let result = exec.execute_asset(AssetCommand::Transfer {
            asset_id: "cpu-001".into(),
            from_scope: BlockchainScope::Device,
            to_scope: BlockchainScope::Network,
        }).expect("test: transfer");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("Transfer queued"));
                assert!(text.contains("cpu-001"));
                assert!(text.contains("Pending"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_asset_transfer_same_scope() {
        let exec = CommandExecutor::new();
        let result = exec.execute_asset(AssetCommand::Transfer {
            asset_id: "a1".into(),
            from_scope: BlockchainScope::Device,
            to_scope: BlockchainScope::Device,
        });
        assert!(matches!(result, Err(CliError::InvalidArgument(_))));
    }

    #[test]
    fn test_asset_transfer_empty_id() {
        let exec = CommandExecutor::new();
        let result = exec.execute_asset(AssetCommand::Transfer {
            asset_id: "".into(),
            from_scope: BlockchainScope::Device,
            to_scope: BlockchainScope::Network,
        });
        assert!(matches!(result, Err(CliError::InvalidArgument(_))));
    }

    #[test]
    fn test_asset_pipeline() {
        let exec = CommandExecutor::new();
        let result = exec.execute_asset(AssetCommand::Pipeline {
            action: PipelineAction::Compress,
            path: "/data/file.bin".into(),
        }).expect("test: pipeline");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("Compress (Brotli)"));
                assert!(text.contains("/data/file.bin"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_asset_pipeline_empty_path() {
        let exec = CommandExecutor::new();
        let result = exec.execute_asset(AssetCommand::Pipeline {
            action: PipelineAction::Encrypt,
            path: "".into(),
        });
        assert!(matches!(result, Err(CliError::InvalidArgument(_))));
    }

    // -- Error handling -------------------------------------------------------

    #[test]
    fn test_invalid_coordinate() {
        let result = make_coordinate(i64::MAX, 0, 0);
        assert!(matches!(result, Err(CliError::InvalidArgument(_))));
    }
}
