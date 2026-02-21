// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Settlement Protocol -- Network-as-Processor autonomous settlement
//!
//! Users publish AcceptanceCriteria once to the Network chain.
//! Any online node can settle for any verified user.
//! Three terminal states: Settled, Expired, Dissolved.

pub mod acceptance;
pub mod gravity;
pub mod protocol;

pub use acceptance::AcceptanceCriteria;
pub use gravity::GravityDissolution;
pub use protocol::SettlementProtocol;
