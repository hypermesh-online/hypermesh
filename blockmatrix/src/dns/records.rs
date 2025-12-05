//! DNS Record Types
//!
//! Defines DNS record types for the DNS-as-Asset system.

use serde::{Serialize, Deserialize};
use std::net::Ipv6Addr;
use std::time::{SystemTime, Duration};
use crate::matrix::MatrixCoordinate;

/// DNS record type
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DnsRecordType {
    /// IPv6 address (AAAA) - IPv6-only network
    AAAA,
    /// Canonical name (CNAME)
    CNAME,
    /// Text record (TXT) - for metadata
    TXT,
    /// Matrix coordinate (custom)
    Matrix,
    /// Service record (SRV)
    SRV,
}

/// DNS record data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DnsRecordData {
    /// IPv6 address
    AAAA(Ipv6Addr),
    /// Canonical name
    CNAME(String),
    /// Text data
    TXT(String),
    /// Matrix coordinate for asset location
    Matrix(MatrixCoordinate),
    /// Service record
    SRV {
        priority: u16,
        weight: u16,
        port: u16,
        target: String,
    },
}

/// DNS record with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsRecord {
    /// Domain name
    pub domain: String,
    /// Record type
    pub record_type: DnsRecordType,
    /// Record data
    pub data: DnsRecordData,
    /// Time-to-live (seconds)
    pub ttl: u32,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Expiration timestamp
    pub expires_at: SystemTime,
    /// Owner node ID (from blockchain)
    pub owner: String,
    /// Blockchain transaction hash
    pub tx_hash: Option<String>,
}

impl DnsRecord {
    /// Create new DNS record
    pub fn new(
        domain: String,
        record_type: DnsRecordType,
        data: DnsRecordData,
        ttl: u32,
        owner: String,
    ) -> Self {
        let now = SystemTime::now();
        let expires_at = now + Duration::from_secs(ttl as u64);

        Self {
            domain,
            record_type,
            data,
            ttl,
            created_at: now,
            expires_at,
            owner,
            tx_hash: None,
        }
    }

    /// Check if record has expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }

    /// Get remaining TTL in seconds
    pub fn remaining_ttl(&self) -> u32 {
        match self.expires_at.duration_since(SystemTime::now()) {
            Ok(duration) => duration.as_secs() as u32,
            Err(_) => 0,
        }
    }

    /// Refresh TTL
    pub fn refresh(&mut self) {
        let now = SystemTime::now();
        self.expires_at = now + Duration::from_secs(self.ttl as u64);
    }

    /// Set blockchain transaction hash
    pub fn set_tx_hash(&mut self, tx_hash: String) {
        self.tx_hash = Some(tx_hash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_record_creation() {
        let record = DnsRecord::new(
            "nike".to_string(),
            DnsRecordType::AAAA,
            DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
            300,
            "node-1".to_string(),
        );

        assert_eq!(record.domain, "nike");
        assert_eq!(record.ttl, 300);
        assert!(!record.is_expired());
    }

    #[test]
    fn test_dns_record_expiration() {
        let mut record = DnsRecord::new(
            "test".to_string(),
            DnsRecordType::AAAA,
            DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
            0, // Expires immediately
            "node-1".to_string(),
        );

        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(record.is_expired());

        record.refresh();
        record.ttl = 300;
        record.expires_at = SystemTime::now() + Duration::from_secs(300);
        assert!(!record.is_expired());
    }

    #[test]
    fn test_remaining_ttl() {
        let record = DnsRecord::new(
            "test".to_string(),
            DnsRecordType::AAAA,
            DnsRecordData::AAAA(Ipv6Addr::LOCALHOST),
            300,
            "node-1".to_string(),
        );

        let remaining = record.remaining_ttl();
        assert!(remaining <= 300);
        assert!(remaining > 295); // Allow small time difference
    }
}
