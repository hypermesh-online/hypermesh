use super::TrustChainDnsClient;
use anyhow::{anyhow, Result};
use hypermesh_lib::NetworkId;
use std::net::{SocketAddr, SocketAddrV6};
use std::sync::Arc;

/// Trusted local routing selection, not authority authentication or a resource grant.
/// This type deliberately has no wire deserializer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryContext {
    protocol: String,
    network_id: NetworkId,
    authority: SocketAddrV6,
}

impl DiscoveryContext {
    /// Select a local protocol/network namespace and supported IPv6 DNS authority.
    pub fn new(
        protocol: impl Into<String>,
        network_id: NetworkId,
        authority: SocketAddr,
    ) -> Result<Self> {
        let protocol = protocol.into();
        if protocol.trim().is_empty() {
            return Err(anyhow!("discovery protocol must not be empty"));
        }
        let SocketAddr::V6(authority) = authority else {
            return Err(anyhow!("discovery authority requires IPv6"));
        };
        if authority.ip().to_ipv4_mapped().is_some() || authority.port() == 0 {
            return Err(anyhow!(
                "discovery authority requires native IPv6 and nonzero port"
            ));
        }
        if authority.scope_id() != 0 {
            return Err(anyhow!(
                "scoped IPv6 authority unsupported by current transport"
            ));
        }
        Ok(Self {
            protocol,
            network_id,
            authority,
        })
    }

    pub fn protocol(&self) -> &str {
        &self.protocol
    }
    pub fn network_id(&self) -> &NetworkId {
        &self.network_id
    }
    pub fn authority(&self) -> SocketAddrV6 {
        self.authority
    }
}

/// Immutable pairing supplied by trusted local configuration.
/// The provider must enforce the selected authority; this is not proof it does so.
#[derive(Clone)]
pub struct DnsBinding {
    context: DiscoveryContext,
    client: Arc<dyn TrustChainDnsClient>,
}

impl DnsBinding {
    pub fn new(context: DiscoveryContext, client: Arc<dyn TrustChainDnsClient>) -> Self {
        Self { context, client }
    }
    pub fn context(&self) -> &DiscoveryContext {
        &self.context
    }
    pub(super) fn client(&self) -> &dyn TrustChainDnsClient {
        self.client.as_ref()
    }
}
