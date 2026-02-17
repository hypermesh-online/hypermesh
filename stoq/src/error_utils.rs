// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Error Construction Utilities
//!
//! Provides convenient builder patterns and helper functions for creating
//! well-structured errors with proper context throughout the STOQ stack.

use crate::errors::*;
use std::net::SocketAddr;

/// Transport error builder for structured error construction
pub struct TransportErrorBuilder {
    remote: Option<String>,
    stream_id: Option<u64>,
    operation: Option<String>,
}

impl TransportErrorBuilder {
    pub fn new() -> Self {
        Self {
            remote: None,
            stream_id: None,
            operation: None,
        }
    }

    pub fn remote(mut self, addr: impl Into<String>) -> Self {
        self.remote = Some(addr.into());
        self
    }

    pub fn stream_id(mut self, id: u64) -> Self {
        self.stream_id = Some(id);
        self
    }

    pub fn operation(mut self, op: impl Into<String>) -> Self {
        self.operation = Some(op.into());
        self
    }

    pub fn connection_failed(self, reason: impl Into<String>) -> StoqError {
        StoqError::Transport(TransportError::ConnectionFailed {
            remote: self.remote.unwrap_or_else(|| "unknown".to_string()),
            reason: reason.into(),
        })
    }

    pub fn connection_closed(self, reason: impl Into<String>) -> StoqError {
        StoqError::Transport(TransportError::ConnectionClosed {
            remote: self.remote.unwrap_or_else(|| "unknown".to_string()),
            reason: reason.into(),
        })
    }

    pub fn stream_error(self, reason: impl Into<String>) -> StoqError {
        StoqError::Transport(TransportError::StreamError {
            stream_id: self.stream_id,
            operation: self.operation.unwrap_or_else(|| "unknown".to_string()),
            reason: reason.into(),
        })
    }

    pub fn endpoint_unreachable(self) -> StoqError {
        StoqError::Transport(TransportError::EndpointUnreachable {
            remote: self.remote.unwrap_or_else(|| "unknown".to_string()),
        })
    }
}

impl Default for TransportErrorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Protocol error builder for structured error construction
pub struct ProtocolErrorBuilder {
    token_id: Option<Vec<u8>>,
    shard_id: Option<u32>,
    service_name: Option<String>,
}

impl ProtocolErrorBuilder {
    pub fn new() -> Self {
        Self {
            token_id: None,
            shard_id: None,
            service_name: None,
        }
    }

    pub fn token_id(mut self, id: Vec<u8>) -> Self {
        self.token_id = Some(id);
        self
    }

    pub fn shard_id(mut self, id: u32) -> Self {
        self.shard_id = Some(id);
        self
    }

    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }

    pub fn validation_failed(self, errors: Vec<String>) -> StoqError {
        StoqError::Protocol(ProtocolError::ValidationFailed {
            token_id: self.token_id.unwrap_or_default(),
            errors,
        })
    }

    pub fn token_expired(self, expired_at: u64, current_time: u64) -> StoqError {
        StoqError::Protocol(ProtocolError::TokenExpired {
            token_id: self.token_id.unwrap_or_default(),
            expired_at,
            current_time,
        })
    }

    pub fn invalid_proof(self, proof_type: ProofType, reason: impl Into<String>) -> StoqError {
        StoqError::Protocol(ProtocolError::InvalidProof {
            proof_type,
            reason: reason.into(),
        })
    }

    pub fn frame_decode_failed(self, frame_type: Option<u64>, reason: impl Into<String>) -> StoqError {
        StoqError::Protocol(ProtocolError::FrameDecodeFailed {
            frame_type,
            reason: reason.into(),
        })
    }

    pub fn frame_encode_failed(self, frame_type: impl Into<String>, reason: impl Into<String>) -> StoqError {
        StoqError::Protocol(ProtocolError::FrameEncodeFailed {
            frame_type: frame_type.into(),
            reason: reason.into(),
        })
    }

    pub fn shard_reassembly_failed(self, reason: impl Into<String>) -> StoqError {
        StoqError::Protocol(ProtocolError::ShardReassemblyFailed {
            shard_id: self.shard_id.unwrap_or(0),
            reason: reason.into(),
        })
    }

    pub fn token_replay_detected(self, token_hash: [u8; 32]) -> StoqError {
        StoqError::Protocol(ProtocolError::TokenReplayDetected { token_hash })
    }

    pub fn service_not_found(self) -> StoqError {
        StoqError::Protocol(ProtocolError::ServiceNotFound {
            service_name: self.service_name.unwrap_or_else(|| "unknown".to_string()),
        })
    }

    pub fn discovery_failed(self, reason: impl Into<String>) -> StoqError {
        StoqError::Protocol(ProtocolError::DiscoveryFailed {
            service_name: self.service_name.unwrap_or_else(|| "unknown".to_string()),
            reason: reason.into(),
        })
    }
}

impl Default for ProtocolErrorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenient error construction functions
pub mod transport {
    use super::*;

    pub fn connection_failed(remote: impl Into<String>, reason: impl Into<String>) -> StoqError {
        TransportErrorBuilder::new()
            .remote(remote)
            .connection_failed(reason)
    }

    pub fn connection_closed(remote: impl Into<String>, reason: impl Into<String>) -> StoqError {
        TransportErrorBuilder::new()
            .remote(remote)
            .connection_closed(reason)
    }

    pub fn stream_error(stream_id: Option<u64>, operation: impl Into<String>, reason: impl Into<String>) -> StoqError {
        let mut builder = TransportErrorBuilder::new().operation(operation);
        if let Some(id) = stream_id {
            builder = builder.stream_id(id);
        }
        builder.stream_error(reason)
    }

    pub fn bind_failed(address: impl Into<String>, port: u16, reason: impl Into<String>) -> StoqError {
        StoqError::Transport(TransportError::BindFailed {
            address: address.into(),
            port,
            reason: reason.into(),
        })
    }

    pub fn config_error(parameter: impl Into<String>, reason: impl Into<String>) -> StoqError {
        StoqError::Transport(TransportError::ConfigError {
            parameter: parameter.into(),
            reason: reason.into(),
        })
    }

    pub fn pool_exhausted(max_connections: usize) -> StoqError {
        StoqError::Transport(TransportError::PoolExhausted { max_connections })
    }

    pub fn endpoint_unreachable(remote: impl Into<String>) -> StoqError {
        TransportErrorBuilder::new()
            .remote(remote)
            .endpoint_unreachable()
    }
}

pub mod protocol {
    use super::*;

    pub fn validation_failed(token_id: Vec<u8>, errors: Vec<String>) -> StoqError {
        ProtocolErrorBuilder::new()
            .token_id(token_id)
            .validation_failed(errors)
    }

    pub fn token_expired(token_id: Vec<u8>, expired_at: u64, current_time: u64) -> StoqError {
        ProtocolErrorBuilder::new()
            .token_id(token_id)
            .token_expired(expired_at, current_time)
    }

    pub fn invalid_proof(proof_type: ProofType, reason: impl Into<String>) -> StoqError {
        ProtocolErrorBuilder::new()
            .invalid_proof(proof_type, reason)
    }

    pub fn frame_decode_failed(frame_type: Option<u64>, reason: impl Into<String>) -> StoqError {
        ProtocolErrorBuilder::new()
            .frame_decode_failed(frame_type, reason)
    }

    pub fn frame_encode_failed(frame_type: impl Into<String>, reason: impl Into<String>) -> StoqError {
        ProtocolErrorBuilder::new()
            .frame_encode_failed(frame_type, reason)
    }

    pub fn shard_reassembly_failed(shard_id: u32, reason: impl Into<String>) -> StoqError {
        ProtocolErrorBuilder::new()
            .shard_id(shard_id)
            .shard_reassembly_failed(reason)
    }

    pub fn token_replay_detected(token_hash: [u8; 32]) -> StoqError {
        ProtocolErrorBuilder::new()
            .token_replay_detected(token_hash)
    }

    pub fn service_not_found(service_name: impl Into<String>) -> StoqError {
        ProtocolErrorBuilder::new()
            .service_name(service_name)
            .service_not_found()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_error_builder() {
        let err = TransportErrorBuilder::new()
            .remote("[::1]:9292")
            .connection_failed("timeout");

        match err {
            StoqError::Transport(TransportError::ConnectionFailed { remote, reason }) => {
                assert_eq!(remote, "[::1]:9292");
                assert_eq!(reason, "timeout");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_protocol_error_builder() {
        let err = ProtocolErrorBuilder::new()
            .token_id(vec![1, 2, 3])
            .validation_failed(vec!["proof failed".to_string()]);

        match err {
            StoqError::Protocol(ProtocolError::ValidationFailed { token_id, errors }) => {
                assert_eq!(token_id, vec![1, 2, 3]);
                assert_eq!(errors.len(), 1);
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_transport_convenience_functions() {
        let err = transport::connection_failed("[::1]:9292", "connection refused");
        assert!(err.to_string().contains("connection refused"));

        let err = transport::stream_error(Some(42), "read", "stream reset");
        assert!(err.to_string().contains("Stream 42"));
    }

    #[test]
    fn test_protocol_convenience_functions() {
        let err = protocol::invalid_proof(ProofType::Stake, "insufficient stake");
        assert!(err.to_string().contains("ProofOfStake"));

        let err = protocol::token_replay_detected([1; 32]);
        assert!(err.to_string().contains("replay"));
    }
}
