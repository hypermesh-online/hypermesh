// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Command execution logic for Caesar CLI commands
//!
//! `CommandExecutor` holds in-memory packet and node state for demonstration
//! and testing purposes. Governor and Oracle commands return static/default
//! values matching the Caesar protocol defaults. A future binary integration
//! will wire this to the real `CaesarProtocol` instance.

use std::collections::HashMap;

use super::commands::*;
use super::output::*;

// ---------------------------------------------------------------------------
// In-memory records
// ---------------------------------------------------------------------------

/// Minimal in-memory record for a packet.
#[derive(Debug, Clone)]
struct PacketRecord {
    id: String,
    sender: String,
    recipient: String,
    value_grams: f64,
    tier: String,
    state: String,
    hop_count: u32,
}

/// Minimal in-memory record for a node.
#[derive(Debug, Clone)]
struct NodeRecord {
    node_id: String,
    settled_count: u64,
    fee_earnings_grams: f64,
}

// ---------------------------------------------------------------------------
// CommandExecutor
// ---------------------------------------------------------------------------

/// Executes parsed CLI commands against in-memory Caesar state.
///
/// Maintains packet and node registries for demonstration and testing.
/// A production implementation would back this with `CaesarProtocol`.
pub struct CommandExecutor {
    packets: HashMap<String, PacketRecord>,
    nodes: HashMap<String, NodeRecord>,
    next_packet_id: u64,
}

impl CommandExecutor {
    /// Create a new executor with empty registries.
    pub fn new() -> Self {
        Self {
            packets: HashMap::new(),
            nodes: HashMap::new(),
            next_packet_id: 1,
        }
    }

    /// Execute a CLI command and return structured output.
    pub fn execute(&mut self, command: CliCommand) -> Result<CliOutput, CliError> {
        match command {
            CliCommand::Packet(cmd) => self.execute_packet(cmd),
            CliCommand::Node(cmd) => self.execute_node(cmd),
            CliCommand::Governor(cmd) => self.execute_governor(cmd),
            CliCommand::Oracle(cmd) => self.execute_oracle(cmd),
        }
    }

    // -----------------------------------------------------------------------
    // Packet commands
    // -----------------------------------------------------------------------

    fn execute_packet(&mut self, cmd: PacketCommand) -> Result<CliOutput, CliError> {
        match cmd {
            PacketCommand::Info { packet_id } => self.packet_info(&packet_id),
            PacketCommand::Status { packet_id } => self.packet_status(&packet_id),
            PacketCommand::List { state_filter } => self.packet_list(state_filter.as_deref()),
            PacketCommand::Mint {
                sender,
                recipient,
                value_grams,
                tier,
            } => self.packet_mint(&sender, &recipient, value_grams, &tier),
        }
    }

    fn packet_info(&self, packet_id: &str) -> Result<CliOutput, CliError> {
        let record = self
            .packets
            .get(packet_id)
            .ok_or_else(|| CliError::NotFound(format!("Packet '{packet_id}'")))?;

        let text = format!(
            "Packet: {}\n  Sender:    {}\n  Recipient: {}\n  Value:     {:.6} g\n  Tier:      {}\n  State:     {}\n  Hops:      {}",
            record.id, record.sender, record.recipient,
            record.value_grams, record.tier, record.state, record.hop_count,
        );
        Ok(CliOutput::Text(text))
    }

    fn packet_status(&self, packet_id: &str) -> Result<CliOutput, CliError> {
        let record = self
            .packets
            .get(packet_id)
            .ok_or_else(|| CliError::NotFound(format!("Packet '{packet_id}'")))?;

        let text = format!("Packet {} state: {}", record.id, record.state);
        Ok(CliOutput::Text(text))
    }

    fn packet_list(&self, state_filter: Option<&str>) -> Result<CliOutput, CliError> {
        // Validate the filter if provided
        let normalized_filter = if let Some(raw) = state_filter {
            Some(parse_packet_state(raw).map_err(CliError::InvalidArgument)?)
        } else {
            None
        };

        let mut table = CliTable::new(vec![
            "Packet ID".into(),
            "Sender".into(),
            "Recipient".into(),
            "Value (g)".into(),
            "Tier".into(),
            "State".into(),
        ]);

        let mut entries: Vec<&PacketRecord> = self
            .packets
            .values()
            .filter(|p| normalized_filter.as_ref().is_none_or(|f| p.state == *f))
            .collect();
        entries.sort_by(|a, b| a.id.cmp(&b.id));

        for record in entries {
            table
                .add_row(vec![
                    record.id.clone(),
                    record.sender.clone(),
                    record.recipient.clone(),
                    format!("{:.6}", record.value_grams),
                    record.tier.clone(),
                    record.state.clone(),
                ])
                .map_err(|e| CliError::ExecutionFailed(e.to_string()))?;
        }

        Ok(CliOutput::Table(table))
    }

    fn packet_mint(
        &mut self,
        sender: &str,
        recipient: &str,
        value_grams: f64,
        tier: &str,
    ) -> Result<CliOutput, CliError> {
        if sender.is_empty() {
            return Err(CliError::InvalidArgument("Sender must not be empty".into()));
        }
        if recipient.is_empty() {
            return Err(CliError::InvalidArgument(
                "Recipient must not be empty".into(),
            ));
        }
        if value_grams <= 0.0 {
            return Err(CliError::InvalidArgument("Value must be positive".into()));
        }

        let normalized_tier = parse_tier(tier).map_err(CliError::InvalidArgument)?;
        let packet_id = format!("pkt-{}", self.next_packet_id);
        self.next_packet_id += 1;

        self.packets.insert(
            packet_id.clone(),
            PacketRecord {
                id: packet_id.clone(),
                sender: sender.to_string(),
                recipient: recipient.to_string(),
                value_grams,
                tier: normalized_tier.clone(),
                state: "Minted".into(),
                hop_count: 0,
            },
        );

        let text = format!(
            "Minted packet '{packet_id}': {value_grams:.6} g ({normalized_tier}) from '{sender}' to '{recipient}'",
        );
        Ok(CliOutput::Text(text))
    }

    // -----------------------------------------------------------------------
    // Node commands
    // -----------------------------------------------------------------------

    fn execute_node(&self, cmd: NodeCommand) -> Result<CliOutput, CliError> {
        match cmd {
            NodeCommand::Status { node_id } => self.node_status(&node_id),
            NodeCommand::List => self.node_list(),
            NodeCommand::Preferences { node_id } => self.node_preferences(&node_id),
        }
    }

    fn node_status(&self, node_id: &str) -> Result<CliOutput, CliError> {
        let record = self
            .nodes
            .get(node_id)
            .ok_or_else(|| CliError::NotFound(format!("Node '{node_id}'")))?;

        let text = format!(
            "Node: {}\n  Settled packets: {}\n  Fee earnings:    {:.6} g",
            record.node_id, record.settled_count, record.fee_earnings_grams,
        );
        Ok(CliOutput::Text(text))
    }

    fn node_list(&self) -> Result<CliOutput, CliError> {
        let mut table = CliTable::new(vec![
            "Node ID".into(),
            "Settled".into(),
            "Earnings (g)".into(),
        ]);

        let mut entries: Vec<&NodeRecord> = self.nodes.values().collect();
        entries.sort_by(|a, b| a.node_id.cmp(&b.node_id));

        for record in entries {
            table
                .add_row(vec![
                    record.node_id.clone(),
                    format!("{}", record.settled_count),
                    format!("{:.6}", record.fee_earnings_grams),
                ])
                .map_err(|e| CliError::ExecutionFailed(e.to_string()))?;
        }

        Ok(CliOutput::Table(table))
    }

    fn node_preferences(&self, node_id: &str) -> Result<CliOutput, CliError> {
        if !self.nodes.contains_key(node_id) {
            return Err(CliError::NotFound(format!("Node '{node_id}'")));
        }

        // Default operator preferences (future: wire to real OperatorPreferences)
        let text = format!(
            "Node '{node_id}' operator preferences:\n  Max concurrent packets: 100\n  Accepted tiers:        L0, L1, L2, L3\n  Min fee threshold:     0.001 g",
        );
        Ok(CliOutput::Text(text))
    }

    // -----------------------------------------------------------------------
    // Governor commands
    // -----------------------------------------------------------------------

    fn execute_governor(&self, cmd: GovernorCommand) -> Result<CliOutput, CliError> {
        match cmd {
            GovernorCommand::Params => self.governor_params(),
            GovernorCommand::Pressure => self.governor_pressure(),
            GovernorCommand::FeeCaps => self.governor_fee_caps(),
        }
    }

    fn governor_params(&self) -> Result<CliOutput, CliError> {
        // Default PID gains from GovernorPid::new()
        let mut table = CliTable::new(vec!["Parameter".into(), "Value".into()]);

        let rows = [
            ("Kp (proportional gain)", "0.5"),
            ("Ki (integral gain)", "0.1"),
            ("Kd (derivative gain)", "0.05"),
            ("Health score", "50"),
            ("Pressure quadrant", "GoldenEra"),
            ("Fee adjustment", "0"),
        ];

        for (param, value) in &rows {
            table
                .add_row(vec![(*param).into(), (*value).into()])
                .map_err(|e| CliError::ExecutionFailed(e.to_string()))?;
        }

        Ok(CliOutput::Table(table))
    }

    fn governor_pressure(&self) -> Result<CliOutput, CliError> {
        let text = "Network pressure: GoldenEra\n  \
                    Description: Golden era: moderate velocity, tight gold band, good liquidity"
            .to_string();
        Ok(CliOutput::Text(text))
    }

    fn governor_fee_caps(&self) -> Result<CliOutput, CliError> {
        let mut table = CliTable::new(vec!["Tier".into(), "Fee Cap".into(), "Percentage".into()]);

        let caps = [
            ("L0 (retail)", "0.05", "5%"),
            ("L1 (professional)", "0.02", "2%"),
            ("L2 (institutional)", "0.005", "0.5%"),
            ("L3 (sovereign)", "0.001", "0.1%"),
        ];

        for (tier, cap, pct) in &caps {
            table
                .add_row(vec![(*tier).into(), (*cap).into(), (*pct).into()])
                .map_err(|e| CliError::ExecutionFailed(e.to_string()))?;
        }

        Ok(CliOutput::Table(table))
    }

    // -----------------------------------------------------------------------
    // Oracle commands
    // -----------------------------------------------------------------------

    fn execute_oracle(&self, cmd: OracleCommand) -> Result<CliOutput, CliError> {
        match cmd {
            OracleCommand::Price => self.oracle_price(),
            OracleCommand::EffectiveRate => self.oracle_effective_rate(),
        }
    }

    fn oracle_price(&self) -> Result<CliOutput, CliError> {
        // Default from CaesarConfig: $2350/oz
        let text = "Gold price (spot):\n  \
                    USD per troy ounce: $2350.00\n  \
                    USD per gram:       $75.56\n  \
                    Grams per troy oz:  31.1035"
            .to_string();
        Ok(CliOutput::Text(text))
    }

    fn oracle_effective_rate(&self) -> Result<CliOutput, CliError> {
        // Default composite: all components at zero/neutral
        let mut table = CliTable::new(vec!["Component".into(), "Value".into()]);

        let rows = [
            ("Network fees component", "0.000"),
            ("Speculation pressure", "0.000"),
            ("Liquidity shadow", "0.000"),
            ("Effective rate (composite)", "1.000"),
        ];

        for (component, value) in &rows {
            table
                .add_row(vec![(*component).into(), (*value).into()])
                .map_err(|e| CliError::ExecutionFailed(e.to_string()))?;
        }

        Ok(CliOutput::Table(table))
    }

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    /// Register a node for testing. Not exposed via CLI commands yet.
    #[cfg(test)]
    fn register_node(&mut self, node_id: &str, settled: u64, earnings: f64) {
        self.nodes.insert(
            node_id.to_string(),
            NodeRecord {
                node_id: node_id.to_string(),
                settled_count: settled,
                fee_earnings_grams: earnings,
            },
        );
    }
}

impl Default for CommandExecutor {
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

    // -- Packet tests -------------------------------------------------------

    #[test]
    fn test_mint_and_info() {
        let mut exec = CommandExecutor::new();

        exec.execute(CliCommand::Packet(PacketCommand::Mint {
            sender: "alice".into(),
            recipient: "bob".into(),
            value_grams: 10.5,
            tier: "l0".into(),
        }))
        .expect("test: mint");

        let result = exec
            .execute(CliCommand::Packet(PacketCommand::Info {
                packet_id: "pkt-1".into(),
            }))
            .expect("test: info");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("pkt-1"));
                assert!(text.contains("alice"));
                assert!(text.contains("bob"));
                assert!(text.contains("10.500000"));
                assert!(text.contains("L0"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_mint_and_status() {
        let mut exec = CommandExecutor::new();

        exec.execute(CliCommand::Packet(PacketCommand::Mint {
            sender: "alice".into(),
            recipient: "bob".into(),
            value_grams: 5.0,
            tier: "l1".into(),
        }))
        .expect("test: mint");

        let result = exec
            .execute(CliCommand::Packet(PacketCommand::Status {
                packet_id: "pkt-1".into(),
            }))
            .expect("test: status");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("Minted"));
                assert!(text.contains("pkt-1"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_packet_list_empty() {
        let mut exec = CommandExecutor::new();
        let result = exec
            .execute(CliCommand::Packet(PacketCommand::List {
                state_filter: None,
            }))
            .expect("test: list empty");

        match result {
            CliOutput::Table(table) => {
                assert_eq!(table.row_count(), 0);
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_packet_list_with_filter() {
        let mut exec = CommandExecutor::new();

        // Mint two packets
        exec.execute(CliCommand::Packet(PacketCommand::Mint {
            sender: "a".into(),
            recipient: "b".into(),
            value_grams: 1.0,
            tier: "l0".into(),
        }))
        .expect("test: mint 1");

        exec.execute(CliCommand::Packet(PacketCommand::Mint {
            sender: "c".into(),
            recipient: "d".into(),
            value_grams: 2.0,
            tier: "l1".into(),
        }))
        .expect("test: mint 2");

        // List all
        let all = exec
            .execute(CliCommand::Packet(PacketCommand::List {
                state_filter: None,
            }))
            .expect("test: list all");
        match &all {
            CliOutput::Table(t) => assert_eq!(t.row_count(), 2),
            other => unreachable!("test: expected Table, got {:?}", other),
        }

        // Filter by "minted" (both are Minted)
        let filtered = exec
            .execute(CliCommand::Packet(PacketCommand::List {
                state_filter: Some("minted".into()),
            }))
            .expect("test: list filtered");
        match &filtered {
            CliOutput::Table(t) => assert_eq!(t.row_count(), 2),
            other => unreachable!("test: expected Table, got {:?}", other),
        }

        // Filter by "delivered" (none match)
        let empty = exec
            .execute(CliCommand::Packet(PacketCommand::List {
                state_filter: Some("delivered".into()),
            }))
            .expect("test: list delivered");
        match &empty {
            CliOutput::Table(t) => assert_eq!(t.row_count(), 0),
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_packet_not_found() {
        let exec = CommandExecutor::new();
        let result = exec.packet_info("nonexistent");
        assert!(matches!(result, Err(CliError::NotFound(_))));
    }

    #[test]
    fn test_mint_validation_empty_sender() {
        let mut exec = CommandExecutor::new();
        let result = exec.execute(CliCommand::Packet(PacketCommand::Mint {
            sender: "".into(),
            recipient: "bob".into(),
            value_grams: 1.0,
            tier: "l0".into(),
        }));
        assert!(matches!(result, Err(CliError::InvalidArgument(_))));
    }

    #[test]
    fn test_mint_validation_negative_value() {
        let mut exec = CommandExecutor::new();
        let result = exec.execute(CliCommand::Packet(PacketCommand::Mint {
            sender: "alice".into(),
            recipient: "bob".into(),
            value_grams: -5.0,
            tier: "l0".into(),
        }));
        assert!(matches!(result, Err(CliError::InvalidArgument(_))));
    }

    #[test]
    fn test_mint_validation_bad_tier() {
        let mut exec = CommandExecutor::new();
        let result = exec.execute(CliCommand::Packet(PacketCommand::Mint {
            sender: "alice".into(),
            recipient: "bob".into(),
            value_grams: 1.0,
            tier: "l9".into(),
        }));
        assert!(matches!(result, Err(CliError::InvalidArgument(_))));
    }

    // -- Node tests ---------------------------------------------------------

    #[test]
    fn test_node_status_not_found() {
        let exec = CommandExecutor::new();
        let result = exec.node_status("unknown-node");
        assert!(matches!(result, Err(CliError::NotFound(_))));
    }

    #[test]
    fn test_node_list_empty() {
        let mut exec = CommandExecutor::new();
        let result = exec
            .execute(CliCommand::Node(NodeCommand::List))
            .expect("test: list nodes");

        match result {
            CliOutput::Table(table) => {
                assert_eq!(table.row_count(), 0);
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_node_status_found() {
        let mut exec = CommandExecutor::new();
        exec.register_node("relay-1", 42, std::f64::consts::PI);

        let result = exec
            .execute(CliCommand::Node(NodeCommand::Status {
                node_id: "relay-1".into(),
            }))
            .expect("test: node status");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("relay-1"));
                assert!(text.contains("42"));
                assert!(text.contains("3.14"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_node_list_populated() {
        let mut exec = CommandExecutor::new();
        exec.register_node("node-a", 10, 1.5);
        exec.register_node("node-b", 20, 2.5);

        let result = exec
            .execute(CliCommand::Node(NodeCommand::List))
            .expect("test: list nodes");

        match result {
            CliOutput::Table(table) => {
                assert_eq!(table.row_count(), 2);
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_node_preferences_not_found() {
        let exec = CommandExecutor::new();
        let result = exec.node_preferences("unknown");
        assert!(matches!(result, Err(CliError::NotFound(_))));
    }

    #[test]
    fn test_node_preferences_found() {
        let mut exec = CommandExecutor::new();
        exec.register_node("relay-1", 0, 0.0);

        let result = exec
            .execute(CliCommand::Node(NodeCommand::Preferences {
                node_id: "relay-1".into(),
            }))
            .expect("test: preferences");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("relay-1"));
                assert!(text.contains("Max concurrent packets"));
                assert!(text.contains("L0, L1, L2, L3"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    // -- Governor tests -----------------------------------------------------

    #[test]
    fn test_governor_params() {
        let mut exec = CommandExecutor::new();
        let result = exec
            .execute(CliCommand::Governor(GovernorCommand::Params))
            .expect("test: governor params");

        match result {
            CliOutput::Table(table) => {
                assert_eq!(table.row_count(), 6);
                let rendered = format!("{table}");
                assert!(rendered.contains("Kp"));
                assert!(rendered.contains("0.5"));
                assert!(rendered.contains("Ki"));
                assert!(rendered.contains("0.1"));
                assert!(rendered.contains("Kd"));
                assert!(rendered.contains("0.05"));
                assert!(rendered.contains("GoldenEra"));
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_governor_pressure() {
        let mut exec = CommandExecutor::new();
        let result = exec
            .execute(CliCommand::Governor(GovernorCommand::Pressure))
            .expect("test: governor pressure");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("GoldenEra"));
                assert!(text.contains("Golden era"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_governor_fee_caps() {
        let mut exec = CommandExecutor::new();
        let result = exec
            .execute(CliCommand::Governor(GovernorCommand::FeeCaps))
            .expect("test: governor fee caps");

        match result {
            CliOutput::Table(table) => {
                assert_eq!(table.row_count(), 4);
                let rendered = format!("{table}");
                assert!(rendered.contains("L0"));
                assert!(rendered.contains("5%"));
                assert!(rendered.contains("L1"));
                assert!(rendered.contains("2%"));
                assert!(rendered.contains("L2"));
                assert!(rendered.contains("0.5%"));
                assert!(rendered.contains("L3"));
                assert!(rendered.contains("0.1%"));
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }

    // -- Oracle tests -------------------------------------------------------

    #[test]
    fn test_oracle_price() {
        let mut exec = CommandExecutor::new();
        let result = exec
            .execute(CliCommand::Oracle(OracleCommand::Price))
            .expect("test: oracle price");

        match result {
            CliOutput::Text(text) => {
                assert!(text.contains("Gold price"));
                assert!(text.contains("2350"));
                assert!(text.contains("31.1035"));
            }
            other => unreachable!("test: expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_oracle_effective_rate() {
        let mut exec = CommandExecutor::new();
        let result = exec
            .execute(CliCommand::Oracle(OracleCommand::EffectiveRate))
            .expect("test: oracle effective rate");

        match result {
            CliOutput::Table(table) => {
                assert_eq!(table.row_count(), 4);
                let rendered = format!("{table}");
                assert!(rendered.contains("Network fees"));
                assert!(rendered.contains("Speculation"));
                assert!(rendered.contains("Liquidity shadow"));
                assert!(rendered.contains("Effective rate"));
                assert!(rendered.contains("1.000"));
            }
            other => unreachable!("test: expected Table, got {:?}", other),
        }
    }
}
