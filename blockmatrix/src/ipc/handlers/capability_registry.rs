// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase K.2 — IPC method-to-capability registry.
//!
//! Maps each registered IPC method name to the [`Capability`] that the
//! caller must possess in their session token. This is consulted by the
//! request-dispatch middleware before the handler runs.
//!
//! ## Fail-closed semantics
//!
//! Methods not in any explicit table fall through to
//! [`Capability::Admin`] — unknown methods cannot be invoked by
//! non-admin tokens. This protects newly-added handlers from being
//! accidentally exposed to lower-scope sessions.
//!
//! ## Tier ordering
//!
//! - [`Capability::ViewOnly`] — read-only queries
//! - [`Capability::Wallet`] — Caesar wallet operations
//! - [`Capability::AssetWrite`] — write asset entries (store, share,
//!   register DNS, etc.)
//! - [`Capability::Admin`] — privileged actions (shutdown, config write,
//!   foundation grants, system updates, session management)
//!
//! `Admin` is a superset; tokens carrying `Admin` pass every check.

#![deny(unsafe_code)]

use crate::auth::Capability;

/// Resolve the [`Capability`] required to invoke `method`.
///
/// Returns [`Capability::Admin`] for any method not explicitly listed
/// (fail-closed). New handlers should be added to the appropriate match
/// arm here when they ship.
pub fn required_capability(method: &str) -> Capability {
    match method {
        // ---- Read-only methods ----------------------------------------
        // Trivial / always-allowed
        "ping"
        // Status & topology
        | "status"
        | "topology.info"
        | "topology.neighbors"
        | "topology.routing_cost"
        | "topology.path"
        // Blockchain reads
        | "blockchain.height"
        | "blockchain.block"
        | "blockchain.validate"
        // Caesar reads
        | "caesar.balance"
        | "caesar.transactions"
        | "caesar.overview"
        // Engauge reads
        | "engauge.capacity"
        | "engauge.traffic"
        | "engauge.routing"
        | "engauge.throttle"
        // TrustChain reads
        | "trustchain.identity"
        | "trustchain.status"
        | "trustchain.certs"
        | "trustchain.federation"
        // Identity (read-only views of pubkey)
        | "identity.pubkey"
        | "peer.pubkey"
        // STOQ stats
        | "stoq.stats"
        | "stoq.connections"
        | "stoq.performance"
        // DNS / asset / dashboard / share / message reads
        | "dns.resolve"
        | "dns.list"
        | "asset.list"
        | "asset.info"
        | "dashboard.list"
        | "dashboard.info"
        | "share.inbox"
        | "message.inbox"
        | "message.history"
        // Network
        | "network.peers"
        // Domain reads
        | "domain.list"
        // Gateway reads
        | "gateway.status"
        | "gateway.list"
        // System / intelligence
        | "system.check_update"
        | "intelligence.stats"
        // Auth listings
        | "auth.list_sessions"
        // Cross-chain receipts
        | "chain.lookup_cross_receipt"
        // Config reads
        | "config.show"
        | "config.get"
            => Capability::ViewOnly,

        // ---- Wallet operations ----------------------------------------
        "caesar.transfer"
        | "caesar.staking"
        | "caesar.rewards"
            => Capability::Wallet,

        // ---- Asset-write operations -----------------------------------
        "asset.register"
        | "store"
        | "fetch"
        | "share.send"
        | "share.accept"
        | "share.reject"
        | "message.send"
        | "message.read"
        | "dashboard.deploy"
        | "dns.register"
        | "domain.register"
        | "domain.join"
        | "domain.create"
        | "gateway.transfer"
            => Capability::AssetWrite,

        // ---- Admin-only operations ------------------------------------
        "shutdown"
        | "config.set"
        | "dns.foundation_grant"
        | "auth.create_session"
        | "auth.revoke_session"
        | "system.apply_update"
        | "trustchain.request_cert"
        | "gateway.initiate_transfer"
        | "identity.rotate"
            => Capability::Admin,

        // ---- Default: fail-closed -------------------------------------
        _ => Capability::Admin,
    }
}

/// Methods that bypass capability enforcement entirely.
///
/// `auth.create_session` is the **only** bootstrap exception: it is
/// the method used to mint capability tokens, so requiring a token to
/// call it would be circular. The handler itself enforces its own
/// access policy: it rejects with "auth not configured" when the
/// issuer is `None` (alpha-default inert), and operators that opt in
/// to enforcement are expected to gate `auth.create_session` at the
/// transport layer (localhost-only socket + OS-level access controls
/// on the daemon socket file).
///
/// Pre-K.3 we keep this list intentionally small (1 method).
pub fn always_public(method: &str) -> bool {
    matches!(method, "auth.create_session")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_methods_map_to_view_only() {
        for m in &[
            "status",
            "ping",
            "blockchain.height",
            "blockchain.block",
            "caesar.balance",
            "caesar.transactions",
            "engauge.capacity",
            "trustchain.identity",
            "stoq.stats",
            "dns.resolve",
            "asset.list",
            "asset.info",
            "topology.info",
            "share.inbox",
            "message.inbox",
            "system.check_update",
            "auth.list_sessions",
            "chain.lookup_cross_receipt",
            "config.show",
        ] {
            assert_eq!(
                required_capability(m),
                Capability::ViewOnly,
                "method {m} should be ViewOnly",
            );
        }
    }

    #[test]
    fn wallet_methods_map_to_wallet() {
        for m in &[
            "caesar.transfer",
            "caesar.staking",
            "caesar.rewards",
        ] {
            assert_eq!(
                required_capability(m),
                Capability::Wallet,
                "method {m} should be Wallet",
            );
        }
    }

    #[test]
    fn write_methods_map_to_asset_write() {
        for m in &[
            "asset.register",
            "store",
            "fetch",
            "share.send",
            "share.accept",
            "message.send",
            "dashboard.deploy",
            "dns.register",
            "domain.register",
            "domain.join",
            "domain.create",
        ] {
            assert_eq!(
                required_capability(m),
                Capability::AssetWrite,
                "method {m} should be AssetWrite",
            );
        }
    }

    #[test]
    fn admin_methods_map_to_admin() {
        for m in &[
            "shutdown",
            "config.set",
            "dns.foundation_grant",
            "auth.create_session",
            "auth.revoke_session",
            "system.apply_update",
            "trustchain.request_cert",
            "gateway.initiate_transfer",
        ] {
            assert_eq!(
                required_capability(m),
                Capability::Admin,
                "method {m} should be Admin",
            );
        }
    }

    #[test]
    fn unknown_method_defaults_to_admin() {
        assert_eq!(required_capability("totally.fake.method"), Capability::Admin);
        assert_eq!(required_capability(""), Capability::Admin);
        assert_eq!(required_capability("future.experiment"), Capability::Admin);
    }

    #[test]
    fn always_public_only_auth_create_session() {
        // The bootstrap method short-circuits enforcement; everything
        // else routes through `required_capability`.
        assert!(always_public("auth.create_session"));
        assert!(!always_public("ping"));
        assert!(!always_public("status"));
        assert!(!always_public("auth.revoke_session"));
        assert!(!always_public("auth.list_sessions"));
    }
}
