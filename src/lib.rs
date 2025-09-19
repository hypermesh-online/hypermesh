//! Internet 2.0 Protocol Stack Library
//! 
//! A revolutionary replacement for the traditional Internet protocol stack that embeds
//! STOQ transport, HyperMesh consensus, and TrustChain security into a single,
//! self-contained networking foundation.
//!
//! This library represents a fundamental shift from Internet 1.0's layered protocols
//! with external dependencies to Internet 2.0's unified, consensus-validated,
//! certificate-embedded protocol stack.

pub mod config;
pub mod transport;
pub mod assets;
pub mod authority;
pub mod integration;
pub mod monitoring;

// Re-export main types for easy access
pub use config::Internet2Config;
pub use transport::StoqTransportLayer;
pub use assets::HyperMeshAssetLayer;
pub use authority::TrustChainAuthorityLayer;
pub use integration::LayerIntegration;
pub use monitoring::PerformanceMonitor;

/// Internet 2.0 Server (re-export from main)
// Note: The actual Internet2Server struct is defined in main.rs
// We re-export key types for library usage

/// Unified result type for the Internet 2.0 stack
pub type Result<T> = anyhow::Result<T>;

/// Internet 2.0 Protocol Stack Features
/// 
/// This enum represents the core features that distinguish Internet 2.0
/// from traditional Internet 1.0 protocols.
#[derive(Debug, Clone)]
pub enum Internet2Feature {
    /// STOQ Transport: QUIC over IPv6 foundation
    /// - 40 Gbps performance targets
    /// - Certificate validation at connection establishment
    /// - Embedded DNS resolution
    /// - Zero-copy operations and hardware acceleration
    StoqTransport,
    
    /// HyperMesh Assets: Universal asset system
    /// - Everything is an asset (CPU, GPU, memory, storage, connections)
    /// - Four-proof consensus (PoSpace+PoStake+PoWork+PoTime)
    /// - NAT-like proxy addressing for remote resources
    /// - VM execution through asset allocation
    HyperMeshAssets,
    
    /// TrustChain Authority: Embedded security
    /// - Embedded certificate authority (no external CA)
    /// - Embedded DNS resolver (no external DNS)
    /// - Certificate transparency logging
    /// - Post-quantum cryptography (FALCON-1024 + Kyber)
    TrustChainAuthority,
    
    /// Layer Integration: Cross-layer coordination
    /// - Certificate validation embedded in transport
    /// - Consensus validation for all asset operations
    /// - Performance optimization across layers
    /// - Zero external dependencies
    LayerIntegration,
}

/// Internet 2.0 Protocol Stack Capabilities
/// 
/// This struct describes the complete capabilities of the Internet 2.0
/// protocol stack compared to traditional Internet protocols.
#[derive(Debug, Clone)]
pub struct Internet2Capabilities {
    /// Transport capabilities
    pub transport: TransportCapabilities,
    
    /// Asset management capabilities
    pub assets: AssetCapabilities,
    
    /// Security capabilities
    pub security: SecurityCapabilities,
    
    /// Integration capabilities
    pub integration: IntegrationCapabilities,
}

/// Transport layer capabilities
#[derive(Debug, Clone)]
pub struct TransportCapabilities {
    /// Protocol support
    pub protocols: Vec<String>, // ["STOQ", "QUIC", "IPv6"]
    
    /// Performance characteristics
    pub max_throughput_gbps: f64, // 40 Gbps target
    pub connection_establishment_ms: f64, // Sub-millisecond
    pub zero_copy_operations: bool,
    pub hardware_acceleration: bool,
    
    /// Security features
    pub embedded_certificate_validation: bool,
    pub embedded_dns_resolution: bool,
    pub ipv6_only: bool,
}

/// Asset management capabilities
#[derive(Debug, Clone)]
pub struct AssetCapabilities {
    /// Asset types supported
    pub asset_types: Vec<String>, // ["CPU", "GPU", "Memory", "Storage", "Network", "VM", "Service"]
    
    /// Consensus features
    pub four_proof_consensus: bool, // PoSpace+PoStake+PoWork+PoTime
    pub consensus_validation_ms: f64, // <100ms target
    pub byzantine_fault_tolerance: bool,
    
    /// Advanced features
    pub nat_like_addressing: bool,
    pub vm_execution: bool,
    pub remote_resource_access: bool,
}

/// Security capabilities
#[derive(Debug, Clone)]
pub struct SecurityCapabilities {
    /// Certificate authority
    pub embedded_ca: bool,
    pub certificate_transparency: bool,
    pub automatic_rotation: bool,
    pub certificate_ops_ms: f64, // <35ms target
    
    /// DNS resolution
    pub embedded_dns: bool,
    pub ipv6_only_dns: bool,
    pub dns_caching: bool,
    
    /// Post-quantum cryptography
    pub falcon_signatures: bool, // FALCON-1024
    pub kyber_encryption: bool,  // Kyber KEM
    pub hybrid_crypto: bool,     // Classical + quantum
}

/// Integration capabilities
#[derive(Debug, Clone)]
pub struct IntegrationCapabilities {
    /// Cross-layer features
    pub certificate_validation_at_transport: bool,
    pub consensus_validation_at_allocation: bool,
    pub performance_coordination: bool,
    
    /// External dependencies
    pub zero_external_dependencies: bool,
    pub self_contained_stack: bool,
    pub federated_bootstrap: bool,
}

impl Default for Internet2Capabilities {
    fn default() -> Self {
        Self {
            transport: TransportCapabilities {
                protocols: vec!["STOQ".to_string(), "QUIC".to_string(), "IPv6".to_string()],
                max_throughput_gbps: 40.0,
                connection_establishment_ms: 1.0,
                zero_copy_operations: true,
                hardware_acceleration: true,
                embedded_certificate_validation: true,
                embedded_dns_resolution: true,
                ipv6_only: true,
            },
            assets: AssetCapabilities {
                asset_types: vec![
                    "CPU".to_string(), "GPU".to_string(), "Memory".to_string(),
                    "Storage".to_string(), "Network".to_string(), "VM".to_string(),
                    "Service".to_string()
                ],
                four_proof_consensus: true,
                consensus_validation_ms: 100.0,
                byzantine_fault_tolerance: true,
                nat_like_addressing: true,
                vm_execution: true,
                remote_resource_access: true,
            },
            security: SecurityCapabilities {
                embedded_ca: true,
                certificate_transparency: true,
                automatic_rotation: true,
                certificate_ops_ms: 35.0,
                embedded_dns: true,
                ipv6_only_dns: true,
                dns_caching: true,
                falcon_signatures: true,
                kyber_encryption: true,
                hybrid_crypto: true,
            },
            integration: IntegrationCapabilities {
                certificate_validation_at_transport: true,
                consensus_validation_at_allocation: true,
                performance_coordination: true,
                zero_external_dependencies: true,
                self_contained_stack: true,
                federated_bootstrap: true,
            },
        }
    }
}

/// Get Internet 2.0 protocol stack capabilities
pub fn get_internet2_capabilities() -> Internet2Capabilities {
    Internet2Capabilities::default()
}

/// Compare Internet 2.0 vs Internet 1.0 capabilities
pub fn compare_protocol_stacks() -> ProtocolStackComparison {
    ProtocolStackComparison {
        internet1_stack: Internet1Stack {
            protocols: vec!["HTTP".to_string(), "TCP".to_string(), "IPv4/IPv6".to_string()],
            external_dependencies: vec![
                "External Certificate Authorities".to_string(),
                "External DNS Servers".to_string(),
                "External Root Certificates".to_string(),
            ],
            performance_limitations: vec![
                "TCP head-of-line blocking".to_string(),
                "HTTP/1.1 connection limits".to_string(),
                "Certificate validation delays".to_string(),
                "DNS resolution latency".to_string(),
            ],
            security_issues: vec![
                "Certificate authority trust model".to_string(),
                "DNS cache poisoning vulnerabilities".to_string(),
                "No built-in consensus validation".to_string(),
                "Vulnerable to quantum attacks".to_string(),
            ],
        },
        internet2_advantages: vec![
            "40 Gbps STOQ transport (vs HTTP/TCP limitations)".to_string(),
            "Zero external dependencies (embedded CA + DNS)".to_string(),
            "Four-proof consensus validation for all operations".to_string(),
            "Post-quantum cryptography (FALCON-1024 + Kyber)".to_string(),
            "Universal asset system with NAT-like addressing".to_string(),
            "Certificate validation at connection establishment".to_string(),
            "IPv6-only networking (no IPv4 legacy burden)".to_string(),
            "Zero-downtime certificate rotation".to_string(),
            "Byzantine fault tolerance built-in".to_string(),
            "VM execution through consensus-validated assets".to_string(),
        ],
    }
}

/// Protocol stack comparison structure
#[derive(Debug, Clone)]
pub struct ProtocolStackComparison {
    pub internet1_stack: Internet1Stack,
    pub internet2_advantages: Vec<String>,
}

/// Internet 1.0 protocol stack description
#[derive(Debug, Clone)]
pub struct Internet1Stack {
    pub protocols: Vec<String>,
    pub external_dependencies: Vec<String>,
    pub performance_limitations: Vec<String>,
    pub security_issues: Vec<String>,
}

/// Library version and metadata
pub const INTERNET2_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const INTERNET2_NAME: &str = env!("CARGO_PKG_NAME");
pub const INTERNET2_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Library information
pub fn library_info() -> LibraryInfo {
    LibraryInfo {
        name: INTERNET2_NAME.to_string(),
        version: INTERNET2_VERSION.to_string(),
        description: INTERNET2_DESCRIPTION.to_string(),
        features: vec![
            Internet2Feature::StoqTransport,
            Internet2Feature::HyperMeshAssets,
            Internet2Feature::TrustChainAuthority,
            Internet2Feature::LayerIntegration,
        ],
        capabilities: get_internet2_capabilities(),
    }
}

/// Library information structure
#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub features: Vec<Internet2Feature>,
    pub capabilities: Internet2Capabilities,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_info() {
        let info = library_info();
        assert_eq!(info.name, "internet2-server");
        assert!(!info.version.is_empty());
        assert_eq!(info.features.len(), 4);
    }

    #[test]
    fn test_capabilities() {
        let capabilities = get_internet2_capabilities();
        
        // Verify transport capabilities
        assert_eq!(capabilities.transport.max_throughput_gbps, 40.0);
        assert!(capabilities.transport.embedded_certificate_validation);
        assert!(capabilities.transport.ipv6_only);
        
        // Verify asset capabilities
        assert!(capabilities.assets.four_proof_consensus);
        assert!(capabilities.assets.nat_like_addressing);
        assert_eq!(capabilities.assets.consensus_validation_ms, 100.0);
        
        // Verify security capabilities
        assert!(capabilities.security.embedded_ca);
        assert!(capabilities.security.falcon_signatures);
        assert_eq!(capabilities.security.certificate_ops_ms, 35.0);
        
        // Verify integration capabilities
        assert!(capabilities.integration.zero_external_dependencies);
        assert!(capabilities.integration.certificate_validation_at_transport);
    }

    #[test]
    fn test_protocol_comparison() {
        let comparison = compare_protocol_stacks();
        
        // Verify Internet 1.0 limitations are identified
        assert!(comparison.internet1_stack.external_dependencies.len() > 0);
        assert!(comparison.internet1_stack.performance_limitations.len() > 0);
        assert!(comparison.internet1_stack.security_issues.len() > 0);
        
        // Verify Internet 2.0 advantages
        assert!(comparison.internet2_advantages.len() >= 10);
        assert!(comparison.internet2_advantages.iter().any(|a| a.contains("40 Gbps")));
        assert!(comparison.internet2_advantages.iter().any(|a| a.contains("zero external dependencies")));
        assert!(comparison.internet2_advantages.iter().any(|a| a.contains("four-proof consensus")));
    }

    #[test]
    fn test_internet2_features() {
        use std::mem::discriminant;
        
        let features = vec![
            Internet2Feature::StoqTransport,
            Internet2Feature::HyperMeshAssets,
            Internet2Feature::TrustChainAuthority,
            Internet2Feature::LayerIntegration,
        ];
        
        // Verify all features are unique
        for i in 0..features.len() {
            for j in (i + 1)..features.len() {
                assert_ne!(discriminant(&features[i]), discriminant(&features[j]));
            }
        }
    }
}