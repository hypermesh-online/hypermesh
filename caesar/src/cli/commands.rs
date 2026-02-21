// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CLI command types for Caesar EVP operations
//!
//! These are pure data types representing parsed CLI commands. No framework
//! dependency (e.g., clap) is used -- a future binary crate will parse CLI
//! arguments and convert them into these types.

// ---------------------------------------------------------------------------
// Top-level command
// ---------------------------------------------------------------------------

/// Top-level CLI command dispatched to sub-command handlers.
#[derive(Debug, Clone, PartialEq)]
pub enum CliCommand {
    /// Packet operations (mint, info, status, list).
    Packet(PacketCommand),
    /// Node operations (status, list, preferences).
    Node(NodeCommand),
    /// Governor queries (params, pressure, fee caps).
    Governor(GovernorCommand),
    /// Oracle queries (gold price, effective rate).
    Oracle(OracleCommand),
}

// ---------------------------------------------------------------------------
// Packet commands
// ---------------------------------------------------------------------------

/// Commands for managing EVP packets.
#[derive(Debug, Clone, PartialEq)]
pub enum PacketCommand {
    /// Show packet details by ID.
    Info { packet_id: String },
    /// Show packet state by ID.
    Status { packet_id: String },
    /// List packets, optionally filtered by state.
    List { state_filter: Option<String> },
    /// Mint a new packet (for testing/demonstration).
    Mint {
        sender: String,
        recipient: String,
        value_grams: f64,
        tier: String,
    },
}

// ---------------------------------------------------------------------------
// Node commands
// ---------------------------------------------------------------------------

/// Commands for querying node status within the Caesar network.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeCommand {
    /// Show node status (settled count, fee earnings).
    Status { node_id: String },
    /// List registered nodes.
    List,
    /// Show operator preferences for a node.
    Preferences { node_id: String },
}

// ---------------------------------------------------------------------------
// Governor commands
// ---------------------------------------------------------------------------

/// Commands for querying the Governor PID controller state.
#[derive(Debug, Clone, PartialEq)]
pub enum GovernorCommand {
    /// Show current governance parameters (PID gains, health, fee adjustment).
    Params,
    /// Show current network pressure classification.
    Pressure,
    /// Show constitutional fee caps per tier.
    FeeCaps,
}

// ---------------------------------------------------------------------------
// Oracle commands
// ---------------------------------------------------------------------------

/// Commands for querying the gold oracle.
#[derive(Debug, Clone, PartialEq)]
pub enum OracleCommand {
    /// Show current gold spot price.
    Price,
    /// Show effective rate composite.
    EffectiveRate,
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Parse a market tier string (case-insensitive).
///
/// Accepts `"l0"`, `"l1"`, `"l2"`, `"l3"`.
///
/// # Errors
///
/// Returns an error message if the string does not match a known tier.
pub fn parse_tier(s: &str) -> Result<String, String> {
    match s.to_lowercase().as_str() {
        "l0" => Ok("L0".into()),
        "l1" => Ok("L1".into()),
        "l2" => Ok("L2".into()),
        "l3" => Ok("L3".into()),
        other => Err(format!(
            "Unknown market tier '{}'. Expected 'l0', 'l1', 'l2', or 'l3'.",
            other,
        )),
    }
}

/// Parse a packet state string (case-insensitive).
///
/// Accepts the canonical packet states: `"minted"`, `"in_transit"`,
/// `"delivered"`, `"settling"`, `"settled"`, `"expired"`, `"refunded"`,
/// `"held"`, `"stalled"`, `"dispersed"`, `"dissolved"`.
///
/// # Errors
///
/// Returns an error message if the string does not match a known state.
pub fn parse_packet_state(s: &str) -> Result<String, String> {
    match s.to_lowercase().as_str() {
        "minted" => Ok("Minted".into()),
        "in_transit" | "intransit" => Ok("InTransit".into()),
        "delivered" => Ok("Delivered".into()),
        "settling" => Ok("Settling".into()),
        "settled" => Ok("Settled".into()),
        "expired" => Ok("Expired".into()),
        "refunded" => Ok("Refunded".into()),
        "held" => Ok("Held".into()),
        "stalled" => Ok("Stalled".into()),
        "dispersed" => Ok("Dispersed".into()),
        "dissolved" => Ok("Dissolved".into()),
        other => Err(format!(
            "Unknown packet state '{}'. Expected one of: minted, in_transit, delivered, \
             settling, settled, expired, refunded, held, stalled, dispersed, dissolved.",
            other,
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
    fn test_parse_tier_valid() {
        assert_eq!(parse_tier("l0"), Ok("L0".into()));
        assert_eq!(parse_tier("L1"), Ok("L1".into()));
        assert_eq!(parse_tier("L2"), Ok("L2".into()));
        assert_eq!(parse_tier("l3"), Ok("L3".into()));
    }

    #[test]
    fn test_parse_tier_invalid() {
        let err = parse_tier("l4").unwrap_err();
        assert!(err.contains("Unknown market tier"));
        assert!(err.contains("l4"));
    }

    #[test]
    fn test_parse_packet_state_valid() {
        assert_eq!(parse_packet_state("minted"), Ok("Minted".into()));
        assert_eq!(parse_packet_state("in_transit"), Ok("InTransit".into()));
        assert_eq!(parse_packet_state("DELIVERED"), Ok("Delivered".into()));
        assert_eq!(parse_packet_state("settling"), Ok("Settling".into()));
        assert_eq!(parse_packet_state("Settled"), Ok("Settled".into()));
        assert_eq!(parse_packet_state("expired"), Ok("Expired".into()));
        assert_eq!(parse_packet_state("refunded"), Ok("Refunded".into()));
        assert_eq!(parse_packet_state("held"), Ok("Held".into()));
        assert_eq!(parse_packet_state("stalled"), Ok("Stalled".into()));
        assert_eq!(parse_packet_state("dispersed"), Ok("Dispersed".into()));
        assert_eq!(parse_packet_state("dissolved"), Ok("Dissolved".into()));
    }

    #[test]
    fn test_parse_packet_state_intransit_alias() {
        assert_eq!(parse_packet_state("intransit"), Ok("InTransit".into()));
    }

    #[test]
    fn test_parse_packet_state_invalid() {
        let err = parse_packet_state("pending").unwrap_err();
        assert!(err.contains("Unknown packet state"));
        assert!(err.contains("pending"));
    }

    #[test]
    fn test_cli_command_variants_debug() {
        let packet = CliCommand::Packet(PacketCommand::Info {
            packet_id: "pkt-1".into(),
        });
        let node = CliCommand::Node(NodeCommand::Status {
            node_id: "n1".into(),
        });
        let gov = CliCommand::Governor(GovernorCommand::Params);
        let oracle = CliCommand::Oracle(OracleCommand::Price);

        // Verify Debug works (no panic)
        let _ = format!("{:?}", packet);
        let _ = format!("{:?}", node);
        let _ = format!("{:?}", gov);
        let _ = format!("{:?}", oracle);
    }

    #[test]
    fn test_cli_command_partial_eq() {
        let a = CliCommand::Governor(GovernorCommand::Params);
        let b = CliCommand::Governor(GovernorCommand::Params);
        let c = CliCommand::Governor(GovernorCommand::Pressure);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_packet_mint_equality() {
        let a = PacketCommand::Mint {
            sender: "alice".into(),
            recipient: "bob".into(),
            value_grams: 10.0,
            tier: "L0".into(),
        };
        let b = PacketCommand::Mint {
            sender: "alice".into(),
            recipient: "bob".into(),
            value_grams: 10.0,
            tier: "L0".into(),
        };
        assert_eq!(a, b);
    }
}
