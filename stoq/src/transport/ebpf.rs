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
    HyperMeshEbpf, EbpfConfig, EbpfError, ShardMetadata,
    // XDP
    XdpManager, PacketDecision, FilterAction, XdpAttachMode, XdpStats, XdpFilterConfig,
    // AF_XDP
    AfXdpManager, AfXdpSocket, AfXdpStats, UmemConfig, RingConfig,
    // Loader
    EbpfLoader, ProgramType,
    // Hooks
    CertificateValidator, PacketValidator, ExtensionValidator,
    ValidationHooks, PassThroughValidator,
    // Policy
    PolicyManager, ValidationPolicy,
    // Headers
    ProofOfStateHeader, AssetHashHeader, MatrixRoutingHeader, PrivacyTierHeader,
    // Validators
    ProofOfStateValidator, AssetHashValidator,
    // Metrics
    HyperMeshMetrics, HyperMeshMetricsCollector, TransportMetrics,
    // Capabilities
    EbpfCapabilities,
};

/// STOQ-specific transport with eBPF acceleration.
///
/// Wraps HyperMeshEbpf with STOQ-specific initialization.
pub struct StoqEbpfTransport {
    inner: HyperMeshEbpf,
}

impl StoqEbpfTransport {
    /// Create new STOQ eBPF transport
    pub fn new() -> Result<Self, EbpfError> {
        let inner = HyperMeshEbpf::new(EbpfConfig::default())?;
        Ok(Self { inner })
    }

    /// Create with custom config
    pub fn with_config(config: EbpfConfig) -> Result<Self, EbpfError> {
        let inner = HyperMeshEbpf::new(config)?;
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
    fn test_stoq_ebpf_capabilities() {
        let transport = StoqEbpfTransport::new()
            .expect("test: create StoqEbpfTransport");
        let caps = transport.capabilities();
        assert!(!caps.kernel_version.is_empty());
    }

    #[test]
    fn test_stoq_ebpf_is_available() {
        let transport = StoqEbpfTransport::new()
            .expect("test: create StoqEbpfTransport");
        // Just verify it doesn't panic; availability depends on system
        let _available = transport.is_available();
    }

    #[test]
    fn test_stoq_ebpf_metrics() {
        let transport = StoqEbpfTransport::new()
            .expect("test: create StoqEbpfTransport");
        let metrics = transport.metrics().collect();
        assert_eq!(metrics.pos_metrics.total_validations, 0);
    }
}
