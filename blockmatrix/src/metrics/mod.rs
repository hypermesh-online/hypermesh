// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Real hardware metrics collection for BlockMatrix nodes.
//!
//! This module provides direct system introspection via `/proc` filesystem
//! parsing on Linux, with no external crate dependencies for data collection.
//!
//! # Modules
//!
//! - [`hardware`] -- CPU, memory, network, and storage metrics from `/proc`.

pub mod hardware;
mod parsers;

pub use hardware::{
    collect, collect_cpu, collect_cpu_usage, collect_memory, collect_network, collect_storage,
    CpuMetrics, InterfaceMetrics, MemoryMetrics, NetworkMetrics, NodeMetrics, StorageMetrics,
};
