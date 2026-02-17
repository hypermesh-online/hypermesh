// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Examples for Catalog VM Matrix Integration
//!
//! This module contains comprehensive examples demonstrating how to use
//! the Catalog VM system with matrix chain integration for various
//! real-world scenarios.

pub mod matrix_execution;

pub use matrix_execution::{
    vehicle_purchase_workflow_example,
    medical_data_processing_example,
    iot_device_coordination_example,
};

/// Run all matrix integration examples
pub async fn run_all_examples() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Running all Matrix Integration Examples\n");
    
    // Vehicle purchase workflow
    if let Err(e) = vehicle_purchase_workflow_example().await {
        eprintln!("❌ Vehicle purchase example failed: {}", e);
    }
    
    println!("\n{}\n", "=".repeat(60));
    
    // Medical data processing
    if let Err(e) = medical_data_processing_example().await {
        eprintln!("❌ Medical data example failed: {}", e);
    }
    
    println!("\n{}\n", "=".repeat(60));
    
    // IoT device coordination
    if let Err(e) = iot_device_coordination_example().await {
        eprintln!("❌ IoT coordination example failed: {}", e);
    }
    
    println!("\n🎉 All examples completed!");
    
    Ok(())
}