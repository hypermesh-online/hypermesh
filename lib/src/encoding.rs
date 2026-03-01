// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Canonical binary encoding for HyperMesh wire protocol.
//!
//! All cross-node serialization MUST use these functions to ensure
//! consistent encoding across implementations. Uses `postcard` for
//! compact, no_std-compatible binary encoding.

use serde::de::DeserializeOwned;
use serde::Serialize;

/// Encode a value to canonical binary format.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodingError> {
    postcard::to_allocvec(value).map_err(|e| EncodingError::Serialize(e.to_string()))
}

/// Decode a value from canonical binary format.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, EncodingError> {
    postcard::from_bytes(bytes).map_err(|e| EncodingError::Deserialize(e.to_string()))
}

/// Encode with a maximum size limit.
///
/// Returns error if encoded size exceeds `max_bytes`.
pub fn encode_bounded<T: Serialize>(
    value: &T,
    max_bytes: usize,
) -> Result<Vec<u8>, EncodingError> {
    let encoded = encode(value)?;
    if encoded.len() > max_bytes {
        return Err(EncodingError::SizeExceeded {
            actual: encoded.len(),
            limit: max_bytes,
        });
    }
    Ok(encoded)
}

/// Encoding/decoding errors.
#[derive(Debug, Clone)]
pub enum EncodingError {
    /// Serialization failed.
    Serialize(String),
    /// Deserialization failed.
    Deserialize(String),
    /// Encoded size exceeded limit.
    SizeExceeded { actual: usize, limit: usize },
}

impl std::fmt::Display for EncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(e) => write!(f, "encode error: {e}"),
            Self::Deserialize(e) => write!(f, "decode error: {e}"),
            Self::SizeExceeded { actual, limit } => {
                write!(f, "encoded size {actual} exceeds limit {limit}")
            }
        }
    }
}

impl std::error::Error for EncodingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestMsg {
        id: u32,
        data: Vec<u8>,
    }

    #[test]
    fn round_trip_encoding() {
        let msg = TestMsg {
            id: 42,
            data: vec![1, 2, 3],
        };
        let bytes = encode(&msg).expect("test: encode");
        let decoded: TestMsg = decode(&bytes).expect("test: decode");
        assert_eq!(msg, decoded);
    }

    #[test]
    fn bounded_encoding_accepts_small() {
        let msg = TestMsg {
            id: 1,
            data: vec![1, 2],
        };
        let result = encode_bounded(&msg, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn bounded_encoding_rejects_oversized() {
        let msg = TestMsg {
            id: 1,
            data: vec![0; 1000],
        };
        let result = encode_bounded(&msg, 10);
        assert!(matches!(result, Err(EncodingError::SizeExceeded { .. })));
    }

    #[test]
    fn decode_invalid_bytes() {
        let result: Result<TestMsg, _> = decode(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn encoding_error_display() {
        let err = EncodingError::SizeExceeded {
            actual: 100,
            limit: 50,
        };
        assert_eq!(err.to_string(), "encoded size 100 exceeds limit 50");
    }
}
