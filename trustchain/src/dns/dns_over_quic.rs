// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS-over-QUIC Implementation (DEPRECATED)
//!
//! This module provides legacy DNS-over-QUIC support while migrating to STOQ transport.
//! All new implementations should use dns_over_stoq.rs instead.

use std::net::SocketAddrV6;
use tracing::{warn, error};

use crate::errors::{DnsError, Result as TrustChainResult};

/// DEPRECATED: DNS-over-QUIC client (use STOQ transport instead)
#[deprecated(note = "Use dns_over_stoq.rs for new implementations")]
pub struct DnsOverQuicClient {
    /// Server identifier (retained for API compatibility)
    #[allow(dead_code)]
    server_id: String,
}

#[allow(deprecated)]
impl DnsOverQuicClient {
    /// Create new DNS-over-QUIC client
    #[deprecated(note = "Use dns_over_stoq.rs for new implementations")]
    pub fn new(server_id: String) -> Self {
        warn!("DNS-over-QUIC is deprecated, use STOQ transport instead");
        Self { server_id }
    }

    /// Send DNS query over QUIC
    #[deprecated(note = "Use dns_over_stoq.rs for new implementations")]
    pub async fn query(&self, _domain: &str) -> TrustChainResult<String> {
        error!("DNS-over-QUIC is deprecated and not implemented");
        Err(DnsError::QuicConnectionFailed {
            reason: "DNS-over-QUIC is deprecated, use STOQ transport".to_string()
        }.into())
    }
}

/// DEPRECATED: DNS-over-QUIC server (use STOQ transport instead)
#[deprecated(note = "Use dns_over_stoq.rs for new implementations")]
pub struct DnsOverQuicServer {
    bind_addr: SocketAddrV6,
}

#[allow(deprecated)]
impl DnsOverQuicServer {
    /// Create new DNS-over-QUIC server
    #[deprecated(note = "Use dns_over_stoq.rs for new implementations")]
    pub fn new(bind_addr: SocketAddrV6) -> Self {
        warn!("DNS-over-QUIC is deprecated, use STOQ transport instead");
        Self { bind_addr }
    }

    /// Start DNS-over-QUIC server
    #[deprecated(note = "Use dns_over_stoq.rs for new implementations")]
    pub async fn start(&self) -> TrustChainResult<()> {
        error!("DNS-over-QUIC is deprecated and not implemented");
        Err(DnsError::ServerBindFailed {
            address: self.bind_addr.ip().to_string(),
            port: self.bind_addr.port(),
        }.into())
    }
}