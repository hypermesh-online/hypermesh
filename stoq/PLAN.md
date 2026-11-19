# stoq delivery plan

**Planning baseline plus bounded executed units; not a ratified protocol, complete native backlog, blanket future implementation lease or user acceptance.** Parent: [core plan](../PLAN.md). Scope evidence: [README.md](README.md). Native roadmap identity `6ad9d99dec368e17fa2f9166cc2345f782f8499fec1884f0525148a7ce2b5edc` is a reference, not a populated backlog. Coordinator `/root/coordinator_stoq` owns transport work; F1/listener integration is historical completed source work, not the next integration gate.

## Current work and next gates — observed 2026-11-18

- Do not reopen the integrated F1/listener unit. Next security work is genuine authenticated session/context association, channel binding, semantic four-proof evidence and replay denial, not a transport-configuration inference.
- Routing/demand correction `fc8a69ae5cc92b842d84496c0a78f8c08a1d5df0639d027f74acd0666f423d29` remains REVIEW evidence; independent focused controls are not private authenticated network E2E. Review remaining authorization/transport gates and obtain explicit bounded ownership before new code.
- Installer and DNS integrations are uncommitted source units with scoped mock/fixture evidence, not STOQ private join, PQ, PoS or hardware validation. Feature → PR → main is the owner exception; no master creation, push or release is authorized here.

## Scope and current gap

Sources: README.md §Transport/Certificate Management/Security. Authenticated session binding and replay controls remain integration gates; F1 is already locally integrated at DMS `7f2d0d0`, as recorded below.

## Next proposed sprint

- **Deliverable/action:** Freeze genuine authenticated QUIC session-to-context binding and replay controls, then implement only under a separate bounded lease; do not reintegrate F1.
- **Proposed exclusive file ownership before code:** `stoq/ bilateral, transport and certificate source and STOQ-owned tests, including stoq/wasm/`. Freeze boundaries before parallel work; cross-owner changes require explicit coordination.
- **Dependency/decision gate:** Coordinate TrustChain semantic four-proof validation and BlockMatrix transport; retain the integrated F1/listener source and historical test evidence without upgrading it to authenticated E2E.
- **Future acceptance criteria:** Tests to add: valid proof binds to intended QUIC session; failed sub-proof, replay and wrong-session proof reject access; UTF-8 identity boundary does not panic.

## Phase status and limits

Expected: a project-specific actionable plan and the reproducible behavior above. Actual: initial drafts were under-specific and omitted three important caveats; documentation QA failed. A subsequent final documentation QA found two wording blockers; this correction remains unverified until independent delta QA; this is a documentation failure, not a product failure. The sprint, its proposed tests and product behavior are not claimed complete.

Follow the [parent workflow](../PLAN.md): development components and their tests precede one consolidated QA covering correctness, security/privacy, efficiency, maintainability and compatibility; fix and reverify failures, with user-only acceptance. Measure relevant resource use against an agreed baseline and record expected versus actual evidence. Feature → PR → main; no unapproved push or release.

Native core frontend reachable on 2026-11-18 using explicit project selector and actor; historical client failures below are not current availability. Native first intake is not full 2,551-census coverage, readiness or acceptance. Private discovery requires owner publication; private networks may be offline with their own CA/DNS and public registration is optional. Signed Catalog discovery references do not grant private access; data and ownership remain sovereign. Local device/network index and bootstrap metadata are not a global consensus chain. Protocol scopes assets; participation does not transfer device ownership. IPv6-only remains a requirement, not demonstrated runtime coverage.

## Post-integration status — 2026-11-11

- The signed listener candidate is locally integrated on DMS at `7f2`; earlier F1 Unicode fix remains integrated and is not reopened. This is not a main push or user acceptance.
- `--stoq-bind-address` defaults to `::` and is CLI-only. Explicit loopback skips STOQ and node kernel acceleration before probe/init; `TransportConfig.enable_kernel_acceleration` defaults true for non-loopback. `accept_next` separates peer error from terminal closure; legacy wrapper remains.
- Bounded real bad ALPN, healthy, idle/closed and legacy cases passed. Production anonymous `::1` bind and occupied rejection passed; SIGTERM exited within 0.166 s without `NoIncoming` warning. The prior log flood happened after SIGTERM, not during idle startup.
- `AcceptAll`/no-client-auth is transport configuration, not proof of application admission bypass. Private authenticated E2E, identity/channel binding and replay defense remain unverified; audit those before two-node or readiness claims. Hardware was unavailable.
