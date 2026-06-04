// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Read-only sysfs backend (Substrate.b, degraded tier).
//!
//! Reads interface and carrier state from `/sys/class/net/{iface}/...` — the same
//! proven pattern `hypermesh-ebpf`'s `NicCapabilities::detect()` uses. Works where
//! netlink is unavailable or unprivileged. It is read-only: it cannot assign
//! addresses or subscribe to events, so it advertises a reduced capability set and
//! the registry only selects it when the netlink backend is absent (R16 graceful
//! degradation: netlink → sysfs → fallback).
//!
//! **Scaffold this pass** — bodies land in Phase 2. See `core/base/SPEC.md`.

use async_trait::async_trait;

use crate::error::{SubstrateError, SubstrateResult};
use crate::link::{InterfaceAddress, InterfaceId, LinkState};
use crate::reachability::Reachability;
use crate::substrate::{SubstrateAdapter, SubstrateCapabilities};

/// Read-only sysfs-backed substrate adapter.
#[derive(Debug, Default)]
pub struct SysfsFallbackAdapter;

impl SysfsFallbackAdapter {
    /// Construct the adapter.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SubstrateAdapter for SysfsFallbackAdapter {
    fn name(&self) -> &'static str {
        "sysfs-fallback"
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        // Read-only: can enumerate and read carrier, cannot assign/watch/discover.
        SubstrateCapabilities {
            enumerate: true,
            carrier: true,
            assign_address: false,
            watch: false,
            reachability: false,
        }
    }

    async fn enumerate_interfaces(&self) -> SubstrateResult<Vec<InterfaceId>> {
        todo!("Phase 2: read /sys/class/net/ directory entries")
    }

    async fn carrier_state(&self, _iface: &InterfaceId) -> SubstrateResult<LinkState> {
        todo!("Phase 2: read /sys/class/net/{{iface}}/carrier")
    }

    async fn assign_address(
        &self,
        _iface: &InterfaceId,
        _addr: InterfaceAddress,
    ) -> SubstrateResult<()> {
        Err(SubstrateError::Unsupported(
            "sysfs fallback is read-only; address assignment requires the netlink backend"
                .to_string(),
        ))
    }

    async fn detect_reachability(&self) -> SubstrateResult<Reachability> {
        Err(SubstrateError::Unsupported(
            "sysfs fallback cannot discover reachability".to_string(),
        ))
    }
}
