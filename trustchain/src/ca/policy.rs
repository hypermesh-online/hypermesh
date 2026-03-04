// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate Policy Engine
//!
//! Policy validation for certificate requests.

use anyhow::Result;

use super::CertificateRequest;
use crate::proof_of_state::StateRequirements;

/// Policy engine for certificate validation
#[derive(Clone)]
pub struct PolicyEngine {
    state_requirements: StateRequirements,
}

impl PolicyEngine {
    /// Create new policy engine
    pub fn new(state_requirements: StateRequirements) -> Self {
        Self {
            state_requirements,
        }
    }

    /// Validate certificate request against policy
    pub async fn validate_request(&self, request: &CertificateRequest) -> Result<bool> {
        // Basic policy validation
        if request.common_name.is_empty() {
            return Ok(false);
        }

        // Validate state proof meets requirements
        if !request
            .state_proof
            .validate_with_requirements(&self.state_requirements)
        {
            return Ok(false);
        }

        Ok(true)
    }
}
