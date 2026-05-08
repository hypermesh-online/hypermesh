# Releasing HyperMesh

This document describes the foundation release process for the `hypermesh` node
binary. It is the operator's guide for cutting a tagged release, signing the
release manifest with the foundation FALCON-1024 root key, and pushing the
signed manifest into the catalog release feed (Phase J).

## Release roadmap context

- **Phase C.1** (this document): cross-platform builds + GitHub Release matrix.
- **Phase J** (already shipped): `release.feed/v1` typedef, daemon-side
  `release_feed_subscriber`, FALCON-1024 signature verification on poll.
- **Phase L** (this sprint and onward): the full distribution pipeline that
  ties Phases C and J together.

The Phase J subscriber does the runtime trust work. Phase C.1 produces the
artifacts the subscriber consumes.

## One-time setup

1. **Foundation FALCON-1024 root key.** Generated and held offline by the
   foundation. The public key bytes are configured at every daemon as
   `DaemonState.release_feed_subscriber.foundation_pubkey`. Do not rotate
   without coordinating a daemon-side config update.

2. **GitHub repo permissions.** The `release.yml` workflow needs
   `contents: write` (already granted in the workflow). No other secrets are
   required for unsigned manifest production. Signing happens offline.

3. **`gh` CLI and `cargo` installed locally** for the operator who performs
   the offline signing step.

## Cutting a release

### 1. Tag and push

```bash
# from a clean tree on main
git tag -s v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

The signed git tag is independent of the FALCON-1024 release-manifest
signature. It establishes provenance of the source; the manifest signature
establishes provenance of the binaries.

### 2. Watch the matrix build

The `Release` workflow (`.github/workflows/release.yml`) fires on the tag
push. It runs three job stages:

1. **`create-release`** — opens the GitHub Release record with auto-generated
   release notes from the commit log.
2. **`build`** — matrix build of the `hypermesh` binary across:
   - `x86_64-unknown-linux-musl` (primary, deploys to trust.hypermesh.online)
   - `aarch64-unknown-linux-musl` (Raspberry Pi, ARM cloud)
   - `x86_64-apple-darwin` *(experimental, see eBPF Feature Gating below)*
   - `aarch64-apple-darwin` *(experimental)*
   - `x86_64-pc-windows-msvc` *(experimental)*
3. **`manifest`** — collects each target's SHA-256 hash and writes an
   **unsigned** `release-manifest.json` matching the `release.feed/v1`
   typedef. Both the per-target archive and the unsigned manifest are
   uploaded to the GitHub Release.

If a non-Linux target fails (`continue-on-error: true`), the workflow still
completes; that target is simply absent from `binary_hashes` in the manifest.

### 3. Pull the unsigned manifest

```bash
gh release download v0.2.0 \
    --repo hypermesh-online/hypermesh \
    --pattern 'release-manifest.json' \
    --dir /tmp/hm-release
```

Inspect it:

```bash
cat /tmp/hm-release/release-manifest.json
```

You should see all built targets in `binary_hashes`, with `signed_by` and
`signature` set to empty strings.

### 4. Sign offline

The signing payload is defined in
`catalog/typedefs/release_feed.json#verification.signing_payload_format`:

```
[version UTF-8] || 0x00 || [channel UTF-8] || 0x00 || [canonical(binary_hashes) UTF-8] || 0x00 || [issued_at u64 LE seconds]
```

`canonical(binary_hashes)` is the JSON object with keys sorted lexicographically
and no whitespace. `issued_at` is a u64 little-endian seconds timestamp,
**not** the RFC 3339 string in the manifest — keep both consistent.

The foundation signs that byte string with the offline FALCON-1024 root key.
The signing tool is intentionally not in this repo (the key never touches a
networked machine). Reference implementation pseudocode:

```rust
let payload = build_signing_payload(&manifest); // see typedef
let signature = falcon1024_sign(&foundation_secret_key, &payload);
manifest.signed_by = hex::encode(foundation_pub_bytes);
manifest.signature = hex::encode(signature);
fs::write("release-manifest.json", serde_json::to_string_pretty(&manifest)?)?;
```

### 5. Re-upload the signed manifest

```bash
gh release upload v0.2.0 \
    --repo hypermesh-online/hypermesh \
    --clobber \
    /tmp/hm-release/release-manifest.json
```

The `--clobber` flag replaces the unsigned manifest in place. The archives
themselves are unchanged (their hashes are already committed inside the
manifest, so re-signing does not invalidate any artifact).

### 6. Verify

A subscribed daemon should pick up the new entry on its next poll cycle
(Phase J `release_feed_subscriber.poll_interval`, default 1 hour). To verify
manually:

```bash
# Pull the signed manifest
gh release download v0.2.0 \
    --repo hypermesh-online/hypermesh \
    --pattern 'release-manifest.json' \
    --dir /tmp/check

# The daemon will:
# 1. Reconstruct the canonical signing payload
# 2. Verify falcon1024_verify(foundation_pubkey, payload, signature)
# 3. If valid, append to available_versions for the channel
# 4. If breaking_changes == true, refuse to auto-upgrade without operator opt-in
```

Successful verification requires the manifest's `signed_by` to match the
daemon's configured `foundation_pubkey`. Any mismatch is a hard reject — the
daemon does not trust new public keys at runtime.

## eBPF feature gating

`hypermesh-ebpf` is a hard dependency of `blockmatrix`. It currently includes
`aya` and `libbpf-sys` unconditionally, both of which require the Linux kernel
build environment. As a result:

| Target | Build status | Features included |
|--------|--------------|-------------------|
| `x86_64-unknown-linux-musl`     | **stable**       | `caesar,intelligence` |
| `aarch64-unknown-linux-musl`    | **stable**       | `caesar,intelligence` |
| `x86_64-apple-darwin`           | **experimental** | `caesar` (intelligence dropped) |
| `aarch64-apple-darwin`          | **experimental** | `caesar` (intelligence dropped) |
| `x86_64-pc-windows-msvc`        | **experimental** | `caesar` (intelligence dropped) |

Experimental targets are marked `continue-on-error: true` in the matrix so
the workflow surfaces porting progress without blocking foundation releases.
If a non-Linux build fails, that target is simply omitted from
`binary_hashes` and the manifest still ships.

The follow-up work (Phase C.2) is to gate `aya` and `libbpf-sys` as
`[target.'cfg(target_os = "linux")']` deps in `hypermesh-ebpf/Cargo.toml`,
guard the source modules with `#[cfg(target_os = "linux")]`, and provide
no-op stubs on other platforms. Once that lands, all five targets become
stable.

## Per-platform install

### Linux (any glibc / musl distro)

```bash
curl -sSfL https://raw.githubusercontent.com/hypermesh-online/core/main/scripts/install.sh \
    | sudo bash -s -- --version v0.2.0
```

Defaults to `/usr/local/bin/hypermesh`, installs systemd units when run as
root with `systemctl` available.

### macOS (Apple Silicon)

```bash
curl -sSfL https://raw.githubusercontent.com/hypermesh-online/core/main/scripts/install.sh \
    | bash -s -- --version v0.2.0
```

Detects `aarch64-apple-darwin` and defaults to `/opt/homebrew/bin` if it
exists, otherwise `/usr/local/bin`. No systemd; operator wires up
`launchd` manually if running as a service.

### macOS (Intel)

Same command as Apple Silicon. Detects `x86_64-apple-darwin`, installs to
`/usr/local/bin`.

### Windows

`install.sh` does not run on Windows. Download the `.zip` archive directly:

```
https://github.com/hypermesh-online/hypermesh/releases/download/v0.2.0/hypermesh-v0.2.0-x86_64-pc-windows-msvc.zip
```

Extract `hypermesh.exe` to a directory on `PATH`. A PowerShell installer
(`install.ps1`) is a Phase C.2 follow-up.

## Updating an existing install

Operators should not re-run `install.sh` for routine updates. Once a daemon
is running with a configured `release_feed_subscriber`, the in-process
upgrade flow handles version transitions:

1. Subscriber polls the catalog release feed
2. Validates the FALCON-1024 signature against the configured foundation pubkey
3. If a newer version on the operator's channel is found, it is added to
   `available_versions`
4. The operator (or auto-upgrade policy) issues `hypermesh update` (Phase J)
5. The daemon downloads the new binary, verifies the SHA-256 against
   `binary_hashes[target_triple]`, atomically swaps, and restarts

`install.sh` is the bootstrap path only. After first install, the daemon
manages itself.

## Troubleshooting

**The `manifest` job ran but `binary_hashes` is empty.**
All matrix builds failed. Inspect the `build` job logs per target. Linux
builds should always succeed; non-Linux failures are expected until Phase C.2.

**Daemon rejects a signed manifest.**
Either the foundation pubkey at the daemon does not match `signed_by`, or
the canonical signing payload was constructed wrong during signing. Re-derive
the payload using the format in
`catalog/typedefs/release_feed.json#verification.signing_payload_format` and
re-sign. The pubkey in `signed_by` is hex-encoded raw FALCON-1024 public key
bytes, not a certificate.

**`install.sh` says SHA mismatch.**
The archive on the GitHub Release was modified after the manifest was
generated, or the manifest was hand-edited. Re-run the workflow on the same
tag (delete the tag first if needed: `git tag -d v0.2.0 && git push --delete
origin v0.2.0 && git tag -s v0.2.0 ... && git push origin v0.2.0`). Do not
hand-edit `binary_hashes`.

**A platform is missing from the release.**
Check the build job for that target. If it failed with a `libbpf-sys` link
error, that's the known Phase C.2 gap. If it failed for another reason,
that's a real bug — file an issue.
