// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ accept loop for the gateway.
//!
//! Runs in parallel with the HTTP/3 listener, accepting STOQ connections
//! and dispatching them to a handler callback. Each accepted connection
//! is spawned as an independent tokio task so the accept loop is never
//! blocked by individual connection processing.

use std::sync::Arc;

use anyhow::Result;
use tracing::{error, info};

use crate::stoq_bridge::StoqBridge;

/// Metadata extracted from an incoming STOQ connection.
#[derive(Debug, Clone)]
pub struct StoqConnectionInfo {
    /// Unique identifier for the connection.
    pub connection_id: String,
    /// Remote address of the peer (formatted as a string).
    pub remote_addr: String,
    /// Privacy mode configured on the bridge at accept time.
    pub privacy_mode: hypermesh_lib::PrivacyMode,
    /// Blockchain scope configured on the bridge at accept time.
    pub blockchain_scope: hypermesh_lib::BlockchainScope,
}

/// STOQ connection accept loop.
///
/// Wraps a `StoqBridge` and continuously accepts incoming connections,
/// extracting connection metadata and dispatching each to a user-provided
/// handler function.
pub struct StoqListener {
    bridge: Arc<StoqBridge>,
}

impl StoqListener {
    /// Create a new listener backed by the given bridge.
    pub fn new(bridge: Arc<StoqBridge>) -> Self {
        Self { bridge }
    }

    /// Run the accept loop.
    ///
    /// For each accepted connection, extracts `StoqConnectionInfo` and
    /// invokes `handler` in a spawned task. The loop continues until the
    /// bridge is shut down or a fatal accept error occurs.
    ///
    /// Non-fatal accept errors cause a brief back-off before retrying.
    pub async fn run<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(StoqConnectionInfo) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        info!("STOQ listener started");

        let mut consecutive_errors: u32 = 0;
        let max_consecutive_errors: u32 = 50;

        loop {
            match self.bridge.accept_connection().await {
                Ok(conn) => {
                    consecutive_errors = 0;

                    let info = StoqConnectionInfo {
                        connection_id: conn.id(),
                        remote_addr: conn.endpoint().to_socket_addr().to_string(),
                        privacy_mode: self.bridge.privacy_mode(),
                        blockchain_scope: self.bridge.blockchain_scope(),
                    };

                    info!(
                        connection_id = %info.connection_id,
                        remote = %info.remote_addr,
                        "STOQ connection accepted"
                    );

                    let handler = handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handler(info).await {
                            error!("STOQ connection handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    consecutive_errors += 1;
                    error!(
                        error = %e,
                        consecutive = consecutive_errors,
                        "STOQ accept error"
                    );

                    if consecutive_errors >= max_consecutive_errors {
                        error!(
                            "Too many consecutive accept errors ({}), stopping STOQ listener",
                            consecutive_errors
                        );
                        return Err(e);
                    }

                    // Brief back-off before retrying; increases with consecutive errors
                    let backoff_ms = std::cmp::min(
                        100 * u64::from(consecutive_errors),
                        5000,
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }

    /// Get a reference to the underlying bridge.
    pub fn bridge(&self) -> &Arc<StoqBridge> {
        &self.bridge
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::{BlockchainScope, PrivacyMode};

    #[test]
    fn connection_info_construction() {
        let info = StoqConnectionInfo {
            connection_id: "conn-42".to_string(),
            remote_addr: "[::1]:9292".to_string(),
            privacy_mode: PrivacyMode::PUBLIC,
            blockchain_scope: BlockchainScope::Device,
        };

        assert_eq!(info.connection_id, "conn-42");
        assert_eq!(info.remote_addr, "[::1]:9292");
        assert_eq!(info.privacy_mode, PrivacyMode::PUBLIC);
        assert_eq!(info.blockchain_scope, BlockchainScope::Device);
    }

    #[test]
    fn connection_info_clone() {
        let info = StoqConnectionInfo {
            connection_id: "conn-1".to_string(),
            remote_addr: "[::1]:8444".to_string(),
            privacy_mode: PrivacyMode::ANONYMOUS,
            blockchain_scope: BlockchainScope::Network,
        };

        let cloned = info.clone();
        assert_eq!(cloned.connection_id, info.connection_id);
        assert_eq!(cloned.remote_addr, info.remote_addr);
        assert_eq!(cloned.privacy_mode, info.privacy_mode);
        assert_eq!(cloned.blockchain_scope, info.blockchain_scope);
    }

    #[test]
    fn connection_info_debug_format() {
        let info = StoqConnectionInfo {
            connection_id: "test-id".to_string(),
            remote_addr: "[::1]:1234".to_string(),
            privacy_mode: PrivacyMode::PRIVATE,
            blockchain_scope: BlockchainScope::Device,
        };

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("test-id"));
        assert!(debug_str.contains("[::1]:1234"));
    }
}
