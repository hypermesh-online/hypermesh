// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Foundation-reserved domain registry (Phase H.1).
//!
//! Reserved domains cannot be registered without a foundation CA-signed
//! [`FoundationGrant`](crate::dns::grant::FoundationGrant). The list is
//! sourced from `blockmatrix/dns_reserved.toml` at the workspace root
//! and is parsed once on first access via `lazy_static`.
//!
//! Lookup is O(1) against a `HashSet<String>` of lowercase-folded names.
//! The reserved-domain check fires before any chain write inside
//! `DnsRegistrar::register_domain`, so a node without a grant simply
//! cannot register a reserved name — this is the protective default
//! and is always on, even on alpha nodes that have not opted into the
//! foundation root key.
//!
//! ## File format
//!
//! ```toml
//! [hypermesh_internal]
//! domains = ["hypermesh", "trust", ...]
//!
//! [fortune_1000]
//! domains = ["apple", "nike", ...]
//!
//! [high_value_tld]
//! domains = ["www", "admin", ...]
//! ```
//!
//! Sections beyond these three are tolerated and merged. Adding entries
//! requires a daemon restart but no rebuild.

use std::collections::{BTreeMap, HashSet};

use lazy_static::lazy_static;
use serde::Deserialize;

/// Embedded copy of `dns_reserved.toml`.
///
/// `include_str!` resolves relative to this source file, which lives at
/// `blockmatrix/src/dns/reserved.rs`, so `../../dns_reserved.toml`
/// points at `blockmatrix/dns_reserved.toml`. Embedding at compile time
/// keeps the lookup self-contained: musl release binaries deployed to
/// trust.hypermesh.online don't need to ship the TOML alongside.
const RESERVED_TOML: &str = include_str!("../../dns_reserved.toml");

/// Top-level structure mirroring `dns_reserved.toml`.
///
/// Each section is a free-form key with a `domains` list; we collect
/// every list and union them into a single set.
#[derive(Debug, Deserialize)]
struct ReservedFile {
    #[serde(flatten)]
    sections: BTreeMap<String, ReservedSection>,
}

#[derive(Debug, Deserialize)]
struct ReservedSection {
    #[serde(default)]
    domains: Vec<String>,
}

lazy_static! {
    /// Compiled reserved-domain set (lowercase-folded, deduplicated).
    static ref RESERVED: HashSet<String> = {
        let parsed: ReservedFile = toml::from_str(RESERVED_TOML)
            .expect("dns_reserved.toml must be valid TOML at compile time");

        let mut set = HashSet::new();
        for (_section_name, section) in parsed.sections {
            for name in section.domains {
                set.insert(name.to_lowercase());
            }
        }
        set
    };
}

/// Returns `true` if `domain_name` is foundation-reserved.
///
/// The check uses the *root* domain component (everything after the
/// last `.`), case-folded to lowercase. This means `nike` and `NIKE`
/// are both reserved, and a federated subdomain `admin.nike` is
/// *also* considered reserved because its root is `nike`.
///
/// Why root-component matching: registering `admin.nike` without a
/// grant would let an attacker masquerade as Nike's authoritative DNS
/// for that subtree. The grant flow scopes a foundation authorization
/// to a specific reserved root, so `admin.nike` registration requires
/// either a grant for `nike` (which the recipient owns) or for
/// `admin.nike` directly.
pub fn is_reserved(domain_name: &str) -> bool {
    let trimmed = domain_name.trim().to_lowercase();
    if trimmed.is_empty() {
        return false;
    }

    // Check the full name first — handles cases like `coca-cola` where
    // the TOML lists the multi-word form directly.
    if RESERVED.contains(&trimmed) {
        return true;
    }

    // Then check the root component (everything after the last `.`)
    // so subdomains of reserved roots are also caught.
    if let Some(root) = trimmed.rsplit('.').next() {
        if RESERVED.contains(root) {
            return true;
        }
    }

    false
}

/// Returns the total number of reserved domains in the compiled set.
///
/// Exposed for diagnostics, status reporting, and tests that need to
/// assert the registry actually loaded.
pub fn reserved_count() -> usize {
    RESERVED.len()
}

/// Returns a sorted snapshot of the reserved-domain set.
///
/// Useful for diagnostic IPC handlers (`dns.reserved_list`) and tests.
/// The set is cloned, so callers cannot mutate the live registry.
pub fn reserved_list() -> Vec<String> {
    let mut v: Vec<String> = RESERVED.iter().cloned().collect();
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_nontrivial_count() {
        // The TOML ships with ~150 entries across three sections.
        assert!(
            reserved_count() > 100,
            "expected reserved set >100 entries, got {}",
            reserved_count()
        );
    }

    #[test]
    fn hypermesh_internal_reserved() {
        assert!(is_reserved("hypermesh"));
        assert!(is_reserved("trust"));
        assert!(is_reserved("caesar"));
        assert!(is_reserved("foundation"));
    }

    #[test]
    fn fortune_brands_reserved() {
        assert!(is_reserved("nike"));
        assert!(is_reserved("apple"));
        assert!(is_reserved("microsoft"));
        assert!(is_reserved("openai"));
    }

    #[test]
    fn case_folded() {
        assert!(is_reserved("NIKE"));
        assert!(is_reserved("Apple"));
        assert!(is_reserved("HYPERMESH"));
    }

    #[test]
    fn user_domains_not_reserved() {
        assert!(!is_reserved("myhomeserver"));
        assert!(!is_reserved("alephpt"));
        assert!(!is_reserved("just-a-random-name"));
    }

    #[test]
    fn subdomain_of_reserved_is_reserved() {
        // admin.nike has root "nike" — reserved
        assert!(is_reserved("admin.nike"));
        assert!(is_reserved("warehouse.admin.nike"));
    }

    #[test]
    fn subdomain_with_unreserved_root_is_not_reserved() {
        // admin.alephpt has root "alephpt" — not reserved
        assert!(!is_reserved("admin.alephpt"));
    }

    #[test]
    fn empty_input_not_reserved() {
        assert!(!is_reserved(""));
        assert!(!is_reserved("   "));
    }

    #[test]
    fn reserved_list_is_sorted_and_lowercase() {
        let list = reserved_list();
        for window in list.windows(2) {
            assert!(window[0] <= window[1], "list must be sorted");
        }
        for entry in &list {
            assert_eq!(entry, &entry.to_lowercase(), "entries must be lowercase");
        }
    }
}
