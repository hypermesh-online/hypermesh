//! Request Validators
//! 
//! Input validation for API requests.

use serde::{Serialize, Deserialize};
use std::net::Ipv6Addr;

use crate::errors::{ApiError, Result as TrustChainResult};

/// Domain name validator
pub fn validate_domain_name(domain: &str) -> TrustChainResult<()> {
    if domain.is_empty() {
        return Err(ApiError::InvalidRequestFormat {
            reason: "Domain name cannot be empty".to_string(),
        }.into());
    }
    
    if domain.len() > 253 {
        return Err(ApiError::InvalidRequestFormat {
            reason: "Domain name too long".to_string(),
        }.into());
    }
    
    // Basic domain validation
    if !domain.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
        return Err(ApiError::InvalidRequestFormat {
            reason: "Invalid characters in domain name".to_string(),
        }.into());
    }
    
    Ok(())
}

/// IPv6 address validator
pub fn validate_ipv6_addresses(addresses: &[Ipv6Addr]) -> TrustChainResult<()> {
    if addresses.is_empty() {
        return Err(ApiError::InvalidRequestFormat {
            reason: "At least one IPv6 address required".to_string(),
        }.into());
    }
    
    for addr in addresses {
        if addr.is_unspecified() {
            return Err(ApiError::InvalidRequestFormat {
                reason: "Unspecified IPv6 address not allowed".to_string(),
            }.into());
        }
    }
    
    Ok(())
}

/// Certificate request validator
pub fn validate_certificate_request(
    common_name: &str,
    san_entries: &[String],
    ipv6_addresses: &[Ipv6Addr],
) -> TrustChainResult<()> {
    validate_domain_name(common_name)?;
    
    for san in san_entries {
        validate_domain_name(san)?;
    }
    
    validate_ipv6_addresses(ipv6_addresses)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_domain() {
        assert!(validate_domain_name("example.com").is_ok());
        assert!(validate_domain_name("test.example.com").is_ok());
        assert!(validate_domain_name("hypermesh").is_ok());
    }

    #[test]
    fn test_invalid_domain() {
        assert!(validate_domain_name("").is_err());
        assert!(validate_domain_name("invalid_domain!").is_err());
    }

    #[test]
    fn test_valid_ipv6() {
        let addresses = vec![Ipv6Addr::LOCALHOST];
        assert!(validate_ipv6_addresses(&addresses).is_ok());
    }

    #[test]
    fn test_invalid_ipv6() {
        let addresses = vec![];
        assert!(validate_ipv6_addresses(&addresses).is_err());
        
        let unspecified = vec![Ipv6Addr::UNSPECIFIED];
        assert!(validate_ipv6_addresses(&unspecified).is_err());
    }
}