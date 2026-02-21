// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CLI module for Caesar Ephemeral Value Protocol operations
//!
//! This is a **library module**, not a binary. It provides:
//!
//! - **Command types** ([`CliCommand`], [`PacketCommand`], [`NodeCommand`],
//!   [`GovernorCommand`], [`OracleCommand`]) representing parsed CLI input.
//! - **Execution logic** ([`CommandExecutor`]) that translates commands into
//!   operations on in-memory Caesar state.
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
//! use caesar::cli::{
//!     CommandExecutor, CliCommand, GovernorCommand, CliOutput,
//! };
//!
//! let mut executor = CommandExecutor::new();
//! let cmd = CliCommand::Governor(GovernorCommand::Params);
//! let output = executor.execute(cmd).expect("execution should succeed");
//! println!("{}", output);
//! ```

pub mod commands;
pub mod executor;
pub mod output;

// Re-export primary public API
pub use commands::{
    CliCommand, GovernorCommand, NodeCommand, OracleCommand, PacketCommand,
    parse_packet_state, parse_tier,
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

    #[test]
    fn test_cli_runner_execute_governor_params() {
        let mut runner = CliRunner::new();
        let cmd = CliCommand::Governor(GovernorCommand::Params);
        let output = runner.execute(cmd).expect("test: execute");
        let text = format!("{}", output);
        assert!(text.contains("Kp"));
    }

    #[test]
    fn test_cli_runner_default() {
        let mut runner = CliRunner::default();
        let cmd = CliCommand::Oracle(OracleCommand::Price);
        let output = runner.execute(cmd).expect("test: oracle price");
        assert!(format!("{}", output).contains("Gold price"));
    }

    #[test]
    fn test_end_to_end_mint_and_query() {
        let mut runner = CliRunner::new();

        // Mint a packet
        runner
            .execute(CliCommand::Packet(PacketCommand::Mint {
                sender: "alice".into(),
                recipient: "bob".into(),
                value_grams: 10.5,
                tier: "l0".into(),
            }))
            .expect("test: mint packet");

        // List should show the packet
        let result = runner
            .execute(CliCommand::Packet(PacketCommand::List {
                state_filter: None,
            }))
            .expect("test: list packets");

        match result {
            CliOutput::Table(table) => {
                assert_eq!(table.row_count(), 1);
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }
}
