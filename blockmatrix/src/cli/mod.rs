// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CLI module for matrix topology queries, node management, and asset operations
//!
//! This is a **library module**, not a binary. It provides:
//!
//! - **Command types** ([`CliCommand`], [`TopologyCommand`], [`NodeCommand`],
//!   [`AssetCommand`]) representing parsed CLI input.
//! - **Execution logic** ([`CommandExecutor`]) that translates commands into
//!   operations on the matrix subsystems.
//! - **Output formatting** ([`CliOutput`], [`CliTable`], [`CliError`]) for
//!   structured, display-ready results.
//!
//! A future binary crate will parse command-line arguments (e.g., via clap),
//! convert them into [`CliCommand`] values, and pass them to
//! [`CommandExecutor::execute`].
//!
//! # Example
//!
//! ```
//! use blockmatrix::cli::{
//!     CommandExecutor, CliCommand, TopologyCommand, CliOutput,
//! };
//!
//! let mut executor = CommandExecutor::new();
//! let cmd = CliCommand::Topology(TopologyCommand::MatrixInfo);
//! let output = executor.execute(cmd).expect("execution should succeed");
//! println!("{}", output);
//! ```

pub mod commands;
pub mod executor;
pub mod output;

// Re-export primary public API
pub use commands::{
    parse_pipeline_action, parse_scope, AssetCommand, CliCommand, NodeCommand, PipelineAction,
    TopologyCommand,
};
pub use executor::CommandExecutor;
pub use output::{CliError, CliOutput, CliTable};

// ---------------------------------------------------------------------------
// CliRunner convenience wrapper
// ---------------------------------------------------------------------------

/// High-level runner that owns a [`CommandExecutor`] and provides a single
/// `execute` entry point.
///
/// This is a thin convenience wrapper. Callers who need more control can use
/// [`CommandExecutor`] directly.
pub struct CliRunner {
    executor: CommandExecutor,
}

impl CliRunner {
    /// Create a new runner with a fresh executor.
    pub fn new() -> Self {
        Self {
            executor: CommandExecutor::new(),
        }
    }

    /// Execute a command and return structured output.
    pub fn execute(&mut self, command: CliCommand) -> Result<CliOutput, CliError> {
        self.executor.execute(command)
    }

    /// Borrow the underlying executor.
    pub fn executor(&self) -> &CommandExecutor {
        &self.executor
    }

    /// Mutably borrow the underlying executor.
    pub fn executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
}

impl Default for CliRunner {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::BlockchainScope;

    #[test]
    fn test_cli_runner_execute() {
        let mut runner = CliRunner::new();
        let cmd = CliCommand::Topology(TopologyCommand::MatrixInfo);
        let output = runner.execute(cmd).expect("test: execute");
        let text = format!("{output}");
        assert!(text.contains("Total nodes: 0"));
    }

    #[test]
    fn test_cli_runner_default() {
        let mut runner = CliRunner::default();
        let cmd = CliCommand::Node(NodeCommand::Register {
            x: 1,
            y: 2,
            z: 3,
            scope: BlockchainScope::Device,
        });
        let output = runner.execute(cmd).expect("test: register");
        assert!(format!("{output}").contains("Registered"));
    }

    #[test]
    fn test_end_to_end_register_and_query() {
        let mut runner = CliRunner::new();

        // Register two nodes
        runner
            .execute(CliCommand::Node(NodeCommand::Register {
                x: 10,
                y: 10,
                z: 10,
                scope: BlockchainScope::Device,
            }))
            .expect("test: register node-1");

        runner
            .execute(CliCommand::Node(NodeCommand::Register {
                x: 15,
                y: 15,
                z: 15,
                scope: BlockchainScope::Network,
            }))
            .expect("test: register node-2");

        // Query neighbors from origin with large radius
        let result = runner
            .execute(CliCommand::Topology(TopologyCommand::QueryNeighbors {
                x: 0,
                y: 0,
                z: 0,
                radius: 100.0,
            }))
            .expect("test: query neighbors");

        match result {
            CliOutput::Table(table) => {
                assert_eq!(table.row_count(), 2);
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }
}
