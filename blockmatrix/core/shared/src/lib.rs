//! Nexus Shared - Common utilities and types for core components
//! 
//! This crate provides foundational types, utilities, and abstractions
//! used across all Nexus core components.

pub mod error;
pub mod id;
pub mod metrics;
pub mod config;
pub mod crypto;
pub mod time;

pub use error::{NexusError, Result};
pub use id::{NodeId, ResourceId, ServiceId};
pub use config::NexusConfig;
pub use crypto::{KeyPair, AuthenticatedMessage, hash, random_bytes};
pub use time::{Timestamp, RateLimiter, TimeWindow};
pub use metrics::{MetricsCollector, Histogram};

/// Current version of the Nexus protocol
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum message size for inter-component communication (16MB)
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Default network buffer size
pub const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

/// Default timeout for network operations
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_INFO: &str = concat!(
    "version=", env!("CARGO_PKG_VERSION"),
    " built=unknown"
);

/// Initialize shared utilities and logging
pub fn init() -> Result<()> {
    // Initialize tracing subscriber
    if let Err(_) = tracing_subscriber::fmt()
        .with_env_filter("nexus=info")
        .with_target(false)
        .try_init()
    {
        // Subscriber already initialized, that's fine
    }

    tracing::info!("Nexus shared utilities initialized - {}", BUILD_INFO);
    Ok(())
}