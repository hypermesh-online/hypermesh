//! HyperMesh Asset Layer - Universal Asset System with Four-Proof Consensus
//!
//! This module implements the HyperMesh asset management system that treats
//! everything as an asset: CPU, GPU, memory, storage, network connections,
//! VMs, and services. Every asset operation requires four-proof consensus
//! validation (PoSpace+PoStake+PoWork+PoTime).
//!
//! # Module Organization
//!
//! - `core` - Core asset types and management
//! - `consensus` - Four-proof consensus validation
//! - `allocation` - Asset allocation and scheduling
//! - `proxy` - NAT-like proxy addressing
//! - `vm` - Virtual machine asset execution
//! - `adapters` - Hardware and resource adapters

pub mod adapters;
pub mod allocation;
pub mod consensus;
pub mod core;
pub mod proxy;
pub mod vm;

// Re-export main types from core
pub use core::{
    Asset, AssetId, AssetLocation, AssetStatistics, AssetStatus,
    AssetSystemStats, AssetType, AllocationId, HyperMeshAssetLayer,
    PrivacyLevel, ResourceAllocation,
};