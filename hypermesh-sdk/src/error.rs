// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Error types for the HyperMesh SDK.

use std::time::Duration;

/// Errors returned by SDK operations.
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    /// Failed to connect to the daemon socket.
    #[error("Connection failed: {0}")]
    Connection(String),

    /// Low-level IPC transport error (read/write failure).
    #[error("IPC error: {0}")]
    Ipc(String),

    /// The daemon returned a JSON-RPC error response.
    #[error("RPC error (code {code}): {message}")]
    Rpc {
        /// JSON-RPC error code.
        code: i64,
        /// Human-readable error message.
        message: String,
    },

    /// Failed to serialize a request or deserialize a response.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// The request did not receive a response within the deadline.
    #[error("Request timed out after {0:?}")]
    Timeout(Duration),

    /// No active connection to the daemon.
    #[error("Not connected to daemon")]
    NotConnected,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_connection() {
        let err = SdkError::Connection("refused".into());
        assert_eq!(err.to_string(), "Connection failed: refused");
    }

    #[test]
    fn display_ipc() {
        let err = SdkError::Ipc("broken pipe".into());
        assert_eq!(err.to_string(), "IPC error: broken pipe");
    }

    #[test]
    fn display_rpc() {
        let err = SdkError::Rpc {
            code: -32601,
            message: "method not found".into(),
        };
        assert_eq!(err.to_string(), "RPC error (code -32601): method not found");
    }

    #[test]
    fn display_serialization() {
        let err = SdkError::Serialization("bad json".into());
        assert_eq!(err.to_string(), "Serialization error: bad json");
    }

    #[test]
    fn display_timeout() {
        let err = SdkError::Timeout(Duration::from_secs(30));
        assert_eq!(err.to_string(), "Request timed out after 30s");
    }

    #[test]
    fn display_not_connected() {
        let err = SdkError::NotConnected;
        assert_eq!(err.to_string(), "Not connected to daemon");
    }
}
