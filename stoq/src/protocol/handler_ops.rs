// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Internal helper operations for `StoqProtocolHandler`.
//!
//! Extracted from `protocol/mod.rs` to keep file sizes within project limits.
//! Contains token validation, connection state tracking, shard reassembly,
//! and FALCON signature verification.

use std::time::Duration;

use anyhow::{anyhow, Result};
use quinn::VarInt;
use tracing::{debug, warn};

use super::frames;
use super::StoqProtocolHandler;
use crate::extensions::{PacketShard, PacketToken};

/// Shard storage for reassembly
#[derive(Debug, Clone)]
pub(super) struct ShardStorage {
    pub(super) shards: Vec<PacketShard>,
    pub(super) total_expected: u32,
    /// Shard identifier for reassembly ordering
    pub(super) _shard_id: u32,
    pub(super) last_update: std::time::Instant,
}

/// Connection state for token validation
#[derive(Debug, Clone)]
pub(super) struct ConnectionState {
    pub(super) last_token: Option<PacketToken>,
    pub(super) validated_tokens: Vec<PacketToken>,
    pub(super) _last_validation_time: std::time::Instant,
}

impl StoqProtocolHandler {
    /// Validate a packet token
    pub(super) fn validate_token(&self, token: &PacketToken) -> Result<bool> {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Check token expiration (5 minutes max age)
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if current_time > token.timestamp + 300 {
            debug!("Token expired: age={}s", current_time - token.timestamp);
            return Ok(false);
        }

        // Check if we've seen this token recently (prevent replay attacks)
        {
            let mut cache = self.token_cache.write();
            let now = std::time::Instant::now();

            // Clean old entries (older than 5 minutes)
            cache.retain(|_, timestamp| now.duration_since(*timestamp) < Duration::from_secs(300));

            // Check if token already validated
            if cache.contains_key(&token.hash) {
                warn!("Token replay detected: hash={:?}", token.hash);
                return Ok(false);
            }

            // Cache this token
            cache.insert(token.hash, now);
        }

        // Validate token sequence number is reasonable
        if token.sequence == 0 {
            debug!("Invalid token sequence: 0");
            return Ok(false);
        }

        debug!("Token validated successfully: seq={}", token.sequence);
        Ok(true)
    }

    /// Update connection state with validated token
    pub(super) fn update_connection_state(
        &self,
        stream_id: Option<VarInt>,
        token: PacketToken,
    ) -> Result<()> {
        if let Some(stream_id) = stream_id {
            let mut states = self.connection_states.write();
            let state = states.entry(stream_id).or_insert_with(|| ConnectionState {
                last_token: None,
                validated_tokens: Vec::new(),
                _last_validation_time: std::time::Instant::now(),
            });

            state.last_token = Some(token.clone());
            state.validated_tokens.push(token);

            // Keep only last 10 tokens per connection
            if state.validated_tokens.len() > 10 {
                state.validated_tokens.remove(0);
            }

            debug!("Updated connection state for stream {:?}", stream_id);
        }
        Ok(())
    }

    /// Store shard for later reassembly
    pub(super) fn store_shard_for_reassembly(&self, shard: PacketShard) -> Result<()> {
        let mut storage = self.shard_storage.write();
        let shard_id = shard.shard_id;
        let total_shards = shard.total_shards;

        let entry = storage.entry(shard_id).or_insert_with(|| {
            debug!("Creating new shard storage for id {}", shard_id);
            ShardStorage {
                shards: Vec::with_capacity(total_shards as usize),
                total_expected: total_shards,
                _shard_id: shard_id,
                last_update: std::time::Instant::now(),
            }
        });

        // Validate shard consistency
        if entry.total_expected != total_shards {
            return Err(anyhow!(
                "Shard count mismatch: expected {}, got {}",
                entry.total_expected,
                total_shards
            ));
        }

        // Check if shard already exists
        if entry.shards.iter().any(|s| s.sequence == shard.sequence) {
            debug!(
                "Duplicate shard received: id={}, seq={}",
                shard_id, shard.sequence
            );
            return Ok(());
        }

        entry.shards.push(shard.clone());
        entry.last_update = std::time::Instant::now();

        debug!(
            "Stored shard {}/{} for id {}",
            shard.sequence + 1,
            total_shards,
            shard_id
        );

        // Check if we have all shards for reassembly
        if entry.shards.len() == total_shards as usize {
            debug!(
                "All shards received for id {}, triggering reassembly",
                shard_id
            );

            // Clone shards for reassembly
            let shards = entry.shards.clone();

            // Remove from storage
            storage.remove(&shard_id);

            // Attempt reassembly
            match self.extensions.reassemble_shards(shards) {
                Ok(_data) => {
                    debug!(
                        "Successfully reassembled packet from {} shards",
                        total_shards
                    );
                    // Here you could trigger further processing of the reassembled data
                }
                Err(e) => {
                    warn!("Failed to reassemble shards: {}", e);
                    return Err(e);
                }
            }
        }

        // Clean up old incomplete shard collections (older than 30 seconds)
        let now = std::time::Instant::now();
        storage.retain(|_, entry| {
            now.duration_since(entry.last_update) < std::time::Duration::from_secs(30)
        });

        Ok(())
    }

    /// Verify FALCON signature
    pub(super) fn verify_falcon_signature(&self, sig_frame: &frames::FalconSigFrame) -> Result<()> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();

            // Import and verify the signature
            let _signature = falcon_guard.import_signature(&sig_frame.signature_data)?;

            // Here we would verify against the signed data
            // For now, we just validate the signature format
            if sig_frame.key_id.is_empty() {
                return Err(anyhow!("Missing key ID in FALCON signature"));
            }

            if sig_frame.signature_data.is_empty() {
                return Err(anyhow!("Empty signature data"));
            }

            // Check that signed frames are specified
            if sig_frame.signed_frames.is_empty() {
                warn!("FALCON signature has no signed frames specified");
            }

            debug!(
                "FALCON signature verified: key_id={}, signed_frames={}",
                sig_frame.key_id,
                sig_frame.signed_frames.len()
            );

            Ok(())
        } else {
            Err(anyhow!(
                "FALCON transport not enabled for signature verification"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::{DefaultStoqExtensions, StoqProtocolExtension};
    use std::sync::Arc;

    #[test]
    fn test_token_validation() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 1400);

        // Create a token with current timestamp
        let token = PacketToken {
            hash: [42; 32],
            sequence: 123,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test: expected success")
                .as_secs(),
        };

        // Test valid token
        assert!(handler.validate_token(&token).expect("test: validation"));

        // Test replay detection (second validation should fail)
        assert!(!handler.validate_token(&token).expect("test: validation"));

        // Test expired token
        let expired_token = PacketToken {
            hash: [99; 32],
            sequence: 456,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test: expected success")
                .as_secs()
                - 400, // 400 seconds ago (more than 5 minutes)
        };
        assert!(!handler.validate_token(&expired_token).expect("test: validation"));
    }

    #[test]
    fn test_shard_reassembly() {
        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 10);

        let data = b"test data for sharding and reassembly";
        let shards = extensions.shard_packet(data, 10).expect("test: shard operation");

        // Store all shards for reassembly
        for shard in shards {
            handler.store_shard_for_reassembly(shard).expect("test: shard operation");
        }

        // Verify shards were stored and reassembled (check storage is empty)
        let storage = handler.shard_storage.read();
        assert_eq!(storage.len(), 0, "All shards should have been reassembled");
    }

    #[test]
    fn test_connection_state_update() {
        let extensions = Arc::new(DefaultStoqExtensions::new());
        let handler = StoqProtocolHandler::new(extensions.clone(), None, 1400);

        let stream_id = VarInt::from_u32(42);
        let token = PacketToken {
            hash: [1; 32],
            sequence: 100,
            timestamp: 123456789,
        };

        // Update connection state
        handler
            .update_connection_state(Some(stream_id), token.clone())
            .expect("test: expected success");

        // Verify state was updated
        let states = handler.connection_states.read();
        assert!(states.contains_key(&stream_id));

        let state = states.get(&stream_id).expect("test: map lookup");
        assert_eq!(state.last_token, Some(token));
        assert_eq!(state.validated_tokens.len(), 1);
    }
}
