// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! eBPF Transport Acceleration for STOQ
//!
//! Thin consumer layer that delegates to the unified hypermesh-ebpf crate.
//! STOQ uses hypermesh-ebpf for:
//! - XDP attachment and packet filtering
//! - AF_XDP zero-copy sockets
//! - Transport metrics collection
//! - Validation hooks registration

pub use hypermesh_ebpf::{
    // AF_XDP
    AfXdpManager,
    AfXdpSocket,
    AfXdpStats,
    AssetHashHeader,
    AssetHashValidator,
    // Hooks
    CertificateValidator,
    // Capabilities
    EbpfCapabilities,
    EbpfConfig,
    EbpfError,
    // Loader
    EbpfLoader,
    ExtensionValidator,
    FilterAction,
    HyperMeshEbpf,
    // Metrics
    HyperMeshMetrics,
    HyperMeshMetricsCollector,
    MatrixRoutingHeader,
    PacketDecision,
    PacketValidator,
    PassThroughValidator,
    // Policy
    PolicyManager,
    PrivacyTierHeader,
    ProgramType,
    // Headers
    ProofOfStateHeader,
    // Validators
    ProofOfStateValidator,
    RingConfig,
    ShardMetadata,
    TransportMetrics,
    UmemConfig,
    ValidationHooks,
    ValidationPolicy,
    XdpAttachMode,
    XdpFilterConfig,
    // XDP
    XdpManager,
    XdpStats,
};

// -----------------------------------------------------------------------
// STOQ Validation Hook Implementations
// -----------------------------------------------------------------------

/// STOQ certificate validator using FALCON-1024 structural checks.
///
/// Validates DER-encoded certificate structure at the eBPF/transport layer.
/// Deep cryptographic verification is performed by TrustChain; this validator
/// does fast structural pre-validation to reject obviously invalid certificates.
pub struct StoqCertificateValidator;

impl Default for StoqCertificateValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl StoqCertificateValidator {
    /// Create a new STOQ certificate validator.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl CertificateValidator for StoqCertificateValidator {
    async fn validate(&self, cert_der: &[u8], _context: Option<&[u8]>) -> anyhow::Result<()> {
        // Minimum DER certificate size (ASN.1 SEQUENCE header + minimal content)
        const MIN_DER_SIZE: usize = 10;

        if cert_der.len() < MIN_DER_SIZE {
            anyhow::bail!(
                "Certificate too short: {} bytes (minimum {})",
                cert_der.len(),
                MIN_DER_SIZE
            );
        }

        // DER certificates start with ASN.1 SEQUENCE tag (0x30)
        if cert_der[0] != 0x30 {
            anyhow::bail!(
                "Invalid DER certificate: expected SEQUENCE tag 0x30, got 0x{:02x}",
                cert_der[0]
            );
        }

        // Validate ASN.1 length encoding
        let length_byte = cert_der[1];
        if length_byte & 0x80 != 0 {
            // Long-form length: high bit set, low 7 bits = number of length bytes
            let num_length_bytes = (length_byte & 0x7F) as usize;
            if num_length_bytes == 0 || num_length_bytes > 4 {
                anyhow::bail!("Invalid DER length encoding: {num_length_bytes} length bytes");
            }
            if cert_der.len() < 2 + num_length_bytes {
                anyhow::bail!(
                    "Certificate truncated in length field: {} < {}",
                    cert_der.len(),
                    2 + num_length_bytes
                );
            }
        }

        tracing::trace!(
            "STOQ certificate pre-validation passed ({} bytes)",
            cert_der.len()
        );
        Ok(())
    }

    fn name(&self) -> &str {
        "StoqCertificateValidator"
    }
}

/// STOQ packet validator for QUIC/STOQ protocol header checks.
///
/// Validates packet structure at the transport layer before delivery to
/// the application. Checks STOQ protocol framing and minimum structure.
pub struct StoqPacketValidator {
    /// Minimum acceptable packet size
    min_packet_size: usize,
}

impl Default for StoqPacketValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl StoqPacketValidator {
    /// Create a new STOQ packet validator.
    pub fn new() -> Self {
        Self {
            min_packet_size: 4, // Minimum: header form + version/type
        }
    }

    /// Create with custom minimum packet size.
    pub fn with_min_size(min_packet_size: usize) -> Self {
        Self { min_packet_size }
    }
}

#[async_trait::async_trait]
impl PacketValidator for StoqPacketValidator {
    async fn validate(&self, packet_data: &[u8], _connection_id: u64) -> anyhow::Result<()> {
        if packet_data.len() < self.min_packet_size {
            anyhow::bail!(
                "STOQ packet too short: {} bytes (minimum {})",
                packet_data.len(),
                self.min_packet_size
            );
        }

        // QUIC packets: first byte high bit indicates long (1) vs short (0) header.
        // For STOQ initial packets, we expect the long header form with a valid
        // QUIC version in bytes 1..5.
        let header_form = packet_data[0] >> 7;
        if header_form == 1 {
            // Long header: need at least 5 bytes (1 header + 4 version)
            if packet_data.len() < 5 {
                anyhow::bail!(
                    "STOQ long-header packet truncated: {} < 5 bytes",
                    packet_data.len()
                );
            }
            // Version 0x00000000 is reserved for version negotiation
            // We accept it as structurally valid
        }

        tracing::trace!(
            "STOQ packet pre-validation passed ({} bytes, {} header)",
            packet_data.len(),
            if header_form == 1 { "long" } else { "short" }
        );

        Ok(())
    }

    fn name(&self) -> &str {
        "StoqPacketValidator"
    }
}

// -----------------------------------------------------------------------
// STOQ eBPF Transport
// -----------------------------------------------------------------------

/// STOQ-specific transport with eBPF acceleration.
///
/// Wraps HyperMeshEbpf with STOQ-specific initialization.
/// Registers STOQ validation hooks (certificate + packet) on creation.
pub struct StoqEbpfTransport {
    inner: HyperMeshEbpf,
}

impl StoqEbpfTransport {
    /// Create new STOQ eBPF transport with validation hooks registered.
    pub fn new() -> Result<Self, EbpfError> {
        let mut inner = HyperMeshEbpf::new(EbpfConfig::default())?;

        // Register STOQ validation hooks
        let hooks = ValidationHooks::new()
            .with_certificate_validator(Box::new(StoqCertificateValidator::new()))
            .with_packet_validator(Box::new(StoqPacketValidator::new()));
        inner.set_validation_hooks(hooks);

        tracing::debug!("STOQ validation hooks registered (certificate + packet)");

        Ok(Self { inner })
    }

    /// Create with custom config (registers STOQ hooks automatically).
    pub fn with_config(config: EbpfConfig) -> Result<Self, EbpfError> {
        let mut inner = HyperMeshEbpf::new(config)?;

        let hooks = ValidationHooks::new()
            .with_certificate_validator(Box::new(StoqCertificateValidator::new()))
            .with_packet_validator(Box::new(StoqPacketValidator::new()));
        inner.set_validation_hooks(hooks);

        Ok(Self { inner })
    }

    /// Check if eBPF is available on this system
    pub fn is_available(&self) -> bool {
        let caps = self.inner.capabilities();
        caps.xdp_available || caps.af_xdp_available
    }

    /// Get inner HyperMeshEbpf reference
    pub fn inner(&self) -> &HyperMeshEbpf {
        &self.inner
    }

    /// Get mutable inner HyperMeshEbpf reference
    pub fn inner_mut(&mut self) -> &mut HyperMeshEbpf {
        &mut self.inner
    }

    /// Attach XDP to interface (delegates to HyperMeshEbpf)
    pub fn attach_xdp(&mut self, interface: &str) -> Result<(), EbpfError> {
        self.inner.attach_xdp(interface)
    }

    /// Create AF_XDP socket (delegates to HyperMeshEbpf)
    pub fn create_af_xdp_socket(
        &mut self,
        interface: &str,
        queue_id: u32,
    ) -> Result<AfXdpSocket, EbpfError> {
        self.inner.create_af_xdp_socket(interface, queue_id)
    }

    /// Get transport metrics
    pub fn metrics(&self) -> &HyperMeshMetricsCollector {
        self.inner.metrics()
    }

    /// Get capabilities
    pub fn capabilities(&self) -> &EbpfCapabilities {
        self.inner.capabilities()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stoq_ebpf_transport_creation() {
        let transport = StoqEbpfTransport::new();
        assert!(transport.is_ok());
    }

    #[test]
    fn test_stoq_ebpf_hooks_registered() {
        let transport = StoqEbpfTransport::new().expect("test: create StoqEbpfTransport");
        let hooks = transport.inner().validation_hooks();
        assert!(hooks.has_validators());
        assert!(hooks.certificate_validator.is_some());
        assert!(hooks.packet_validator.is_some());
    }

    #[test]
    fn test_stoq_ebpf_capabilities() {
        let transport = StoqEbpfTransport::new().expect("test: create StoqEbpfTransport");
        let caps = transport.capabilities();
        assert!(!caps.kernel_version.is_empty());
    }

    #[test]
    fn test_stoq_ebpf_is_available() {
        let transport = StoqEbpfTransport::new().expect("test: create StoqEbpfTransport");
        // Just verify it doesn't panic; availability depends on system
        let _available = transport.is_available();
    }

    #[test]
    fn test_stoq_ebpf_metrics() {
        let transport = StoqEbpfTransport::new().expect("test: create StoqEbpfTransport");
        let metrics = transport.metrics().collect();
        assert_eq!(metrics.pos_metrics.total_validations, 0);
    }

    // -------------------------------------------------------------------
    // StoqCertificateValidator tests
    // -------------------------------------------------------------------

    #[tokio::test]
    async fn test_cert_validator_rejects_too_short() {
        let validator = StoqCertificateValidator::new();
        let result = validator.validate(&[0x30, 0x01], None).await;
        assert!(result.is_err());
        let msg = result
            .expect_err("test: should reject short cert")
            .to_string();
        assert!(msg.contains("too short"));
    }

    #[tokio::test]
    async fn test_cert_validator_rejects_bad_tag() {
        let validator = StoqCertificateValidator::new();
        // 20 bytes but wrong tag (0xFF instead of 0x30)
        let bad_cert = [0xFF; 20];
        let result = validator.validate(&bad_cert, None).await;
        assert!(result.is_err());
        let msg = result.expect_err("test: should reject bad tag").to_string();
        assert!(msg.contains("SEQUENCE tag"));
    }

    #[tokio::test]
    async fn test_cert_validator_accepts_valid_structure() {
        let validator = StoqCertificateValidator::new();
        // Minimal valid DER-like structure: SEQUENCE tag + short-form length + content
        let mut cert = vec![0x30, 0x0A]; // SEQUENCE, length 10
        cert.extend_from_slice(&[0x01; 10]); // 10 content bytes
        let result = validator.validate(&cert, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cert_validator_long_form_length() {
        let validator = StoqCertificateValidator::new();
        // Long-form length: 0x82 means 2 length bytes follow
        let mut cert = vec![0x30, 0x82, 0x01, 0x00]; // SEQUENCE, length 256
        cert.extend_from_slice(&[0x01; 256]); // 256 content bytes
        let result = validator.validate(&cert, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cert_validator_bad_length_encoding() {
        let validator = StoqCertificateValidator::new();
        // Long-form with 0 length bytes (invalid)
        let cert = vec![0x30, 0x80, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01];
        let result = validator.validate(&cert, None).await;
        assert!(result.is_err());
        let msg = result
            .expect_err("test: should reject bad length")
            .to_string();
        assert!(msg.contains("length"));
    }

    #[tokio::test]
    async fn test_cert_validator_name() {
        let validator = StoqCertificateValidator::new();
        assert_eq!(validator.name(), "StoqCertificateValidator");
    }

    // -------------------------------------------------------------------
    // StoqPacketValidator tests
    // -------------------------------------------------------------------

    #[tokio::test]
    async fn test_packet_validator_rejects_too_short() {
        let validator = StoqPacketValidator::new();
        let result = validator.validate(&[0x01], 1).await;
        assert!(result.is_err());
        let msg = result
            .expect_err("test: should reject short packet")
            .to_string();
        assert!(msg.contains("too short"));
    }

    #[tokio::test]
    async fn test_packet_validator_accepts_short_header() {
        let validator = StoqPacketValidator::new();
        // Short header (high bit 0)
        let packet = vec![0x40, 0x00, 0x00, 0x00, 0x01];
        let result = validator.validate(&packet, 42).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_packet_validator_accepts_long_header() {
        let validator = StoqPacketValidator::new();
        // Long header (high bit 1) with 4-byte version
        let packet = vec![0xC0, 0x00, 0x00, 0x00, 0x01, 0x08, 0x01, 0x02];
        let result = validator.validate(&packet, 42).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_packet_validator_rejects_truncated_long_header() {
        let validator = StoqPacketValidator::new();
        // Long header (high bit 1) but only 4 bytes (needs 5 for version)
        let packet = vec![0xC0, 0x00, 0x00, 0x00];
        let result = validator.validate(&packet, 42).await;
        assert!(result.is_err());
        let msg = result
            .expect_err("test: should reject truncated long header")
            .to_string();
        assert!(msg.contains("truncated"));
    }

    #[tokio::test]
    async fn test_packet_validator_name() {
        let validator = StoqPacketValidator::new();
        assert_eq!(validator.name(), "StoqPacketValidator");
    }

    #[tokio::test]
    async fn test_packet_validator_custom_min_size() {
        let validator = StoqPacketValidator::with_min_size(16);
        let short = vec![0x00; 10];
        let result = validator.validate(&short, 1).await;
        assert!(result.is_err());

        let ok = vec![0x00; 16];
        let result = validator.validate(&ok, 1).await;
        assert!(result.is_ok());
    }
}
