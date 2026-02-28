// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! PrivacyMode Consistency Integration Tests
//!
//! Verifies the two-axis privacy model (scope x tracked) is consistent
//! across hypermesh_lib and blockmatrix, including eBPF encoding and
//! Caesar reward multipliers.

use hypermesh_lib::{PrivacyMode, AccessScope};

#[test]
fn ebpf_encoding_matches_spec() {
    // ANONYMOUS=0, PRIVATE=2, PUBLIC=3
    assert_eq!(PrivacyMode::ANONYMOUS.to_ebpf_u8(), 0);
    assert_eq!(PrivacyMode::PRIVATE.to_ebpf_u8(), 2);
    assert_eq!(PrivacyMode::PUBLIC.to_ebpf_u8(), 3);
}

#[test]
fn caesar_multipliers_match_spec() {
    let eps = f64::EPSILON;
    assert!((PrivacyMode::ANONYMOUS.caesar_multiplier() - 0.0).abs() < eps);
    assert!((PrivacyMode::PRIVATE.caesar_multiplier() - 0.5).abs() < eps);
    assert!((PrivacyMode::PUBLIC.caesar_multiplier() - 1.0).abs() < eps);
}

#[test]
fn two_axis_model_correctness() {
    // ANONYMOUS: Unbounded scope, not tracked
    assert_eq!(PrivacyMode::ANONYMOUS.scope, AccessScope::Unbounded);
    assert!(!PrivacyMode::ANONYMOUS.tracked);

    // PRIVATE: Bounded scope, tracked
    assert_eq!(PrivacyMode::PRIVATE.scope, AccessScope::Bounded);
    assert!(PrivacyMode::PRIVATE.tracked);

    // PUBLIC: Unbounded scope, tracked
    assert_eq!(PrivacyMode::PUBLIC.scope, AccessScope::Unbounded);
    assert!(PrivacyMode::PUBLIC.tracked);
}

#[test]
fn blockmatrix_reexport_is_same_type() {
    // blockmatrix re-exports PrivacyMode from hypermesh_lib.
    // Compile-time: assignment between the two proves type identity.
    let lib_mode: hypermesh_lib::PrivacyMode = PrivacyMode::PUBLIC;
    let bm_mode: blockmatrix::PrivacyMode = lib_mode;
    assert_eq!(bm_mode.to_ebpf_u8(), 3);
    assert_eq!(bm_mode.caesar_multiplier(), 1.0);
}

#[test]
fn serde_round_trip_all_presets() {
    for mode in [PrivacyMode::ANONYMOUS, PrivacyMode::PRIVATE, PrivacyMode::PUBLIC] {
        let json = serde_json::to_string(&mode)
            .unwrap_or_else(|e| panic!("serialize {:?}: {}", mode, e));
        let back: PrivacyMode = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("deserialize {:?}: {}", mode, e));
        assert_eq!(mode, back);
    }
}

#[test]
fn graduated_connection_timeouts() {
    let anon = PrivacyMode::ANONYMOUS.connection_timeout_secs();
    let priv_ = PrivacyMode::PRIVATE.connection_timeout_secs();
    let pub_ = PrivacyMode::PUBLIC.connection_timeout_secs();
    assert!(anon < priv_, "anonymous timeout < private timeout");
    assert!(priv_ < pub_, "private timeout < public timeout");
}
