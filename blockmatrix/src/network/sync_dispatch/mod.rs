// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Dispatches sync-related `MatrixMessage` variants to the appropriate
//! subsystems (`SyncManager`, `ReflectorPool`).
//!
//! This wiring layer bridges Gaps 1, 2, 4, and 5 by:
//! - Converting `MatrixMessage::SyncRequest/SyncResponse/SyncAnnounce`
//!   into `SyncMessage` values and forwarding them to `SyncManager`.
//! - Converting `MatrixMessage::ReflectorHeartbeat` into a
//!   `register_reflector` / `update_health` call on `ReflectorPool`.

mod dispatcher;
mod reflector_handler;
mod sync_manager_handler;
mod transport_sync_driver;

#[cfg(test)]
mod tests;

pub use dispatcher::{DispatchResponse, SyncDispatcher};
pub use transport_sync_driver::TransportSyncDriver;
