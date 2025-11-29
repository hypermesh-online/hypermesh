//! Julia VM macros

/// Julia macro support
pub struct JuliaMacros {
    pub enabled: bool,
}

impl Default for JuliaMacros {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl JuliaMacros {
    pub fn new() -> Self {
        Self::default()
    }
}