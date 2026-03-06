// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Embedded default dashboard HTML content.
//!
//! Two scopes are compiled into the binary via `include_str!` so a node
//! can serve a dashboard without any external files on disk.

/// Public onboarding dashboard — fetches node status, domains, and assets
/// via `/api/v1/*` endpoints. Dark theme, vanilla JS, no external deps.
pub const DEFAULT_PUBLIC_HTML: &str = include_str!("default_content/public.html");

/// Private dashboard — the full node management UI. Auto-refreshes every
/// 5 s, shows blockchain height, peers, DNS, assets, domains, recent blocks,
/// plus management controls (DNS registration, peer connect, domain
/// registration, config viewer, dashboard list, and node shutdown).
pub const DEFAULT_PRIVATE_HTML: &str = include_str!("default_content/private.html");

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
    fn test_private_html_contains_admin_controls() {
        assert!(
            DEFAULT_PRIVATE_HTML.contains("<input"),
            "private.html must contain input elements (admin controls)"
        );
        assert!(
            DEFAULT_PRIVATE_HTML.contains("<button"),
            "private.html must contain button elements (admin controls)"
        );
    }

    #[test]
    fn test_all_html_valid_doctype() {
        for (name, html) in [
            ("public", DEFAULT_PUBLIC_HTML),
            ("private", DEFAULT_PRIVATE_HTML),
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
        ] {
            assert!(
                html.len() > 100,
                "{name}.html must have more than 100 bytes, got {}",
                html.len()
            );
        }
    }
}
