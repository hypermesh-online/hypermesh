// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Data Sharing Configuration
//!
//! Configuration for data sharing policies, approval workflows, and anonymization preferences.

mod types;
mod reporting;
mod validation;
mod defaults;

pub use types::*;
pub use reporting::*;
// validation and defaults only provide impl blocks, no additional exports needed
