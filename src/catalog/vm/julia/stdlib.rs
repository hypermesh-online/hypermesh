//! Julia VM standard library

/// Julia standard library support
pub struct JuliaStdLib {
    pub enabled: bool,
}

impl Default for JuliaStdLib {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl JuliaStdLib {
    pub fn new() -> Self {
        Self::default()
    }
}