// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use std::net::SocketAddr;
use thiserror::Error;

/// Errors that can occur during gateway operation.
#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("no backend found for path: {path}")]
    NoBackendForPath { path: String },

    #[error("no backend found for domain: {domain}")]
    NoBackendForDomain { domain: String },

    #[error("authentication failed: {reason}")]
    AuthFailed { reason: String },

    #[error("rate limit exceeded for {client}")]
    RateLimitExceeded { client: String },

    #[error("backend unavailable: {backend} at {addr}")]
    BackendUnavailable { backend: String, addr: SocketAddr },

    #[error("protocol bridge error: {0}")]
    ProtocolBridge(String),

    #[error("scope routing error: {0}")]
    ScopeRouting(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl GatewayError {
    /// Map the error variant to an HTTP status code.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::NoBackendForPath { .. } | Self::NoBackendForDomain { .. } => 404,
            Self::AuthFailed { .. } => 401,
            Self::RateLimitExceeded { .. } => 429,
            Self::BackendUnavailable { .. } => 503,
            Self::ProtocolBridge(_) | Self::ScopeRouting(_) => 502,
            Self::Config(_) | Self::Tls(_) | Self::Internal(_) => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_codes_map_correctly() {
        let cases: Vec<(GatewayError, u16)> = vec![
            (
                GatewayError::NoBackendForPath {
                    path: "/foo".into(),
                },
                404,
            ),
            (
                GatewayError::NoBackendForDomain {
                    domain: "x.com".into(),
                },
                404,
            ),
            (
                GatewayError::AuthFailed {
                    reason: "bad token".into(),
                },
                401,
            ),
            (
                GatewayError::RateLimitExceeded {
                    client: "1.2.3.4".into(),
                },
                429,
            ),
            (
                GatewayError::BackendUnavailable {
                    backend: "tc".into(),
                    addr: "[::1]:50053".parse().expect("test: valid addr"),
                },
                503,
            ),
            (GatewayError::ProtocolBridge("x".into()), 502),
            (GatewayError::ScopeRouting("y".into()), 502),
            (GatewayError::Config("bad".into()), 500),
            (GatewayError::Tls("oops".into()), 500),
        ];

        for (err, expected) in cases {
            assert_eq!(err.status_code(), expected, "wrong code for {err}");
        }
    }

    #[test]
    fn display_messages_contain_context() {
        let err = GatewayError::NoBackendForPath {
            path: "/api/v1/missing".into(),
        };
        assert!(err.to_string().contains("/api/v1/missing"));

        let err = GatewayError::BackendUnavailable {
            backend: "trustchain".into(),
            addr: "[::1]:50053".parse().expect("test: valid addr"),
        };
        let msg = err.to_string();
        assert!(msg.contains("trustchain"));
        assert!(msg.contains("50053"));
    }

    #[test]
    fn internal_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("something broke");
        let gw_err: GatewayError = anyhow_err.into();
        assert_eq!(gw_err.status_code(), 500);
        assert!(gw_err.to_string().contains("something broke"));
    }
}
