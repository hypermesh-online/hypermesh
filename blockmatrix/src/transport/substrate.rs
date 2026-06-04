// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Substrate → STOQ injection seam (Substrate Phase A).
//!
//! The Substrate layer (`base`) produces a node's sovereign `fd48:4d00::/32`
//! address and its reachability; STOQ takes plain `Ipv6Addr`/`Option<Ipv6Addr>`
//! values and never depends on `base`. This module is the single place where
//! `blockmatrix` (the orchestration layer) bridges the two: it constructs a
//! [`base::DefaultSubstrate`], derives the address from the node's identity, and
//! applies the Substrate-derived values onto a `stoq::TransportConfig`.
//!
//! ## Why advertise, not bind (Phase A)
//! The derived `fd48:4d00::/32` address is the node's *identity* address. In
//! Phase A it is **advertised** (`public_ipv6`), not **bound** (`bind_address`):
//! the address is not yet assigned to a live OS interface (that is Phase B,
//! R16), so binding a socket to it would fail. We therefore leave the caller's
//! existing `bind_address` (`::`/`localhost`) untouched and set `public_ipv6` to
//! the sovereign address. This matches STOQ's existing advertise seam
//! (`public_ipv6().or(bind_address)`) and keeps every node binding successfully.
//! `ebpf_interface` stays `None` (Phase B fills it via `active_interface()`).
//!
//! ## Layering
//! `blockmatrix` imports `base`; `base` depends only on `hypermesh-lib`. STOQ is
//! never touched. Verified by `cargo tree -p stoq | grep base` staying empty.

use base::{DefaultSubstrate, Substrate};
use hypermesh_lib::NodeId;
use stoq::TransportConfig;

use crate::transport::error::TransportError;

/// Apply Substrate-derived addressing onto an existing `TransportConfig`.
///
/// Derives the node's sovereign `fd48:4d00::/32` address from `node_id` (R15) and
/// advertises it via `public_ipv6`. `bind_address` is left as the caller set it
/// (Phase A does not bind the identity address — see module docs), and
/// `ebpf_interface` is left `None` (Phase B).
///
/// The derived address is logged so an operator can see the sovereign address the
/// node advertises even before Phase B assigns it to an interface.
///
/// Reachability is consulted: if the Substrate has learned an externally observed
/// address (e.g. via a reflector echo, Phase A Part 2), that address is advertised
/// instead of the derived one. Until then reachability is `Unknown` and the
/// derived address is advertised as-is.
pub async fn apply_substrate_addressing(
    config: &mut TransportConfig,
    node_id: &NodeId,
) -> Result<(), TransportError> {
    let substrate = DefaultSubstrate::new();

    let derived = substrate
        .local_address(node_id)
        .await
        .map_err(|e| TransportError::Configuration(format!("substrate address derivation: {e}")))?;

    let reach = substrate
        .reachability()
        .await
        .map_err(|e| TransportError::Configuration(format!("substrate reachability: {e}")))?;

    // Advertise the externally observed address when known, else the sovereign
    // derived address. Phase A Part 1: reachability is Unknown -> advertise derived.
    let advertised = reach.public_v6.unwrap_or(derived);

    tracing::info!(
        node_id = %node_id,
        derived_address = %derived,
        advertised_address = %advertised,
        reachability = ?reach.path,
        "Substrate: advertising sovereign address (bind_address unchanged; \
         interface assignment is Phase B)"
    );

    config.public_ipv6 = Some(advertised);
    // bind_address: left as caller set it — Phase A does not bind the ULA.
    // ebpf_interface: left None — Phase B resolves it via active_interface().

    Ok(())
}

/// Build a `TransportConfig` from `base_config` with Substrate addressing applied.
///
/// Convenience wrapper over [`apply_substrate_addressing`] for call sites that
/// want a fresh config value rather than mutating in place.
pub async fn substrate_transport_config(
    mut base_config: TransportConfig,
    node_id: &NodeId,
) -> Result<TransportConfig, TransportError> {
    apply_substrate_addressing(&mut base_config, node_id).await?;
    Ok(base_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base::{derive_address, SUBNET_DEVICE_SCOPE};

    fn node(name: &[u8]) -> NodeId {
        NodeId::from_public_key(name)
    }

    #[tokio::test]
    async fn advertises_derived_address_without_changing_bind() {
        let nid = node(b"inject-test-key");
        let mut config = TransportConfig {
            bind_address: std::net::Ipv6Addr::UNSPECIFIED,
            ..TransportConfig::default()
        };

        apply_substrate_addressing(&mut config, &nid)
            .await
            .expect("apply substrate addressing");

        let expected = derive_address(&nid, SUBNET_DEVICE_SCOPE).expect("derive");
        assert_eq!(config.public_ipv6, Some(expected));
        // Bind address must remain bindable (unchanged).
        assert_eq!(config.bind_address, std::net::Ipv6Addr::UNSPECIFIED);
        // Phase B field stays None.
        assert_eq!(config.ebpf_interface, None);
    }

    #[tokio::test]
    async fn builder_returns_config_with_advertised_address() {
        let nid = node(b"builder-test-key");
        let config = substrate_transport_config(
            TransportConfig {
                bind_address: std::net::Ipv6Addr::LOCALHOST,
                ..TransportConfig::default()
            },
            &nid,
        )
        .await
        .expect("build config");

        let expected = derive_address(&nid, SUBNET_DEVICE_SCOPE).expect("derive");
        assert_eq!(config.public_ipv6, Some(expected));
        assert_eq!(config.bind_address, std::net::Ipv6Addr::LOCALHOST);
    }
}
