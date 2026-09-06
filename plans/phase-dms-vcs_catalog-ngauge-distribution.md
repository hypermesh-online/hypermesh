<!-- Written by Richard Christopher, Copyright 2026 HyperMesh Foundation -->

# Catalog = VCS / NGauge = DMS — Phased Roadmap

Owner-ratified 2026-08-10. Memory: [[catalog-vcs-ngauge-dms]]. Branch: `cluster-dms-vcs` off `cluster-e-network-identity`.

## Resolved architecture (settled)
- **Asset's OWN CHAIN is the single lineage substrate** (blockmatrix `AssetLineage`, `lineage.rs:48`; node≡asset≡index). Neither Catalog nor NGauge owns a version store — both PROCESS the same chain, for two reasons.
- **Catalog = VCS view**: versions / branches / progression. Version = asset-chain entry (`prev_asset_entry`/`asset_seq` already exist — NON-format). Branch = fork carrying a parent pointer in `AssetData.metadata` JSON (content-bound, non-format). Keep the real inter-package dependency-DAG.
- **NGauge = DMS driver**: for each version H, DRIVES mirror (replicate to the asset's device pool) + reflect (serve/relay to consumers — the swarm seed side; each asset has a POOL of reflecting devices). NGauge owns DECISIONS; blockmatrix EXECUTES I/O behind traits `MirrorExecutor`/`ReflectExecutor` **defined in ngauge** (blockmatrix already depends on ngauge; ngauge deps = lib only). NGauge must NOT own the syscalls (crate-dep direction + non-Send-MutexGuard-across-await, `feed.rs:28`).
- **Handoff = the asset-chain is the bus**: Catalog writes a version entry; blockmatrix observes new asset heads → drives DMS. No direct catalog→ngauge wire.
- `Block::calculate_hash` preimage FROZEN. Any typed StateProof/Block field = FORMAT change → quarantine into the deferred STEP7 batched-migration branch, never in a behavior-preserving PR.

## Phases

### Phase 0 — DMS decision seam in ngauge (pure, non-format, G/H-independent)
New `ngauge/src/placement/dms.rs`: `DmsPlan{mirror:Vec<MirrorAction>, reflect:Vec<ReflectAction>}` (lib types only); traits `MirrorExecutor::fetch_and_register` + `ReflectExecutor::announce` (dyn-safe, lib types); `DmsDriver::plan(&SwarmAnalytics, network, candidates, coords)->DmsPlan` folding `ReplicationTrigger::check_in_network` + `DispersionAdvisor::recommend_placement_in_network` + the dormant `replica_selection::ReplicaSelector`/`order_by_proximity` (turns it LIVE). Planner is a PURE function so the caller holds the `std::sync::Mutex<SwarmAnalytics>` guard, builds the plan, drops it BEFORE any await. Carry `FallbackStrategy` in `MirrorAction`.
- **QA:** `cargo build/test -p ngauge`; invariant: ngauge deps unchanged (lib only).

### Phase 1 — blockmatrix poll loop becomes a thin executor (behavior-preserving)
`replication_service/poll.rs`: replace the inline signal loop + **delete `select_dispersion_source` (poll.rs:210-288, the W1 duplicate)** with `plan = DmsDriver::plan(&guard); drop(guard); for a in plan.mirror { executor.fetch_and_register(a).await }`. New `replication_service/executor.rs`: `impl MirrorExecutor for StoqDmsExecutor` (wraps `StoqShardTransport`+`ShardLocationIndex`+`node_id`, = the existing fetch_shard→register_provider→set_replica_count sequence) + `impl ReflectExecutor` (consumer_provider announce). `ReplicationService` builds the executor from Arcs it already holds.
- **QA:** replication_service tests + E.2 convergence test (replica count still reaches target); invariant: same intervals/urgency>0.5 filter/feedback; guard dropped before await.
- **Overlap:** SUBSUMES cluster-H replication-source-selection dedup (W1-34/35/38/39 half). Coordinate: H builds on `MirrorExecutor`, not poll.rs.

### Phase 2 — Catalog version substrate: READ the asset-chain (behavior-preserving)
`catalog/src/registry/catalog_registry.rs`: add a chain handle (catalog already deps blockmatrix → call `NodeBlockchain::asset_lineage`/`asset_head` directly, `chain.rs:257/610`). New `list_versions(name)->Vec<VersionRef>` from `AssetLineage::entries`+`sequence()`; `head_version()` from `head()`. Keep `resolve_dependencies`/`DependencyGraph` untouched (orthogonal). `type_hash=BLAKE3(schema)` = per-version content id; `asset_hash` = stable typedef identity across versions.
- **QA:** registry tests; dep-DAG tests unchanged.
- **RESOLVE HERE (flag to owner):** (1) `asset_hash` = genesis identity carried forward by later entries (recommended (a)) vs fresh-hash-per-version. (2) relax the single-slot rejection (`:264`) to "same name + new version = new chain entry"; name-uniqueness only at genesis.

### Phase 3 — Catalog WRITES versions to the asset-chain (handoff producer) — GATES ON CLUSTER G
`register_type`/progress → `NodeBlockchain::register_asset_record` (genesis, `mutations.rs:717`) or successor `BlockAssetEntry::new_bound`+`set_asset_lineage(prev.lineage_id, prev.asset_seq+1)`+`add_block` (`mutations.rs:152`). Branch = new asset-chain whose genesis `AssetData.metadata` = `{"branch_parent":..., "branch_of_asset":...}` (non-format). Real bilateral StateProof construction = **cluster-G touch point → land G first**.
- **QA:** register v1→v2→branch; `AssetLineage::verify()` green; branch genesis valid + names parent; `calculate_hash` untouched.
- **Format flags:** NONE if branch pointer stays in metadata. Typed `StateProof.branch_parent` → STEP7 migration only.

### Phase 4 — Wire the handoff: blockmatrix observes new heads → DMS drive — CO-BUILD WITH CLUSTER H
blockmatrix observer on new asset head (hook `mutations::add_block` completion / `AssetChainIndex` snapshot `chain.rs:670`) seeds the asset's shard set into `SwarmAnalytics`/`ShardLocationIndex` → Phase-1 executor mirrors+reflects on next tick. In-process (`DaemonState::catalog_registry` + `wire_catalog_registry`).
- **QA:** E2E: catalog registers H → within one poll interval, H's shards mirrored to the device pool + announced. Invariant: torrent model — any handshake-authed peer mirrors PUBLIC; PRIVATE requires a Grant on the asset chain (check before mirror).
- **Overlap:** SAME surface as cluster-H placement-execution — ONE work item, don't double-build. Depends on H's single-sharding-stack (landed).

### Phase 5 — (Deferred, optional) typed branch-capable lineage — BATCHED MIGRATION ONLY
Only if owner rejects metadata pointer: typed `StateProof`/`BlockAssetEntry` branch field + `AssetLineage` DAG-verify. FORMAT change → rides STEP7 wipe. Phases 0-4 designed to not need this.

## Execution order (two hard gates)
`0 → 1` (DMS seam + dedup, ship immediately, behavior-preserving) → `2` (catalog read) → **[gate: cluster G]** → `3` (catalog write) → **[gate/merge: cluster H placement-execution]** → `4` (handoff) → `5` deferred.
Branch: `cluster-dms-vcs` off cluster-e. Phases 0-2 = small independent PRs (each green). Phase 3 waits behind G. Phase 4 lands with H's placement PR. Every PR NON-format; StateProof/Block touches quarantined to STEP7.

## Concern map (after)
VCS version graph → Catalog (view over AssetLineage) · lineage substrate/verify → blockmatrix AssetLineage · dep-DAG → Catalog · DMS decisions → NGauge `DmsDriver::plan` · DMS I/O → blockmatrix `StoqDmsExecutor` behind ngauge traits · handoff trigger → blockmatrix head-observer · PoS proofs → cluster G · sharding+placement-exec → cluster H (Phase 4 co-builds).

## Risks
Version-identity ambiguity (Phase 2 Q1 — resolve before 3); handoff in-process assumption (confirm catalog+ReplicationService share a process, else chain-on-disk observer); non-Send guard regressions (plan-drop-execute discipline enforced in review); Grant enforcement for PRIVATE assets in the mirror path (Phase 4). Possible R17 (hard requirement) — confirm w/ owner, add to papers/HYPERMESH.md §3 + CLAUDE.md.
