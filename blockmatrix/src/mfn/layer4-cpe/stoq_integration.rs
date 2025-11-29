//! STOQ Protocol Integration
//!
//! This module provides integration with the STOQ protocol for efficient
//! data streaming and network optimization.

#[cfg(feature = "stoq-integration")]
use stoq::{StoqClient, StoqMessage};

use anyhow::Result;
use crate::{ContextVector, prediction::PredictionResult};

/// STOQ protocol integration for CPE
#[cfg(feature = "stoq-integration")]
pub struct StoqIntegration {
    client: Option<StoqClient>,
}

#[cfg(feature = "stoq-integration")]
impl StoqIntegration {
    pub fn new(client: Option<StoqClient>) -> Self {
        Self { client }
    }
    
    /// Send prediction through STOQ protocol
    pub async fn send_prediction(&self, prediction: &PredictionResult) -> Result<()> {
        if let Some(ref client) = self.client {
            let message = StoqMessage::new(
                "prediction".to_string(),
                serde_json::to_vec(prediction)?,
            );
            client.send(message).await?;
        }
        Ok(())
    }
    
    /// Receive context through STOQ protocol
    pub async fn receive_context(&self) -> Result<Option<ContextVector>> {
        if let Some(ref client) = self.client {
            if let Some(message) = client.receive().await? {
                let context: ContextVector = serde_json::from_slice(&message.data)?;
                return Ok(Some(context));
            }
        }
        Ok(None)
    }
}

#[cfg(not(feature = "stoq-integration"))]
pub struct StoqIntegration;

#[cfg(not(feature = "stoq-integration"))]
impl StoqIntegration {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn send_prediction(&self, _prediction: &PredictionResult) -> Result<()> {
        Ok(())
    }
    
    pub async fn receive_context(&self) -> Result<Option<ContextVector>> {
        Ok(None)
    }
}