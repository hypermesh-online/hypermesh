// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Substrate.c — physical/radio backend (device-to-device, zero ISP).
//!
//! This is the endgame of substrate sovereignty: links that exist with NO
//! incumbent infrastructure — wireless mesh radio, opportunistic peering. The
//! network exists before any provider does.
//!
//! **This is a roadmap stub, not built.** It exists so pilot-funded R&D has a
//! concrete home in the adapter registry without restructuring. It advertises NO
//! capabilities, so the registry never selects it today. It is intentionally NOT
//! given a protocol R-number — radio is not yet "concrete and testable" (the bar
//! for R-numbers per `core/CLAUDE.md`); it lives as roadmap prose in
//! `papers/SUBSTRATE.md` §8.
//!
//! Open R&D questions (see `papers/SUBSTRATE.md` §8): radio hardware abstraction,
//! driver access, neighbor discovery without a router, opportunistic peering,
//! spectrum/regulatory constraints, power.

use async_trait::async_trait;

use crate::error::{SubstrateError, SubstrateResult};
use crate::link::{InterfaceAddress, InterfaceId, LinkState};
use crate::reachability::Reachability;
use crate::substrate::{SubstrateAdapter, SubstrateCapabilities};

/// Substrate.c radio backend — roadmap stub, advertises no capabilities.
#[derive(Debug, Default)]
pub struct RadioMeshAdapter;

impl RadioMeshAdapter {
    /// Construct the (non-functional) stub adapter.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SubstrateAdapter for RadioMeshAdapter {
    fn name(&self) -> &'static str {
        "radio-mesh"
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        // Deliberately empty: future R&D. The registry must never select this yet.
        SubstrateCapabilities::default()
    }

    async fn enumerate_interfaces(&self) -> SubstrateResult<Vec<InterfaceId>> {
        Err(SubstrateError::Unsupported(
            "radio-mesh (Substrate.c) is roadmap R&D — not implemented".to_string(),
        ))
    }

    async fn carrier_state(&self, _iface: &InterfaceId) -> SubstrateResult<LinkState> {
        Err(SubstrateError::Unsupported(
            "radio-mesh (Substrate.c) is roadmap R&D — not implemented".to_string(),
        ))
    }

    async fn assign_address(
        &self,
        _iface: &InterfaceId,
        _addr: InterfaceAddress,
    ) -> SubstrateResult<()> {
        Err(SubstrateError::Unsupported(
            "radio-mesh (Substrate.c) is roadmap R&D — not implemented".to_string(),
        ))
    }

    async fn detect_reachability(&self) -> SubstrateResult<Reachability> {
        Err(SubstrateError::Unsupported(
            "radio-mesh (Substrate.c) is roadmap R&D — not implemented".to_string(),
        ))
    }
}
