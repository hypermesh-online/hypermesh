// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Rust SDK for the HyperMesh daemon.
//!
//! Provides a typed async client that communicates with a running HyperMesh
//! daemon over the JSON-RPC 2.0 IPC protocol (Unix domain sockets).
//!
//! # Example
//! ```no_run
//! # async fn example() -> Result<(), hypermesh_sdk::SdkError> {
//! let client = hypermesh_sdk::HyperMeshClient::connect_local().await?;
//! let status = client.node().status().await?;
//! println!("node {} at height {}", status.node_id, status.chain_height);
//! # Ok(())
//! # }
//! ```

pub mod api;
pub mod client;
pub mod error;

pub use client::{ConnectionMode, HyperMeshClient};
pub use error::SdkError;
