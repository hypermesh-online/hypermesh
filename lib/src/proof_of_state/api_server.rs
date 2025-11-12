//! ⚠️ DEPRECATED - DO NOT USE
//!
//! **Migration Date**: 2025-10-25
//! **Status**: REPLACED by STOQ API
//! **Replacement**: See `stoq_api.rs` for STOQ-based implementation
//!
//! This file contained HTTP-based (warp) consensus validation API server.
//! All HTTP transport has been removed in favor of STOQ protocol (pure QUIC over IPv6).
//!
//! **Migration Path**:
//! - HTTP warp → STOQ API (`hypermesh/src/consensus/stoq_api.rs`)
//! - `/consensus/validation/certificate` → `consensus/validate_certificate` (STOQ handler)
//! - `/consensus/validation/four-proof` → `consensus/validate_proofs` (STOQ handler)
//! - warp::Filter → `ApiHandler` trait implementation
//!
//! **Reason for Removal**:
//! - External dependency removal (zero HTTP dependencies)
//! - 100% standalone system-level execution
//! - STOQ provides 2-4x lower latency, better multiplexing, no head-of-line blocking
//!
//! **Documentation**:
//! - `/STOQ_MIGRATION_GUIDE.md` - Step-by-step migration instructions
//! - `/MIGRATION_COMPLETE.md` - Full migration status
//! - `/HTTP_REMOVED.md` - HTTP dependency removal report
//!
//! This file is preserved for historical reference only and MUST NOT be compiled.
//!
//! ---
//!
//! Original file content follows (commented out):

/*
//! HyperMesh Consensus API Server
//!
//! This module provides HTTP API endpoints for external services like TrustChain
//! to request consensus validation through HyperMesh's four-proof system.

use std::sync::Arc;
use std::convert::Infallible;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use warp::{Filter, Reply, Rejection};
use warp::http::StatusCode;

use super::validation_service::{
    ConsensusValidationService, CertificateValidationRequest, FourProofValidationRequest,
    ValidationResult, ValidationStatus, ValidationServiceMetrics,
};

// ... [rest of file content - 474 lines omitted for brevity] ...
*/
