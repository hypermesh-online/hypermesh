use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;
use std::time::SystemTime;

/// Service type enumeration for discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    /// Caesar wallet/exchange service
    Caesar,
    /// BlockMatrix coordination service
    BlockMatrix,
    /// TrustChain CA service
    TrustChain,
    /// Catalog VM service
    Catalog,
    /// STOQ transport service
    Stoq,
    /// HyperMesh dashboard
    HyperMesh,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ServiceType::Caesar => "caesar",
            ServiceType::BlockMatrix => "blockmatrix",
            ServiceType::TrustChain => "trustchain",
            ServiceType::Catalog => "catalog",
            ServiceType::Stoq => "stoq",
            ServiceType::HyperMesh => "hypermesh",
        };
        write!(f, "{s}")
    }
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service name
    pub name: String,

    /// IPv6 address
    pub address: Ipv6Addr,

    /// Port number
    pub port: u16,

    /// Optional server name for TLS
    pub server_name: Option<String>,

    /// Service metadata
    pub metadata: ServiceMetadata,

    /// Time when this record expires
    pub expires_at: SystemTime,
}

/// Service metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceMetadata {
    /// Service version
    pub version: Option<String>,

    /// Service capabilities
    pub capabilities: Vec<String>,

    /// Service priority (lower is higher priority)
    pub priority: u32,

    /// Service weight for load balancing
    pub weight: u32,

    /// Matrix position in Block-MATRIX topology
    pub matrix_position: Option<(u32, u32, u32)>,
}
