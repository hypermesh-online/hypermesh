// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Linux netlink backend (Substrate.a + Substrate.b) — the real near-term adapter.
//!
//! Uses `rtnetlink` + `netlink-packet-route` to enumerate interfaces, read carrier
//! state, assign addresses lease-free, and subscribe to `RTMGRP_LINK` events. This
//! is the backend that replaces the hardcoded interface guess and the manual
//! `public_ipv6` setting (R16, and the assignment half of R15).
//!
//! Compiled only with the `rtnetlink-backend` feature. **Scaffold this pass** —
//! method bodies land in Phase 1 (enumerate/reachability) and Phase 2 (carrier,
//! assign, watch). See `core/base/SPEC.md`.

use async_trait::async_trait;

use crate::error::SubstrateResult;
use crate::link::{InterfaceAddress, InterfaceId, LinkState};
use crate::reachability::Reachability;
use crate::substrate::{SubstrateAdapter, SubstrateCapabilities};

/// Linux netlink-backed substrate adapter.
#[derive(Debug, Default)]
pub struct RtnetlinkLinuxAdapter;

impl RtnetlinkLinuxAdapter {
    /// Construct the adapter.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SubstrateAdapter for RtnetlinkLinuxAdapter {
    fn name(&self) -> &'static str {
        "rtnetlink-linux"
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        // Full link-management capability; reachability discovery arrives with the
        // reflector extension (Phase 1+), so it is advertised here as the intended
        // home even though the body is scaffolded.
        SubstrateCapabilities {
            enumerate: true,
            carrier: true,
            assign_address: true,
            watch: true,
            reachability: true,
        }
    }

    async fn enumerate_interfaces(&self) -> SubstrateResult<Vec<InterfaceId>> {
        todo!("Phase 1: rtnetlink RTM_GETLINK enumeration")
    }

    async fn carrier_state(&self, _iface: &InterfaceId) -> SubstrateResult<LinkState> {
        todo!("Phase 2: rtnetlink carrier/operstate read")
    }

    async fn assign_address(
        &self,
        _iface: &InterfaceId,
        _addr: InterfaceAddress,
    ) -> SubstrateResult<()> {
        todo!("Phase 2: rtnetlink RTM_NEWADDR lease-free assignment")
    }

    async fn detect_reachability(&self) -> SubstrateResult<Reachability> {
        todo!("Phase 1+: reachability via STOQ reflector extension")
    }
}
