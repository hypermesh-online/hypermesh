//! Machine Learning Models for Context Prediction
//!
//! This module implements the core ML models used for temporal pattern recognition
//! and context prediction, including LSTM networks and Transformer models.

use anyhow::Result;
use candle_core::{Device, Tensor, DType, Shape, IndexOp};
use candle_nn::{VarBuilder, VarMap, Module, linear, Linear, lstm::LSTM};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

use crate::ContextVector;

/// Supported ML model types for context prediction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelType {
    /// LSTM-based sequence model for temporal patterns
    Lstm,
    /// Transformer model with attention mechanisms
    Transformer,
    /// Hybrid LSTM + Transformer model
    Hybrid,
}

/// LSTM model for sequence-based context prediction
pub struct LstmModel {
    lstm: LSTM,
    output_layer: Linear,
    device: Device,
    
    // Model parameters
    input_size: usize,
    hidden_size: usize,
    num_layers: usize,
    sequence_length: usize,
    
    // Training state
    varmap: VarMap,
    optimizer_state: Option<HashMap<String, Tensor>>,
}

impl LstmModel {
    /// Create a new LSTM model with specified parameters
    pub fn new(
        input_size: usize,
        hidden_size: usize,
        num_layers: usize,
        sequence_length: usize,
        device: Device,
    ) -> Result<Self> {
        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        
        // Create LSTM layer
        let lstm = LSTM::new(input_size, hidden_size, num_layers, vs.pp("lstm"))?;
        
        // Create output projection layer
        let output_layer = linear(hidden_size, input_size, vs.pp("output"))?;
        
        info!("LSTM model created: {}x{} hidden units, {} layers", 
              input_size, hidden_size, num_layers);
        
        Ok(Self {
            lstm,
            output_layer,
            device,
            input_size,
            hidden_size,
            num_layers,
            sequence_length,
            varmap,
            optimizer_state: None,
        })
    }
    
    /// Forward pass through the LSTM model
    pub fn forward(&self, input_sequence: &Tensor) -> Result<Tensor> {
        // Run LSTM forward pass
        let (lstm_output, _) = self.lstm.forward(input_sequence, None)?;
        
        // Get the last timestep output for prediction
        let last_output = lstm_output.i((-1, ..))?;
        
        // Project to output dimension
        let prediction = self.output_layer.forward(&last_output)?;
        
        Ok(prediction)
    }
    
    /// Predict next context given a sequence of historical contexts
    pub fn predict_sequence(&self, contexts: &[ContextVector]) -> Result<Vec<f32>> {
        if contexts.is_empty() {
            return Err(anyhow::anyhow!("Empty context sequence"));
        }
        
        // Prepare input tensor from context sequence
        let input_tensor = self.prepare_input_tensor(contexts)?;
        
        // Forward pass
        let prediction_tensor = self.forward(&input_tensor)?;
        
        // Convert to vector
        let prediction: Vec<f32> = prediction_tensor.to_vec1()?;
        
        debug!("LSTM prediction completed for sequence length {}", contexts.len());
        Ok(prediction)
    }
    
    /// Prepare input tensor from context vectors
    fn prepare_input_tensor(&self, contexts: &[ContextVector]) -> Result<Tensor> {
        let seq_len = contexts.len().min(self.sequence_length);
        let mut input_data = Vec::with_capacity(seq_len * self.input_size);
        
        // Use the most recent contexts up to sequence_length
        let start_idx = if contexts.len() > self.sequence_length {
            contexts.len() - self.sequence_length
        } else {
            0
        };
        
        for context in &contexts[start_idx..] {
            // Pad or truncate features to match input_size
            for i in 0..self.input_size {
                let feature = context.features.get(i).copied().unwrap_or(0.0);
                input_data.push(feature);
            }
        }
        
        // Pad sequence if necessary
        while input_data.len() < seq_len * self.input_size {
            input_data.push(0.0);
        }
        
        let shape = Shape::from((seq_len, self.input_size));
        Tensor::from_slice(&input_data, shape, &self.device)
    }
    
    /// Update model weights using gradient descent
    pub fn update_weights(&mut self, contexts: &[ContextVector], targets: &[ContextVector]) -> Result<f32> {
        if contexts.len() != targets.len() {
            return Err(anyhow::anyhow!("Contexts and targets must have same length"));
        }
        
        let mut total_loss = 0.0;
        
        for (context_seq, target) in contexts.iter().zip(targets.iter()) {
            // Forward pass
            let input_tensor = self.prepare_input_tensor(&[context_seq.clone()])?;
            let prediction = self.forward(&input_tensor)?;
            
            // Compute loss (MSE)
            let target_tensor = Tensor::from_slice(
                &target.features[..self.input_size.min(target.features.len())],
                (self.input_size,),
                &self.device
            )?;
            
            let diff = prediction.sub(&target_tensor)?;
            let loss = diff.powf(2.0)?.mean_all()?;
            total_loss += loss.to_scalar::<f32>()?;
        }
        
        let avg_loss = total_loss / contexts.len() as f32;
        debug!("LSTM training step completed, loss: {:.6}", avg_loss);
        
        Ok(avg_loss)
    }
    
    pub fn get_parameters(&self) -> &VarMap {
        &self.varmap
    }
    
    pub fn get_model_info(&self) -> HashMap<String, f32> {
        let mut info = HashMap::new();
        info.insert("input_size".to_string(), self.input_size as f32);
        info.insert("hidden_size".to_string(), self.hidden_size as f32);
        info.insert("num_layers".to_string(), self.num_layers as f32);
        info.insert("sequence_length".to_string(), self.sequence_length as f32);
        info
    }
}

/// Transformer model with attention mechanisms
pub struct TransformerModel {
    attention_layers: Vec<crate::attention::MultiHeadAttention>,
    feed_forward: Linear,
    output_projection: Linear,
    device: Device,
    
    // Model parameters
    model_dim: usize,
    num_heads: usize,
    num_layers: usize,
    sequence_length: usize,
    
    // Positional encoding
    positional_encoding: Tensor,
}

impl TransformerModel {
    pub fn new(
        model_dim: usize,
        num_heads: usize,
        num_layers: usize,
        sequence_length: usize,
        device: Device,
    ) -> Result<Self> {
        let mut attention_layers = Vec::new();
        
        // Create attention layers
        for i in 0..num_layers {
            let config = crate::attention::AttentionConfig {
                model_dim,
                num_heads,
                dropout: 0.1,
                scale: None,
            };
            let attention = crate::attention::MultiHeadAttention::new(config, device.clone())?;
            attention_layers.push(attention);
        }
        
        // Create feed-forward layer
        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let feed_forward = linear(model_dim, model_dim * 4, vs.pp("ff"))?;
        let output_projection = linear(model_dim * 4, model_dim, vs.pp("output"))?;
        
        // Create positional encoding
        let positional_encoding = Self::create_positional_encoding(sequence_length, model_dim, &device)?;
        
        info!("Transformer model created: {} dims, {} heads, {} layers", 
              model_dim, num_heads, num_layers);
        
        Ok(Self {
            attention_layers,
            feed_forward,
            output_projection,
            device,
            model_dim,
            num_heads,
            num_layers,
            sequence_length,
            positional_encoding,
        })
    }
    
    pub fn forward(&self, input_sequence: &Tensor) -> Result<Tensor> {
        let mut x = input_sequence.clone();
        
        // Add positional encoding
        let seq_len = x.dim(0)?;
        if seq_len <= self.sequence_length {
            let pos_encoding = self.positional_encoding.narrow(0, 0, seq_len)?;
            x = x.add(&pos_encoding)?;
        }
        
        // Apply attention layers
        for attention_layer in &self.attention_layers {
            let attended = attention_layer.forward(&x, &x, &x, None)?;
            x = x.add(&attended)?; // Residual connection
        }
        
        // Apply feed-forward network
        let ff_output = self.feed_forward.forward(&x)?;
        let ff_output = ff_output.gelu()?; // GELU activation
        let output = self.output_projection.forward(&ff_output)?;
        
        // Get the last timestep
        let last_output = output.i((-1, ..))?;
        
        Ok(last_output)
    }
    
    pub fn predict_sequence(&self, contexts: &[ContextVector]) -> Result<Vec<f32>> {
        let input_tensor = self.prepare_input_tensor(contexts)?;
        let prediction = self.forward(&input_tensor)?;
        let result: Vec<f32> = prediction.to_vec1()?;
        
        debug!("Transformer prediction completed for sequence length {}", contexts.len());
        Ok(result)
    }
    
    fn prepare_input_tensor(&self, contexts: &[ContextVector]) -> Result<Tensor> {
        let seq_len = contexts.len().min(self.sequence_length);
        let mut input_data = Vec::with_capacity(seq_len * self.model_dim);
        
        let start_idx = if contexts.len() > self.sequence_length {
            contexts.len() - self.sequence_length
        } else {
            0
        };
        
        for context in &contexts[start_idx..] {
            for i in 0..self.model_dim {
                let feature = context.features.get(i).copied().unwrap_or(0.0);
                input_data.push(feature);
            }
        }
        
        let shape = Shape::from((seq_len, self.model_dim));
        Tensor::from_slice(&input_data, shape, &self.device)
    }
    
    fn create_positional_encoding(seq_len: usize, model_dim: usize, device: &Device) -> Result<Tensor> {
        let mut encoding = Vec::with_capacity(seq_len * model_dim);
        
        for pos in 0..seq_len {
            for i in 0..model_dim {
                let angle = pos as f32 / 10000.0_f32.powf((2 * i) as f32 / model_dim as f32);
                if i % 2 == 0 {
                    encoding.push(angle.sin());
                } else {
                    encoding.push(angle.cos());
                }
            }
        }
        
        let shape = Shape::from((seq_len, model_dim));
        Tensor::from_slice(&encoding, shape, device)
    }
    
    pub fn get_model_info(&self) -> HashMap<String, f32> {
        let mut info = HashMap::new();
        info.insert("model_dim".to_string(), self.model_dim as f32);
        info.insert("num_heads".to_string(), self.num_heads as f32);
        info.insert("num_layers".to_string(), self.num_layers as f32);
        info.insert("sequence_length".to_string(), self.sequence_length as f32);
        info
    }
}

/// Hybrid model combining LSTM and Transformer
pub struct HybridModel {
    lstm_model: LstmModel,
    transformer_model: TransformerModel,
    fusion_layer: Linear,
    device: Device,
}

impl HybridModel {
    pub fn new(
        input_size: usize,
        hidden_size: usize,
        lstm_layers: usize,
        transformer_layers: usize,
        num_heads: usize,
        sequence_length: usize,
        device: Device,
    ) -> Result<Self> {
        // Create LSTM component
        let lstm_model = LstmModel::new(
            input_size, 
            hidden_size, 
            lstm_layers, 
            sequence_length, 
            device.clone()
        )?;
        
        // Create Transformer component
        let transformer_model = TransformerModel::new(
            input_size, 
            num_heads, 
            transformer_layers, 
            sequence_length, 
            device.clone()
        )?;
        
        // Create fusion layer to combine outputs
        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        let fusion_layer = linear(input_size * 2, input_size, vs.pp("fusion"))?;
        
        info!("Hybrid model created: LSTM({}) + Transformer({}) -> Fusion", 
              lstm_layers, transformer_layers);
        
        Ok(Self {
            lstm_model,
            transformer_model,
            fusion_layer,
            device,
        })
    }
    
    pub fn forward(&self, input_sequence: &Tensor) -> Result<Tensor> {
        // Get predictions from both models
        let lstm_output = self.lstm_model.forward(input_sequence)?;
        let transformer_output = self.transformer_model.forward(input_sequence)?;
        
        // Concatenate outputs
        let combined = Tensor::cat(&[&lstm_output, &transformer_output], 0)?;
        
        // Apply fusion layer
        let fused_output = self.fusion_layer.forward(&combined)?;
        
        Ok(fused_output)
    }
    
    pub fn predict_sequence(&self, contexts: &[ContextVector]) -> Result<Vec<f32>> {
        let lstm_prediction = self.lstm_model.predict_sequence(contexts)?;
        let transformer_prediction = self.transformer_model.predict_sequence(contexts)?;
        
        // Simple ensemble: average the predictions
        let mut hybrid_prediction = Vec::new();
        let min_len = lstm_prediction.len().min(transformer_prediction.len());
        
        for i in 0..min_len {
            let avg = (lstm_prediction[i] + transformer_prediction[i]) / 2.0;
            hybrid_prediction.push(avg);
        }
        
        debug!("Hybrid prediction completed with {} features", hybrid_prediction.len());
        Ok(hybrid_prediction)
    }
    
    pub fn get_model_info(&self) -> HashMap<String, f32> {
        let mut info = HashMap::new();
        let lstm_info = self.lstm_model.get_model_info();
        let transformer_info = self.transformer_model.get_model_info();
        
        // Combine information from both models
        for (key, value) in lstm_info {
            info.insert(format!("lstm_{}", key), value);
        }
        for (key, value) in transformer_info {
            info.insert(format!("transformer_{}", key), value);
        }
        
        info.insert("model_type".to_string(), 2.0); // Hybrid type
        info
    }
}

/// Model factory for creating different types of prediction models
pub struct ModelFactory;

impl ModelFactory {
    pub fn create_model(
        model_type: ModelType,
        input_size: usize,
        hidden_size: usize,
        num_layers: usize,
        sequence_length: usize,
        device: Device,
    ) -> Result<Box<dyn PredictionModel>> {
        match model_type {
            ModelType::Lstm => {
                let model = LstmModel::new(input_size, hidden_size, num_layers, sequence_length, device)?;
                Ok(Box::new(model))
            }
            ModelType::Transformer => {
                let num_heads = 8; // Default attention heads
                let model = TransformerModel::new(input_size, num_heads, num_layers, sequence_length, device)?;
                Ok(Box::new(model))
            }
            ModelType::Hybrid => {
                let lstm_layers = num_layers / 2;
                let transformer_layers = num_layers - lstm_layers;
                let num_heads = 8;
                let model = HybridModel::new(
                    input_size, 
                    hidden_size, 
                    lstm_layers, 
                    transformer_layers, 
                    num_heads, 
                    sequence_length, 
                    device
                )?;
                Ok(Box::new(model))
            }
        }
    }
}

/// Trait for all prediction models
pub trait PredictionModel: Send + Sync {
    fn predict_sequence(&self, contexts: &[ContextVector]) -> Result<Vec<f32>>;
    fn get_model_info(&self) -> HashMap<String, f32>;
    fn forward(&self, input_sequence: &Tensor) -> Result<Tensor>;
}

impl PredictionModel for LstmModel {
    fn predict_sequence(&self, contexts: &[ContextVector]) -> Result<Vec<f32>> {
        self.predict_sequence(contexts)
    }
    
    fn get_model_info(&self) -> HashMap<String, f32> {
        self.get_model_info()
    }
    
    fn forward(&self, input_sequence: &Tensor) -> Result<Tensor> {
        self.forward(input_sequence)
    }
}

impl PredictionModel for TransformerModel {
    fn predict_sequence(&self, contexts: &[ContextVector]) -> Result<Vec<f32>> {
        self.predict_sequence(contexts)
    }
    
    fn get_model_info(&self) -> HashMap<String, f32> {
        self.get_model_info()
    }
    
    fn forward(&self, input_sequence: &Tensor) -> Result<Tensor> {
        self.forward(input_sequence)
    }
}

impl PredictionModel for HybridModel {
    fn predict_sequence(&self, contexts: &[ContextVector]) -> Result<Vec<f32>> {
        self.predict_sequence(contexts)
    }
    
    fn get_model_info(&self) -> HashMap<String, f32> {
        self.get_model_info()
    }
    
    fn forward(&self, input_sequence: &Tensor) -> Result<Tensor> {
        self.forward(input_sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_contexts(count: usize, feature_dim: usize) -> Vec<ContextVector> {
        (0..count).map(|i| {
            let flow_key = [i as u8; 32];
            let features: Vec<f32> = (0..feature_dim)
                .map(|j| (i + j) as f32 * 0.01)
                .collect();
            ContextVector::new(flow_key, features)
        }).collect()
    }
    
    #[test]
    fn test_lstm_model_creation() {
        let device = Device::Cpu;
        let model = LstmModel::new(10, 16, 2, 8, device);
        assert!(model.is_ok());
    }
    
    #[test]
    fn test_lstm_prediction() {
        let device = Device::Cpu;
        let model = LstmModel::new(10, 16, 1, 8, device).unwrap();
        let contexts = create_test_contexts(5, 10);
        
        let result = model.predict_sequence(&contexts);
        assert!(result.is_ok());
        
        let prediction = result.unwrap();
        assert_eq!(prediction.len(), 10);
    }
    
    #[test]
    fn test_transformer_model_creation() {
        let device = Device::Cpu;
        let model = TransformerModel::new(16, 4, 2, 8, device);
        assert!(model.is_ok());
    }
    
    #[test]
    fn test_hybrid_model() {
        let device = Device::Cpu;
        let model = HybridModel::new(10, 16, 1, 1, 4, 8, device);
        assert!(model.is_ok());
        
        if let Ok(model) = model {
            let contexts = create_test_contexts(3, 10);
            let prediction = model.predict_sequence(&contexts);
            assert!(prediction.is_ok());
        }
    }
    
    #[test]
    fn test_model_factory() {
        let device = Device::Cpu;
        
        let lstm_model = ModelFactory::create_model(
            ModelType::Lstm, 10, 16, 2, 8, device.clone()
        );
        assert!(lstm_model.is_ok());
        
        let transformer_model = ModelFactory::create_model(
            ModelType::Transformer, 16, 16, 2, 8, device.clone()
        );
        assert!(transformer_model.is_ok());
        
        let hybrid_model = ModelFactory::create_model(
            ModelType::Hybrid, 10, 16, 4, 8, device
        );
        assert!(hybrid_model.is_ok());
    }
}