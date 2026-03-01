// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Validation Hook Traits
//!
//! Provides extension points for applications (STOQ, blockmatrix) to inject
//! custom validation logic into the eBPF pipeline. The eBPF crate defines
//! the mechanism; consumers provide the policy.
//!
//! STOQ implements CertificateValidator and PacketValidator.
//! BlockMatrix implements ExtensionValidator for HyperMesh headers.

use anyhow::Result;
use async_trait::async_trait;

// -----------------------------------------------------------------------
// Certificate Validation Hook
// -----------------------------------------------------------------------

/// Certificate validator trait for custom certificate validation.
///
/// Applications implement this to inject validation logic beyond
/// standard X.509. For example, HyperMesh injects Proof of State
/// validation of TrustChain certificates.
#[async_trait]
pub trait CertificateValidator: Send + Sync {
    /// Validate certificate with custom logic.
    ///
    /// # Arguments
    /// * `cert_der` - DER-encoded certificate bytes
    /// * `context` - Optional application-defined validation context
    async fn validate(&self, cert_der: &[u8], context: Option<&[u8]>) -> Result<()>;

    /// Validate a certificate chain (optional override).
    /// Default: validates each certificate individually.
    async fn validate_chain(&self, chain: &[&[u8]]) -> Result<()> {
        for cert in chain {
            self.validate(cert, None).await?;
        }
        Ok(())
    }

    /// Validator name for logging
    fn name(&self) -> &str {
        "GenericCertificateValidator"
    }
}

// -----------------------------------------------------------------------
// Packet Validation Hook
// -----------------------------------------------------------------------

/// Packet validator trait for custom packet-level validation.
///
/// Applications implement this to validate packet contents at the
/// transport layer before delivery to the application.
#[async_trait]
pub trait PacketValidator: Send + Sync {
    /// Validate packet with custom logic.
    ///
    /// # Arguments
    /// * `packet_data` - Raw packet data
    /// * `connection_id` - Connection identifier
    async fn validate(&self, packet_data: &[u8], connection_id: u64) -> Result<()>;

    /// Validator name for logging
    fn name(&self) -> &str {
        "GenericPacketValidator"
    }
}

// -----------------------------------------------------------------------
// Extension Validation Hook
// -----------------------------------------------------------------------

/// Extension validator trait for STOQ extension header validation.
///
/// STOQ treats extension headers as opaque byte blobs. Applications
/// implement this trait to interpret and validate them. HyperMesh uses
/// this for PoS, asset hash, matrix routing, and privacy headers.
#[async_trait]
pub trait ExtensionValidator: Send + Sync {
    /// Validate an extension header.
    ///
    /// # Arguments
    /// * `extension_type` - Extension type identifier (e.g., 0x1000 for PoS)
    /// * `extension_data` - Raw extension data
    async fn validate(&self, extension_type: u16, extension_data: &[u8]) -> Result<()>;

    /// Get the extension types this validator handles
    fn supported_extensions(&self) -> Vec<u16>;

    /// Validator name for logging
    fn name(&self) -> &str {
        "GenericExtensionValidator"
    }
}

// -----------------------------------------------------------------------
// Validation Hooks Configuration
// -----------------------------------------------------------------------

/// Container for registered validation hooks.
///
/// Consumers (STOQ, blockmatrix) register their validators here.
/// The eBPF subsystem calls them during packet processing.
#[derive(Default)]
pub struct ValidationHooks {
    /// Certificate validator (optional)
    pub certificate_validator: Option<Box<dyn CertificateValidator>>,
    /// Packet validator (optional)
    pub packet_validator: Option<Box<dyn PacketValidator>>,
    /// Extension validator (optional)
    pub extension_validator: Option<Box<dyn ExtensionValidator>>,
}

impl ValidationHooks {
    /// Create empty validation hooks (no validators)
    pub fn new() -> Self {
        Self::default()
    }

    /// Set certificate validator (builder pattern)
    pub fn with_certificate_validator(mut self, validator: Box<dyn CertificateValidator>) -> Self {
        self.certificate_validator = Some(validator);
        self
    }

    /// Set packet validator (builder pattern)
    pub fn with_packet_validator(mut self, validator: Box<dyn PacketValidator>) -> Self {
        self.packet_validator = Some(validator);
        self
    }

    /// Set extension validator (builder pattern)
    pub fn with_extension_validator(mut self, validator: Box<dyn ExtensionValidator>) -> Self {
        self.extension_validator = Some(validator);
        self
    }

    /// Validate certificate if a validator is configured
    pub async fn validate_certificate(&self, cert_der: &[u8]) -> Result<()> {
        if let Some(validator) = &self.certificate_validator {
            tracing::debug!("Validating certificate with {}", validator.name());
            validator.validate(cert_der, None).await?;
        }
        Ok(())
    }

    /// Validate packet if a validator is configured
    pub async fn validate_packet(&self, packet_data: &[u8], connection_id: u64) -> Result<()> {
        if let Some(validator) = &self.packet_validator {
            tracing::trace!("Validating packet with {}", validator.name());
            validator.validate(packet_data, connection_id).await?;
        }
        Ok(())
    }

    /// Validate extension header if a validator is configured
    pub async fn validate_extension(
        &self,
        extension_type: u16,
        extension_data: &[u8],
    ) -> Result<()> {
        if let Some(validator) = &self.extension_validator {
            if validator.supported_extensions().contains(&extension_type) {
                tracing::trace!(
                    "Validating extension {:04x} with {}",
                    extension_type,
                    validator.name()
                );
                validator.validate(extension_type, extension_data).await?;
            }
        }
        Ok(())
    }

    /// Check if any validators are registered
    pub fn has_validators(&self) -> bool {
        self.certificate_validator.is_some()
            || self.packet_validator.is_some()
            || self.extension_validator.is_some()
    }
}

// -----------------------------------------------------------------------
// Built-in pass-through validator
// -----------------------------------------------------------------------

/// Pass-through validator that accepts everything.
/// Useful for testing and as a no-op default.
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
    async fn test_validation_hooks_with_validator() {
        let hooks =
            ValidationHooks::new().with_certificate_validator(Box::new(PassThroughValidator));

        assert!(hooks.certificate_validator.is_some());
        assert!(hooks.packet_validator.is_none());
        assert!(hooks.has_validators());

        assert!(hooks.validate_certificate(&[1, 2, 3]).await.is_ok());
    }

    #[tokio::test]
    async fn test_no_validators() {
        let hooks = ValidationHooks::new();

        assert!(!hooks.has_validators());

        // Should succeed when no validators configured (pass-through)
        assert!(hooks.validate_certificate(&[1, 2, 3]).await.is_ok());
        assert!(hooks.validate_packet(&[1, 2, 3], 123).await.is_ok());
        assert!(hooks.validate_extension(0x1000, &[1, 2, 3]).await.is_ok());
    }
}
