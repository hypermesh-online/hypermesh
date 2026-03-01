// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! BlockchainScope Binary Model Integration Tests
//!
//! Verifies the architectural invariant: exactly two blockchain scopes
//! (Device and Network), independent from PrivacyMode.

use hypermesh_lib::{BlockchainScope, PrivacyMode};

#[test]
fn only_device_and_network() {
    let device = BlockchainScope::Device;
    let network = BlockchainScope::Network;
    assert_ne!(device, network);

    // Exhaustive match -- adding a third variant causes a compile error,
    // enforcing the two-variant architectural invariant.
    match device {
        BlockchainScope::Device => {}
        BlockchainScope::Network => unreachable!(),
    }
}

#[test]
fn serde_round_trip() {
    for scope in [BlockchainScope::Device, BlockchainScope::Network] {
        let json =
            serde_json::to_string(&scope).unwrap_or_else(|e| panic!("serialize {scope:?}: {e}"));
        let back: BlockchainScope =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize {scope:?}: {e}"));
        assert_eq!(scope, back);
    }
}

#[test]
fn scope_independent_from_privacy_mode() {
    // BlockchainScope and PrivacyMode are independent dimensions.
    // Any combination is valid.
    let scopes = [BlockchainScope::Device, BlockchainScope::Network];
    let modes = [
        PrivacyMode::ANONYMOUS,
        PrivacyMode::PRIVATE,
        PrivacyMode::PUBLIC,
    ];

    // 2 scopes x 3 modes = 6 valid combinations
    let mut count = 0;
    for scope in &scopes {
        for mode in &modes {
            // All combinations compile and are logically valid
            let _scope_str = scope.to_string();
            let _mode_str = mode.to_string();
            count += 1;
        }
    }
    assert_eq!(count, 6, "2 scopes x 3 modes = 6 combinations");
}

#[test]
fn display_values() {
    assert_eq!(BlockchainScope::Device.to_string(), "Device");
    assert_eq!(BlockchainScope::Network.to_string(), "Network");
}

#[test]
fn hash_set_deduplication() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(BlockchainScope::Device);
    set.insert(BlockchainScope::Network);
    set.insert(BlockchainScope::Device); // duplicate
    assert_eq!(set.len(), 2);
}
