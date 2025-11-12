//! Satchel - HyperMesh Asset Library
//! 
//! Manages all asset types: CPU, GPU, Memory, Storage, Network, Container, Economic

pub mod core;
pub mod adapters;
pub mod privacy;
pub mod proxy;

// Re-export commonly used types
pub use core::*;
pub use adapters::*;
pub use privacy::*;
pub use proxy::*;
