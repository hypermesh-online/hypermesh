// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Substrate backend adapters and the capability-selecting registry.
//!
//! Mirrors the `AdapterRegistry` pattern in
//! `blockmatrix/src/assets/adapters/mod.rs`. The registry collects available
//! backends and selects the highest-capability one at runtime, degrading across
//! tiers (netlink → sysfs → fallback) the way `hypermesh-ebpf` selects kernel
//! capability tiers (`papers/HYPERMESH.md` §5.2, R16).
//!
//! Backends scaffolded this pass: [`rtnetlink_linux`] (real near-term, feature
//! `rtnetlink-backend`), [`sysfs_fallback`] (read-only degraded), [`radio_mesh`]
//! (Substrate.c, roadmap stub), [`windows`] (future, roadmap stub).

#[cfg(feature = "rtnetlink-backend")]
pub mod rtnetlink_linux;
pub mod radio_mesh;
pub mod sysfs_fallback;
pub mod windows;

#[cfg(feature = "rtnetlink-backend")]
pub use rtnetlink_linux::RtnetlinkLinuxAdapter;
pub use radio_mesh::RadioMeshAdapter;
pub use sysfs_fallback::SysfsFallbackAdapter;
pub use windows::WindowsAdapter;

use std::sync::Arc;

use crate::substrate::{SubstrateAdapter, SubstrateCapabilities};

/// Registry of available Substrate backends.
///
/// Holds every compiled-in adapter and selects the best one for a required
/// capability. Selection is "highest capability wins"; stub adapters advertise no
/// capabilities and are therefore never selected.
pub struct SubstrateAdapterRegistry {
    adapters: Vec<Arc<dyn SubstrateAdapter>>,
}

impl SubstrateAdapterRegistry {
    /// Build a registry with all compiled-in adapters.
    ///
    /// The netlink backend is only present when the `rtnetlink-backend` feature is
    /// enabled; otherwise the registry contains the read-only sysfs fallback plus
    /// the (capability-less) roadmap stubs.
    pub fn with_defaults() -> Self {
        let mut adapters: Vec<Arc<dyn SubstrateAdapter>> = Vec::new();

        #[cfg(feature = "rtnetlink-backend")]
        adapters.push(Arc::new(RtnetlinkLinuxAdapter::new()));

        adapters.push(Arc::new(SysfsFallbackAdapter::new()));
        adapters.push(Arc::new(RadioMeshAdapter::new()));
        adapters.push(Arc::new(WindowsAdapter::new()));

        Self { adapters }
    }

    /// All registered adapters.
    pub fn adapters(&self) -> &[Arc<dyn SubstrateAdapter>] {
        &self.adapters
    }

    /// Select the first adapter whose capabilities satisfy `predicate`.
    ///
    /// Adapters are tried in registration order (most capable first), so this
    /// yields the highest-tier backend that can do the requested job.
    pub fn select<F>(&self, predicate: F) -> Option<Arc<dyn SubstrateAdapter>>
    where
        F: Fn(&SubstrateCapabilities) -> bool,
    {
        self.adapters
            .iter()
            .find(|a| predicate(&a.capabilities()))
            .cloned()
    }

    /// Convenience: the best adapter that can enumerate interfaces.
    pub fn enumerator(&self) -> Option<Arc<dyn SubstrateAdapter>> {
        self.select(|c| c.enumerate)
    }

    /// Convenience: the best adapter that can assign addresses lease-free.
    pub fn address_assigner(&self) -> Option<Arc<dyn SubstrateAdapter>> {
        self.select(|c| c.assign_address)
    }
}

impl Default for SubstrateAdapterRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}
