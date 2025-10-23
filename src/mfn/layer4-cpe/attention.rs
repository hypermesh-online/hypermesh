//! Attention Mechanisms for Context Prediction
//!
//! This module implements various attention mechanisms including self-attention,
//! multi-head attention, and cross-attention for the Transformer-based models.

use anyhow::Result;
use candle_core::{Device, Tensor, DType, Shape, IndexOp};
use candle_nn::{VarBuilder, VarMap, Module, linear, Linear};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

/// Configuration for attention layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionConfig {
    pub model_dim: usize,
    pub num_heads: usize,
    pub dropout: f64,
    pub scale: Option<f32>,
}

impl Default for AttentionConfig {
    fn default() -> Self {
        Self {
            model_dim: 256,
            num_heads: 8,
            dropout: 0.1,
            scale: None,
        }
    }
}

/// Multi-head attention layer implementation
pub struct MultiHeadAttention {
    query_projection: Linear,
    key_projection: Linear,
    value_projection: Linear,
    output_projection: Linear,
    
    config: AttentionConfig,
    device: Device,
    head_dim: usize,
    scale: f32,
}

impl MultiHeadAttention {
    pub fn new(config: AttentionConfig, device: Device) -> Result<Self> {
        if config.model_dim % config.num_heads != 0 {
            return Err(anyhow::anyhow!(
                "Model dimension ({}) must be divisible by number of heads ({})",
                config.model_dim, config.num_heads
            ));
        }
        
        let head_dim = config.model_dim / config.num_heads;
        let scale = config.scale.unwrap_or((head_dim as f32).sqrt().recip());
        
        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        
        // Create projection layers
        let query_projection = linear(config.model_dim, config.model_dim, vs.pp("q_proj"))?;
        let key_projection = linear(config.model_dim, config.model_dim, vs.pp("k_proj"))?;
        let value_projection = linear(config.model_dim, config.model_dim, vs.pp("v_proj"))?;
        let output_projection = linear(config.model_dim, config.model_dim, vs.pp("o_proj"))?;
        
        debug!("MultiHeadAttention created: {} heads, {} dim per head", 
               config.num_heads, head_dim);
        
        Ok(Self {
            query_projection,
            key_projection,
            value_projection,
            output_projection,
            config,
            device,
            head_dim,
            scale,
        })
    }
    
    /// Forward pass through multi-head attention
    pub fn forward(
        &self, 
        query: &Tensor, 
        key: &Tensor, 
        value: &Tensor, 
        mask: Option<&Tensor>
    ) -> Result<Tensor> {
        let (seq_len, model_dim) = (query.dim(0)?, query.dim(1)?);
        let batch_size = 1; // Assuming batch size of 1 for simplicity
        
        // Project to Q, K, V
        let q = self.query_projection.forward(query)?;
        let k = self.key_projection.forward(key)?;
        let v = self.value_projection.forward(value)?;
        
        // Reshape for multi-head attention
        let q = self.reshape_for_attention(&q, seq_len)?;
        let k = self.reshape_for_attention(&k, seq_len)?;
        let v = self.reshape_for_attention(&v, seq_len)?;
        
        // Compute scaled dot-product attention
        let attention_output = self.scaled_dot_product_attention(&q, &k, &v, mask)?;
        
        // Reshape back and apply output projection
        let output = self.reshape_from_attention(&attention_output, seq_len)?;
        let final_output = self.output_projection.forward(&output)?;
        
        Ok(final_output)
    }
    
    /// Reshape tensor for multi-head attention computation
    fn reshape_for_attention(&self, tensor: &Tensor, seq_len: usize) -> Result<Tensor> {
        // Reshape from [seq_len, model_dim] to [num_heads, seq_len, head_dim]
        let reshaped = tensor.reshape((seq_len, self.config.num_heads, self.head_dim))?;
        reshaped.transpose(0, 1) // [num_heads, seq_len, head_dim]
    }
    
    /// Reshape tensor back from multi-head attention
    fn reshape_from_attention(&self, tensor: &Tensor, seq_len: usize) -> Result<Tensor> {
        // Reshape from [num_heads, seq_len, head_dim] back to [seq_len, model_dim]
        let transposed = tensor.transpose(0, 1)?; // [seq_len, num_heads, head_dim]
        transposed.reshape((seq_len, self.config.model_dim))
    }
    
    /// Scaled dot-product attention computation
    fn scaled_dot_product_attention(
        &self,
        query: &Tensor,
        key: &Tensor,
        value: &Tensor,
        mask: Option<&Tensor>,
    ) -> Result<Tensor> {
        // Compute attention scores: Q @ K^T / sqrt(d_k)
        let key_transposed = key.transpose(-2, -1)?;
        let attention_scores = query.matmul(&key_transposed)?;
        let scaled_scores = attention_scores.mul(&Tensor::new(self.scale, &self.device)?)?;
        
        // Apply mask if provided
        let masked_scores = if let Some(mask) = mask {
            scaled_scores.add(mask)?
        } else {
            scaled_scores
        };
        
        // Apply softmax to get attention weights
        let attention_weights = masked_scores.softmax(-1)?;
        
        // Apply attention to values: Attention @ V
        let attention_output = attention_weights.matmul(value)?;
        
        Ok(attention_output)
    }
    
    pub fn get_config(&self) -> &AttentionConfig {
        &self.config
    }
    
    pub fn get_attention_info(&self) -> HashMap<String, f32> {
        let mut info = HashMap::new();
        info.insert("model_dim".to_string(), self.config.model_dim as f32);
        info.insert("num_heads".to_string(), self.config.num_heads as f32);
        info.insert("head_dim".to_string(), self.head_dim as f32);
        info.insert("scale".to_string(), self.scale);
        info
    }
}

/// Self-attention layer for processing sequences
pub struct SelfAttentionLayer {
    multi_head_attention: MultiHeadAttention,
    layer_norm1: LayerNorm,
    layer_norm2: LayerNorm,
    feed_forward: FeedForwardNetwork,
    dropout: f64,
}

impl SelfAttentionLayer {
    pub fn new(config: AttentionConfig, device: Device) -> Result<Self> {
        let multi_head_attention = MultiHeadAttention::new(config.clone(), device.clone())?;
        let layer_norm1 = LayerNorm::new(config.model_dim, device.clone())?;
        let layer_norm2 = LayerNorm::new(config.model_dim, device.clone())?;
        let feed_forward = FeedForwardNetwork::new(config.model_dim, device)?;
        
        Ok(Self {
            multi_head_attention,
            layer_norm1,
            layer_norm2,
            feed_forward,
            dropout: config.dropout,
        })
    }
    
    pub fn forward(&self, input: &Tensor, mask: Option<&Tensor>) -> Result<Tensor> {
        // Self-attention with residual connection and layer norm
        let attention_output = self.multi_head_attention.forward(input, input, input, mask)?;
        let attention_residual = input.add(&attention_output)?;
        let attention_normalized = self.layer_norm1.forward(&attention_residual)?;
        
        // Feed-forward with residual connection and layer norm
        let ff_output = self.feed_forward.forward(&attention_normalized)?;
        let ff_residual = attention_normalized.add(&ff_output)?;
        let output = self.layer_norm2.forward(&ff_residual)?;
        
        Ok(output)
    }
}

/// Cross-attention layer for attending to external context
pub struct CrossAttentionLayer {
    multi_head_attention: MultiHeadAttention,
    layer_norm1: LayerNorm,
    layer_norm2: LayerNorm,
    feed_forward: FeedForwardNetwork,
    dropout: f64,
}

impl CrossAttentionLayer {
    pub fn new(config: AttentionConfig, device: Device) -> Result<Self> {
        let multi_head_attention = MultiHeadAttention::new(config.clone(), device.clone())?;
        let layer_norm1 = LayerNorm::new(config.model_dim, device.clone())?;
        let layer_norm2 = LayerNorm::new(config.model_dim, device.clone())?;
        let feed_forward = FeedForwardNetwork::new(config.model_dim, device)?;
        
        Ok(Self {
            multi_head_attention,
            layer_norm1,
            layer_norm2,
            feed_forward,
            dropout: config.dropout,
        })
    }
    
    pub fn forward(
        &self, 
        query: &Tensor, 
        key: &Tensor, 
        value: &Tensor, 
        mask: Option<&Tensor>
    ) -> Result<Tensor> {
        // Cross-attention with residual connection and layer norm
        let attention_output = self.multi_head_attention.forward(query, key, value, mask)?;
        let attention_residual = query.add(&attention_output)?;
        let attention_normalized = self.layer_norm1.forward(&attention_residual)?;
        
        // Feed-forward with residual connection and layer norm
        let ff_output = self.feed_forward.forward(&attention_normalized)?;
        let ff_residual = attention_normalized.add(&ff_output)?;
        let output = self.layer_norm2.forward(&ff_residual)?;
        
        Ok(output)
    }
}

/// Layer normalization implementation
pub struct LayerNorm {
    weight: Tensor,
    bias: Tensor,
    eps: f32,
}

impl LayerNorm {
    pub fn new(normalized_shape: usize, device: Device) -> Result<Self> {
        let weight = Tensor::ones((normalized_shape,), DType::F32, &device)?;
        let bias = Tensor::zeros((normalized_shape,), DType::F32, &device)?;
        let eps = 1e-5;
        
        Ok(Self { weight, bias, eps })
    }
    
    pub fn forward(&self, input: &Tensor) -> Result<Tensor> {
        let mean = input.mean_keepdim(-1)?;
        let var = input.var_keepdim(-1)?;
        let std = (var + self.eps)?.sqrt()?;
        
        let normalized = input.sub(&mean)?.div(&std)?;
        let output = normalized.mul(&self.weight)?.add(&self.bias)?;
        
        Ok(output)
    }
}

/// Feed-forward network for transformer layers
pub struct FeedForwardNetwork {
    linear1: Linear,
    linear2: Linear,
    dropout: f64,
}

impl FeedForwardNetwork {
    pub fn new(model_dim: usize, device: Device) -> Result<Self> {
        let hidden_dim = model_dim * 4; // Standard transformer FF expansion
        
        let varmap = VarMap::new();
        let vs = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        
        let linear1 = linear(model_dim, hidden_dim, vs.pp("linear1"))?;
        let linear2 = linear(hidden_dim, model_dim, vs.pp("linear2"))?;
        let dropout = 0.1;
        
        Ok(Self { linear1, linear2, dropout })
    }
    
    pub fn forward(&self, input: &Tensor) -> Result<Tensor> {
        let hidden = self.linear1.forward(input)?;
        let activated = hidden.gelu()?; // GELU activation
        let output = self.linear2.forward(&activated)?;
        
        Ok(output)
    }
}

/// Attention layer factory for creating different attention types
pub struct AttentionLayer;

impl AttentionLayer {
    pub fn new(
        attention_type: &str,
        model_dim: usize,
        num_heads: usize,
        device: Device,
    ) -> Result<Box<dyn AttentionModule>> {
        let config = AttentionConfig {
            model_dim,
            num_heads,
            dropout: 0.1,
            scale: None,
        };
        
        match attention_type {
            "self_attention" => {
                let layer = SelfAttentionLayer::new(config, device)?;
                Ok(Box::new(layer))
            }
            "cross_attention" => {
                let layer = CrossAttentionLayer::new(config, device)?;
                Ok(Box::new(layer))
            }
            "multi_head" => {
                let layer = MultiHeadAttention::new(config, device)?;
                Ok(Box::new(layer))
            }
            _ => Err(anyhow::anyhow!("Unknown attention type: {}", attention_type)),
        }
    }
}

/// Trait for attention modules
pub trait AttentionModule: Send + Sync {
    fn forward_attention(&self, query: &Tensor, key: &Tensor, value: &Tensor, mask: Option<&Tensor>) -> Result<Tensor>;
    fn get_attention_weights(&self, query: &Tensor, key: &Tensor) -> Result<Tensor>;
}

impl AttentionModule for MultiHeadAttention {
    fn forward_attention(&self, query: &Tensor, key: &Tensor, value: &Tensor, mask: Option<&Tensor>) -> Result<Tensor> {
        self.forward(query, key, value, mask)
    }
    
    fn get_attention_weights(&self, query: &Tensor, key: &Tensor) -> Result<Tensor> {
        // Compute attention weights without applying to values
        let q = self.query_projection.forward(query)?;
        let k = self.key_projection.forward(key)?;
        
        let seq_len = query.dim(0)?;
        let q = self.reshape_for_attention(&q, seq_len)?;
        let k = self.reshape_for_attention(&k, seq_len)?;
        
        let key_transposed = k.transpose(-2, -1)?;
        let attention_scores = q.matmul(&key_transposed)?;
        let scaled_scores = attention_scores.mul(&Tensor::new(self.scale, &self.device)?)?;
        let attention_weights = scaled_scores.softmax(-1)?;
        
        Ok(attention_weights)
    }
}

impl AttentionModule for SelfAttentionLayer {
    fn forward_attention(&self, query: &Tensor, _key: &Tensor, _value: &Tensor, mask: Option<&Tensor>) -> Result<Tensor> {
        // For self-attention, query is used as key and value
        self.forward(query, mask)
    }
    
    fn get_attention_weights(&self, query: &Tensor, _key: &Tensor) -> Result<Tensor> {
        self.multi_head_attention.get_attention_weights(query, query)
    }
}

impl AttentionModule for CrossAttentionLayer {
    fn forward_attention(&self, query: &Tensor, key: &Tensor, value: &Tensor, mask: Option<&Tensor>) -> Result<Tensor> {
        self.forward(query, key, value, mask)
    }
    
    fn get_attention_weights(&self, query: &Tensor, key: &Tensor) -> Result<Tensor> {
        self.multi_head_attention.get_attention_weights(query, key)
    }
}

/// Utility function to create causal mask for autoregressive generation
pub fn create_causal_mask(seq_len: usize, device: &Device) -> Result<Tensor> {
    let mut mask_data = Vec::with_capacity(seq_len * seq_len);
    
    for i in 0..seq_len {
        for j in 0..seq_len {
            if j > i {
                mask_data.push(f32::NEG_INFINITY); // Mask future positions
            } else {
                mask_data.push(0.0); // Allow current and past positions
            }
        }
    }
    
    let shape = Shape::from((seq_len, seq_len));
    Tensor::from_slice(&mask_data, shape, device)
}

/// Utility function to create padding mask
pub fn create_padding_mask(seq_lengths: &[usize], max_len: usize, device: &Device) -> Result<Tensor> {
    let batch_size = seq_lengths.len();
    let mut mask_data = Vec::with_capacity(batch_size * max_len);
    
    for &seq_len in seq_lengths {
        for pos in 0..max_len {
            if pos >= seq_len {
                mask_data.push(f32::NEG_INFINITY); // Mask padding positions
            } else {
                mask_data.push(0.0); // Allow valid positions
            }
        }
    }
    
    let shape = Shape::from((batch_size, max_len));
    Tensor::from_slice(&mask_data, shape, device)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Tensor;
    
    #[test]
    fn test_multi_head_attention_creation() {
        let device = Device::Cpu;
        let config = AttentionConfig::default();
        let attention = MultiHeadAttention::new(config, device);
        assert!(attention.is_ok());
    }
    
    #[test]
    fn test_attention_forward() {
        let device = Device::Cpu;
        let config = AttentionConfig {
            model_dim: 64,
            num_heads: 8,
            dropout: 0.1,
            scale: None,
        };
        
        let attention = MultiHeadAttention::new(config, device.clone()).unwrap();
        
        // Create test tensors
        let seq_len = 10;
        let model_dim = 64;
        let input_data = vec![0.1f32; seq_len * model_dim];
        let input_tensor = Tensor::from_slice(&input_data, (seq_len, model_dim), &device).unwrap();
        
        let result = attention.forward(&input_tensor, &input_tensor, &input_tensor, None);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert_eq!(output.dims(), &[seq_len, model_dim]);
    }
    
    #[test]
    fn test_layer_norm() {
        let device = Device::Cpu;
        let layer_norm = LayerNorm::new(64, device.clone()).unwrap();
        
        let input_data = vec![1.0f32; 64];
        let input_tensor = Tensor::from_slice(&input_data, (64,), &device).unwrap();
        
        let result = layer_norm.forward(&input_tensor);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert_eq!(output.dims(), &[64]);
    }
    
    #[test]
    fn test_causal_mask_creation() {
        let device = Device::Cpu;
        let mask = create_causal_mask(4, &device);
        assert!(mask.is_ok());
        
        let mask_tensor = mask.unwrap();
        assert_eq!(mask_tensor.dims(), &[4, 4]);
    }
    
    #[test]
    fn test_attention_layer_factory() {
        let device = Device::Cpu;
        
        let self_attention = AttentionLayer::new("self_attention", 64, 8, device.clone());
        assert!(self_attention.is_ok());
        
        let multi_head = AttentionLayer::new("multi_head", 64, 8, device.clone());
        assert!(multi_head.is_ok());
        
        let cross_attention = AttentionLayer::new("cross_attention", 64, 8, device);
        assert!(cross_attention.is_ok());
    }
}