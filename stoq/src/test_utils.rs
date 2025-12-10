//! Test utilities for STOQ
//!
//! Provides common test helpers including crypto provider initialization

use std::sync::Once;

/// Global test initialization
static TEST_INIT: Once = Once::new();

/// Initialize test environment including crypto provider
pub fn init_test_crypto() {
    TEST_INIT.call_once(|| {
        // Install crypto provider for tests
        if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
            // Provider might already be installed, which is fine
            eprintln!("Test crypto provider initialization: {:?}", e);
        }
    });
}

/// Helper macro to ensure crypto is initialized in tests
#[macro_export]
macro_rules! init_test {
    () => {
        $crate::test_utils::init_test_crypto();
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_crypto() {
        // First call initializes
        init_test_crypto();

        // Second call is safe (no-op due to Once)
        init_test_crypto();
    }
}