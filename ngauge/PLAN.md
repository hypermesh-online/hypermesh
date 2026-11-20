# ngauge delivery plan

**Planning baseline plus bounded executed units; not a ratified protocol, complete native backlog, blanket future implementation lease or user acceptance.** Parent: [core plan](../PLAN.md). Scope evidence: [README.md](README.md). Native roadmap identity `ce1670a571a049c58a7a4baad1ac66388f3541984cb4898ae56ae7cdd545995a` is a reference, not a populated backlog. Coordinator `/root/coordinator_ngauge` owns the DNS lane; current task evidence is recorded below, not inferred from a handoff ACK.

## Current work and next gates — observed 2026-11-19 at `4913bb92`

- Network-scoped DNS is committed in `52d037f3`. Independent original-source checks passed ten DNS controls, two integration fixtures and a workspace all-targets compile; the earlier shared-cache escape remains historical evidence. No delegated authority, real-network authentication, PQ, semantic PoS or IPv6 operation follows.
- Canonical provider identity is committed in `b04b391e`. Independent original-source checks passed six NGauge identity tests and five BlockMatrix seam tests. Earlier rehash and unindexed-provider failures remain corrected history. This demonstrates bounded identity/index membership, not live authorization or servability.
- NGauge server-feature macro imports are fixed in `df53b039`; the exact repair independently passed NGauge-package all-features/all-targets compilation. The broader workspace all-features check remains failed on separate targets (see the [core plan](../PLAN.md)).
- Next: bind canonical providers to currently servable bytes and prove scoped authority and admission on a real network. Asset-specific IPv6 locator/authority integration remains design only (`5407fa27e9ea61ff60c14e73466f3cf4c6e169cfa38c6cd06734b50bf1124dbf`); no new owner or version authority is inferred.

## Scope and current gap

Sources: README.md §Architecture/Modules; crate-status.toml:33–35. Fetched bytes need current servability and phantom-provider protection; universal disk persistence is not approved. Original provider rehash mismatch has a bounded RED reproduction; corrected invariant failure and actual-original trial evidence are described above, not real-network outage proof.

## Next proposed sprint

- **Deliverable/action:** Deliver a canonical index → planner → daemon transport lookup ID contract and active fetched-byte serving boundary.
- **Proposed exclusive file ownership before code:** `ngauge/ DMS, index, planner and daemon transport source plus NGauge-owned tests`. Freeze boundaries before parallel work; cross-owner changes require explicit coordination.
- **Dependency/decision gate:** Freeze provider identity and Catalog/BlockMatrix/STOQ active-serving boundaries; investigate existing exchange retention and avoid another version store.
- **Future acceptance criteria:** Tests to add: canonical matching ID locates currently servable bytes; mismatched/unknown IDs, empty fetch and unavailable serving cannot advertise phantom providers. A departed mirror is not active; historical context is distinct. No blanket restart persistence assertion. Five bounded unit passes are not end-to-end evidence.

## Phase status and limits

Expected: a project-specific actionable plan and the reproducible behavior above. Actual: initial drafts were under-specific and omitted three important caveats; documentation QA failed. These corrections remain unverified until independent consolidated final QA; this is a documentation failure, not a product failure. The sprint, its proposed tests and product behavior are not claimed complete.

Follow the [parent workflow](../PLAN.md): development components and their tests precede one consolidated QA covering correctness, security/privacy, efficiency, maintainability and compatibility; fix and reverify failures, with user-only acceptance. Measure relevant resource use against an agreed baseline and record expected versus actual evidence. Feature → PR → main; no unapproved push or release.

Native core frontend reachable on 2026-11-18 using explicit project selector and actor; historical client failures below are not current availability. Native first intake is not full 2,551-census coverage, readiness or acceptance. Private discovery requires owner publication; private networks may be offline with their own CA/DNS and public registration is optional. Signed Catalog discovery references do not grant private access; data and ownership remain sovereign. Local device/network index and bootstrap metadata are not a global consensus chain. Protocol scopes assets; participation does not transfer device ownership. IPv6-only remains a requirement, not demonstrated runtime coverage.
