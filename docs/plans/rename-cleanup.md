# Rename Cleanup Plan: core → hypermesh

**Status**: Draft — open decisions pending (see "Open Decisions")
**Scope**: Repository rename (core → hypermesh), stale-doc correction, build artifact hygiene, CI/build repair
**Canonical replacement**: `github.com/hypermesh-online/core` → `github.com/hypermesh-online/hypermesh`

## Context

The repository was renamed from `core` to `hypermesh`. The tree still carries extensive
references to the old repo name (`hypermesh-online/core`), the old clone layout
(`/home/persist/hypermesh/core`), legacy `web3`/`web3-ecosystem` branding, committed build
artifacts, and documentation that no longer matches the code.

This repo is currently at `github.com/knightingalelmao/hypermesh`. **The plan is to submit a
pull request back into the original repository `github.com/hypermesh-online/hypermesh`.**
Therefore, the canonical replacement target for step 1 is:

- `github.com/hypermesh-online/core` → `github.com/hypermesh-online/hypermesh`
- `github.com/hypermesh-online/hypermesh` → unchanged (already correct)
- Organization `hypermesh-online` → unchanged

## Open Decisions (do not touch these areas until resolved)

| # | Decision | Files affected | Notes |
|---|----------|----------------|-------|
| D1 | Go SDK module path | `sdk/go/go.mod`, 4 example imports, `sdk/go/README.md`, `sdk/go/GUIDE.md`, `docs/ROADMAP.md:355`, `docs/plans/phase3-sdk-dashboard.md:757` | `github.com/hypermesh-online/sdk-go` is a live import path; renaming is a breaking API change for consumers. Options: keep as-is (if sdk-go remains its own repo), or monorepo path `github.com/hypermesh-online/hypermesh/sdk/go`. |
| D2 | Sibling repo references | `scripts/deploy/sync-repos.sh`, `scripts/deploy/deploy-all.sh`, `catalog/Cargo.toml:8`, `trustchain/docs/todo_resolution_plan.md` | References to `hypermesh-online/{trustchain,stoq,catalog,ngauge,caesar}` stay correct only if those repos still exist under the org. Verify before touching. |
| D3 | Root package name | `Cargo.toml:161` | `name = "web3"` is stale legacy branding. Proposal: `name = "hypermesh"`. Confirmation needed because package name is API surface. |
| D4 | `repository` fields in Cargo manifests | `blockmatrix/Cargo.toml:7` (`github.com/hypermesh/hypermesh` — wrong org), `catalog/Cargo.toml:8`, `trustchain/Cargo.toml:8` (`github.com/web3-ecosystem/trustchain`) | Proposed: all → `https://github.com/hypermesh-online/hypermesh` (monorepo), or per-repo if D2 keeps them separate. |
| D5 | README title branding | `README.md:1` | `# HyperMesh Core` — keep "Core" or rename to `# HyperMesh`? |
| D6 | `caesar-token/stripe-gateway` | `caesar/stripe-gateway/package.json` | Org `caesar-token` — verify it's intentional before changing. |

## Phase 1: Naming Sweep (mechanical, one commit)

Replace `hypermesh-online/core` → `hypermesh-online/hypermesh` and fix wrong-org variants
(`web3-ecosystem`, `github.com/hypermesh/hypermesh`). Do not touch D1–D6 areas.

### 1a. `hypermesh-online/core` → `hypermesh-online/hypermesh`

| File | Lines | Notes |
|------|-------|-------|
| `CHANGELOG.md` | 32, 33 | compare/release URLs |
| `.github/ISSUE_TEMPLATE/config.yml` | 4 | SECURITY.md link |
| `docs/guides/ONBOARDING.md` | 18, 26 | release download URL + `git clone` |
| `docs/guides/INSTALL.md` | 31, 44 | install.sh URL + release tarball |
| `docs/RELEASING.md` | 191, 201, 219, 226 | install.sh/install.ps1 raw URLs |
| `gateway/src/onboarding.rs` | 76, 98 | user-facing onboarding HTML (download + docs links) |
| `scripts/install.sh` | 3, 22 | **critical**: stale `GITHUB_ORG` → every released install currently 404s |
| `scripts/install.ps1` | 3, 28 | same |
| `systemd/*.service` (7 files) | 3–4 | `Documentation=` lines |
| `sdk/python/pyproject.toml` | 29, 30 | Homepage/Repository fields |
| `CLAUDE.md` | 39 | org link |

### 1b. `cd core` / clone-layout references

| File | Lines | Notes |
|------|-------|-------|
| `README.md` | 14 | `cd hypermesh/core` — inconsistent with clone URL on line 13 |
| `docs/guides/ONBOARDING.md` | 27 | `cd core` |

### 1c. Old machine paths (remove or convert to relative monorepo paths)

- `docs/plans/phase1-cli-architecture.md` — 31 refs to `/home/persist/hypermesh/core`
- `docs/plans/phase2-domains-naming.md` — 31 refs
- `docs/plans/phase3-sdk-dashboard.md` — 43 refs
- `blockmatrix/test_hypermesh.py`, `gateway/WEEK1_ENDPOINTS_IMPLEMENTATION.md`,
  `gateway/src/bin/test_week1_endpoints.rs`, `scripts/security/*.py`, `scripts/start-ca.sh`,
  `scripts/start-multi-node.sh`, `scripts/test-multi-node.sh`,
  `stoq/STOQ_POS_INTEGRATION.md`, `stoq/TRANSPORT_PROTOCOL_ARCHITECTURE.md` —
  `/home/persist/repos/projects/web3` paths (historical docs: convert to relative paths or leave with a note)
- `trustchain/security_audit.py`, `trustchain/test_output.txt`, `trustchain/security_audit_report.json` —
  see Phase 2 (test outputs get removed anyway)

### 1d. `web3` legacy branding

| File | Notes |
|------|-------|
| `Cargo.toml:161,165` | `name = "web3"`, description — see D3 |
| `.cargo/config.toml:1`, `CLAUDE.md:1`, `CONTRIBUTING.md:3` | header comments |
| `scripts/build/*.sh`, `scripts/deploy/deploy-all.sh`, `scripts/deploy/start-all-services.sh`, `scripts/deploy/sync-repos.sh`, `scripts/demo.sh`, `scripts/demo-simple.sh` | header comments + echo text |
| `blockmatrix/examples/phoenix/phoenix_demo.rs`, `blockmatrix/src/integration/bootstrap/mod.rs` | comments |

## Phase 2: Hygiene — Committed Artifacts + .gitignore

1. `git rm -r --cached` (keep local files, stop tracking):
   - `sdk/csharp/bin/`, `sdk/csharp/obj/` (18 files; **contain `/home/persist` machine paths and stale org in sourcelink/PDB metadata**)
   - `caesar/scrolls-app/plaid-api/dist/` (4 files), `caesar/scrolls-app/satchel-wallet/dist/` (92 files)
   - `caesar/concept/__pycache__/` (3 `.pyc` files)
   - `caesar/scrolls-app/satchel-wallet/test-results/*.png` (9 screenshots), `bundle-analysis/` (5 files)
   - `trustchain/test_output.txt`, `trustchain/security_audit_report.json` (regenerated outputs)
   - Do NOT touch `reports/security/*.log` (intentionally exempt per .gitignore comment)
2. `.gitignore` fixes:
   - **Remove `*.lock`** — it currently swallows `Cargo.lock` (never committed!) and would swallow
     future npm/pnpm lockfiles. Replace with nothing (commit lockfiles) — binary-producing
     workspace needs `Cargo.lock` for reproducible builds; CI cache key
     `hashFiles('**/Cargo.lock')` is currently a constant empty string.
   - Add: `__pycache__/`, `*.pyc`, `**/dist/`, `sdk/csharp/bin/`, `sdk/csharp/obj/`
   - Clean up dead rules: `certs/*.crt`, `certs/*.key` (no `certs/` dir exists)
3. Commit `Cargo.lock` and a `ui/frontend` lockfile (see Phase 5 for package-manager decision).
4. Add `sdk/csharp/.gitignore` (bin/, obj/) so artifacts can't be re-committed.

## Phase 3: Restore the Canonical Requirements Source

`papers/HYPERMESH.md` **does not exist** — only `papers/CAES-NGauge.md` remains. CLAUDE.md
cites it as the canonical source for protocol requirements R1–R16 ("check if it's already in
papers/HYPERMESH.md Section 3"). It was deleted 2026-02-27 in commit `1f15dbda`.

1. Recover from git history (`git log --all --full-history -- papers/HYPERMESH.md` /
   `docs/components/HYPERMESH.md`) and restore to `papers/HYPERMESH.md` — or recreate
   Section 3 from CLAUDE.md's R1–R16 list if the historical file is too stale.
2. Reconcile R-numbering: CLAUDE.md header says "R1-R14" but lists R1–R16;
   `base/SPEC.md` uses R15/R16 labels with different meanings. Pick one numbering and
   make all three files consistent.
3. Update CLAUDE.md's citation rule to point at the restored file.

## Phase 4: Documentation Accuracy Pass

Regenerate all stats with `./scripts/sync-status.sh`, then fix the stale claims:

| Doc | Stale claim | Reality |
|-----|-------------|---------|
| `README.md:5` | 12 crates / 1,035 files / 361,784 lines / 4,553 tests | 13 crates / 1,127 files / 398,672 lines / ~4,906 test annotations |
| `README.md:182` | "~11MB static-pie ELF" | contradicts CLAUDE.md's "~9.5MB" — measure and pick one |
| `README.md:211` | "Cross-network asset transfers" under What Works | blockmatrix status: inDevelopment (alpha-default inert) — move or annotate |
| `README.md:116,217` | "55 connected components" | ui/frontend has ~150 components |
| `ARCHITECTURE.md:7` | "10 Rust crates plus a Svelte UI" | 13 crates, React 19 (Vite) — Svelte is gone |
| `ARCHITECTURE.md:9` | 999 files / 289,405 lines / 1,885 tests | regenerate |
| `ARCHITECTURE.md` per-layer table | all line/test counts | regenerate |
| `ARCHITECTURE.md:145` | "whole-blob encryption (not per-shard)" | per-segment BLAKE3-HKDF → AES-256-GCM |
| `ARCHITECTURE.md:313` | "9 dependents" | 11 dependents; graph omits base, hypermesh-sdk, hypermesh-ffi |
| `VISION.md` §5.5 | "Worlds" model as scaling spine | deleted by HEAD commit (de-worlds) — rewrite around network/mirror model |
| `VISION.md` §6/§7.4 | "STOQ requires eBPF, no fallback" | code has graceful degradation (eBPF → userspace) |
| `CLAUDE.md` headline | "~60-70%, 340/440 items, 4220 tests" | ~643/754 items (~85%), ~4,906 tests — regenerate |
| `CLAUDE.md` component table | catalog 100%, gateway 100% (194 tests), UI 32%, etc. | actual: catalog 87%, gateway 80%, UI 91%, stoq 88%, trustchain 73% — regenerate from crate-status.toml |
| `CLAUDE.md` | "12 crates in core workspace", "6 services" | 13 members, 7 systemd units |
| `CLAUDE.md` key files | `blockmatrix/src/blockchain_scope.rs` | doesn't exist — `BlockchainScope` lives in `lib/src/types.rs:313` |
| `CLAUDE.md` | "PoS primary in /trustchain/src/proof_of_state/" | canonical types now in `lib/src/proof.rs` (S1 consolidation); trustchain re-exports |
| `CLAUDE.md` | "pre-push hook auto-syncs status" | hook not installed anywhere — see Phase 5 |

## Phase 5: CI / Build / Tooling Repair

### 5a. CI workflow matrices
- Add `base`, `hypermesh-sdk`, `hypermesh-ffi` to the component matrices in
  `.github/workflows/ci.yml`, `testing.yml`, `quality-gates.yml` (currently 10 of 13 crates covered).

### 5b. Orphaned integration tests
- `tests/` (11 files: byzantine, chaos, http3_integration, etc.) never compiles — root package
  `web3` has no `src/`. Either wire it as a workspace test crate or delete it. Add a CI lane
  that actually compiles them.

### 5c. Release pipeline (currently broken end-to-end)
- `release.yml` packages binary + install.sh only, but install.sh installs 6 systemd units
  whose binaries aren't in the archive → dead code path. Bundle `systemd/` units (and any
  co-shipped binaries) or fix install.sh's unit list.
- install.sh unit list references retired `trustchain.service` (F9 split). Real units:
  `trustchain-http3.service` + `trustchain-stoq.service`. `systemctl enable --now trustchain`
  is a no-op.
- `systemd/catalog.service`, `systemd/ngauge.service`: `After=trustchain.service` → 
  `After=trustchain-http3.service trustchain-stoq.service`.
- `systemd/blockmatrix.service`: hardcoded `--network-id trustnet-test` / `--privacy public`
  — production unit shouldn't carry test values.

### 5d. `.cargo/config.toml`
- Target-specific `rustflags` (`target-cpu=native`) override the `RUSTFLAGS` env var, so
  `Dockerfile.build` and `scripts/deploy/build-release.sh` "portable" builds are silently
  built with builder CPU features. Use `build.rustflags` (lower precedence) or a dedicated
  profile/target section.
- `panic = "abort"` breaks `cargo test --release` (test harness needs unwind) — 
  `performance-monitoring.yml` steps are all `|| true`, so benchmarks silently produce
  nothing. Use a test profile or `panic = "unwind"` for release tests.

### 5e. Dockerfile.build
- Produces glibc binaries (deployment policy: musl static-pie only).
- Copies dev-only `trustchain_ca` instead of the production `trustchain-http3-server` /
  `trustchain-stoq-server`; doesn't build caesar or ngauge-server; copies ALL systemd units
  including ones with no matching binary.
- Fix: build/copy `gateway, trustchain-http3-server, trustchain-stoq-server, hypermesh,
  catalog-server, caesar, ngauge-server`; align with the musl policy.

### 5f. Toolchain + hooks + package managers
- No `rust-toolchain.toml`; CONTRIBUTING.md says nightly, CI uses stable — reconcile.
- CONTRIBUTING.md and CLAUDE.md claim a pre-commit hook (no unwrap/panic) and a pre-push
  hook (sync-status) — neither exists. Add `.githooks/` + install mechanism, or correct docs.
- `ui/frontend` has no lockfile; `release.yml` references `pnpm-lock.yaml`;
  `ui/package.json` says `packageManager: bun` while CI uses npm/pnpm. Pick one, commit the
  lockfile, use frozen installs (`npm ci` / `pnpm install --frozen-lockfile`).
- `ui/package.json:2` name `leap-app` — stale template name.

### 5g. Smaller CI items
- `codecov/codecov-action@v4` without `CODECOV_TOKEN` (silently fails, `fail_ci_if_error: false`).
- Drop stale `cargo audit --ignore RUSTSEC-2024-0421 --ignore RUSTSEC-2025-0009` after re-running audit.
- `performance-monitoring.yml` PR comment hardcodes fake numbers — replace with real comparison.
- `scripts/sync-status.sh` requires Node ≥ 22.6 (`--experimental-strip-types`) — add a version guard.

## Phase 6: Verification

1. `cargo build --workspace` + `cargo test --workspace` + `cargo clippy --workspace` (full run —
   compilation status is currently unverified; metadata parses).
2. `./scripts/sync-status.sh` → confirm generated `scripts/output/*.ts` match the tree.
3. Fresh-clone smoke test of `scripts/install.sh` (with a fake/real release URL) to prove the
   install path works post-rename.
4. Grep sweep for leftovers: `hypermesh-online/core`, `/home/persist`, `web3-ecosystem`,
   `github.com/hypermesh/hypermesh`, package name `web3`.
5. For the PR back upstream: rebase onto `hypermesh-online/hypermesh`, re-run CI matrices.

## Execution Order

1. Phase 1 (naming sweep) — after D3–D5 are decided; D1/D2/D6 areas excluded until resolved
2. Phase 2 (hygiene) — independent, can land first or with Phase 1
3. Phase 3 (papers/HYPERMESH.md) — unblocks the CLAUDE.md requirements workflow
4. Phase 4 (docs accuracy) — after stats regenerate
5. Phase 5 (CI/build) — independent of 1–4, largest risk surface
6. Phase 6 (verification) — final gate before PR
