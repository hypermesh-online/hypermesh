// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Manager
//!
//! Re-exports the STOQ certificate manager for TrustChain integration.
//! This avoids code duplication and ensures consistent certificate handling.


// Re-export the STOQ CertificateManager as the authoritative implementation
pub use stoq::transport::certificates::{
    CertificateManager,
    CertificateConfig,
    CertificateMode,
    StoqNodeCertificate,
    TrustChainClient,
};

// For backward compatibility, provide a simple factory function
pub async fn create_certificate_manager(config: CertificateConfig) -> anyhow::Result<CertificateManager> {
    CertificateManager::new(config).await
}