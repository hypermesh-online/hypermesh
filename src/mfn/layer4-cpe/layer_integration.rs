//! Layer Integration Module
//!
//! This module provides integration utilities and adapters for connecting
//! CPE Layer 4 with other MFN layers (IFR, DSR, ALM).

#[cfg(feature = "layer-integration")]
use mfn_layer2_dsr::{DsrSystem, SimilarityResult};

use anyhow::Result;
use crate::{ContextVector, prediction::PredictionResult};

/// Layer 2 DSR integration adapter
#[cfg(feature = "layer-integration")]
pub struct Layer2Adapter {
    dsr_system: Option<DsrSystem>,
}

#[cfg(feature = "layer-integration")]
impl Layer2Adapter {
    pub fn new(dsr_system: Option<DsrSystem>) -> Self {
        Self { dsr_system }
    }
    
    /// Get similarity from DSR system
    pub async fn get_similarity(&self, context: &ContextVector) -> Result<Option<f32>> {
        if let Some(ref dsr) = self.dsr_system {
            // Convert context to DSR format and get similarity
            let similarity_result = dsr.process_similarity(&context.features, None).await?;
            Ok(Some(similarity_result.similarity_score))
        } else {
            Ok(None)
        }
    }
    
    /// Send prediction feedback to DSR
    pub async fn send_feedback(&self, prediction: &PredictionResult) -> Result<()> {
        // Implementation would send feedback to DSR system
        Ok(())
    }
}

/// Layer 3 ALM integration adapter  
pub struct Layer3Adapter;

impl Layer3Adapter {
    pub fn new() -> Self {
        Self
    }
    
    /// Send routing decision to ALM
    pub async fn send_routing_decision(&self, _prediction: &PredictionResult) -> Result<()> {
        // Implementation would communicate with Layer 3 ALM
        Ok(())
    }
}

/// Layer 1 IFR integration adapter
pub struct Layer1Adapter;

impl Layer1Adapter {
    pub fn new() -> Self {
        Self
    }
    
    /// Get flow information from IFR
    pub async fn get_flow_info(&self, _flow_key: &[u8; 32]) -> Result<Option<String>> {
        // Implementation would query Layer 1 IFR for flow information
        Ok(Some("flow_info".to_string()))
    }
}