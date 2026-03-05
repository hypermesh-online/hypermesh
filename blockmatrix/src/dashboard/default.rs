// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Embedded default dashboard HTML content.
//!
//! Three tiers are compiled into the binary via `include_str!` so a node
//! can serve a dashboard without any external files on disk.

/// Static welcome page for public visitors (no JavaScript).
pub const DEFAULT_PUBLIC_HTML: &str = include_str!("default_content/public.html");

/// Dashboard with live metric fetching for authenticated private peers.
pub const DEFAULT_PRIVATE_HTML: &str = include_str!("default_content/private.html");

/// Admin dashboard with DNS registration and peer connection controls.
pub const DEFAULT_ADMIN_HTML: &str = include_str!("default_content/admin.html");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_html_contains_hypermesh() {
        assert!(
            DEFAULT_PUBLIC_HTML.contains("HyperMesh"),
            "public.html must mention HyperMesh"
        );
    }

    #[test]
    fn test_private_html_contains_fetch() {
        assert!(
            DEFAULT_PRIVATE_HTML.contains("fetch("),
            "private.html must use fetch() for live data"
        );
    }

    #[test]
    fn test_admin_html_contains_form() {
        assert!(
            DEFAULT_ADMIN_HTML.contains("<input"),
            "admin.html must contain input elements"
        );
        assert!(
            DEFAULT_ADMIN_HTML.contains("<button"),
            "admin.html must contain button elements"
        );
    }

    #[test]
    fn test_all_html_valid_doctype() {
        for (name, html) in [
            ("public", DEFAULT_PUBLIC_HTML),
            ("private", DEFAULT_PRIVATE_HTML),
            ("admin", DEFAULT_ADMIN_HTML),
        ] {
            assert!(
                html.starts_with("<!DOCTYPE html>"),
                "{name}.html must start with <!DOCTYPE html>"
            );
        }
    }

    #[test]
    fn test_html_not_empty() {
        for (name, html) in [
            ("public", DEFAULT_PUBLIC_HTML),
            ("private", DEFAULT_PRIVATE_HTML),
            ("admin", DEFAULT_ADMIN_HTML),
        ] {
            assert!(
                html.len() > 100,
                "{name}.html must have more than 100 bytes, got {}",
                html.len()
            );
        }
    }
}
