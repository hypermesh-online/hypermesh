# Secure alpha, durable storage, and reproducibility remediation plan

**Status: planning baseline plus bounded execution; only Richard signs off.** This document is not a blanket implementation lease. Current native records supersede dated availability/execution assertions below; dated ledgers remain historical evidence, not current instructions.

<a id="current-user-demonstrable-gate-and-trial-order"></a>

## New session — start here (2026-11-20 UTC)

**Resume at the current verified published feature tip, without resetting.**
`012e30bd` is the demo/CI evidence baseline, not a checkout instruction.
[PR #6](https://github.com/hypermesh-online/hypermesh/pull/6) is a draft on
`cluster-dms-vcs`; publication is not a release or owner acceptance. Only Richard
signs off. Re-read actual HEAD, remote and current checks before work; this note
claims no CI result for its own prospective documentation commit.

### Find the work, then take one bounded assignment

On the existing operator host, the checkout is `~/hypermesh/core`.
Read `~/AGENTS.md` and the applicable role/project instructions first. Native
PDL is locally bound; if explicit selection is needed, use the real core project:

```sh
cd ~/hypermesh/core
export PDL_PROJECT=a48e58b0-92d4-4c52-8766-2255e7ad1e86
pdl status
pdl find 'four coordinator'
pdl get 6458dbd05472b3e9a9ce5c2d81022f6b061644b982d37111b4b8496c6e8012d9
```

That governor handoff links the core roadmap
`591ad60f82abd853251c6e59d1238cee1ae693d0bd89afc7a5a174bdbbae3452` and
current native work. Read its latest revision, then find/get the actual task;
a dated task state is not an implementation lease. Four coordinator lanes are
[Catalog](catalog/PLAN.md), [NGauge/STOQ](ngauge/PLAN.md),
[BlockMatrix](blockmatrix/README.md), and [installation/docs](docs/guides/INSTALL.md).
Operators report through their coordinator to the governor; independent trial
and exact-image review precede integration/publication. No implementation is
claimed to be running by this handoff.

### What someone can actually see

The public [45-second video and accessible transcript](docs/demos/stoq-xdp/README.md)
are a labeled replay of independently captured output, not a live public service.
The [reproducible guest fixture](tools/demo_gate/stoq_guest/README.md) exchanged a
fresh, identical 41-byte message using real STOQ/QUIC over IPv6 loopback in one
isolated Linux guest. Direct-libbpf XDP target counters advanced after handshake
21→23; the other-port control left them 25→25; detach returned ID 0. These are
packet counters, not payload-byte accounting or delegated STOQ/AF_XDP proof.
Separately, BlockMatrix locally reconstructed 36,000 identical bytes with four
data/two parity shards. Neither result is distributed asset retrieval, real PoS
authorization, two installations, public alpha or beta. Sources and reproduction
limits are in the [evidence index](docs/REMEDIATION_EVIDENCE_INDEX.md) and
[no-start local installation guide](docs/guides/INSTALL.md#private-offline-install).

### Current CI is red, not the predecessor diagnosis

At `012e30bd`, [CI Pipeline](https://github.com/hypermesh-online/hypermesh/actions/runs/44654638697)
failed on three `stoq/tests/ebpf_integration.rs` cases (`EADDRINUSE`) and 23
`hypermesh-ebpf` Clippy errors. The adaptive suite passed 7/7; the old NGauge lint
and pipeline-example diagnoses are not these failures.
[Quality Gates](https://github.com/hypermesh-online/hypermesh/actions/runs/44654638656)
failed compilation on stale `blockmatrix/tests/extension_loading_test.rs` APIs
(22 errors); its separate PR-comment HTTP 403 is a workflow permission failure,
not a code diagnosis. Historical missing BPF objects and compiler SIGILL remain
separate observations, not established causes of these current failures.

### Next gates — not optional shortcuts

- **First assignment — authenticated producer:** get native task
  `f245edfe64fd9ad0818a604e73569e8936e3de7d554b33084fc6cd5ceae851a1`
  for the legitimate authority producer from Catalog creator/delegation binding.
  This precedes the downstream asset/network/action enforcement in actual shard
  seed and fetch paths. Joined-network membership, a known hash, a signature envelope
  or an unbound grant is not authorization. Preserve meaningful affirmative
  behavior and deny wrong scope/authority, revoked grants and replay before
  announcements or bytes; test missing-policy recovery and mirror loss without
  fabricated demand or premature completion. The attempted handoff retry patch
  is withdrawn/held after independent failures: do not resurrect or merge it.
  Find the native task with `pdl find 'authoritative asset scope'`.
- **Real demonstration:** on two independently installed IPv6 nodes, resolve
  one stable asset through network-scoped Catalog, authorize the operation with
  authentic contextual four-proof PoS, reconstruct identical BlockMatrix bytes
  over STOQ, and attribute that transfer to attached kernel XDP against a control.
  Include wrong/revoked/replayed authorization and mirror-loss cases. Local and
  guest prerequisites do not satisfy this gate; private proof need not be public.
- **Public alpha:** project/instance binding and deployed artifact health for
  `trust.hypermesh.online` remain blocked. Resume with read-only discovery under
  the native deployment gate; do not provision, enable APIs, change IAM or deploy
  on inferred project identity. DNS, authentication and timed-out probes alone
  prove neither a running service nor an outage. Require signed artifact identity,
  independent health and rollback evidence before acceptance.
- **Final product contract:** complete Caesar's financial transmission-only
  bridge and the paired SDK last; no persistent circulating coin is authorized.
  Catalog's remaining kind contracts, genuine proof sources, missing BPF/backend
  contracts and clean-clone lock/toolchain reproducibility also remain open.

### Preserve the checkout and resume safely

Keep existing dirty/untracked work, including the nineteen historical formatting
overlays; do not wipe, stash/reset, mass-format or blindly merge retained branches.
Use reviewed immutable SSD candidates under `/usr/lib/hypermesh`, never `/tmp`,
and pin/review cache, lock and toolchain inputs; a clean clone lacks the ignored
root lock. Follow the linked guides, use bounded resource/containment gates and
coordinate the shared Cargo lease. Resume by reporting the actual native task,
preimages, intended check and coordinator ownership—not by rerunning a stale
ledger command or treating an old PASS as current. No main merge or release is
implied.

Handoff signature: OpenAI model/effort unreported — 01a1f16b-5a78-72a1-a2ad-4948857cebf9.

## Historical prerequisite gate — superseded by the start-here status

The evidence below is **separate bounded paths**, not one working end-to-end
system: seven isolated software STOQ cases; 36,000 locally reconstructed bytes;
33 NGauge tests plus strict Clippy; and an independent guest XDP diagnostic with
port 9292 counted, off-target 9293 absent, and attachment confirmed removed.
These do not establish production STOQ-to-kernel wiring, authentic four-proof
admission, a two-install transfer or alpha/beta readiness.

1. **Reproduce prerequisites independently:** start with the [local no-start
   installation recipe](docs/guides/INSTALL.md#private-offline-install). Repeat
   the bounded software checks, then the guest-only kernel prerequisite below;
   retain exact source/artifact identity and failures, not just exit summaries.
2. **Pass the user-visible asset gate:** independent installations must exercise
   real scoped asset/action and binary four-proof validation, authenticated
   session binding, wrong-scope/invalid-proof/replay rejection, mirror loss and
   reconstructed-byte identity. On two independently installed IPv6 nodes,
   resolve one stable asset through the network-scoped Catalog; authorize the
   operation with real Proof of State and reconstruct identical BlockMatrix bytes
   over STOQ, attributing the transfer itself to the attached kernel XDP program
   against a control. Demonstrate the actual interaction path; local
   reconstruction is not a substitute. No public disclosure of private data or
   evidence is required. Caesar's financial transmission-only token behavior
   and final SDK contract remain explicit completion gates, not proven outcomes.
3. **Qualify the public alpha separately:** the intended public CA/CT/DNS role
   at `trust.hypermesh.online` requires a proven running signed artifact, independent health checks, tested
   rollback and bounded operational evidence before acceptance. DNS observations
   and SSH/HTTP3 timeouts do not prove remote health or that the service is down.
   No new deployment is asserted; only Richard signs off.

**Independently tried guest recipe:** see the paired
[guest README](tools/demo_gate/guest_xdp/README.md). Run
`./tools/demo_gate/guest_xdp/run.sh` from the checkout. It requires read/write
access to `/dev/kvm`, a readable packaged kernel, clang's BPF target, GCC,
libbpf headers/runtime, QEMU, cpio/gzip and GNU utilities, and a caller-owned
writable SSD output root set through `HYPERMESH_GUEST_OUT_ROOT`;
`HYPERMESH_GUEST_KERNEL` selects the kernel. No sudo, installation or download
is performed, and unavailable KVM fails rather than falling back to emulation.
The runner uses a disposable 512-MiB/one-vCPU guest, no guest NIC/disks/host
mounts, and a 90-second QEMU timeout. Guest root loads the tracked C object via
libbpf, attaches generic XDP on guest loopback, sends ordinary IPv6 UDP to 9292
and 9293, checks the connection map, then requires `post_id=0` after detach.
It never attaches to the host kernel. This is **not STOQ traffic or the
production Aya loader**, not transfer-linked counters, authorization or
asset durability. The independent no-NIC KVM trial passed at 00:39:47Z
(2026-11-20 UTC): target-port map count 1, off-port count 0 and detach 0.
Independent reproduction is required for each proposed release; the real
production interaction gate above remains outstanding.

## Historical execution and next work — observed 2026-11-19; fixture baseline `9f775b3b`

- **Catalog:** Twelve of 15 kind-specific system payload descriptors are committed (`27bd37a4`, `8cdc1633`); the exact four-kind candidate independently passed 33 focused tests and a Catalog library check. See the [Catalog plan](catalog/PLAN.md). Container, Economic and Blockchain definition contracts await owner clarification; application schemas remain separate. Scoped versioned registry, lineage/authority integration and real Proof of State (PoS) remain open.
- **NGauge / STOQ:** Canonical indexed provider IDs (`b04b391e`) and network-scoped DNS (`52d037f3`) are committed and have independent bounded tests. NGauge server-feature macro imports (`df53b039`) passed independent NGauge-package all-features/all-targets compilation, not a workspace-wide check. Remaining work: active serving and authenticated session/channel, scoped authority and IPv6 operation. Protocol scopes assets; participation does not transfer ownership.
- **Installer / guards:** Guarded offline packaging passed 68 Bash mock-artifact cases. A separate independently reproduced Linux x86_64 GNU trial installed the locally built BlockMatrix `hypermesh` without starting a node; relocated `--help` and `--version` exited 0 (`blockmatrix-node 1.0.0`). This is local development-binary installation, not authenticated vendor provenance, another-host portability or private-network join. See the [offline recipe](docs/guides/INSTALL.md#private-offline-install). The observed latest tag `v0.1.0-alpha` has only the legacy `x86_64-linux` archive, not the expected Rust-triple archive/binary manifest: verified online installation is not available. Do not bypass integrity checks.
- **Build and tests:** Historical clean snapshot `46eebc2f` passed default-feature workspace all-target compilation and 93 focused tests. At the newer baseline, the two feature-gated fixture repairs in `4913bb92` passed their focused checks: BlockMatrix Gate 3 proxy 7/7 (one print-only summary), its privacy unit 1/1 (nine pair assertions), and root integration target 21/21 (four print-only placeholders). These are not end-to-end validation. The earlier all-features workspace snapshot failed on two QUIC examples and three missing BPF test objects; signed `9f775b3b` fixed the performance-fixture fields and passed its focused target check, not a QUIC, BPF or workspace clearance. Canonical open/scheduler programs for those objects are absent and the Linux BPF backend is explicitly simulation-only; compilation would not prove real kernel behavior. The formatting candidate’s initial compile check failed when cbindgen tried to rewrite its generated header on read-only source. The independently reviewed successor added that exact header to the 497 formatted Rust files: only the order of five prototypes changed, not their declarations. With all source mounted read-only, that candidate passed the default-feature workspace all-targets check (exit 0, 163.623 s) using pinned Rust 1.98.1 and the declared ignored lockfile. This is not an all-features or CI pass; integration and publication are tracked via PR #6. CI uses a moving stable Rust toolchain, and the repository does not track Cargo.lock; the reproducibility policy is awaiting owner clarification.
- **Delivery limits:** BlockMatrix asset-specific IPv6 locator/authority work remains design only. The next gates are owner/network/action binding, independent four-proof evidence, replay denial and real-network durability. The 17-project programme and 2,551 line-item backlog are unfinished; the bounded checks do not authorize main merge, release or owner acceptance. Preserve unrelated user work. The historical focused/default-workspace evidence above retains its stated snapshots; the current fixture compatibility bound is signed `9f775b3b` (2026-11-19), while the real local no-start installation was independently reproduced that day from the earlier source-bound development build. Neither establishes a newer whole-workspace pass. PR #6 tracks publication and integration. Feature → PR → main is the owner exception; only Richard signs off.

**Historical successor observations — 2026-11-19 (not current HEAD):** Independent isolated plain-QUIC `::1` trials succeeded for both the basic split and working examples; wrong certificates were rejected, and no-clobber/invalid-input cases failed closed. These are plain QUIC, not STOQ, post-quantum exchange or application admission. `88dd3454` makes the repository’s GNU default CPU configuration portable; the small fixture A/B check is not all-platform validation. At `9f775b3b`, actual GitHub formatting passed, while Clippy failed first on the filter warning; missing BPF objects and a `serde_derive` SIGILL also failed CI. The latest filter source check is not a full CI pass or a real BPF-backend result. A production private single-node start was **not attempted**: mandatory hardware collection and synthetic fallback violated the no-fabricated-proofs boundary. These dated outcomes supersede the QUIC-example status above, not the remaining security, BPF or workspace gates.

### Historical baseline and ledgers

The following baseline, phase expected/actual entries and dated ledgers retain their observation dates; they do not negate the bounded execution above. Follow current native task gates for further work.

## Baseline and governance

See [portable evidence index](docs/REMEDIATION_EVIDENCE_INDEX.md) for sources and measured limitations. Expectations are paper-level requirements, not completed features. Current source observations and isolated tests do not prove real-network operation. The 2026-11-06 baseline worktree was on `cluster-dms-vcs` at `13f6f078`, 17 commits ahead of local `main` (`27f6fc6a`), with substantial existing changes; preserve all dirty and untracked work. No local or remote-tracking `master` exists in the local inventory; remote freshness is unverified. Inventory and reconcile valuable integration units with the user before any change to them.

Authority: user decisions and applicable instructions govern; the owner-ratified Catalog/NGauge plan governs its explicitly settled contract; papers and VISION express protocol intent; ROADMAP identifies gaps; source and test artifacts establish only their observed behavior; real-network evidence establishes only its tested environment. Documentation may drift: record discrepancies, do not silently elevate code or an inconsistent paper into policy. Corrected architecture rejects Substrate's obsolete R15 node-address model; asset addressing remains distinct. Exact trust, sync scope, routing ownership, version identity and branch behavior remain user decisions. Four-proof acceptance is binary; an envelope signature cannot turn failed sub-proofs into authorization.

**Historical draft Git constraints (superseded by owner feature → PR → main exception above):** Do not overwrite or discard existing work. Identify dependency-ordered, reviewable integration units and their base/diff and preserve them before integrating valuable work into local `main`. Resolve the current branch/layout and any conflicting instructions with the user: no commit to `main` without a PR under `~/COORDINATOR.md`; do not merge remote `master` without owner sign-off; do not push unless the applicable remote-master merge condition is met, and obtain an explicit exception if publishing an unmerged PR source is required. Any conflict is an unresolved decision, not permission to choose a branch policy. No commits, merges, pushes or publication are authorized by this draft. Breaking wire/storage formats and hash-preimage changes are deferred pending explicit approval and a migration plan.

**Execution discipline:** Each sequential development phase freezes shared contracts, integration boundaries and non-overlapping file ownership before parallel work. Components include their own tests. Only after all component work in the phase finishes, run one consolidated QA stage: correctness and integration, security/privacy, efficiency, maintainability/code quality and compatibility. Failures require fixes and re-verification; do not waive failures to preserve a single pass. Record deliverables, actions, metrics, expected versus actual, evidence and decisions below after each phase. User alone signs off. No development, runtime QA, launch or release is represented as done here.

## Decisions required before dependent work

1. Approve this plan and scope; determine the Git/PR workflow consistent with the constraints above and how existing changes are preserved and integrated.
2. Specify identity-to-QUIC channel binding and replay defense; decide whether Private alpha may ship before strict PKI/CT/DNS, with explicit exposure and rollback conditions. Self-signed TLS plus a parsed proof is not authenticated channel binding.
3. Decide asset-chain presentation versus network-genesis adoption and the Private/Public synchronization and failure-domain guarantees, including Public spatial filtering. Device-local genesis/index must not be overwritten.
4. Decide Catalog version identity, same-name successor registration and metadata branch pointer. Inventory actual wire compatibility by endpoint/path and decide legacy behavior; protected operations must never bypass signature/signer verification, binary four-proof validation, session/channel binding or scoped authorization. Design, test and obtain approval for any required rejection, deprecation or migration; do not silently preserve an insecure fallback. Confirm the NGauge routing mandate where documents differ.
5. Approve software-only claim boundaries, hardware exclusions, supported platforms and release targets. Provide alpha GCP project, instance, zone and authorized read-only access/approved external vantage. Approve live canary scope separately.

## Phase 0 — Baseline, decisions, and integration preparation

- **Deliverables/actions:** Capture revision, branch, dirty/untracked inventory and dependency-ordered candidate integration units without altering them. Classify each claim as intent, source support, isolated reproduction or real-network validation. Resolve decisions above; freeze initial contracts and file owners for Phase 1. Document exclusions, migration constraints and recovery/rollback.
- **Claims completeness gate:** During the approved baseline and reproduction phases, enumerate every selected in-scope protocol, paper and user-visible claim, not just the seven findings. Give each a stable claim/source ID, exact conditions and prerequisites, real reproduction commands or runbook, negative controls, expected versus actual outcomes, evidence path, and verified, unverified, unsupported, unavailable or deferred disposition. Explicitly list deliberately excluded claims and why; missing claims or fields do not pass. Populate and verify this register during execution, not in this draft.
- **Integration deliverable (approval-gated):** Record each candidate feature-series and dirty unit with source, base, diff, dependency order and an accept/defer/reject ledger; none of the 17 commits is presumed valuable or approved. Preserve user changes and prepare a tested merge/revert recovery method. After consolidated QA and user sign-off, integrate only accepted units locally under the resolved branch/PR policy; remote `master` merge and publication are separate, separately gated actions. The absent `master` and main-commit rules remain unresolved; no merge, commit or push is authorized now.
- **Success metric:** User-approved boundaries, preservation/integration method and phase contracts; every finding maps to an owner and evidence. Accepted units are integrated only after the specified gates, with recovery verified and outcomes recorded.
- **Expected / actual:** Plan and evidence ready for review / draft only; no approval or integration. Public alpha exposure is undetermined: after the user reported updated permissions, 14:46–14:47 UTC retries hit local TCP/HTTP3 and strict known-host noninteractive SSH socket denial; gcloud could not initialize its credential store on the read-only home. No remote command ran; no node health or authentication conclusion follows.

## Phase 1 — Secure identity, admission and network boundaries

- **Contracts/ownership before work:** Freeze proof validation, channel-binding and admission interfaces; assign disjoint `stoq/`, `trustchain/`, `blockmatrix/` network integration and tests; assign a gateway owner for `gateway/src/dashboard_server.rs` and its dashboard tests/fixtures, and a separate CAESAR security-review owner for the dirty `caesar/src/cross_chain_bridge.rs` change and its relevant tests/fixtures. The CAESAR task is review only unless separately authorized. One integration owner controls shared `lib/` types and common fixtures; freeze exact non-overlapping file assignments before work.
- **Deliverables/actions:** Implement user-approved bilateral authentication and replay resistance, verified-or-dropped admission, scoped authorization, and independent Device identity/network membership. Correct Unicode identity rejection panic, validate received four-proof semantics before adoption, and resolve dashboard owner/admin scope mismatch and direct-message sender/key binding, recipient/signature validation and duplicate-ID handling; separately review the security-sensitive CAESAR recipient-check removal before considering that dirty unit for integration. Inventory actual raw-proof and signed-envelope behavior per endpoint/path. The documented fallback cannot bypass signature/signer verification, binary four-proof validation, session/channel binding or scoped authorization on protected operations; user decides compatibility after risks and tested rejection/deprecation/migration options are presented. Preserve frozen hashes unless user approves migration. Restrict XDP claims to actual encrypted QUIC enforcement; prevent unrelated traffic regression.
- **QA and metrics:** One consolidated phase QA as above. Invalid, replayed, substituted or unbound proofs fail; four valid proofs and authorized channel succeed; identity persists across restart/rotation; unrelated traffic behaves as specified. Test Private grant and Public admission boundaries; verify compatibility and no secret leakage. Record artifact hashes, test commands/results and limitations. Include Unicode-boundary rejection, validly signed invalid proof, dashboard scope matrix, direct-message spoof/wrong-recipient/overwrite tests and TLS relay/MITM threat tests.
- **Expected / actual:** Authenticated, scoped admission with negative controls / not implemented or verified under this plan.

## Phase 2 — Durable asset storage and Catalog VCS / NGauge DMS

- **Contracts/ownership before work:** Freeze Catalog version/branch semantics, asset-chain handoff, NGauge decision traits, BlockMatrix I/O and storage interfaces, STOQ transport, sync object and guarantees. Assign disjoint `catalog/`, `ngauge/`, `blockmatrix/` storage/swarm and transport integration owners; one owner controls shared interfaces and cross-component tests. No direct Catalog→NGauge wire or second lineage/version store.
- **Deliverables/actions:** Catalog writes/reads version and branch lineage with dependency DAG; BlockMatrix observes asset heads, executes mirror and reflection over real STOQ using Catalog-issued assets; NGauge selects actions only. Verify independent lineage before indexing, Private Grant before mirroring, Public authenticated admission, confidentiality and reference scope. Verify fetched bytes and durably store shards before provider advertisement; freeze canonical provider identity and exercise real index→plan→transport lookup (inconsistency is a contract risk, not a proven live outage). Preserve compression→whole-asset encryption→Reed–Solomon shard→distribution and copy-then-redirect as applicable; no post-creation shard splitting.
- **QA and metrics:** One consolidated phase QA. Real three-node distinct persistent processes and durable stores: issue version/branch, transfer, serve after writer stops, restart providers, validate lineage/identity, Private authorized and denied grants, Public admission, node failure and recovery; report placements and failure domains. Exercise RS reconstruction from k valid distinct shards and rejection below k and of corrupt shards; count duplicate copies separately, never call copies k-of-n. Record actual k/n and configuration, data hashes and exact fault sequence. Include wrong-byte, empty-fetch and failed-store negative controls, plus canonical identity index→planner→daemon transport. No unproven durability guarantee.
- **Expected / actual:** Durable verified multi-node asset service and DMS/VCS handoff / not established; three-node behavior and failure tolerance unverified.

## Phase 3 — Reproduction kit, installer and network creation

- **Contracts/ownership before work:** Freeze supported environment/claims, manifests and result schema, installer/config interfaces and test topology. Assign disjoint reproduction kit, installer, docs/examples and clean-machine E2E owners; shared test harness has one owner. All examples use real STOQ I/O and Catalog-issued HyperMesh assets.
- **Deliverables/actions:** Publish pinned source and dependency manifests, the authoritative versioned specifications (papers are a sibling of the core clone), binaries/build inputs and hashes, host/kernel/config/network topology, deterministic setup and teardown, exact commands, expected outputs, fault schedules and negative controls. Run named claims matrix in the evidence register; capture raw data and define statistical conditions before performance claims. No sleeps, mocks, clamps, disabled suites or simulated scale presented as measurements. Explicitly mark unavailable hardware/physical adapter assertions; preserve compatibility, not replace real functionality with stubs.
- **QA and metrics:** One consolidated phase QA. Independent clean machines install and create a Private network; test isolation and denied ungranted peer; create/join a Public network with correct admission, fail safely on invalid join, rollback failed installs/joins without damaging existing identities/data; verify reproducibility from manifests and negative controls. Record versions, actual commands/results, failures and resource costs. Unsupported platforms and untested configurations remain unclaimed.
- **Expected / actual:** Repeatable software evidence and recoverable setup on stated clean environments / not run; no hardware validation.

## Phase 4 — Controlled GCP canary and release decision

- **Prerequisites/actions:** Owner separately authorizes access, approved vantage, endpoint/identity scope, test window and rollback. Read-only inventory correlates instance IP/DNS, firewall, active service units/configs, listeners and deployed artifact hashes with revision. Then run only bounded benign DNS/TLS/HTTP3 health and valid STOQ handshake/authenticated-operation checks with approved identities. Clean unauthenticated client must be rejected for protected actions; health alone is not an auth bypass. No exploit, malformed traffic, scanning, brute force or load tests against live alpha.
- **QA and metrics:** This dedicated integration phase verifies pre/post expected versus actual ingress, valid and denied access, logs without secrets, service health and rollback; any failure is fixed and reverified with authorization. Only user decides launch. Post-launch monitor, record incidents/claims and feed the next ideation cycle.
- **Expected / actual:** Bounded verified canary and user release decision / blocked on access and approved vantage; no live auth conclusion.

## Draft plan QA status

2026-11-06 consolidated draft-document QA passed after corrections to ownership, integration gating, raw-proof fallback and claims completeness. User approval, implementation, runtime and security tests, and launch remain pending.

## Phase 0 bounded baseline ledger — 2026-11-09

- **Preservation/actual:** Read-only `core/` Git inventory before phase-0 document edits: branch `cluster-dms-vcs`, HEAD `13f6f0781746eab96f7211e9a21e05fc95e1ab0c`, 60 tracked dirty/deleted and 7 untracked paths (including this plan/report and `.pdl/`). This is a historical snapshot, not a claim that the working tree stayed unchanged; the later [alpha exposure report](reports/alpha-exposure-2026-11-09.md) added an untracked artifact. The parent `/home/persist/hypermesh` is not a Git repository. No Git mutation, integration or source implementation was performed.
- **Expected versus actual:** Existing findings 1–7 were mapped to current source and candidate focused tests; finding 8 remains a separately flagged dirty CAESAR review item. The first storage attempt failed (`Permission denied`) before cargo. Governor then authorized a narrow private verification directory; it was created without touching an existing path. Focused offline/locked STOQ identity and NGauge planner unit filters passed, but neither proves Unicode boundary safety or the provider index→transport contract; four-proof admission tests were not run. Exact commands/results/limits are in the evidence register.
- **Metric/gate:** STOQ 2 passed, 244 filtered, 564 s; NGauge 5 passed, 255 filtered, 490 s; both exits 0. No phase acceptance, runtime/security claim, frozen contract or file owner established. Claims completeness and decisions remain open. Full raw compiler output was not captured; a marked incomplete result excerpt is stored in the private verification directory.
- **Next exact unit:** Independent consolidated QA may review this completed bounded baseline; component owners/contracts require user decisions. This is research/design baseline only; component development and phase QA have not started.

## Branch consolidation ledger — 2026-11-09

- **Owner direction / scope:** Consolidate existing branch work before new implementation, and establish scoped PDL status/development tracking for each subproject. This entry records bounded inventory/report results only; draft policy, architecture decisions and user acceptance remain unchanged. No merge, branch action, source implementation or automatic acceptance occurred.
- **Deliverables / evidence:** Private preservation and branch report `/usr/lib/hypermesh/verification/branch-reconcile-20261109-xpudas39/branch-reconciliation-report.md`, SHA-256 `8453d4337ec87bcd0afc6a46a3efb315b75d21075b971d0aa63ab7479d4ed2c5`; independent [branch reconciliation QA](reports/branch-reconciliation-qa-2026-11-09.md), SHA-256 `becbdc0d823c65b70d31858c8ccb65b91c64d54bebe0b3e22f237c5396163808`: PASS bounded point-in-time inventory/preservation/report only. Integration, runtime, security, product, native coverage and alpha acceptance remain blocked/unverified.
- **Expected versus actual / metrics:** Requested reconciliation before new work; actual preserved snapshots and conditional review order, no accepted integration. Of 21 non-main core branches, 17 are contained in main; the 7-ahead E-series is already within the 17 patch-unique DMS-series commits, so do not merge twice. Clouds, security and EVP candidate units remain proposed, not accepted. Captured core diff-check failed: 15 output lines, 13 diagnostics; no waiver or correction of source is implied. F1 Unicode diagnostic remains unfixed by existing branch content, based on source review, not a new runtime test.
- **Native tracking / limits:** Six pointers (five roadmaps and one root phase) were last read back at revision 1/status idea, not accepted/active. Subsequent root 40-second and core 15-second status calls timed out; checkpoint-focus outcome remains UNKNOWN, with no retry. The 25-component map is PROPOSED, not registered; census 2,551 candidates is staged, not imported. No PDL systems/service/store/config/tooling work is authorized or performed by this maintenance.
- **Alpha / later vantage:** [Latest alpha report](reports/alpha-exposure-2026-11-09.md), SHA-256 `ebf130f8eeb66a6b265062852db64a7faf0cfdb2291c8cd957c9e84ed94cabac`, identifies owner account `alephpt1@gmail.com` and project `hypermesh-online` (display name HyperMesh Online), ACTIVE. Compute `SERVICE_DISABLED` prevents instance enumeration/IP binding; neither empty stdout nor prior timeouts establish zero VMs, alpha association, health or safety. Cloud changes are not authorized here; review account redaction before public publication.
- **Actions / next unit / workflow blocker:** User must resolve the observed main-only layout across all four remotes versus the PR/master policy and any publication exception, select exact accepted units and recovery method, then freeze contract/file owners. Preserve immutable refs/commit objects and verify restore before mutation; integrate only selected units in dependency order with consolidated correctness, security/privacy, efficiency and code-quality QA, then explicit owner acceptance. New F1 implementation follows branch consolidation, not before. Native per-subproject coverage and alpha association remain separate unmet actions; do not infer permission to change PDL systems or enable Compute.
- **Preservation / post-snapshot delta:** Earlier 60 tracked/7 untracked baseline remains historical; reconciliation captured later counts separately. This PLAN append is an intentional post-snapshot delta, not covered by the prior preservation snapshot. A fresh source/ref/dirty-byte preservation fence is required immediately before any merge or other mutation. Ledger fact-check is documentation-only, not product acceptance.

## Owner intent / pending decision — 2026-11-10

- **Attribution:** Owner direction in this session, 2026-11-10 (no message ID supplied): `trust.hypermesh.online` should provide HyperMesh CA/CT/DNS for the global-public scope endpoint and mesh, permitting listing/registration of public or private hubs, gateways, networks and assets.
- **Evidence level / limits:** OWNER-STATED INTENT, not verification of deployed service, protocol proof or API implementation authorization. Existing draft, branch-consolidation-first, workflow, acceptance and execution gates remain unchanged.
- **Pending owner decision:** Private-resource discoverability: hidden unless publication is explicitly opted into, versus a default public stub. Neither option is selected; no exposure/default policy is inferred.

## Component tracking status — 2026-11-10

- **Expected / actual:** Previously proposed map now has 25 component roadmap pointers created/read back at revision 1/status idea within core authority: 13 Cargo members plus 12 verified manifests. Parent core roadmap revision 2 changed notes only for owner trust intent. Governing Coordinator: `/root/coord_core`; component Coordinators UNASSIGNED. Core PDL client responded; this is not global service-health evidence, and no PDL infrastructure was touched. Nov9 proposed-map/timeouts remain historical.
- **Pointer evidence / limited QA:** Metadata QA PASS for pointer accuracy only: `/usr/lib/hypermesh/verification/component-qa-20261110-zXEy9u1g/report.md`, SHA-256 `84181b4b1e26676c66f8a5ecd76e6be9d3b76c5d653f2979a37fe9334a08995e`; machine checks SHA-256 `36f65de3bfe90aa4149ca1d33dde8464828df3f0d0be404bc75989ac13968f65`. Native ID map: `/usr/lib/hypermesh/verification/component-tracking-20261110-wQpEz278/handoff.md`, SHA-256 `e0639f089b5af054db8e371b579d72b07758b54f51aedf886204df2980acc966`.
- **Limits / next actions:** Pointers are not populated backlogs, implementation, acceptance or deployment; census 2,551 remains staged, not imported. Private-resource visibility remains PENDING owner decision; component backlog population and Coordinator assignments are future work. Branch consolidation/workflow gates remain unchanged. This append is post-QA recordkeeping, not a new development/QA phase or acceptance, and remains a post-snapshot delta requiring a fresh pre-mutation preservation fence.

## Core project map and owner policy ledger — 2026-11-10

- **Scope and expected / actual:** Correct the earlier component-pointer classification and Coordinator hierarchy without changing product implementation. The 25 existing pointers now comprise 15 direct core projects and 10 subordinate workscopes; direct `sdk` and `ui` pointers were added, for 17 direct projects and 10 subordinate pointers. Existing 25 records were revised notes-only from revision 1 to 2; new `sdk` and `ui` are revision 1, `idea`. Core roadmap `591ad60f82abd853251c6e59d1238cee1ae693d0bd89afc7a5a174bdbbae3452` notes changed revision 2 to 3. All 28 successful native changes used revision preconditions and were read back; one invalid upsert/precondition request was refused without a write. Parent references for subordinate workscopes are notes, not native containment relations. IDs and per-record captures are in `/usr/lib/hypermesh/verification/docs-mapping-20261110-p5riRvda/`.
- **Hierarchy / lanes:** `/root` Governor directly coordinates core product projects; Operators report through their respective project/phase Coordinators to `/root`. `/root/coord_core` is a temporary integration phase Coordinator, not a permanent core product umbrella. Project Coordinators are planned, unassigned until actually assigned. Documentation has its own `/root/coord_papers` lane; public and EVP support are sibling repositories, and papers belongs to the documentation lane. No peer coordinator is implied for docs, plans, reports, scripts, tests, systemd, target or dot-directories. This supersedes earlier governing-Coordinator statements, retained above as historical evidence.
- **Owner-confirmed policy and source:** Owner direction in this session as relayed by `/root/coord_papers`, 2026-11-10, no message ID supplied: target `main` via feature -> PR -> main, no direct-main commits or unapproved push/release and no fabricated `master`; consolidate existing branches before new implementation. Private entries are hidden unless the owner publishes discovery. Private networks may self-host/offline with their own CA/DNS; public registration is optional. The trust directory holds signed Catalog-issued discovery records/references; asset data, ownership and private access remain owner-controlled through Catalog/STOQ, with no implied private grant or separate version store. This supersedes the earlier pending visibility and branch-policy statements; endpoint roles remain owner intent, not verified deployment or authorization to implement or release.
- **Deliverables, metrics and limits:** 17 direct + 10 subordinate scoping pointers; 28 guarded native metadata writes and readbacks, no state promotion or Coordinator assignment. Source census 2,551 remains STAGED, not imported; pointers are not populated backlogs, readiness, acceptance or deployment. Prior component QA addressed the old pointer set only and does not verify this correction. No code, Git, cloud or PDL infrastructure work is claimed. This ledger is a post-snapshot documentation delta.
- **Next actions:** Coordinator to arrange one independent consolidated metadata/documentation QA after the writing phase, including correctness, security/privacy, efficiency and code quality; correct and reverify any failures. Existing branch consolidation and separate approval gates remain before implementation; assign Coordinators only when authorized. User acceptance and deployment remain unclaimed.

## Metadata QA correction — 2026-11-10

- **Gate / expected:** Independent consolidated metadata/docs QA failed on 25 current pointer notes retaining the superseded phrase `workflow choice pending`; see `/usr/lib/hypermesh/verification/docs-mapping-qa-20261110-oCGHgEfv/report.md`. The authorized correction was an exact notes-only replacement, then independent re-verification. Earlier 28-write ledger remains historical.
- **Actual / stop:** Five corrections were confirmed and read back at revision 3: desktop, mobile, sdk/python, sdk/csharp and sdk/go. The sixth guarded write, sdk/cpp ID `3cafd76e2358701e9b45db3aaa85782768dfae24d5fb3d28810980ac9c30a7df`, timed out; its outcome is UNKNOWN. No replay or further native operations were performed. Thus 33 metadata writes are known successful in the overall phase; 34 are possible. The remaining 19 records were not attempted. Captures and per-record journal: `/usr/lib/hypermesh/verification/docs-mapping-20261110-p5riRvda/`. Independent QA has not reverified this correction; the failure remains open.
- **Next actions:** Coordinator to resolve the unknown outcome through an authorized, bounded read after service availability, without blind replay, then decide how to complete the remaining correction and independent re-verification within the approved budget. No product, release or user acceptance is claimed.

## Post-verification status maintenance — 2026-11-10
- **Authority / expected versus actual:** Governor released the prior original-byte fence for this ONE append after independent read-only warm-phase procedure/evidence PASS; workspace completion was expected but remains INCOMPLETE. This is status maintenance, not product QA, security acceptance or lock policy approval.
- **Actual team assignment:** All 17 core project Coordinators ACKed provider-team handoffs; exact role/ID map `/usr/lib/hypermesh/verification/docs-mapping-20261110-p5riRvda/provider-team-assignments.md`, SHA-256 `83f7f15d5c7c73364f750d06ce22121955d98158502226ce74f8abad43d1986c`. Operators → respective project/phase Coordinator → Governor; docs is separate, `/root/coord_core` temporary integration phase only; ten nested scopes inherit caesar/sdk/stoq/ui parents. Actual native assignments remain UNWRITTEN.
- **Native gap retained:** 33 known successful metadata writes, possible 34; sdk/cpp outcome UNKNOWN after bounded read also timed out, 19 note fixes unattempted. Metadata QA FAIL/INCOMPLETE remains unwaived; canonical 17/10 maps and captures remain `/usr/lib/hypermesh/verification/docs-mapping-20261110-p5riRvda/handoff.md`.
- **Confirmed owner policy unchanged:** feature → PR → main; no direct-main commit or unapproved push/release. Private discovery opt-in only, private self-host/offline CA/DNS with optional public registration; signed Catalog discovery references, owner-controlled assets/access via Catalog/STOQ, no implied private grant or duplicate version store; endpoint intent is not deployment.
- **New replacement actual:** Commit `13f6f0781746eab96f7211e9a21e05fc95e1ab0c` with explicitly unchanged COPIED ignored preincident Cargo.lock SHA-256 `daef800d7371cb32c4c1b6b8ecee51cc1a49d2ca65f3a1287bcf3b3c08ef0b43`: NGauge five passed / 255 filtered. Old unsafe 14-substitution QA remains FAILED/excluded, not recovered by replacement evidence.
- **Workspace actual / limits:** Cold 600s + 10s kill returned -9; warm 1200s returned 124; both INCOMPLETE, no Finished or error diagnostic. Unused-import/eBPF warnings observed; last compiling FFI is context, NOT established cause. Warm report `/usr/lib/hypermesh/verification/warm-check-20261110-33sTKP8u/report.md`, SHA-256 `ab9a15a4213c473ecf3077e0c081495be1e78d71066e961f674c187846fa8f02`; Governor received independent read-only native QA PASS for bounded procedure/evidence, not workspace/product completion.
- **Preservation:** All protected original fences and 18 candidate inputs unchanged through warm post-fences; no owned compiler remained. This authorized append is a later documentation delta; no code, merge, commit, push, security/product acceptance or new lock policy follows.
- **Next intent, NOT execution lease:** FFI build-stall SOURCE investigation, not a third blind rerun; STOQ F1 two-file fix plus bounded private handshake-helper decomposition under 500/50 limits, exact code lease forthcoming, with no API/hash/wire/proof semantic change. Branch de-dup found no existing fix; necessary integration repair, not duplicate feature work.
- **Cloud artifact audit:** Independently verified error consumer `projects/hypermesh-online` matches projectId; numeric `731944638137` absent from error, so no independent numeric match. SERVICE_DISABLED establishes no VM binding, NOT zero VMs; source alpha report unchanged. Hardware harnesses absent; source 2,551 census remains staged, not populated backlogs.
- **Action / new fence:** Freeze PLAN at its post-append SHA before F1 worktree/new fences. No PDL systems/cutover/importer tooling, replay or infrastructure repair; later recordkeeping requires a new Governor lease. User alone gives final integration/release acceptance.

## Post-phase documentation and bounded execution ledger — 2026-11-10

- **Documentation deliverables / expected vs actual:** 17 source-specific project PLANs are DRAFT under actual assigned Coordinators; native IDs are identity references only, not populated backlogs or native controller assignments. Initial generic drafts, three architecture omissions and two role/ABI wording blockers failed documentation QA historically; corrections received the same independent consolidated final bounded docs QA PASS for the 17 plans. Evidence: `/usr/lib/hypermesh/verification/project-plans-20261110-OVgdZi3e`. This is not product acceptance.
- **STOQ F1:** signed locally `e767abadec1597d567a6cd8785b7958cd53ee02c` atop `13f6`, exactly two files, 15 bilateral tests and independent QA; no merge, push or product acceptance.
- **FFI bounded check:** corrected isolated phase `/usr/lib/hypermesh/verification/ffi-remainder-20261110-EtvYnhwv` used 1,300 manifest files, 170,713,002 bytes, and one offline locked j6 all-targets check: exit 0 in 73.915 seconds; independent bounded FFI QA PASS. No tests were executed; this is not workspace, security or product PASS. TrustChain unused-import warning observed.
- **Excluded evidence:** old unsafe 14-substitution QA, raw-label defects and failed 240-second staging capture remain FAILED/excluded. No blanket rehabilitation or root-cause proof follows.
- **Native and census:** CORE read timed out at 124 with empty output (`/usr/lib/hypermesh/verification/cpp-native-reconcile-sbn5tgnm`); no GET or retry write followed. sdk/cpp outcome UNKNOWN, 19 fixes pending, 33 known writes and possibly 34; 2,551-item census remains STAGED, not backlogs.
- **Next actions, intent only:** Full workspace check on the 18-commit F1 candidate with verified SSD cache and focused BlockMatrix F4 contract research; this documentation phase executed neither. Record outcomes and unresolved decisions before user-only acceptance.
- **External limits:** Compute SERVICE_DISABLED is not evidence of zero VMs; the numeric project identifier was absent from the error and alpha association remains unbound. Hardware harnesses remain absent. Existing owner policies remain unchanged; no deployment or product acceptance is claimed.

## Post-verification compile and F4 research ledger — 2026-11-10

- **Host compile / expected vs actual:** Candidate `e767abadec1597d567a6cd8785b7958cd53ee02c` with copied ignored lock `daef800d7371cb32c4c1b6b8ecee51cc1a49d2ca65f3a1287bcf3b3c08ef0b43`, pinned Rust 1.98.1 and private SSD cache ran exactly one `cargo check --offline --locked -j6 --workspace --all-targets`: exit 0, 295.023 seconds, Finished 4m49. Report `/usr/lib/hypermesh/verification/stoq-f1-workspace-20261110-v496nP9U/report.md`, SHA-256 `b383ba7bfb5c77f73a38e4c72e7518de5ee76733af9e1b1c4bb5d706a0e4939c`.
- Independent bounded QA PASS covers the 13-Rust-member host compile and matching original/candidate, 18 inputs, 1,300 cache files, root PLAN and 17 docs. No tests, 17-project build, runtime, security, real STOQ, kernel or hardware operation, or user acceptance is established. eBPF program compile failure and future Rust compatibility warning require separate component gates, not a whole-host compile failure.
- Original preflight STOP resulted from a malformed 62-character digest; no Cargo ran then. Authoritative 64-character cache validation `7a28bc8141bcac83de12fa8e1a3011fff19038653183f8188b72c2d5bc87b346` was revalidated before the one check; no tampering conclusion follows. All historical failures remain excluded.
- **F4 / Catalog research:** Source supports executor discarded bytes and false providership, plus store memory success despite disk failure, without durable acknowledgement/reload rehash. Existing signed BlockAssetEntry and Sharded pointer are reuse candidates, not proven asset/version/owner provenance.
- Ratified `plans/phase-dms-vcs_catalog-ngauge-distribution.md` D28 leaves stable versus fresh per-version asset hash open; D9/25–26 specify version ENTRY, D36 its selected shard set, and D37 private Grant first / public authenticated mirror. Private discovery is not a Grant. Open gaps are identity mapping, owner/delegation signer binding and exact version ordinal/commitment fields.
- **Decisions/actions:** Two asynchronous user choices remain pending: strict atomic fsync/restart/idempotent retry, and stable asset ID plus version commitments versus new per-version ID. No F4 implementation lease or approval is implied. Next intent only: classify eBPF compile warning with its project Coordinator; no kernel load or system change.
- Native PDL sync remains unavailable: sdk/cpp UNKNOWN, 33 known writes and possibly 34, 19 pending corrections; 2,551 census items remain staged. Alpha association and absent hardware conditions remain unchanged. No product acceptance.

## Local repair consolidation ledger — 2026-11-10

- **eBPF fix / expected vs actual:** One C-file repair uses private `syscall_exit_ctx`, two handlers, and static offset 16 / size 24 checks for x86_64 source-derived Linux 7.2. RED four errors became GREEN BPF object; the actual `ebpf-loader` feature C compile gate returned 0. Standalone object is not embedding, kernel verification/loading, other-architecture or actual Zen ABI evidence.
- Bounded 500/50/3 QA PASS followed an initially insufficient inactive gate. Fresh source binder exited 0 with matching pre/post SHA-256 `65a8a0be051ba33199be3838537081b13172fe75f2d1b4e43dcba2d6473d76fc`; GREEN and feature objects shared SHA-256 `ebd86cddf19596f66a6ea91e8bd621c9988384182298dcacad7c32669fb8cc58`. Earlier missing test-source hash limitation remains.
- Signed local `49d48cf7bc68a08d436b921227bce118ca53c6da`, parent F1 `e767abadec1597d567a6cd8785b7958cd53ee02c`, contains exactly the C file and signature G. One legitimate commit used configured plain GPG with execution-only fail-closed batch/no-TTY/pinentry-error/no-autostart/no-retrieval wrapper; no persistent config, key or signing-policy change, unsigned commit or human approval is claimed. Evidence: `/usr/lib/hypermesh/verification/ebpf-commit-20261110-UqHlbMu9`, `/usr/lib/hypermesh/verification/ebpf-sys-exit-20261110-LBPOGu`.
- Prior QA made two official public web reads despite its restriction; historical stash and two-tag pre-phase baseline gap remains disclosed and unproven. Fresh commit and integration comparisons of all refs including tags and stash unchanged except owned heads do not rehabilitate the old gap.
- **Local integration:** Original `cluster-dms-vcs` fast-forwarded `13f6` → `49d48cf7bc68a08d436b921227bce118ca53c6da`, exactly two selected signed repairs, one FF exit 0 / 4.616s, three source/index paths updated, `ORIG_HEAD` at `13f6`. The 88 preserved states, 27 pre-existing untracked paths, 17 plans, user dirty blobs/modes, unrelated staged entries, lock and other fresh refs were preserved; a different independent read-only integration QA passed. Report `/usr/lib/hypermesh/verification/local-ff-consolidate-20261110-dEikBlFD/report.md` SHA-256 `0f60844713af877f4090972b12e08a4b5db55c65a42e49836574569871a723f5`; recovery bundle SHA-256 `7dcfa75af0c4fcfb5cbc4d54d230f04bcb8093157fa45046cec277d6952f2ac2`.
- No main/remotes, push, PR, release, cloud, PDL or kernel changes. Current user-dirty source is not validated by the earlier clean 18-commit host 13-member check or affected C checks; programme completion and release readiness are not established.
- Seventeen DRAFT plans have bounded docs QA PASS; native sync remains unavailable: 33 known writes, possibly 34, 19 corrections pending, sdk/cpp UNKNOWN, 2,551 census items STAGED. No new calls.
- F4 strict disk acknowledgement and stable versus new asset identity remain two owner questions; no implicit approval, F4 code, provenance or wire choice. Alpha association, runtime E2E, install, hardware and production lock policy remain unverified/open.

## Owner decision record — stable asset identity

- **Expected / actual:** Phase 2 Q1 logical identity was open; the owner decided: “stable identity is key - assets contain their own chains - host nodes validate/sign/mirror/reflect”. Stable logical asset identity across versions is settled, not an implementation claim. Phase 2 Q2/name-slot remains open.
- `VersionRef`/IPC wire encoding, format migration, ownership transfer and `Block.calculate_hash` remain undecided; no signer, cryptography or schema choice is implied.
- Durability question #1 is **not approved**: owner response was “I don't know what you're asking”. The Governor rephrased it as whether a stored mirror survives restart; this does not require every reflector to persist. Further owner clarification is pending.

## Owner clarification — mirror availability and context

- Owner: “if the mirror is gone it's no longer active and it's previous work stays with the previous squashed chain within that context and would be treated ephemerally but the interactive nodes would store the interactions on their local exchanges (that's how data persists without exploding)”. This supersedes the earlier mandatory stored-mirror-restart premise, not approval of that durability proposal.
- Engineering derivation: active availability/count is separate from historical context; departed mirrors are inactive, prior work remains with the previous squashed chain in context, and participating nodes store interactions on local exchanges. No universal persistence, deletion, TTL or squash format is prescribed. Investigate existing documentation and code and test retained interactions; no full-payload, durability or reconstruction guarantee is claimed.
- F4 discarded bytes can still defeat actual current servability; absence of storage alone is not necessarily a durability violation. Stable logical identity and asset-owned chains remain settled; hosts validate/sign/mirror/reflect. Encoding, typed format, ownership transfer and `Block.calculate_hash` remain undecided. This records intent, not implementation or approval.

## Active-mirror post-phase status — 2026-11-11

- **Owner intent:** Investigate/test the existing model; no payload-retention choice or permanent mirror is awaited. RAM serves, fresh RAM is empty, optional disk reopen works in the private component probe, and hints are independent. Two initial attempts ran no tests because ignored lock and then untracked `cluster_sim` were missing; corrected inputs manifested 24 target paths. Empty/genesis initial state is not history proof.
- **History probe:** Nonempty signed members and old proofs were retained in a historic seal. Full-block `from_blocks` kept the seal but cleared the pool; substantive pruning removed seal body and latest lookup while header identity remained. Wrong member/path/root and tampered FALCON were rejected by appropriate separate checks; structural membership is not signature validation. Evidence: `/usr/lib/hypermesh/verification/ffi-remainder-20261110-EtvYnhwv/context-retention-probe-20261111/history-probe-attempt{1,2}`.
- **Implementation / actual:** Signed local `3490c59738327999f3e9756c392c9fac8dbf2784` atop `49d48cf7bc68a08d436b921227bce118ca53c6da`, four production and two test files: decode and byte-hash mirrored bytes, hold them in shared serving store before provider/count publication; index-owned hold survives ordinary owner release, retries are idempotent, publication/removal ordered, withdraw/expiry releases hold, and failed claims do not grow empty buckets.
- Borrowed-reference/removal-race and 64-empty-bucket-leak QA failures were corrected and independently reverified. Mislabeled store-executor and old codec gates remain excluded as lifecycle proof. Evidence: `/usr/lib/hypermesh/verification/active-mirror-implementation-20261111-LH9HQb/{implementation-plan.json,final2_executor,final2_store,commit-evidence-20261111}`. Synchronous disk I/O inside the store lock remains an efficiency action.
- Local original DMS fast-forward `49d48cf7` → `3490c597` ran once, exit 0 / 0.515s, exactly six paths/index entries; other dirty, untracked, staged and fresh refs including tags/stash, 17 plans, lock and root were preserved. Different independent integration QA PASS: `/usr/lib/hypermesh/verification/active-mirror-local-ff-20261111-CoLewcwy/report.md`, SHA-256 `88d31f32c093644921e024f15331a64530848ccebf27cf9dcfb8c9639c7afbe6`; recovery SHA-256 `e66cc45bb8962064a935882abcd18a3cc3b5c2f81cc9a4a12ffcda25d4bd94af`.
- **Limits:** No authentication, owner/version provenance, historical journal, physical durability, real STOQ/Catalog, current user-dirty source or product proof follows. No main, push, release, PDL systems, alpha or kernel change is claimed. Source phase is not programme completion.
- Participant receipt/witness production journaling, delivery and trusted historic-root recovery remain unwired. Daemon `owner=None` fails closed; `fixture.with_owner` is not the production owner path. Full blocks in memory are not disk-restart proof; payload recovery is not demonstrated, genesis StoragePointer is not shard data, and offline is not network isolation.
- **Next intent, not a user question or code lease:** Read-only contract design for verified OWNER versus HOST binding and bounded participant-receipt persistence, without a global version store or format policy. Native client last unavailable: 33 known writes, possibly 34, 19 pending, sdk/cpp UNKNOWN, 2,551 census staged; no new native calls.

## Signed integration and remaining gates — 2026-11-11

- **Expected / actual:** Integrate the scoped private candidate without disturbing user work. Signed `53f64c0ab9e6d0f36c4d2df6eaec55e253ae75d0` (parent `3490c59738327999f3e9756c392c9fac8dbf2784`, 20 files) and signed Catalog fixture `7f2d0d0eae872b939619a4a47e4135f3b3d6ff40` (parent `53f64c0`, one file) were locally fast-forwarded once on original `cluster-dms-vcs` from `3490` to `7f2`, exit 0 in 1.001 s, 21 target paths. Independent integration QA PASS is bounded to this clean candidate; other user-dirty code is not validated.
- **Preservation / metric:** Before mutation, two-file/index/ref and 89-state backups plus bare restores of both heads and 21 blobs were captured. Two transient document reversals were followed by mandatory mapping; no reset, stash or force. Both original user comments remain exact and unstaged (CLI `aeed4efe9978e579d28e7fb42f8bd387c9b22497f4adedc4f72e15ab66bf28bc`; PeerContext `689d95b7b6593da7bf4fc2d2d0b8960e58fb2bca7ebc8c29f0d7f91830af2388`). Other 87 states, 27 untracked names, 39 fresh refs except DMS advance, staged patch, lock and private feature were preserved; `ORIG_HEAD=3490`. This five-file documentation append is an intentional post-fence delta.
- **Listener / test evidence:** CLI `--stoq-bind-address` defaults to `::`, not persistent configuration, admission or advertised-endpoint policy. Explicit loopback skips both STOQ and node kernel acceleration before probing; the public Rust/serde flag defaults true for non-loopback. `accept_next` distinguishes connection, terminal closure and per-peer error, while the old wrapper remains compatible. Bounded bad-ALPN, healthy, idle/closed and legacy error cases passed. Anonymous production `::1` bind, occupied rejection and SIGTERM exit within 0.166 s passed, without `NoIncoming` warning. The earlier 3,950,306-warning / 406,893,412-byte flood in about 11 s occurred **after SIGTERM**, not at idle startup; its failed observation remains historical.
- **Workspace / Catalog:** Initial 13-member all-targets check failed (exit 101, 155 s) on a missing `Arc<ShardStore>` argument. The signed fix uses the same fixture store for Seeder and ConsumerProviderManager without weakening assertions. Corrected candidate plus two user-comment overlays passed one `cargo --workspace --all-targets` check (Cargo 0.91 s; wrapper 2.00064 s); this compiled 13 Rust workspace members, not 17 projects, and executed no tests. The named Catalog version-mirror/reflect test executed once and passed in 186.007 s using an in-process `MockShardTransport`, not real wire E2E.
- **Native tracking:** The old sdk/cpp unknown was settled at revision 3; the 25 workflow corrections are accounted for. Seventeen direct draft chains contain 17 planned tasks: 51 explicit creates plus 119 native-generated standard steps, 170 records, bounded independent metadata QA PASS. This first intake is not the staged 2,551 census, full backlogs, native controller assignments or approval. Provider project Coordinators report to `/root`; Operators report to their Coordinators; documentation has its own lane and `/root/coord_core` is temporary integration coordination.
- **Security and remaining work:** Private TLS `AcceptAll`/no-client-auth is verified transport configuration, not by itself proof of an application admission bypass. Audit real FALCON/PoS, channel binding, identity scope and replay before claiming private two-node/shard, Catalog authentication or installer behavior. Asset identity and asset-owned chains are settled; hosts validate/sign/mirror/reflect. Production owner-bound creation, participant witness retention/delivery and trusted context anchor remain unwired; no permanent mirror, mandatory fsync or global-history policy is inferred. Legacy long/deep code and synchronous ShardStore I/O under lock remain efficiency/quality work. Copied ignored lock was build input only. Historical compiler, import, visibility, layout and hook failures were corrected; the old unsafe 14-substitution QA remains failed and excluded.
- **Alpha / gate:** HyperMesh-only Compute list on 2026-11-11 14:46:15–25 returned `SERVICE_DISABLED`, exit 1, for consumer `projects/hypermesh-online`; `[]` proves neither VM count, binary nor health. No API, IAM, cloud, kernel, main, remote, push, PR or release change. `trust.hypermesh.online` CA/CT/DNS is intent, not deployment proof. Hardware is unavailable. No user acceptance, private-auth proof, real-network durability or product readiness follows; only the owner accepts. Next: independently review this post-phase documentation/native delta, then separately authorize further security and integration work.


## Private validation and participation reconciliation — 2026-11-11

- **Expected / owner intent:** Nodes/identity and peripherals are assets; nothing needs public proof when privately validated, but private validation is not no validation (owner source `aeeb6ecaf58ade151b46e65bff45ca86bbbbb86ae61813302a2f45f70d3342f2`, rev 2). “Security comes ONLY Through Privacy” is owner intent, not a proven security theorem. Requirement `053ec618d7939997952f6707f210c41fd3939d90da9bfdba4eac48f42d877c87`, rev 2, records an obligation, not a passed gate.
- **Settled participation boundary:** Owner answer: “Network-scoped participation; device ownership stays unchanged” (question `9bdc8932736d68cbbd49a22f2a8f3aa78fc90ad8720738783d00862412f45bb6`, rev 2). A bank/network controls asset/identity participation scoped to its network; owner-controlled peripherals and device ownership remain unchanged. Joining or host signing is neither title transfer nor an arbitrary resource grant. No public identity/evidence mandate or remote enforcement over unmanaged devices follows.
- **Actual / bounded evidence:** The 19-file uncommitted registry candidate has independent scoped PASS evidence `b4c2450c8aba54d3ffa5669d880bb59f02a79d5189593b2284b72062041e1b09`: local baseline RED, corrected synthetic controls, codec and 13-member compilation only, not live private authentication. The private resolver is a design, not an implementation; genuine authenticated session/context association and four independent evidence sources are missing. Mocks or structural signature checks do not fill these gaps. Routing/demand corrective design is underway, with no implementation at this dispatch (`fc8a69ae5cc92b842d84496c0a78f8c08a1d5df0639d027f74acd0666f423d29`). Seventeen first-intake tasks do not cover the staged 2,551 census or establish completion.
- **Explicit alpha correction / history preserved:** The documented project from CLAUDE is `project-f56ebf7e-1ca0-45ca-b92`, not the earlier `hypermesh-online` target. Its earlier `SERVICE_DISABLED` result does not establish failure of the documented alpha project. Finite local gcloud/SSH attempts (describe, direct Compute, strict SSH and gcloud version) reached cutoffs with empty output; DNS establishes only `35.208.78.211`, not a host handshake, VM health or project association. Native alpha task `fb6ef50aa5489df06a3ff47d1d0ccec0864c82916b8c9003ac00500f5f07dc3c`, REVIEW rev 10; no cloud changes.
- **Remaining actionable gates:** Bind the genuine authenticated session to its authorized security scope/context and channel; obtain independent evidence for all four proofs, enforce binary acceptance and deny replay. Implement and independently verify the resolver and corrective routing/demand work under separate authorization; independently review this two-plan documentation phase. Private resolver contract `/usr/lib/hypermesh/verification/peer-session-20261111-y9i3z7t4/private-resolver-design/contract.txt`, SHA-256 `0640d3f7538fd1fc90878a94c15933d627d15f56095915bd2250f1efaa7048e9`, §7 is owner-answered by the participation decision above. No new cryptography/wire choice, readiness, hardware/physical proof, source-publication approval or user acceptance is claimed.
