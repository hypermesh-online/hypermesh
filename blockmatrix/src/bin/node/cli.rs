// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CLI argument structs for the BlockMatrix node binary.

use clap::{Parser, Subcommand};

use blockmatrix::bootstrap::PrivacyMode;

#[derive(Parser, Debug)]
#[clap(name = "blockmatrix-node")]
#[clap(about = "BlockMatrix node with unified TrustChain bootstrap")]
#[clap(version)]
pub struct Cli {
    /// Enable debug logging
    #[clap(short, long)]
    pub debug: bool,

    /// Node X coordinate in matrix
    #[clap(short = 'x', long, default_value = "0")]
    pub coord_x: i64,

    /// Node Y coordinate in matrix
    #[clap(short = 'y', long, default_value = "0")]
    pub coord_y: i64,

    /// Node Z coordinate in matrix
    #[clap(short = 'z', long, default_value = "0")]
    pub coord_z: i64,

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
    #[arg(long, default_value = "public-hypermesh-alpha")]
    pub network_id: String,

    /// Data directory for blockchain persistence
    #[clap(long, default_value = "~/.blockmatrix")]
    pub data_dir: String,

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

    /// Engauge analytics operations
    Engauge {
        #[clap(subcommand)]
        action: EngaugeAction,
    },

    /// Catalog registry operations
    Catalog {
        #[clap(subcommand)]
        action: CatalogAction,
    },

    /// Destroy all node data (blockchain, identity, shards, config)
    Destroy {
        /// Skip confirmation prompt
        #[clap(long)]
        chaotic: bool,
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
pub enum EngaugeAction {
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
