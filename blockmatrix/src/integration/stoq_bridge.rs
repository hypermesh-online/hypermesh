//! STOQ Integration Bridge - Replaces HTTP API Bridge
//!
//! Provides inter-component communication over STOQ protocol instead of HTTP REST APIs.
//! Connects HyperMesh, TrustChain, STOQ, Catalog, and Caesar via native STOQ messaging.

use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn, instrument};

use stoq::{StoqApiServer, StoqApiClient, ApiHandler, ApiRequest, ApiResponse, ApiError};
use stoq::transport::{StoqTransport, TransportConfig};

// Re-export for compatibility (types moved from api_bridge to stoq_bridge)
pub use UnifiedStoqBridge as UnifiedApiBridge;
pub use StoqBridgeConfig as ApiConfig;

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service endpoint
    pub endpoint: String,
}

/// Endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointInfo {
    /// Endpoint path
    pub path: String,
    /// Endpoint method
    pub method: String,
    /// Endpoint description
    pub description: String,
}

/// Asset request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRequest {
    /// Asset ID
    pub asset_id: String,
    /// Asset type
    pub asset_type: String,
    /// Request data
    pub data: serde_json::Value,
}

/// Asset response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetResponse {
    /// Asset ID
    pub asset_id: String,
    /// Response status
    pub success: bool,
    /// Response data
    pub data: serde_json::Value,
}

/// Certificate request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateRequest {
    /// Domain
    pub domain: String,
    /// Certificate type
    pub cert_type: String,
}

/// Certificate response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateResponse {
    /// Certificate
    pub certificate: Vec<u8>,
    /// Private key
    pub private_key: Vec<u8>,
}

/// Transaction request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    /// Transaction type
    pub tx_type: String,
    /// Transaction data
    pub data: serde_json::Value,
}

/// Transaction response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    /// Transaction ID
    pub tx_id: String,
    /// Transaction status
    pub status: String,
    /// Transaction result
    pub result: serde_json::Value,
}

/// Inter-component bridge configuration
#[derive(Debug, Clone)]
pub struct StoqBridgeConfig {
    /// Local service bind address
    pub bind_address: String,
    /// Service name for registration
    pub service_name: String,
    /// Enable request logging
    pub enable_logging: bool,
}

impl Default for StoqBridgeConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::1]:9292".to_string(),
            service_name: "hypermesh".to_string(),
            enable_logging: true,
        }
    }
}

/// Unified STOQ Bridge for all components
pub struct UnifiedStoqBridge {
    /// STOQ API server for receiving requests
    server: Arc<StoqApiServer>,
    /// STOQ API client for making requests
    client: Arc<StoqApiClient>,
    /// Configuration
    config: StoqBridgeConfig,
    /// STOQ transport
    transport: Arc<StoqTransport>,
}

impl UnifiedStoqBridge {
    /// Create new STOQ bridge
    #[instrument(skip(config))]
    pub async fn new(config: StoqBridgeConfig) -> Result<Self> {
        info!("Creating STOQ Integration Bridge for {}", config.service_name);

        // Parse bind address
        let bind_addr: std::net::Ipv6Addr = config.bind_address.split(':')
            .next()
            .and_then(|addr| addr.trim_matches(|c| c == '[' || c == ']').parse().ok())
            .ok_or_else(|| anyhow!("Invalid IPv6 bind address"))?;

        let port: u16 = config.bind_address.split(':')
            .nth(1)
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| anyhow!("Invalid port"))?;

        // Create STOQ transport
        let transport_config = TransportConfig {
            bind_address: bind_addr,
            port,
            ..Default::default()
        };

        let transport = Arc::new(StoqTransport::new(transport_config).await?);

        // Create server and client
        let server = Arc::new(StoqApiServer::new(Arc::clone(&transport)));
        let client = Arc::new(StoqApiClient::new(Arc::clone(&transport)));

        info!("STOQ bridge created for {}", config.service_name);

        Ok(Self {
            server,
            client,
            config,
            transport,
        })
    }

    /// Register an API handler
    pub fn register_handler(&self, handler: Arc<dyn ApiHandler>) {
        self.server.register_handler(handler);
    }

    /// Get API client for making requests
    pub fn client(&self) -> Arc<StoqApiClient> {
        Arc::clone(&self.client)
    }

    /// Start the bridge server
    #[instrument(skip(self))]
    pub async fn serve(&self) -> Result<()> {
        info!("Starting STOQ bridge server for {}", self.config.service_name);
        self.server.listen().await
    }

    /// Stop the bridge
    pub fn stop(&self) {
        info!("Stopping STOQ bridge for {}", self.config.service_name);
        self.server.stop();
    }

    /// Call a remote service method
    pub async fn call_service<T, R>(
        &self,
        service: &str,
        method: &str,
        payload: &T,
    ) -> Result<R, ApiError>
    where
        T: Serialize,
        R: serde::de::DeserializeOwned,
    {
        self.client.call(service, method, payload).await
    }
}

/// Example: TrustChain certificate validation via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidationRequest {
    pub certificate_pem: String,
    pub chain_pem: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidationResponse {
    pub valid: bool,
    pub error: Option<String>,
}

/// Example: HyperMesh asset registration via STOQ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRegistrationRequest {
    pub asset_type: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRegistrationResponse {
    pub asset_id: String,
    pub registered: bool,
}

/// Example usage:
/// ```rust,no_run
/// use blockmatrix::integration::stoq_bridge::*;
///
/// async fn example() -> Result<()> {
///     let bridge = UnifiedStoqBridge::new(StoqBridgeConfig::default()).await?;
///
///     // Call TrustChain for certificate validation
///     let request = CertificateValidationRequest {
///         certificate_pem: "...".to_string(),
///         chain_pem: vec![],
///     };
///
///     let response: CertificateValidationResponse = bridge
///         .call_service("trustchain", "validate_certificate", &request)
///         .await?;
///
///     Ok(())
/// }
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add STOQ bridge integration tests
}
