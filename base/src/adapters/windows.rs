// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Windows backend (Substrate.a + Substrate.b on Windows) — future cross-platform.
//!
//! HyperMesh already has a cross-platform OS abstraction
//! (`blockmatrix/src/os_integration/`), so a Windows substrate backend is a known
//! direction. This stub justifies the adapter abstraction and reserves the home;
//! it advertises NO capabilities and is never selected today.
//!
//! **Scaffold/roadmap stub, not built.** A real implementation would use the
//! Windows IP Helper API (`GetAdaptersAddresses`, `NotifyAddrChange`) rather than
//! netlink. See `papers/SUBSTRATE.md` §12 roadmap.

use async_trait::async_trait;

use crate::error::{SubstrateError, SubstrateResult};
use crate::link::{InterfaceAddress, InterfaceId, LinkState};
use crate::reachability::Reachability;
use crate::substrate::{SubstrateAdapter, SubstrateCapabilities};

/// Windows substrate backend — future cross-platform stub, no capabilities yet.
#[derive(Debug, Default)]
pub struct WindowsAdapter;

impl WindowsAdapter {
    /// Construct the (non-functional) stub adapter.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SubstrateAdapter for WindowsAdapter {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        SubstrateCapabilities::default()
    }

    async fn enumerate_interfaces(&self) -> SubstrateResult<Vec<InterfaceId>> {
        Err(SubstrateError::Unsupported(
            "windows backend is future cross-platform R&D — not implemented".to_string(),
        ))
    }

    async fn carrier_state(&self, _iface: &InterfaceId) -> SubstrateResult<LinkState> {
        Err(SubstrateError::Unsupported(
            "windows backend is future cross-platform R&D — not implemented".to_string(),
        ))
    }

    async fn assign_address(
        &self,
        _iface: &InterfaceId,
        _addr: InterfaceAddress,
    ) -> SubstrateResult<()> {
        Err(SubstrateError::Unsupported(
            "windows backend is future cross-platform R&D — not implemented".to_string(),
        ))
    }

    async fn detect_reachability(&self) -> SubstrateResult<Reachability> {
        Err(SubstrateError::Unsupported(
            "windows backend is future cross-platform R&D — not implemented".to_string(),
        ))
    }
}
