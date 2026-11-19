# catalog delivery plan

**Planning baseline plus bounded executed units; not a ratified protocol, complete native backlog, blanket future implementation lease or user acceptance.** Parent: [core plan](../PLAN.md). Scope evidence: [README.md](README.md). Native roadmap identity `fbbe3bf213677913388844479efdd7ef1adcfc7b9d8caa77a12a392343aaa214` is a reference, not a populated backlog. Coordinator `/root/coordinator_catalog` owns the bounded current implementation lane; native task state is recorded below, not inferred from a handoff ACK.

## Current work and next gates — observed 2026-11-18

- Catalog task `041b1bffed64526dc260d6d5042e2d87651860be0290977f71260acc8d06a24a` RUNNING rev14: EIGHT of15 distinct system payload descriptors are INTEGRATED UNCOMMITTED: first five plus SystemInvitation, Message and Transmission. Independent actual ORIGINAL-source focused trial, no candidate overlay, passed24/24 exit0 in15.908s (`catalog-transmission-actual-original-postintegration-trial`). Messaging task `536b1b61013df05644f277de7be80d59b51b2ad13a1b9309d0aa1add278e8c03` REVIEW rev6; Transmission `92bf5ab7def3a2a286e2b71c018c621cd83805c8a6b62022b37701a7a6e94d92` REVIEW rev6. Remaining SEVEN: Container, Economic, Blockchain, DNS, Dashboard, Identity, KeyRotation; network-scoped versioned registry, lineage and authority remain OPEN. Owner requires ALL15; application schemas stay separate. Transmission `bandwidth_bps: u64` is declared relay capacity, zero=no capacity, not a transfer/reservation or verified capacity. Structure-only validation proves no hardware, PQ, PoS or admission.
- Historical first-five evidence: initial8 focused passes, independentFAIL2, corrected10 passes and independent correctionPASS remain retained in `catalog-first-five-independent-qa-findings` and `catalog-first-five-correction-independent-qa-pass`; the newer24-pass result does not erase failures or accept the product. Next: freeze remaining seven payload contracts and registry/authority integration before further implementation.
- Scoped versioned registry and authorized owner/chain producer integration remain open. Design task `48cd440823207171cc2257a3ce32ad630009c97fcb44222e46e2229cf4168e49` and authority task `5407fa27e9ea61ff60c14e73466f3cf4c6e169cfa38c6cd06734b50bf1124dbf` are REVIEW design evidence, not deployed authorization. Preserve one lineage/version authority and network-scoped private publication gates.

## Scope and current gap

Sources: README.md §Architecture/Integration; crate-status.toml:50–52. Deployable Catalog–BlockMatrix registry seam is not established by documentation.

## Next proposed sprint

- **Deliverable/action:** Specify and implement a scoped registry integration with signed discovery references and owner-controlled access.
- **Proposed exclusive file ownership before code:** `catalog/src/ and catalog-owned integration tests`. Freeze boundaries before parallel work; cross-owner changes require explicit coordination.
- **Dependency/decision gate:** Freeze registry and NGauge routing, version identity, same-name successor and branch-pointer decisions; no extra version store.
- **Future acceptance criteria:** Tests to add: authorized signed registration resolves; invalid signature, wrong owner and unpublished private record are rejected or hidden.

## Phase status and limits

Expected: a project-specific actionable plan and the reproducible behavior above. Actual: initial drafts were under-specific and omitted three important caveats; documentation QA failed. These corrections remain unverified until independent consolidated final QA; this is a documentation failure, not a product failure. The sprint, its proposed tests and product behavior are not claimed complete.

Follow the [parent workflow](../PLAN.md): development components and their tests precede one consolidated QA covering correctness, security/privacy, efficiency, maintainability and compatibility; fix and reverify failures, with user-only acceptance. Measure relevant resource use against an agreed baseline and record expected versus actual evidence. Feature → PR → main; no unapproved push or release.

Native core frontend reachable on 2026-11-18 using explicit project selector and actor; historical client failures below are not current availability. Native first intake is not full 2,551-census coverage, readiness or acceptance. Private discovery requires owner publication; private networks may be offline with their own CA/DNS and public registration is optional. Signed Catalog discovery references do not grant private access; data and ownership remain sovereign. Local device/network index and bootstrap metadata are not a global consensus chain. Protocol scopes assets; participation does not transfer device ownership. IPv6-only remains a requirement, not demonstrated runtime coverage.

## Post-integration status — 2026-11-11

- Initial full 13-member all-targets check failed on missing `Arc<ShardStore>` argument. Signed `7f2d0d0` shares the same fixture store between Seeder and ConsumerProviderManager, without weakened assertions, and was locally fast-forwarded onto original DMS.
- Corrected candidate with two user-comment overlays passed workspace all-targets check (Cargo 0.91 s; wrapper 2.00064 s); it compiled 13 Rust members, not 17 projects, and did not execute tests.
- `dms_handoff_new_version_mirrors_and_reflects` executed once and passed in 186.007 s with in-process `MockShardTransport`; it is logical fixture evidence, not real network E2E, private authorization, hardware or user acceptance. Other dirty code remains unvalidated.
- **Next gate:** Audit actual authorization and channel binding before private Catalog/two-node claims; preserve Catalog-issued discovery references and owner-controlled asset access without a second version store.
