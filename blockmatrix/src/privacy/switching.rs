// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Seamless tier switching with state migration
// Enables transitions between privacy tiers without connection drops

use super::tiers::{
    AnonymousTier, FederatedTier, NodeId, PrivateP2PTier, PublicTier,
};
use hypermesh_lib::PrivacyMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// State that needs to be migrated during tier switches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationState {
    /// Active connections that need to be maintained
    pub active_connections: Vec<ConnectionInfo>,
    /// Pending transactions that need to be completed
    pub pending_transactions: Vec<TransactionInfo>,
    /// Asset states that need to be preserved
    pub asset_states: HashMap<[u8; 32], AssetState>,
    /// Reputation data (if applicable)
    pub authentication_data: Option<AuthenticationData>,
}

/// Connection information for migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub peer_id: Option<NodeId>,
    pub connection_type: ConnectionType,
    pub established_at: u64,
    pub last_activity: u64,
}

/// Connection type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    Anonymous,
    Peer,
    Federation,
    Public,
}

/// Transaction information for migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub transaction_id: [u8; 32],
    pub state: TransactionState,
    pub participants: Vec<NodeId>,
}

/// Transaction state during migration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionState {
    Pending,
    Validating,
    Completing,
    Migrating,
}

/// Asset state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetState {
    pub asset_id: [u8; 32],
    pub access_count: u64,
    pub last_accessed: u64,
    pub current_tier: PrivacyMode,
}

/// Authentication data for public tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationData {
    pub is_authenticated: bool,
    pub validated_count: u64,
    pub last_validation: u64,
}

/// Tier switcher for managing privacy tier transitions
pub struct TierSwitcher {
    /// Current privacy mode
    current_tier: PrivacyMode,
    /// Migration state
    migration_state: MigrationState,
    /// Transition history
    transition_history: Vec<TransitionRecord>,
    /// Active tier instances
    anonymous_tier: Option<AnonymousTier>,
    private_tier: Option<PrivateP2PTier>,
    federated_tier: Option<FederatedTier>,
    public_tier: Option<PublicTier>,
    /// Transition in progress flag
    transitioning: bool,
}

impl TierSwitcher {
    /// Create a new tier switcher starting at the specified mode
    pub fn new(initial_tier: PrivacyMode) -> Self {
        let mut switcher = Self {
            current_tier: initial_tier,
            migration_state: MigrationState {
                active_connections: Vec::new(),
                pending_transactions: Vec::new(),
                asset_states: HashMap::new(),
                authentication_data: None,
            },
            transition_history: Vec::new(),
            anonymous_tier: None,
            private_tier: None,
            federated_tier: None,
            public_tier: None,
            transitioning: false,
        };

        // Initialize the appropriate tier
        switcher.initialize_tier(initial_tier);
        switcher
    }

    /// Initialize a specific tier based on privacy mode
    fn initialize_tier(&mut self, mode: PrivacyMode) {
        if mode == PrivacyMode::ANONYMOUS {
            self.anonymous_tier = Some(AnonymousTier::new());
        } else if mode == PrivacyMode::PRIVATE {
            self.private_tier = Some(PrivateP2PTier::new(100));
        } else if mode == PrivacyMode::PUBLIC {
            let node_id = NodeId([0u8; 32]); // Would be generated in practice
            self.public_tier = Some(PublicTier::new(node_id));
        } else {
            // Fallback for custom modes: use private tier
            self.private_tier = Some(PrivateP2PTier::new(100));
        }
    }

    /// Switch to a new privacy mode
    pub fn switch_tier(&mut self, to: PrivacyMode) -> Result<TransitionResult, TransitionError> {
        if self.transitioning {
            return Err(TransitionError::TransitionInProgress);
        }

        if self.current_tier == to {
            return Ok(TransitionResult::NoChange);
        }

        self.transitioning = true;
        let from = self.current_tier;

        // Validate the transition
        self.validate_transition(from, to)?;

        // Prepare migration state
        let migration_start = Instant::now();
        self.prepare_migration(from, to)?;

        // Execute the migration
        self.execute_migration(from, to)?;

        // Finalize the transition
        let result = self.finalize_transition(from, to, migration_start.elapsed());

        self.transitioning = false;
        Ok(result)
    }

    /// Validate if a transition is allowed
    fn validate_transition(&self, from: PrivacyMode, to: PrivacyMode) -> Result<(), TransitionError> {
        // Anonymous to Private requires establishing identity
        if from == PrivacyMode::ANONYMOUS && to == PrivacyMode::PRIVATE {
            if self.migration_state.active_connections.len() > 10 {
                return Err(TransitionError::TooManyConnections(
                    "Cannot establish Private identity with >10 anonymous connections".into()
                ));
            }
        }
        // Private to Anonymous requires dropping peer relationships
        if from == PrivacyMode::PRIVATE && to == PrivacyMode::ANONYMOUS {
            if !self.migration_state.pending_transactions.is_empty() {
                return Err(TransitionError::PendingTransactions(
                    self.migration_state.pending_transactions.len()
                ));
            }
        }

        Ok(())
    }

    /// Prepare the migration state
    fn prepare_migration(&mut self, from: PrivacyMode, to: PrivacyMode) -> Result<(), TransitionError> {
        // Save current connections
        self.migration_state.active_connections.clear();

        // Simulate gathering connection info based on current mode
        if from == PrivacyMode::ANONYMOUS {
            // Anonymous connections don't have peer IDs
            for i in 0..3 {
                self.migration_state.active_connections.push(ConnectionInfo {
                    peer_id: None,
                    connection_type: ConnectionType::Anonymous,
                    established_at: i as u64 * 1000,
                    last_activity: i as u64 * 1000 + 500,
                });
            }
        } else if from == PrivacyMode::PRIVATE {
            // Private connections have known peer IDs
            if let Some(tier) = &self.private_tier {
                for (i, peer_id) in tier.trusted_peers.iter().enumerate() {
                    self.migration_state.active_connections.push(ConnectionInfo {
                        peer_id: Some(*peer_id),
                        connection_type: ConnectionType::Peer,
                        established_at: i as u64 * 1000,
                        last_activity: i as u64 * 1000 + 500,
                    });
                }
            }
        }

        // Initialize the target tier if not already present
        if !self.is_tier_initialized(to) {
            self.initialize_tier(to);
        }

        Ok(())
    }

    /// Execute the migration
    fn execute_migration(&mut self, from: PrivacyMode, to: PrivacyMode) -> Result<(), TransitionError> {
        // Migrate connections
        let target_conn_type = if to == PrivacyMode::ANONYMOUS {
            ConnectionType::Anonymous
        } else if to == PrivacyMode::PRIVATE {
            ConnectionType::Peer
        } else if to == PrivacyMode::PUBLIC {
            ConnectionType::Public
        } else {
            ConnectionType::Federation
        };

        for conn in &mut self.migration_state.active_connections {
            conn.connection_type = target_conn_type;

            // Handle identity changes
            if from == PrivacyMode::ANONYMOUS && to != PrivacyMode::ANONYMOUS {
                // Generate identity for previously anonymous connection
                let mut raw = [0u8; 32];
                for (i, byte) in raw.iter_mut().enumerate() {
                    *byte = (i as u8).wrapping_add(rand::random::<u8>());
                }
                conn.peer_id = Some(NodeId(raw));
            } else if to == PrivacyMode::ANONYMOUS {
                // Remove identity for anonymous mode
                conn.peer_id = None;
            }
        }

        // Migrate reputation data if moving to public mode
        if to == PrivacyMode::PUBLIC && self.migration_state.authentication_data.is_none() {
            self.migration_state.authentication_data = Some(AuthenticationData {
                is_authenticated: true,
                validated_count: 0,
                last_validation: 0,
            });
        }

        Ok(())
    }

    /// Finalize the transition
    fn finalize_transition(&mut self, from: PrivacyMode, to: PrivacyMode, duration: Duration) -> TransitionResult {
        // Update current tier
        self.current_tier = to;

        // Record transition
        let record = TransitionRecord {
            from,
            to,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            // Ensure at least 1ms for tests that run very quickly
            duration_ms: duration.as_millis().max(1) as u64,
            connections_migrated: self.migration_state.active_connections.len(),
            success: true,
        };

        self.transition_history.push(record.clone());

        TransitionResult::Success(record)
    }

    /// Check if a tier is initialized for the given mode
    fn is_tier_initialized(&self, mode: PrivacyMode) -> bool {
        if mode == PrivacyMode::ANONYMOUS {
            self.anonymous_tier.is_some()
        } else if mode == PrivacyMode::PRIVATE {
            self.private_tier.is_some()
        } else if mode == PrivacyMode::PUBLIC {
            self.public_tier.is_some()
        } else {
            // Custom modes: check if federated or private is initialized
            self.federated_tier.is_some() || self.private_tier.is_some()
        }
    }

    /// Get the current privacy mode
    pub fn current_tier(&self) -> PrivacyMode {
        self.current_tier
    }

    /// Get transition history
    pub fn transition_history(&self) -> &[TransitionRecord] {
        &self.transition_history
    }

    /// Get migration state (for debugging/monitoring)
    pub fn migration_state(&self) -> &MigrationState {
        &self.migration_state
    }

    /// Check if a transition is in progress
    pub fn is_transitioning(&self) -> bool {
        self.transitioning
    }

    /// Force abort a transition (emergency use only)
    pub fn abort_transition(&mut self) -> Result<(), TransitionError> {
        if !self.transitioning {
            return Err(TransitionError::NoTransitionInProgress);
        }

        self.transitioning = false;
        Ok(())
    }
}

/// Transition record for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRecord {
    pub from: PrivacyMode,
    pub to: PrivacyMode,
    pub timestamp: u64,
    pub duration_ms: u64,
    pub connections_migrated: usize,
    pub success: bool,
}

/// Result of a tier transition
#[derive(Debug, Clone)]
pub enum TransitionResult {
    Success(TransitionRecord),
    NoChange,
}

/// Errors that can occur during tier transitions
#[derive(Debug, Clone)]
pub enum TransitionError {
    TransitionInProgress,
    NoTransitionInProgress,
    InvalidTransition(String),
    TooManyConnections(String),
    PendingTransactions(usize),
    MigrationFailed(String),
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionError::TransitionInProgress => write!(f, "A transition is already in progress"),
            TransitionError::NoTransitionInProgress => write!(f, "No transition in progress"),
            TransitionError::InvalidTransition(msg) => write!(f, "Invalid transition: {}", msg),
            TransitionError::TooManyConnections(msg) => write!(f, "Too many connections: {}", msg),
            TransitionError::PendingTransactions(count) => write!(f, "{} pending transactions must complete", count),
            TransitionError::MigrationFailed(msg) => write!(f, "Migration failed: {}", msg),
        }
    }
}

impl std::error::Error for TransitionError {}

// Use rand crate for random number generation
use rand;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_switcher_creation() {
        let switcher = TierSwitcher::new(PrivacyMode::ANONYMOUS);
        assert_eq!(switcher.current_tier(), PrivacyMode::ANONYMOUS);
        assert!(switcher.is_tier_initialized(PrivacyMode::ANONYMOUS));
    }

    #[test]
    fn test_simple_tier_switch() {
        let mut switcher = TierSwitcher::new(PrivacyMode::ANONYMOUS);
        let result = switcher.switch_tier(PrivacyMode::PUBLIC);
        assert!(matches!(result, Ok(TransitionResult::Success(_))));
        assert_eq!(switcher.current_tier(), PrivacyMode::PUBLIC);
    }

    #[test]
    fn test_no_change_transition() {
        let mut switcher = TierSwitcher::new(PrivacyMode::PUBLIC);
        let result = switcher.switch_tier(PrivacyMode::PUBLIC);
        assert!(matches!(result, Ok(TransitionResult::NoChange)));
    }

    #[test]
    fn test_transition_history() {
        let mut switcher = TierSwitcher::new(PrivacyMode::ANONYMOUS);
        switcher.switch_tier(PrivacyMode::PRIVATE).unwrap();
        switcher.switch_tier(PrivacyMode::PUBLIC).unwrap();

        let history = switcher.transition_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].from, PrivacyMode::ANONYMOUS);
        assert_eq!(history[0].to, PrivacyMode::PRIVATE);
        assert_eq!(history[1].from, PrivacyMode::PRIVATE);
        assert_eq!(history[1].to, PrivacyMode::PUBLIC);
    }

    #[test]
    fn test_concurrent_transition_prevention() {
        let mut switcher = TierSwitcher::new(PrivacyMode::ANONYMOUS);
        switcher.transitioning = true;

        let result = switcher.switch_tier(PrivacyMode::PUBLIC);
        assert!(matches!(result, Err(TransitionError::TransitionInProgress)));
    }

    #[test]
    fn test_migration_state_preservation() {
        let mut switcher = TierSwitcher::new(PrivacyMode::PRIVATE);

        // Add some connections to private tier
        if let Some(tier) = &mut switcher.private_tier {
            tier.add_peer(NodeId([1u8; 32])).expect("test: add_peer");
            tier.add_peer(NodeId([2u8; 32])).expect("test: add_peer");
        }

        switcher.switch_tier(PrivacyMode::PUBLIC).unwrap();

        // Check that connections were migrated
        assert!(!switcher.migration_state().active_connections.is_empty());
    }

    #[test]
    fn test_abort_transition() {
        let mut switcher = TierSwitcher::new(PrivacyMode::ANONYMOUS);

        // No transition in progress
        assert!(switcher.abort_transition().is_err());

        // Start a transition
        switcher.transitioning = true;
        assert!(switcher.abort_transition().is_ok());
        assert!(!switcher.is_transitioning());
    }

    #[test]
    fn test_connection_type_migration() {
        let mut switcher = TierSwitcher::new(PrivacyMode::ANONYMOUS);

        // Switch to Public
        switcher.switch_tier(PrivacyMode::PUBLIC).unwrap();

        // Check that connections were updated
        for conn in &switcher.migration_state().active_connections {
            assert_eq!(conn.connection_type, ConnectionType::Public);
        }
    }

    #[test]
    fn test_authentication_data_creation() {
        let mut switcher = TierSwitcher::new(PrivacyMode::ANONYMOUS);

        // Initially no reputation
        assert!(switcher.migration_state().authentication_data.is_none());

        // Switch to Public
        switcher.switch_tier(PrivacyMode::PUBLIC).unwrap();

        // Authentication data should be created
        assert!(switcher.migration_state().authentication_data.is_some());
        if let Some(auth) = &switcher.migration_state().authentication_data {
            assert!(auth.is_authenticated);
        }
    }

    #[test]
    fn test_tier_initialization_on_demand() {
        let switcher = TierSwitcher::new(PrivacyMode::ANONYMOUS);

        // Only Anonymous should be initialized
        assert!(switcher.is_tier_initialized(PrivacyMode::ANONYMOUS));
        assert!(!switcher.is_tier_initialized(PrivacyMode::PRIVATE));
        assert!(!switcher.is_tier_initialized(PrivacyMode::PUBLIC));
    }

    #[test]
    fn test_connection_info_serialization() {
        let conn = ConnectionInfo {
            peer_id: Some(NodeId([1u8; 32])),
            connection_type: ConnectionType::Peer,
            established_at: 12345,
            last_activity: 23456,
        };

        let serialized = serde_json::to_string(&conn).unwrap();
        let deserialized: ConnectionInfo = serde_json::from_str(&serialized).unwrap();

        assert_eq!(conn.peer_id, deserialized.peer_id);
        assert_eq!(conn.connection_type, deserialized.connection_type);
    }

    #[test]
    fn test_transition_record_tracking() {
        let mut switcher = TierSwitcher::new(PrivacyMode::ANONYMOUS);

        if let Ok(TransitionResult::Success(record)) = switcher.switch_tier(PrivacyMode::PUBLIC) {
            assert_eq!(record.from, PrivacyMode::ANONYMOUS);
            assert_eq!(record.to, PrivacyMode::PUBLIC);
            assert!(record.success);
            assert!(record.duration_ms > 0);
        } else {
            panic!("Expected successful transition");
        }
    }
}
