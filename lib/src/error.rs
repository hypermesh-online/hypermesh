// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Shared error types

use thiserror::Error;

/// Top-level HyperMesh error type
#[derive(Debug, Error)]
pub enum HypermeshError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Consensus error: {0}")]
    Consensus(String),

    #[error("Asset error: {0}")]
    Asset(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
