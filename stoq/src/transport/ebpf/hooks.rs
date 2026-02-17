// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Generic Validation Hooks for STOQ Transport
//!
//! Provides extension points for applications to inject custom validation logic.
//! STOQ provides the mechanism, applications provide the policy.

use anyhow::Result;
use async_trait::async_trait;

/// Generic certificate validator trait
///
/// Applications can implement this trait to provide custom certificate validation
/// beyond standard X.509 validation. This allows HyperMesh to inject Proof of State
/// validation, or other applications to implement their own validation logic.
#[async_trait]
pub trait CertificateValidator: Send + Sync {
    /// Validate certificate with custom logic
    ///
    /// # Arguments
    /// * `cert_der` - DER-encoded certificate
    /// * `context` - Optional validation context (application-defined)
    ///
    /// # Returns
    /// * `Ok(())` if certificate is valid
    /// * `Err(_)` if certificate is invalid with reason
    async fn validate(&self, cert_der: &[u8], context: Option<&[u8]>) -> Result<()>;

    /// Validate certificate chain (optional)
    async fn validate_chain(&self, chain: &[&[u8]]) -> Result<()> {
        // Default: validate each certificate individually
        for cert in chain {
            self.validate(cert, None).await?;
        }
        Ok(())
    }

    /// Get validator name for logging
    fn name(&self) -> &str {
        "GenericValidator"
    }
}

/// Generic packet validator trait
///
/// Applications can implement this trait to validate packet contents
/// at the transport layer before delivery to the application.
#[async_trait]
pub trait PacketValidator: Send + Sync {
    /// Validate packet with custom logic
    ///
    /// # Arguments
    /// * `packet_data` - Raw packet data
    /// * `connection_id` - Connection identifier
    ///
    /// # Returns
    /// * `Ok(())` if packet is valid
    /// * `Err(_)` if packet is invalid with reason
    async fn validate(&self, packet_data: &[u8], connection_id: u64) -> Result<()>;

    /// Get validator name for logging
    fn name(&self) -> &str {
        "GenericPacketValidator"
    }
}

/// Generic extension validator trait
///
/// Applications can implement this trait to validate extension headers
/// carried in STOQ packets. STOQ treats extensions as opaque byte blobs;
/// this trait allows applications to interpret and validate them.
#[async_trait]
pub trait ExtensionValidator: Send + Sync {
    /// Validate extension header
    ///
    /// # Arguments
    /// * `extension_type` - Extension type identifier
    /// * `extension_data` - Raw extension data
    ///
    /// # Returns
    /// * `Ok(())` if extension is valid
    /// * `Err(_)` if extension is invalid with reason
    async fn validate(&self, extension_type: u16, extension_data: &[u8]) -> Result<()>;

    /// Get supported extension types
    fn supported_extensions(&self) -> Vec<u16>;

    /// Get validator name for logging
    fn name(&self) -> &str {
        "GenericExtensionValidator"
    }
}

/// Validation hooks configuration
pub struct ValidationHooks {
    /// Certificate validator (optional)
    pub certificate_validator: Option<Box<dyn CertificateValidator>>,
    /// Packet validator (optional)
    pub packet_validator: Option<Box<dyn PacketValidator>>,
    /// Extension validator (optional)
    pub extension_validator: Option<Box<dyn ExtensionValidator>>,
}

impl Default for ValidationHooks {
    fn default() -> Self {
        Self {
            certificate_validator: None,
            packet_validator: None,
            extension_validator: None,
        }
    }
}

impl ValidationHooks {
    /// Create new validation hooks with no validators
    pub fn new() -> Self {
        Self::default()
    }

    /// Set certificate validator
    pub fn with_certificate_validator(
        mut self,
        validator: Box<dyn CertificateValidator>,
    ) -> Self {
        self.certificate_validator = Some(validator);
        self
    }

    /// Set packet validator
    pub fn with_packet_validator(
        mut self,
        validator: Box<dyn PacketValidator>,
    ) -> Self {
        self.packet_validator = Some(validator);
        self
    }

    /// Set extension validator
    pub fn with_extension_validator(
        mut self,
        validator: Box<dyn ExtensionValidator>,
    ) -> Self {
        self.extension_validator = Some(validator);
        self
    }

    /// Validate certificate if validator is configured
    pub async fn validate_certificate(&self, cert_der: &[u8]) -> Result<()> {
        if let Some(validator) = &self.certificate_validator {
            tracing::debug!("Validating certificate with {}", validator.name());
            validator.validate(cert_der, None).await?;
        }
        Ok(())
    }

    /// Validate packet if validator is configured
    pub async fn validate_packet(&self, packet_data: &[u8], connection_id: u64) -> Result<()> {
        if let Some(validator) = &self.packet_validator {
            tracing::trace!("Validating packet with {}", validator.name());
            validator.validate(packet_data, connection_id).await?;
        }
        Ok(())
    }

    /// Validate extension if validator is configured
    pub async fn validate_extension(&self, extension_type: u16, extension_data: &[u8]) -> Result<()> {
        if let Some(validator) = &self.extension_validator {
            if validator.supported_extensions().contains(&extension_type) {
                tracing::trace!("Validating extension {:04x} with {}", extension_type, validator.name());
                validator.validate(extension_type, extension_data).await?;
            }
        }
        Ok(())
    }
}

/// Example: Pass-through validator (accepts everything)
pub struct PassThroughValidator;

#[async_trait]
impl CertificateValidator for PassThroughValidator {
    async fn validate(&self, _cert_der: &[u8], _context: Option<&[u8]>) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "PassThroughValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_hooks() {
        let hooks = ValidationHooks::new()
            .with_certificate_validator(Box::new(PassThroughValidator));

        assert!(hooks.certificate_validator.is_some());
        assert!(hooks.packet_validator.is_none());

        // Should not fail with pass-through validator
        assert!(hooks.validate_certificate(&[1, 2, 3]).await.is_ok());
    }

    #[tokio::test]
    async fn test_no_validators() {
        let hooks = ValidationHooks::new();

        // Should succeed when no validators configured
        assert!(hooks.validate_certificate(&[1, 2, 3]).await.is_ok());
        assert!(hooks.validate_packet(&[1, 2, 3], 123).await.is_ok());
        assert!(hooks.validate_extension(0x1000, &[1, 2, 3]).await.is_ok());
    }
}
