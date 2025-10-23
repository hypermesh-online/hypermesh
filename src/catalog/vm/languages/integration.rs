//! Multi-language integration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageIntegration {
    pub supported_languages: Vec<String>,
}

impl Default for LanguageIntegration {
    fn default() -> Self {
        Self {
            supported_languages: vec![
                "julia".to_string(),
                "python".to_string(),
                "rust".to_string(),
            ],
        }
    }
}

impl LanguageIntegration {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_language_supported(&self, language: &str) -> bool {
        self.supported_languages.contains(&language.to_string())
    }
}