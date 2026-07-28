// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! D4 — per-network validation rules.
//!
//! CANONICAL MODEL (VISION §5, the Recursion): a proof is checked against the
//! rules of the NETWORK that owns the asset — "a network's rules ARE its
//! genesis SystemAssets". There is no single global bar; validation SELECTS a
//! [`StateRequirements`] by the asset's [`NetworkScope`].
//!
//! This module is the structural seam for that selection. It does NOT invent
//! any magnitude: it maps a scope to an *existing* requirements object. The
//! only quantitative bound a `StateRequirements` carries is the WHEN proof's
//! clock freshness (`max_time_offset`); scope selection is by IDENTITY, never
//! by a quantity.
//!
//! ## Non-regression (ABSOLUTE)
//!
//! Today no network publishes its own genesis ruleset — the genesis-SystemAsset
//! machinery is a later phase (VISION §5, §8). Meanwhile a live path already
//! produces non-`Global` assets: the IPC store handler stamps
//! `NetworkScope::Private` on Bounded-privacy stores
//! (`ipc/handlers/store.rs::node_private_scope`). Those assets validate against
//! the one existing bar today. Therefore the resolver's conservative default is
//! **inherit the default requirements for every scope without a published
//! ruleset** — `Global` and unseen-private alike — so day-one behaviour is
//! byte-identical. Divergence becomes real only once [`publish`] is fed a real
//! genesis ruleset.
//!
//! [`publish`]: NetworkRuleSet::publish

use std::collections::HashMap;

use crate::assets::core::asset_id::NetworkScope;
use trustchain::proof_of_state::StateRequirements;

/// A network named by an asset's scope for which we cannot produce a ruleset.
///
/// Returned only by the DOCUMENTED, INACTIVE fail-closed path
/// ([`NetworkRuleSet::resolve_strict`]). The live path
/// ([`NetworkRuleSet::resolve`]) never produces this — it inherits the default
/// instead, which is what keeps currently-valid traffic flowing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownNetwork(pub String);

impl std::fmt::Display for UnknownNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "network {} has published no genesis ruleset — cannot produce its validation rules",
            self.0,
        )
    }
}

impl std::error::Error for UnknownNetwork {}

/// Resolver: `NetworkScope` -> `StateRequirements`.
///
/// Holds one `default` (the migration anchor — the requirements the owning
/// [`ValidationService`](super::validation_service::ValidationService) was
/// constructed with) plus an initially-empty table of per-network published
/// rulesets. The table stays empty until network-genesis rulesets exist.
#[derive(Clone, Debug)]
pub struct NetworkRuleSet {
    /// The anchor: applied to `Global` and to any scope without a published
    /// ruleset. Bound to the service's fixed requirements so behaviour is
    /// byte-identical to the pre-D4 single-bar model.
    default: StateRequirements,
    /// Per-network rulesets, keyed by scope. Empty today (no live producer);
    /// populated only from a network's genesis SystemAssets.
    published: HashMap<NetworkScope, StateRequirements>,
}

impl NetworkRuleSet {
    /// New resolver whose default (anchor) is `default`.
    ///
    /// Binding the default to the caller's fixed requirements — rather than
    /// hardcoding `production()` — is what guarantees byte-identical behaviour
    /// across every construction path (`new` → `default()`,
    /// `with_requirements(r)` → `r`, `for_production` → `production()`).
    pub fn new(default: StateRequirements) -> Self {
        Self {
            default,
            published: HashMap::new(),
        }
    }

    /// LIVE resolution — conservative and non-regressing.
    ///
    /// `Global` → the default anchor. Any scope WITHOUT a published ruleset →
    /// the default anchor. A scope WITH a published ruleset → that ruleset
    /// (real per-network divergence, once genesis rulesets exist). This never
    /// rejects on an unseen network — rejecting would drop the `Private`-scope
    /// assets the store handler already produces.
    pub fn resolve(&self, scope: &NetworkScope) -> &StateRequirements {
        match scope {
            NetworkScope::Global => &self.default,
            other => self.published.get(other).unwrap_or(&self.default),
        }
    }

    /// DOCUMENTED, INACTIVE fail-closed hook.
    ///
    /// Rejects an asset naming a network whose rules we cannot produce, EXCEPT
    /// `Global` (always the anchor). This is the VISION §5 end state — "joining
    /// a network means adopting its genesis" — where an entry naming a network
    /// we have not adopted is dropped rather than validated against a bar that
    /// is not that network's.
    ///
    /// It is NOT wired into any live validation path, because enabling it today
    /// would REGRESS: the IPC store handler produces `NetworkScope::Private`
    /// assets that have no published ruleset, and this would reject every one
    /// of them. It flips on — one call site,
    /// [`ValidationService::resolve_rules`](super::validation_service) — only
    /// once [`publish`] is fed real genesis rulesets.
    ///
    /// [`publish`]: NetworkRuleSet::publish
    // activated when network-genesis rulesets exist
    pub fn resolve_strict(
        &self,
        scope: &NetworkScope,
    ) -> Result<&StateRequirements, UnknownNetwork> {
        match scope {
            NetworkScope::Global => Ok(&self.default),
            other => self
                .published
                .get(other)
                .ok_or_else(|| UnknownNetwork(format!("{other:?}"))),
        }
    }

    /// Publish a network's ruleset — the rules carried by that network's
    /// genesis SystemAssets (VISION §5).
    ///
    /// No live producer yet: genesis-rules machinery is a later phase. This is
    /// the seam a future phase writes into so a network's rules can diverge
    /// from the anchor without touching validation call sites.
    pub fn publish(&mut self, scope: NetworkScope, requirements: StateRequirements) {
        self.published.insert(scope, requirements);
    }

    /// The default anchor (migration reference). Exposed for tests and future
    /// callers that need to compare a resolved ruleset against the anchor.
    pub fn default_requirements(&self) -> &StateRequirements {
        &self.default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::asset_id::NodeFingerprint;
    use std::time::Duration;

    fn anchor() -> StateRequirements {
        StateRequirements::production()
    }

    fn private(seed: u8) -> NetworkScope {
        NetworkScope::Private(NodeFingerprint([seed; 32]))
    }

    /// The migration anchor: `Global` resolves to the exact default the
    /// resolver was built with — byte-identical to the pre-D4 single bar.
    #[test]
    fn global_resolves_to_the_anchor() {
        let rules = NetworkRuleSet::new(anchor());
        assert_eq!(*rules.resolve(&NetworkScope::Global), anchor());
    }

    /// Conservative default: a scope with no published ruleset inherits the
    /// anchor. This is what keeps the store handler's `Private` assets valid.
    #[test]
    fn unseen_private_inherits_the_anchor() {
        let rules = NetworkRuleSet::new(anchor());
        assert_eq!(*rules.resolve(&private(7)), anchor());
    }

    /// Real per-network divergence once a ruleset is published: the published
    /// scope resolves to ITS rules, others still inherit the anchor.
    #[test]
    fn published_ruleset_diverges_from_anchor() {
        let mut rules = NetworkRuleSet::new(anchor());
        let stricter = StateRequirements {
            max_time_offset: Duration::from_secs(5),
        };
        rules.publish(private(1), stricter.clone());

        assert_eq!(*rules.resolve(&private(1)), stricter);
        assert_ne!(*rules.resolve(&private(1)), anchor());
        // A DIFFERENT private network still inherits the anchor.
        assert_eq!(*rules.resolve(&private(2)), anchor());
        // Global is untouched.
        assert_eq!(*rules.resolve(&NetworkScope::Global), anchor());
    }

    /// The inactive fail-closed hook: `Global` always resolves; an unseen
    /// network is rejected; a published one resolves to its rules. Proves the
    /// hook is correct SO the decision to leave it inactive is deliberate, not
    /// a gap.
    #[test]
    fn strict_hook_fails_closed_on_unseen_but_not_on_honest() {
        let mut rules = NetworkRuleSet::new(anchor());
        // Honest, adopted network: resolvable.
        rules.publish(private(9), anchor());

        assert!(rules.resolve_strict(&NetworkScope::Global).is_ok());
        assert!(rules.resolve_strict(&private(9)).is_ok());
        // Unseen network: the strict path rejects (fail-closed), while the
        // LIVE path (`resolve`) would inherit the anchor — proving why the
        // hook must stay inactive today.
        assert!(rules.resolve_strict(&private(42)).is_err());
        assert_eq!(*rules.resolve(&private(42)), anchor());
    }
}
