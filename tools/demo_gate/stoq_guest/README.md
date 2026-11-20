# Guest-only STOQ packet attribution

This is a bounded diagnostic for **real local STOQ/QUIC packets** reaching the
tracked HyperMesh XDP C program in an isolated Linux guest. It does not test
production Aya attachment, AF_XDP zero-copy, PoS, Catalog, two hosts, or a
public alpha. The guest runner attaches the exact C object with **direct
libbpf**; the separate Rust fixture uses the actual `StoqTransport` library
with its local/test certificate strategy and `enable_kernel_acceleration=false`
so it cannot silently install a second XDP program.

## Safe, offline reproduction

Supply only a **trusted ELF you built from the reviewed source**. The runner
invokes `ldd` on the host to collect its libraries; `ldd` is unsafe for untrusted
executables. Do **not** supply a downloaded, externally supplied, or otherwise
untrusted binary, even though execution of the fixture is intended for a guest.

A clean clone does **not** contain the ignored root `Cargo.lock`. Before this
recipe, independently review/provide a compatible lockfile, a populated private
SSD Cargo cache, and the already-installed pinned Rust 1.98.1 toolchain. These are
explicit local inputs, not reproducible public artifacts supplied by this repo.
See the [evidence index](../../../docs/REMEDIATION_EVIDENCE_INDEX.md) for the
lock/cache limitations. Do not generate a new lock or download inputs merely to
make this offline recipe pass. Hold the exclusive Cargo/runtime lease.

From the reviewed core source root (this fixture and the tracked XDP C source):

```sh
set -eu
: "${CARGO_HOME:?set CARGO_HOME to the reviewed private SSD Cargo cache}"
test -d "$CARGO_HOME"
test -f Cargo.lock
out=$(mktemp -d /usr/lib/hypermesh/verification/stoq-build-XXXXXXXX)
export CARGO_TARGET_DIR="$out/target"
export CARGO_NET_OFFLINE=true CARGO_NET_RETRY=0
rustup run 1.98.1 cargo \
  --config "build.target-dir=\"$out/target\"" \
  --config "build.build-dir=\"$out/intermediates\"" \
  build --offline --locked -j2 -p stoq --example guest_loopback_attribution
HYPERMESH_STOQ_GUEST_BIN="$out/target/debug/examples/guest_loopback_attribution" \
  ./tools/demo_gate/stoq_guest/run.sh
```

Both Cargo output directories are explicitly redirected to this private SSD
operation directory. `rustup run` requires the installed toolchain rather than
installing it; missing offline dependencies or lock inputs are failures.

Build and run must use the **same source tree**. Record the source revision,
`Cargo.lock`, fixture source, compiler command, and resulting executable hash;
`run.sh` records hashes of inputs and the ELF it packages, but a supplied
binary path alone does not prove its provenance. No credential from a real
network is needed or copied. Local self-signed/test material is ephemeral in
the guest.

Requirements: KVM, packaged kernel (default `/boot/vmlinuz-linux`), clang BPF
target, GCC, libbpf headers/runtime, QEMU, cpio, gzip, timeout, and a writable
SSD output root (default `/usr/lib/hypermesh/verification`). Override kernel
with `HYPERMESH_GUEST_KERNEL`; override output root with
`HYPERMESH_GUEST_OUT_ROOT`. The runner refuses missing KVM rather than
falling back to emulation. It uses 768 MiB, 2 vCPUs, a 90-second direct QEMU
timeout, `-nic none`, no guest disks and no host mounts. All build/run artifacts
and raw logs stay in a private mode-0700 SSD directory printed as `evidence=`;
no binaries or logs belong in the source patch.

The Rust fixture starts a STOQ server on `::1:9292` and client on `::1` with
an OS-assigned source port. After `connect`, it pauses; the guest runner reads
the **program-specific `connection_map`** before releasing a fresh 41-byte
payload. It checks byte-identical echo and prints BLAKE3 of the payload, then
pauses for the second map snapshot. A second STOQ server/client on `::1:9293`
is the off-port control, with the same staged echo and snapshots. The guest
prints kernel program/map IDs, per-stage map packet totals, child exit,
detach result and post-detach ID. Nonzero exit or missing pass/detach markers
fails the script.

Interpretation: a positive port-9292 map delta after the echo is **associated
with the bounded payload phase**, but the delta can include QUIC ACK/control
frames; it is not a count of application payload packets. The off-port control
should not add entries to the 9292-only XDP map. A pass proves only that this
source-built STOQ path emitted real UDP/QUIC traffic observed by the separately
attached project XDP program in this guest. It does **not** prove kernel-side
authentication, application authorization, AF_XDP use, or transfer of a
Catalog-backed asset.
