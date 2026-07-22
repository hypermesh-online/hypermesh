// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CLI argument structs for the BlockMatrix node binary.

use clap::{Parser, Subcommand};

use blockmatrix::bootstrap::PrivacyMode;
use blockmatrix::light_client::LightMode;

#[derive(Parser, Debug)]
#[clap(name = "blockmatrix-node")]
#[clap(about = "BlockMatrix node with unified TrustChain bootstrap")]
#[clap(version)]
pub struct Cli {
    /// Enable debug logging
    #[clap(short, long)]
    pub debug: bool,

    /// Matrix X coordinate OVERRIDE (joined-network placement).
    ///
    /// Device-auth invariant: a node's cell is DERIVED from its device
    /// identity by default (`MatrixCoordinate::derive_cell`). These flags are
    /// a validated override for joining a specific network position — NOT a
    /// free self-declaration. Leave at 0 (the default) to use the derived
    /// cell; set all three non-default to override.
    #[clap(short = 'x', long, default_value = "0")]
    pub coord_x: i64,

    /// Matrix Y coordinate OVERRIDE (see `--coord-x`).
    #[clap(short = 'y', long, default_value = "0")]
    pub coord_y: i64,

    /// Matrix Z coordinate OVERRIDE (see `--coord-x`).
    #[clap(short = 'z', long, default_value = "0")]
    pub coord_z: i64,

    /// Require validated hardware authentication (device-auth invariant).
    ///
    /// When set: (1) the composed device fingerprint must draw on at least
    /// two independent hardware sources or startup fails closed, and (2) the
    /// continuity gate hard-fails startup if the live device fingerprint does
    /// not match the one recorded in the genesis block — this is what rejects
    /// a copied identity directory run on a different machine.
    ///
    /// The device fingerprint is ALWAYS captured and recorded at genesis
    /// regardless of this flag; only enforcement is gated.
    #[clap(long)]
    pub require_hardware_auth: bool,

    /// Initial privacy mode
    #[clap(short, long, default_value = "private")]
    pub privacy: PrivacyModeArg,

    /// Bootstrap nodes (IPv6 addresses)
    #[clap(short = 'b', long)]
    pub bootstrap: Vec<String>,

    /// STOQ port
    #[clap(short = 's', long, default_value = "9292")]
    pub stoq_port: u16,

    /// Run as a reflector (public peer that accepts and relays)
    #[clap(long)]
    pub reflector: bool,

    /// Network ID for multi-node sync (nodes with the same ID sync blocks)
    #[arg(long, default_value = "trustnet-test")]
    pub network_id: String,

    /// Data directory for blockchain persistence
    #[clap(long, default_value = "~/.blockmatrix")]
    pub data_dir: String,

    /// DNS name to register for this node at boot (e.g., "trust", "persist")
    #[clap(long, env = "HYPERMESH_NAME")]
    pub name: Option<String>,

    /// Phase K.1 — runtime mode selector.
    ///
    /// - `full` (default): full block hosting + shard storage + asset pipeline
    /// - `light`: header-only sync, no shard or pipeline state (~256MB RAM)
    /// - `thin`: reserved for K.2 — no local chain, remote daemon via SDK
    ///
    /// K.1 ships the flag and the `HeaderSyncManager` scaffolding. Full
    /// startup-path minimization (skipping `ShardStore`, `PipelineEngine`,
    /// Caesar, ngauge, etc.) is staged as K.1.5.
    #[clap(long, default_value = "full")]
    pub mode: LightModeArg,

    /// Output in JSON format
    #[clap(long, global = true)]
    pub json: bool,

    /// Path to config file
    #[clap(long, global = true)]
    pub config: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum PrivacyModeArg {
    Private,
    Anonymous,
    P2P,
    Public,
}

/// Phase K.1 — light-mode CLI selector.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum LightModeArg {
    /// Full block hosting + shard storage + asset pipeline (default).
    Full,
    /// Header-only sync, no shard hosting, no asset pipeline writes.
    Light,
    /// No local chain — connect to a remote daemon over capability-token SDK.
    /// Reserved for K.2.
    Thin,
}

impl From<LightModeArg> for LightMode {
    fn from(arg: LightModeArg) -> Self {
        match arg {
            LightModeArg::Full => LightMode::Full,
            LightModeArg::Light => LightMode::Light,
            LightModeArg::Thin => LightMode::ThinClient,
        }
    }
}

impl From<PrivacyModeArg> for PrivacyMode {
    fn from(arg: PrivacyModeArg) -> Self {
        match arg {
            PrivacyModeArg::Private => PrivacyMode::PRIVATE,
            PrivacyModeArg::Anonymous => PrivacyMode::ANONYMOUS,
            PrivacyModeArg::P2P => PrivacyMode::PRIVATE,
            PrivacyModeArg::Public => PrivacyMode::PUBLIC,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Connect to the mesh (starts daemon if not running)
    Connect {
        /// Privacy mode for network participation
        #[clap(value_enum, default_value = "public")]
        privacy: PrivacyModeArg,
        /// Run in foreground (don't background)
        #[clap(long)]
        foreground: bool,
    },

    /// Disconnect from the mesh (stop daemon)
    Disconnect,

    /// Ping a remote HyperMesh node (STOQ handshake + RTT)
    Ping {
        /// Target address (IPv4, IPv6, or hostname with optional :port, default port 9292)
        target: String,
        /// Number of pings
        #[clap(short = 'c', long, default_value = "1")]
        count: u32,
    },

    /// [DEPRECATED] Use 'connect' instead
    #[clap(hide = true)]
    Start,

    /// Show node status
    Status,

    /// Transition to different privacy mode
    SetPrivacy {
        #[clap(value_enum)]
        mode: PrivacyModeArg,
    },

    /// Store a file as a distributed asset
    Store {
        /// Path to the file to store
        path: std::path::PathBuf,
    },

    /// Fetch a distributed asset
    Fetch {
        /// Asset ID to fetch
        asset_id: String,
        /// Output path (default: stdout)
        #[clap(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// DNS operations -- register, resolve, or list names
    Dns {
        #[clap(subcommand)]
        action: DnsAction,
    },

    /// Domain operations -- register, create sub-domains, invite peers
    Domain {
        #[clap(subcommand)]
        action: DomainAction,
    },

    /// Join a domain's network
    Join {
        /// Domain name (e.g., "home.persist.hypermesh")
        network: String,
        /// Invitation token (required for private domains)
        #[clap(long)]
        invite: Option<String>,
    },

    /// Manage configuration
    Config {
        #[clap(subcommand)]
        action: ConfigCommand,
    },

    /// Dashboard operations
    Dashboard {
        #[clap(subcommand)]
        action: DashboardAction,
    },

    /// Caesar EVP operations
    Caesar {
        #[clap(subcommand)]
        action: CaesarAction,
    },

    /// TrustChain CA operations
    Trustchain {
        #[clap(subcommand)]
        action: TrustchainAction,
    },

    /// NGauge analytics operations
    NGauge {
        #[clap(subcommand)]
        action: NGaugeAction,
    },

    /// Catalog registry operations
    Catalog {
        #[clap(subcommand)]
        action: CatalogAction,
    },

    /// Cross-scope asset transfer operations (Device <-> Network)
    Gateway {
        #[clap(subcommand)]
        action: GatewayAction,
    },

    /// Share an asset with a peer
    Share {
        #[clap(subcommand)]
        action: ShareAction,
    },

    /// Private messaging
    Message {
        #[clap(subcommand)]
        action: MessageAction,
    },

    /// Destroy all node data (blockchain, identity, shards, config)
    Destroy {
        /// Skip confirmation prompt
        #[clap(long)]
        chaotic: bool,
    },

    /// Phase J.1 — apply a foundation-published update.
    ///
    /// Validates that a release-feed entry exists for the requested
    /// version + channel and returns the upgrade plan (binary hash to
    /// expect, release-notes URL). Actual binary swap is gated behind
    /// foundation-pubkey opt-in and is deferred to a follow-up.
    Update {
        /// Release channel (stable, beta, nightly).
        #[clap(long, default_value = "stable")]
        channel: String,
        /// Specific version to apply. If omitted, uses the latest
        /// available entry on the channel.
        #[clap(long)]
        version: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Get a config value by dotted key path
    Get {
        /// Dotted key path (e.g. "network.stoq_port")
        key: String,
    },
    /// Set a config value
    Set {
        /// Dotted key path (e.g. "network.stoq_port")
        key: String,
        /// Value to set (parsed as JSON if possible, otherwise string)
        value: String,
    },
    /// Initialize default config file
    Init,
}

#[derive(Subcommand, Debug)]
pub enum DashboardAction {
    /// Deploy a dashboard from a directory containing dashboard.toml
    Deploy {
        /// Path to directory containing dashboard.toml
        path: std::path::PathBuf,
    },
    /// List registered dashboards
    List,
    /// Show dashboard info
    Info {
        /// Dashboard name
        name: String,
    },
    /// Initialize a new dashboard project (scaffold)
    Init {
        /// Project name (default: my-dashboard)
        name: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum DomainAction {
    /// Register a new top-level domain on this node's blockchain
    Register {
        /// Domain name (e.g., "hypermesh")
        name: String,
        /// Privacy mode for the domain network
        #[clap(long, value_enum, default_value = "private")]
        privacy: PrivacyModeArg,
    },
    /// Create a sub-domain under an existing domain
    Create {
        /// Sub-domain name (e.g., "home.hypermesh")
        name: String,
        /// Privacy mode for the sub-domain network
        #[clap(long, value_enum, default_value = "private")]
        privacy: PrivacyModeArg,
    },
    /// List all registered domains
    List,
    /// Show nodes in a domain's network
    Nodes {
        /// Domain name to query
        domain: String,
    },
    /// Create an invitation token for a domain
    Invite {
        /// Domain name to invite to
        domain: String,
        /// Target peer node ID (or "open" for any node)
        #[clap(long)]
        peer: String,
        /// Invitation validity in seconds
        #[clap(long, default_value = "3600")]
        ttl: u64,
    },
}

#[derive(Subcommand, Debug)]
pub enum DnsAction {
    /// Register a DNS name for this node (writes to local blockchain)
    Register {
        /// Domain name to register (e.g., "persist")
        name: String,
        /// IPv6 address to point to (default: this node's address)
        #[clap(short, long)]
        addr: Option<String>,
    },
    /// Resolve a DNS name
    Resolve {
        /// Domain name to resolve
        name: String,
    },
    /// List all registered DNS names
    List,
}

#[derive(Subcommand, Debug)]
pub enum CaesarAction {
    /// Show wallet info
    Wallet,
    /// Show balance
    Balance,
    /// List recent transactions
    Transactions {
        /// Maximum number of transactions
        #[clap(long, default_value = "10")]
        limit: u32,
    },
    /// Show reward earnings
    Rewards,
    /// Route a payment packet
    Route {
        /// Destination node
        destination: String,
        /// Amount in gold grams
        amount: f64,
    },
    /// Show governor parameters
    Governor,
}

#[derive(Subcommand, Debug)]
pub enum TrustchainAction {
    /// List certificates
    Certs,
    /// Issue a new certificate
    Issue {
        /// Certificate subject
        subject: String,
        /// Scope (anonymous, private, public)
        #[clap(long, default_value = "private")]
        scope: String,
    },
    /// Validate a certificate
    Validate {
        /// Path to PEM certificate file
        cert_path: String,
    },
    /// Revoke a certificate
    Revoke {
        /// Certificate ID
        cert_id: String,
    },
    /// List DNS zones
    Zones,
}

#[derive(Subcommand, Debug)]
pub enum NGaugeAction {
    /// Show capacity metrics
    Capacity,
    /// Show traffic analysis
    Traffic,
    /// List marketplace offerings
    Marketplace,
    /// Show node metrics
    Metrics,
    /// List active leases
    Leases,
}

#[derive(Subcommand, Debug)]
pub enum CatalogAction {
    /// Browse packages
    Browse {
        /// Search query
        #[clap(long)]
        query: Option<String>,
        /// Page number
        #[clap(long, default_value = "1")]
        page: u32,
    },
    /// Search packages
    Search {
        /// Search query
        query: String,
    },
    /// Get package info
    Info {
        /// Package name
        name: String,
    },
    /// Show registry statistics
    Stats,
}

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
