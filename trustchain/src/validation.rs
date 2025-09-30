//! Input validation module for TrustChain
//! Provides comprehensive validation for all external inputs

use anyhow::{Result, anyhow};
use regex::Regex;
use std::net::{IpAddr, Ipv6Addr};

/// Validate node ID format
pub fn validate_node_id(node_id: &str) -> Result<()> {
    if node_id.is_empty() {
        return Err(anyhow!("Node ID cannot be empty"));
    }

    if node_id.len() > 128 {
        return Err(anyhow!("Node ID too long (max 128 characters)"));
    }

    // Must be alphanumeric with hyphens
    let re = Regex::new(r"^[a-zA-Z0-9-]+$").unwrap();
    if !re.is_match(node_id) {
        return Err(anyhow!("Invalid node ID format"));
    }

    Ok(())
}

/// Validate certificate request
pub fn validate_certificate_request(common_name: &str, san_entries: &[String]) -> Result<()> {
    // Validate common name
    if common_name.is_empty() {
        return Err(anyhow!("Common name cannot be empty"));
    }

    if common_name.len() > 64 {
        return Err(anyhow!("Common name too long (max 64 characters)"));
    }

    // Validate SAN entries
    for san in san_entries {
        if san.len() > 253 {
            return Err(anyhow!("SAN entry too long"));
        }
    }

    if san_entries.len() > 100 {
        return Err(anyhow!("Too many SAN entries (max 100)"));
    }

    Ok(())
}

/// Validate IPv6 address
pub fn validate_ipv6(addr: &str) -> Result<Ipv6Addr> {
    addr.parse::<Ipv6Addr>()
        .map_err(|_| anyhow!("Invalid IPv6 address: {}", addr))
}

/// Validate consensus proof size
pub fn validate_consensus_proof(proof_data: &[u8]) -> Result<()> {
    if proof_data.is_empty() {
        return Err(anyhow!("Consensus proof cannot be empty"));
    }

    if proof_data.len() > 10_000 {
        return Err(anyhow!("Consensus proof too large (max 10KB)"));
    }

    Ok(())
}

/// Sanitize user input to prevent injection attacks
pub fn sanitize_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .take(256)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_node_id() {
        assert!(validate_node_id("node-123").is_ok());
        assert!(validate_node_id("").is_err());
        assert!(validate_node_id("node@123").is_err());
    }

    #[test]
    fn test_sanitize_input() {
        assert_eq!(sanitize_input("hello-world_123"), "hello-world_123");
        assert_eq!(sanitize_input("../../etc/passwd"), "..etcpasswd");
        assert_eq!(sanitize_input("'; DROP TABLE users; --"), "DROPTABLEusers");
    }
}
