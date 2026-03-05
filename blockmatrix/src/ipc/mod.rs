// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! IPC layer for HyperMesh daemon/client communication.
//!
//! JSON-RPC 2.0 over Unix domain sockets with newline-delimited framing.

pub mod client;
pub mod config;
pub mod handler;
pub mod handlers;
pub mod http_api;
pub mod protocol;
pub mod server;
pub mod state;

pub use client::IpcClient;
pub use config::HypermeshConfig;
pub use handler::{HandlerFn, RequestHandler};
pub use handlers::register_all;
pub use protocol::{RpcError, RpcRequest, RpcResponse};
pub use server::IpcServer;
pub use state::DaemonState;
