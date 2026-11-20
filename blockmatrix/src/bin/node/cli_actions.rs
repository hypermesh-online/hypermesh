// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Gateway, sharing, and message CLI actions.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum GatewayAction {
    /// Transfer an asset between blockchain scopes (Device <-> Network)
    Transfer {
        /// Asset ID to transfer
        asset_id: String,
        /// Source scope (device or network)
        #[arg(long, default_value = "device")]
        from: String,
        /// Target scope (device or network)
        #[arg(long, default_value = "network")]
        to: String,
    },
    /// Get the status of a transfer by ID
    Status {
        /// Transfer ID (e.g., "gw-tx-1")
        transfer_id: String,
    },
    /// List all transfers
    List,
}

#[derive(Subcommand, Debug)]
pub enum ShareAction {
    /// Send a share invite for an asset to a peer
    Send {
        /// Asset ID to share
        asset_id: String,
        /// Recipient node ID or DNS name
        #[arg(long)]
        with: String,
    },
    /// List received share invites
    Inbox {
        /// Maximum number of invites to show
        #[arg(long, default_value = "50")]
        limit: u64,
    },
    /// Accept a received share invite
    Accept {
        /// Invite ID to accept
        invite_id: String,
    },
    /// Reject a received share invite
    Reject {
        /// Invite ID to reject
        invite_id: String,
    },
    /// Show this node's public keys (FALCON + Kyber)
    Pubkey,
    /// Look up a peer's public key by node ID
    PeerPubkey {
        /// Peer node ID
        node_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum MessageAction {
    /// Send a message to a peer
    Send {
        /// Recipient name or node ID
        #[arg(long)]
        to: String,
        /// Message body
        body: String,
        /// Content type
        #[arg(long, default_value = "text/plain")]
        content_type: String,
        /// Reply to message ID
        #[arg(long)]
        reply_to: Option<String>,
    },
    /// List received messages
    Inbox {
        /// Maximum number of messages to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Message history with a peer
    History {
        /// Peer name or node ID
        peer: String,
        /// Maximum number of messages to show
        #[arg(long, default_value = "50")]
        limit: usize,
    },
    /// Read a specific message
    Read {
        /// Message ID
        message_id: String,
    },
}
