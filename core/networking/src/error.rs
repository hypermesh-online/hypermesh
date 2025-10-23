//! Networking error types

use nexus_shared::NexusError;
use std::net::SocketAddr;

/// Result type alias for networking operations
pub type Result<T> = std::result::Result<T, NetworkError>;

/// Network-specific error types
#[derive(thiserror::Error, Debug)]
pub enum NetworkError {
    #[error("Service discovery error: {message}")]
    ServiceDiscovery { message: String },

    #[error("Load balancing error: {message}")]
    LoadBalancing { message: String },

    #[error("Circuit breaker error: {message}")]
    CircuitBreaker { message: String },

    #[error("Health check error: {message}")]
    HealthCheck { message: String },

    #[error("Routing error: {message}")]
    Routing { message: String },

    #[error("DHT error: {message}")]
    Dht { message: String },

    #[error("Transport error: {message}")]
    Transport { message: String },


    #[error("No healthy instances available for service: {service_name}")]
    NoHealthyInstances { service_name: String },

    #[error("Connection failed to {address}: {error}")]
    ConnectionFailed { address: SocketAddr, error: String },

    #[error("Request failed: {message}")]
    RequestFailed { message: String },

    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,
    
    #[error("Service not found: {service_id}")]
    ServiceNotFound { service_id: nexus_shared::ServiceId },
    
    #[error("No backends available for service: {service_id}")]
    NoBackendsAvailable { service_id: nexus_shared::ServiceId },
    
    #[error("No route found for path: {path}")]
    NoRouteFound { path: String },

    #[error("Timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("DNS resolution failed: {hostname}")]
    DnsResolution { hostname: String },

    #[error("Invalid address format: {address}")]
    InvalidAddress { address: String },

    #[error("Rate limit exceeded for {address}")]
    RateLimitExceeded { address: SocketAddr },

    #[error("Authentication failed: {reason}")]
    Authentication { reason: String },

    #[error("Authorization failed: {resource}")]
    Authorization { resource: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
}

impl NetworkError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            NetworkError::ConnectionFailed { .. } => true,
            NetworkError::RequestFailed { .. } => true,
            NetworkError::Timeout { .. } => true,
            NetworkError::ServiceDiscovery { .. } => true,
            NetworkError::DnsResolution { .. } => true,
            NetworkError::Io(_) => true,
            _ => false,
        }
    }

    /// Check if the error is related to service health
    pub fn is_health_related(&self) -> bool {
        matches!(
            self,
            NetworkError::NoHealthyInstances { .. } |
            NetworkError::CircuitBreakerOpen |
            NetworkError::HealthCheck { .. }
        )
    }

    /// Check if the error is related to configuration
    pub fn is_configuration_error(&self) -> bool {
        matches!(
            self,
            NetworkError::Configuration { .. } |
            NetworkError::InvalidAddress { .. }
        )
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            NetworkError::ServiceDiscovery { .. } => "service_discovery",
            NetworkError::LoadBalancing { .. } => "load_balancing",
            NetworkError::CircuitBreaker { .. } => "circuit_breaker",
            NetworkError::HealthCheck { .. } => "health_check",
            NetworkError::Routing { .. } => "routing",
            NetworkError::Dht { .. } => "dht",
            NetworkError::Transport { .. } => "transport",
            NetworkError::ServiceNotFound { .. } => "service_not_found",
            NetworkError::NoBackendsAvailable { .. } => "no_backends",
            NetworkError::NoRouteFound { .. } => "no_route",
            NetworkError::NoHealthyInstances { .. } => "no_healthy_instances",
            NetworkError::ConnectionFailed { .. } => "connection_failed",
            NetworkError::RequestFailed { .. } => "request_failed",
            NetworkError::CircuitBreakerOpen => "circuit_breaker_open",
            NetworkError::Timeout { .. } => "timeout",
            NetworkError::Configuration { .. } => "configuration",
            NetworkError::DnsResolution { .. } => "dns_resolution",
            NetworkError::InvalidAddress { .. } => "invalid_address",
            NetworkError::RateLimitExceeded { .. } => "rate_limit",
            NetworkError::Authentication { .. } => "authentication",
            NetworkError::Authorization { .. } => "authorization",
            NetworkError::Serialization(_) => "serialization",
            NetworkError::Io(_) => "io",
            NetworkError::Join(_) => "join",
            NetworkError::AddrParse(_) => "addr_parse",
        }
    }

    /// Get suggested action for the error
    pub fn suggested_action(&self) -> &'static str {
        match self {
            NetworkError::ServiceNotFound { .. } => "Check service name and registration",
            NetworkError::NoBackendsAvailable { .. } => "Register backends for the service",
            NetworkError::NoRouteFound { .. } => "Configure routing rules for the path",
            NetworkError::NoHealthyInstances { .. } => "Check service health and scaling",
            NetworkError::ConnectionFailed { .. } => "Verify network connectivity and service availability",
            NetworkError::CircuitBreakerOpen => "Wait for circuit breaker to close or check service health",
            NetworkError::Timeout { .. } => "Increase timeout or check service performance",
            NetworkError::Configuration { .. } => "Review configuration settings",
            NetworkError::DnsResolution { .. } => "Check DNS configuration and hostname",
            NetworkError::RateLimitExceeded { .. } => "Reduce request rate or increase limits",
            NetworkError::Authentication { .. } => "Check authentication credentials",
            NetworkError::Authorization { .. } => "Verify permissions and access rights",
            _ => "Check logs for more details",
        }
    }
}

impl From<NetworkError> for NexusError {
    fn from(err: NetworkError) -> Self {
        match err {
            NetworkError::Io(io_err) => NexusError::Network(io_err),
            NetworkError::Configuration { message } => NexusError::Config(message),
            NetworkError::Authentication { reason } => NexusError::Authentication { reason },
            NetworkError::Authorization { resource } => NexusError::Authorization { resource },
            NetworkError::Timeout { duration_ms } => NexusError::Timeout { duration_ms },
            other => NexusError::Internal {
                message: other.to_string(),
            },
        }
    }
}

/// Convenience macros for creating network errors
#[macro_export]
macro_rules! service_discovery_error {
    ($msg:expr) => {
        NetworkError::ServiceDiscovery {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! load_balancing_error {
    ($msg:expr) => {
        NetworkError::LoadBalancing {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! circuit_breaker_error {
    ($msg:expr) => {
        NetworkError::CircuitBreaker {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! routing_error {
    ($msg:expr) => {
        NetworkError::Routing {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! request_failed_error {
    ($msg:expr) => {
        NetworkError::RequestFailed {
            message: $msg.to_string(),
        }
    };
}