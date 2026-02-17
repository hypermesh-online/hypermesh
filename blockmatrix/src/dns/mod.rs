// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS-as-Asset System - Multi-Tier Resolution
//!
//! Sprint 3.3: DNS registration with Proof of State validation, blockchain-backed
//! multi-tier DNS resolution (P2P direct, Public, Federated, Fully Federated).
//!
//! Architecture:
//! - TrustChain provides DNS|CA|CT service layer (similar to how UDP provides DNS transport)
//! - STOQ provides transport layer (QUIC over IPv6)
//! - BlockMatrix provides DNS-as-Asset orchestration with multi-tier resolution
//!
//! DNS Resolution Tiers:
//! 1. P2P Direct: http3://peer-id → Direct connection, no DNS
//! 2. Public DNS: http3://nike → Blockchain query, global pool
//! 3. Federated Private: http3://admin.nike → Network-scoped pool
//! 4. Fully Federated: http3://classified.gov → Zero public access

pub mod records;
pub mod pools;
pub mod resolver;
pub mod registration;
pub mod validation;
pub mod cache;
pub mod trustchain;

// Re-export public API
pub use records::{DnsRecord, DnsRecordType, DnsRecordData};
pub use pools::{DnsPoolManager, DnsPool, DnsPoolType, PoolVisibility};
pub use resolver::{DnsResolver, DnsResolutionTier, DnsQuery, DnsResponse};
pub use registration::{DnsRegistrar, DnsRegistration, RegistrationStatus};
pub use validation::{DnsValidator, ValidationResult};
pub use cache::{DnsCache, CacheEntry};
pub use trustchain::{TrustChainDnsService, TrustChainDnsClient};

use anyhow::Result;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use std::net::Ipv6Addr;

/// DNS-as-Asset error types
#[derive(Debug, Error)]
pub enum DnsError {
    #[error("Domain not found: {domain}")]
    DomainNotFound { domain: String },

    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },

    #[error("Validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Pool not found: {pool_id}")]
    PoolNotFound { pool_id: String },

    #[error("Registration failed: {reason}")]
    RegistrationFailed { reason: String },

    #[error("TrustChain service error: {0}")]
    TrustChainError(String),

    #[error("Blockchain error: {0}")]
    BlockchainError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Invalid domain format: {domain}")]
    InvalidDomain { domain: String },

    #[error("Privacy boundary violation: {reason}")]
    PrivacyViolation { reason: String },
}

pub type DnsResult<T> = Result<T, DnsError>;

/// Domain format (no-TLD)
/// Examples: http3://nike, http3://admin.nike, http3://classified.gov
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Domain {
    /// Full domain string
    pub full: String,
    /// Root domain (e.g., "nike", "gov")
    pub root: String,
    /// Subdomain parts (e.g., ["admin"], ["classified"])
    pub subdomains: Vec<String>,
}

impl Domain {
    /// Parse domain from string
    pub fn parse(domain: &str) -> DnsResult<Self> {
        // Remove http3:// prefix if present
        let domain = domain.trim_start_matches("http3://");

        if domain.is_empty() {
            return Err(DnsError::InvalidDomain {
                domain: domain.to_string(),
            });
        }

        let parts: Vec<&str> = domain.split('.').collect();
        if parts.is_empty() {
            return Err(DnsError::InvalidDomain {
                domain: domain.to_string(),
            });
        }

        let root = parts.last().unwrap().to_string();
        let subdomains: Vec<String> = parts[..parts.len() - 1]
            .iter()
            .map(|s| s.to_string())
            .collect();

        Ok(Self {
            full: domain.to_string(),
            root,
            subdomains,
        })
    }

    /// Check if domain is public (no subdomains)
    pub fn is_public(&self) -> bool {
        self.subdomains.is_empty()
    }

    /// Check if domain is federated (has subdomains)
    pub fn is_federated(&self) -> bool {
        !self.subdomains.is_empty()
    }

    /// Get parent domain (e.g., "admin.nike" → "nike")
    pub fn parent(&self) -> Option<Self> {
        if self.subdomains.is_empty() {
            None
        } else {
            let mut subdomains = self.subdomains.clone();
            subdomains.remove(0);
            let full = if subdomains.is_empty() {
                self.root.clone()
            } else {
                format!("{}.{}", subdomains.join("."), self.root)
            };
            Some(Self {
                full,
                root: self.root.clone(),
                subdomains,
            })
        }
    }
}

/// DNS configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsConfig {
    /// Enable DNS-as-Asset system
    pub enabled: bool,
    /// Default TTL for DNS records (seconds)
    pub default_ttl: u32,
    /// Cache size (number of entries)
    pub cache_size: usize,
    /// Cache TTL (seconds)
    pub cache_ttl: u32,
    /// TrustChain DNS service endpoint
    pub trustchain_endpoint: Ipv6Addr,
    /// TrustChain DNS service port
    pub trustchain_port: u16,
    /// Enable PoS validation for all DNS operations
    pub require_pos_validation: bool,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_ttl: 300, // 5 minutes
            cache_size: 10000,
            cache_ttl: 300,
            trustchain_endpoint: Ipv6Addr::LOCALHOST,
            trustchain_port: 8053,
            require_pos_validation: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_parsing() {
        // Public domain
        let domain = Domain::parse("nike").unwrap();
        assert_eq!(domain.root, "nike");
        assert_eq!(domain.subdomains.len(), 0);
        assert!(domain.is_public());
        assert!(!domain.is_federated());

        // Federated domain
        let domain = Domain::parse("admin.nike").unwrap();
        assert_eq!(domain.root, "nike");
        assert_eq!(domain.subdomains, vec!["admin"]);
        assert!(!domain.is_public());
        assert!(domain.is_federated());

        // Multi-level federated
        let domain = Domain::parse("warehouse.admin.nike").unwrap();
        assert_eq!(domain.root, "nike");
        assert_eq!(domain.subdomains, vec!["warehouse", "admin"]);
    }

    #[test]
    fn test_domain_parent() {
        let domain = Domain::parse("warehouse.admin.nike").unwrap();
        let parent = domain.parent().unwrap();
        assert_eq!(parent.full, "admin.nike");
        assert_eq!(parent.subdomains, vec!["admin"]);

        let parent = parent.parent().unwrap();
        assert_eq!(parent.full, "nike");
        assert_eq!(parent.subdomains.len(), 0);

        assert!(parent.parent().is_none());
    }

    #[test]
    fn test_http3_prefix() {
        let domain = Domain::parse("http3://nike").unwrap();
        assert_eq!(domain.root, "nike");
        assert_eq!(domain.full, "nike");
    }
}
