# Portable remediation evidence index

**2026-11-20 — independently reproduced guest STOQ/XDP transport:** a fresh
41-byte identical echo used real STOQ/QUIC over guest `lo` through direct-libbpf
XDP. The target map advanced after handshake (21→23), the off-port control
payload left it unchanged (25→25), and detachment yielded ID 0. See the
[trusted-build recipe](../tools/demo_gate/stoq_guest/README.md). This is not two
installations, delegated STOQ/AF_XDP acceleration, or authorization; packet
deltas may include QUIC control frames, not just application payload.

Historical source/recipe baseline: `9f775b3b97cfcd55a238b0d64915c193a02c8149`.
For superseding dated QUIC, GNU CPU-default and CI observations, see
[the current PLAN observations](../PLAN.md); plain-QUIC trial success is not STOQ/PQ/admission proof.
This index replaces a link to a local-only historical report. Private raw reports
are not published here; source and commands are entrypoints, not proof that a
new clone has reproduced a result or that the product is accepted.

## User-demonstrable quality gate

Follow the [current gate and trial order](../PLAN.md#current-user-demonstrable-gate-and-trial-order):
independent prerequisite trials → genuine scoped asset/proof/replay/mirror-loss
and byte-identity interaction across installations → separately qualified public
alpha artifact, health and rollback. Seven isolated software STOQ cases, 36,000
locally reconstructed bytes, 33 NGauge tests/strict Clippy and guest XDP
9292-versus-9293/detach evidence are **not one combined path**. Caesar's
financial transmission-only token and SDK completion remain gates. Private
validation does not require publishing private data or proofs.

The paired [guest README](../tools/demo_gate/guest_xdp/README.md) and
`tools/demo_gate/guest_xdp/run.sh` provide the command and permissions described
in PLAN. Its independent no-NIC KVM trial passed at 00:39:47Z (2026-11-20 UTC):
target-port count 1, off-port count 0 and detach 0. Do not mistake
that ordinary guest UDP/kernel diagnostic for production STOQ, PQ/admission or
a two-node asset transfer. Independent reproduction is required for each
proposed release, and the real production gate remains outstanding. DNS alone or SSH/HTTP3 timeouts establish neither running alpha
health nor service failure; no deployment or readiness is claimed here.

| Scope | Stable source / check | Conditions and historical limitations |
|---|---|---|
| Offline installer | [Source](../scripts/install/hypermesh-install.sh), [recipe](guides/INSTALL.md#private-offline-install); `sh tests/integration/installer_private_offline.sh` ([fixture](../tests/integration/installer_private_offline.sh)) | 68 hermetic mock-artifact cases are packaging checks, not genuine-node evidence. Separately, real Linux x86_64 GNU local no-start installation and relocated help/version were independently reproduced. No start, join, vendor provenance or other-host support established. |
| Performance fixture compatibility | [Source](../blockmatrix/tests/test_performance_validation.rs); the SSD-directed compile command below (after preparing the guide’s fresh operation directory) | Signed `9f775b3b` field repair passed a focused target check, not performance measurements, QUIC, BPF or broader workspace clearance. This is a compile entrypoint, not an executed benchmark. |
| Build inputs | [Workspace](../Cargo.toml), [BlockMatrix](../blockmatrix/Cargo.toml) | Local evidence used Rust 1.98.1, compatible cached dependencies and an explicitly supplied ignored lock. Cargo.lock is untracked; a clean clone is not a reproducible locked/offline build. Prepare/review local inputs before the recipe. |
| Release packaging | [Workflow](../.github/workflows/release.yml), [online script](../scripts/install.sh) | Observed latest v0.1.0-alpha provides legacy x86_64-linux archive only, without expected Rust-triple archive/binary manifest. Verified online installation unavailable; no bypass. Workflow intent is not proof of published artifacts. |

Performance-target compile entrypoint, **after** the guide’s recipe has prepared
`operation` on a caller-owned absolute writable SSD root and a reviewed local
lock/cache/toolchain is available. This is the observed `--all-features` target
check, with both output directories explicitly redirected; do not run a bare
Cargo command that falls back to checkout storage:

```sh
set -eu
: "${operation:?first prepare the fresh SSD operation directory in the guide}"
case "$operation" in /*) ;; *) echo 'operation must be absolute' >&2; exit 1;; esac
test -d "$operation" && test -w "$operation"
export CARGO_TARGET_DIR="$operation/target"
export CARGO_NET_OFFLINE=true CARGO_NET_RETRY=0
cargo +1.98.1 --config "build.target-dir=\"$CARGO_TARGET_DIR\"" \
  --config "build.build-dir=\"$operation/intermediates\"" \
  check --offline --locked -p blockmatrix --all-features --test test_performance_validation
```

Inspect immutable inputs without trusting a dirty checkout:

```sh
git show 9f775b3b:PLAN.md
git show 9f775b3b:scripts/install/hypermesh-install.sh
git show 9f775b3b:blockmatrix/Cargo.toml
```

Run bounded checks only with the stated prerequisites. Hash equality is byte
integrity, not publisher authentication. Mock/simulation/focused compilation
results do not establish real-network operation, four-proof admission, release
provenance, hardware behavior or readiness. [PLAN](../PLAN.md) retains the open
security, missing-BPF-object and broader workspace gates; the earlier QUIC-example
failures are superseded by the dated plain-QUIC outcomes in PLAN, not by a
whole-workspace or private-admission clearance.
