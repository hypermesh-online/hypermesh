//! Julia VM primitives

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JuliaPrimitives {
    pub math_functions: bool,
    pub array_operations: bool,
    pub string_functions: bool,
}

impl Default for JuliaPrimitives {
    fn default() -> Self {
        Self {
            math_functions: true,
            array_operations: true,
            string_functions: true,
        }
    }
}

pub struct PrimitiveFunction {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: String,
}

impl JuliaPrimitives {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_available_functions(&self) -> Vec<PrimitiveFunction> {
        vec![
            PrimitiveFunction {
                name: "add".to_string(),
                parameters: vec!["x".to_string(), "y".to_string()],
                return_type: "Number".to_string(),
            },
        ]
    }
}