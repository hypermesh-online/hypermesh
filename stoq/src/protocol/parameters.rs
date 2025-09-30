//! STOQ Transport Parameters for QUIC
//!
//! This module handles custom transport parameters for STOQ protocol extensions
//! that are negotiated during the QUIC handshake.

use bytes::{BufMut, BytesMut};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use tracing::{debug, trace};

use super::transport_params;

/// STOQ transport parameters negotiated during handshake
#[derive(Debug, Clone)]
pub struct StoqParameters {
    /// Whether STOQ protocol extensions are enabled
    pub extensions_enabled: bool,

    /// Whether FALCON quantum crypto is enabled
    pub falcon_enabled: bool,

    /// Peer's FALCON public key (if provided)
    pub falcon_public_key: Option<Vec<u8>>,

    /// Maximum shard size for packet fragmentation
    pub max_shard_size: u32,

    /// Tokenization algorithm identifier
    pub token_algorithm: TokenAlgorithm,

    /// Custom parameters
    pub custom: HashMap<u64, Vec<u8>>,
}

/// Tokenization algorithms supported
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenAlgorithm {
    /// SHA-256 based tokenization (default)
    Sha256,
    /// SHA-384 based tokenization
    Sha384,
    /// SHA3-256 based tokenization
    Sha3_256,
    /// Blake3 based tokenization
    Blake3,
}

impl Default for TokenAlgorithm {
    fn default() -> Self {
        Self::Sha256
    }
}

impl TokenAlgorithm {
    /// Convert to wire format ID
    pub fn to_id(&self) -> u8 {
        match self {
            Self::Sha256 => 0,
            Self::Sha384 => 1,
            Self::Sha3_256 => 2,
            Self::Blake3 => 3,
        }
    }

    /// Parse from wire format ID
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Sha256),
            1 => Some(Self::Sha384),
            2 => Some(Self::Sha3_256),
            3 => Some(Self::Blake3),
            _ => None,
        }
    }
}

impl Default for StoqParameters {
    fn default() -> Self {
        Self {
            extensions_enabled: true,
            falcon_enabled: false,
            falcon_public_key: None,
            max_shard_size: 1400, // Default MTU-safe size
            token_algorithm: TokenAlgorithm::default(),
            custom: HashMap::new(),
        }
    }
}

impl StoqParameters {
    /// Create parameters for client
    pub fn client_default() -> Self {
        Self {
            extensions_enabled: true,
            falcon_enabled: true, // Offer FALCON support
            falcon_public_key: None,
            max_shard_size: 9000, // Support jumbo frames
            token_algorithm: TokenAlgorithm::Sha256,
            custom: HashMap::new(),
        }
    }

    /// Create parameters for server
    pub fn server_default() -> Self {
        Self {
            extensions_enabled: true,
            falcon_enabled: true, // Support FALCON
            falcon_public_key: None,
            max_shard_size: 9000, // Support jumbo frames
            token_algorithm: TokenAlgorithm::Sha256,
            custom: HashMap::new(),
        }
    }

    /// Encode parameters for wire format
    pub fn encode(&self) -> Vec<(u64, Vec<u8>)> {
        let mut params = Vec::new();

        // STOQ extensions enabled
        params.push((
            transport_params::STOQ_EXTENSIONS_ENABLED,
            vec![if self.extensions_enabled { 1 } else { 0 }],
        ));

        // FALCON enabled
        params.push((
            transport_params::FALCON_ENABLED,
            vec![if self.falcon_enabled { 1 } else { 0 }],
        ));

        // FALCON public key
        if let Some(ref key) = self.falcon_public_key {
            params.push((
                transport_params::FALCON_PUBLIC_KEY,
                key.clone(),
            ));
        }

        // Maximum shard size
        params.push((
            transport_params::MAX_SHARD_SIZE,
            self.max_shard_size.to_be_bytes().to_vec(),
        ));

        // Token algorithm
        params.push((
            transport_params::TOKEN_ALGORITHM,
            vec![self.token_algorithm.to_id()],
        ));

        // Add custom parameters
        for (id, value) in &self.custom {
            params.push((*id, value.clone()));
        }

        trace!("Encoded {} STOQ transport parameters", params.len());
        params
    }

    /// Decode parameters from wire format
    pub fn decode(params: &[(u64, Vec<u8>)]) -> Result<Self> {
        let mut result = Self::default();

        for (id, value) in params {
            match *id {
                transport_params::STOQ_EXTENSIONS_ENABLED => {
                    if value.len() != 1 {
                        return Err(anyhow!("Invalid STOQ_EXTENSIONS_ENABLED parameter"));
                    }
                    result.extensions_enabled = value[0] != 0;
                    debug!("STOQ extensions: {}", result.extensions_enabled);
                }
                transport_params::FALCON_ENABLED => {
                    if value.len() != 1 {
                        return Err(anyhow!("Invalid FALCON_ENABLED parameter"));
                    }
                    result.falcon_enabled = value[0] != 0;
                    debug!("FALCON enabled: {}", result.falcon_enabled);
                }
                transport_params::FALCON_PUBLIC_KEY => {
                    if value.is_empty() {
                        return Err(anyhow!("Empty FALCON_PUBLIC_KEY parameter"));
                    }
                    result.falcon_public_key = Some(value.clone());
                    debug!("Received FALCON public key: {} bytes", value.len());
                }
                transport_params::MAX_SHARD_SIZE => {
                    if value.len() != 4 {
                        return Err(anyhow!("Invalid MAX_SHARD_SIZE parameter"));
                    }
                    result.max_shard_size = u32::from_be_bytes([
                        value[0], value[1], value[2], value[3]
                    ]);
                    debug!("Max shard size: {}", result.max_shard_size);
                }
                transport_params::TOKEN_ALGORITHM => {
                    if value.len() != 1 {
                        return Err(anyhow!("Invalid TOKEN_ALGORITHM parameter"));
                    }
                    result.token_algorithm = TokenAlgorithm::from_id(value[0])
                        .ok_or_else(|| anyhow!("Unknown token algorithm: {}", value[0]))?;
                    debug!("Token algorithm: {:?}", result.token_algorithm);
                }
                id if id >= 0xfe00 && id <= 0xfeff => {
                    // Custom STOQ parameter range
                    result.custom.insert(id, value.clone());
                    trace!("Custom parameter 0x{:04x}: {} bytes", id, value.len());
                }
                _ => {
                    // Ignore unknown parameters (forward compatibility)
                    trace!("Ignoring unknown parameter 0x{:04x}", id);
                }
            }
        }

        Ok(result)
    }

    /// Negotiate parameters between client and server
    pub fn negotiate(client: &Self, server: &Self) -> Self {
        Self {
            // Extensions enabled if both support
            extensions_enabled: client.extensions_enabled && server.extensions_enabled,

            // FALCON enabled if both support
            falcon_enabled: client.falcon_enabled && server.falcon_enabled,

            // Exchange public keys
            falcon_public_key: server.falcon_public_key.clone(), // Client receives server's key

            // Use minimum of max shard sizes
            max_shard_size: client.max_shard_size.min(server.max_shard_size),

            // Server chooses token algorithm
            token_algorithm: server.token_algorithm,

            // Merge custom parameters (server wins conflicts)
            custom: {
                let mut custom = client.custom.clone();
                custom.extend(server.custom.clone());
                custom
            },
        }
    }

    /// Check if parameters are compatible
    pub fn is_compatible(&self, other: &Self) -> bool {
        // Must agree on extensions
        if self.extensions_enabled != other.extensions_enabled {
            return false;
        }

        // If FALCON is required by either, both must support
        if (self.falcon_enabled && !other.falcon_enabled) ||
           (!self.falcon_enabled && other.falcon_enabled && other.falcon_public_key.is_some()) {
            return false;
        }

        true
    }
}

/// Parameter encoder for building transport parameters
pub struct ParameterEncoder {
    buffer: BytesMut,
}

impl ParameterEncoder {
    /// Create new encoder
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
        }
    }

    /// Add a parameter
    pub fn add_param(&mut self, id: u64, value: &[u8]) {
        // Encode parameter ID as varint
        self.encode_varint(id);

        // Encode parameter length as varint
        self.encode_varint(value.len() as u64);

        // Add parameter value
        self.buffer.put_slice(value);
    }

    /// Encode a variable-length integer
    fn encode_varint(&mut self, val: u64) {
        if val < 0x40 {
            self.buffer.put_u8(val as u8);
        } else if val < 0x4000 {
            self.buffer.put_u16((0x4000 | val) as u16);
        } else if val < 0x40000000 {
            self.buffer.put_u32((0x80000000 | val) as u32);
        } else {
            self.buffer.put_u64(0xc000000000000000 | val);
        }
    }

    /// Build the encoded parameters
    pub fn build(self) -> Vec<u8> {
        self.buffer.freeze().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_encoding() {
        let params = StoqParameters {
            extensions_enabled: true,
            falcon_enabled: true,
            falcon_public_key: Some(vec![1, 2, 3, 4, 5]),
            max_shard_size: 2048,
            token_algorithm: TokenAlgorithm::Blake3,
            custom: HashMap::new(),
        };

        let encoded = params.encode();
        assert!(!encoded.is_empty());

        // Decode and verify
        let decoded = StoqParameters::decode(&encoded).unwrap();
        assert_eq!(decoded.extensions_enabled, params.extensions_enabled);
        assert_eq!(decoded.falcon_enabled, params.falcon_enabled);
        assert_eq!(decoded.falcon_public_key, params.falcon_public_key);
        assert_eq!(decoded.max_shard_size, params.max_shard_size);
        assert_eq!(decoded.token_algorithm, params.token_algorithm);
    }

    #[test]
    fn test_parameter_negotiation() {
        let client = StoqParameters {
            extensions_enabled: true,
            falcon_enabled: true,
            falcon_public_key: None,
            max_shard_size: 9000,
            token_algorithm: TokenAlgorithm::Sha256,
            custom: HashMap::new(),
        };

        let server = StoqParameters {
            extensions_enabled: true,
            falcon_enabled: true,
            falcon_public_key: Some(vec![10, 20, 30]),
            max_shard_size: 1500,
            token_algorithm: TokenAlgorithm::Blake3,
            custom: HashMap::new(),
        };

        let negotiated = StoqParameters::negotiate(&client, &server);
        assert!(negotiated.extensions_enabled);
        assert!(negotiated.falcon_enabled);
        assert_eq!(negotiated.max_shard_size, 1500); // Minimum
        assert_eq!(negotiated.token_algorithm, TokenAlgorithm::Blake3); // Server choice
        assert_eq!(negotiated.falcon_public_key, Some(vec![10, 20, 30]));
    }

    #[test]
    fn test_compatibility() {
        let params1 = StoqParameters {
            extensions_enabled: true,
            falcon_enabled: false,
            ..Default::default()
        };

        let params2 = StoqParameters {
            extensions_enabled: true,
            falcon_enabled: false,
            ..Default::default()
        };

        assert!(params1.is_compatible(&params2));

        let params3 = StoqParameters {
            extensions_enabled: false,
            ..Default::default()
        };

        assert!(!params1.is_compatible(&params3));
    }
}