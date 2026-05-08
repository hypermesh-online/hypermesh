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

`hypermesh-ebpf` is a hard dependency of `blockmatrix`. As of Phase C.2,
`aya`, `aya-log`, `libbpf-sys`, and `libc` are gated as
`[target.'cfg(target_os = "linux")'.dependencies]` in
`hypermesh-ebpf/Cargo.toml`. All `aya`/`libbpf-sys` source-level usage is
already behind `#[cfg(feature = "kernel-attach")]`, and the
`kernel-attach` feature itself only enables `libc` on Linux. The result:
the eBPF crate's userspace API surface (`HyperMeshEbpf::new()`,
`set_routing_rule`, `set_privacy_tier`, `metrics()`, etc.) compiles on
all targets, with kernel-attach functionality silently unavailable on
non-Linux (graceful degradation per the project's "full eBPF →
eBPF without AF_XDP → userspace" model).

| Target | Build status | Features included | Notes |
|--------|--------------|-------------------|-------|
| `x86_64-unknown-linux-musl`     | **stable**       | `caesar,intelligence` | Primary deploy target |
| `aarch64-unknown-linux-musl`    | **stable**       | `caesar,intelligence` | Raspberry Pi, ARM cloud |
| `x86_64-apple-darwin`           | **experimental** | `caesar`              | hypermesh-ebpf compiles; broader cross-platform work pending (C.2.5) |
| `aarch64-apple-darwin`          | **experimental** | `caesar`              | Same as above |
| `x86_64-pc-windows-msvc`        | **experimental** | `caesar`              | Same as above |

Experimental targets are still marked `continue-on-error: true` in the
matrix because **other** Linux-specific code in the workspace has not yet
been gated — notably:

- `blockmatrix/src/assets/proxy/nat_translation/translation.rs`
  unconditionally imports `libc::{mmap, munmap, MAP_PRIVATE, ...}` for
  NAT-translated memory mapping. Needs `cfg(unix)` gating with a Windows
  fallback (e.g. `VirtualAlloc`/`VirtualFree`).
- Various `cfg(target_os = "linux")` gates in `blockmatrix/src/container/`,
  `metrics/hardware.rs`, and `os_integration/` whose non-Linux branches
  return placeholder values rather than real platform implementations.

The follow-up sprint (**Phase C.2.5**) is the broader cross-platform port
of `blockmatrix`. Once that lands, the non-Linux entries in this matrix
flip to `stable` and `continue-on-error` is removed.

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

PowerShell installer (Phase C.2):

```powershell
iwr -useb https://raw.githubusercontent.com/hypermesh-online/core/main/scripts/install.ps1 | iex
```

Or with explicit options:

```powershell
# Download install.ps1 first, then run with arguments
iwr -useb https://raw.githubusercontent.com/hypermesh-online/core/main/scripts/install.ps1 -OutFile install.ps1
.\install.ps1 -Version v0.2.0 -Prefix "$env:LOCALAPPDATA\HyperMesh"
```

Defaults to `%ProgramFiles%\HyperMesh\bin\hypermesh.exe` when run elevated;
falls back to `%LOCALAPPDATA%\HyperMesh\bin\hypermesh.exe` for non-elevated
sessions. Adds the install dir to the user-level `PATH` automatically (open
a new shell to pick it up). SHA-256 verification against
`release-manifest.json` runs by default; pass `-NoVerify` to skip.

If you prefer manual install, download the archive:

```
https://github.com/hypermesh-online/hypermesh/releases/download/v0.2.0/hypermesh-v0.2.0-x86_64-pc-windows-msvc.zip
```

Extract `hypermesh.exe` to a directory on `PATH`.

> Note: Until Phase C.2.5 ships the broader cross-platform port (see "eBPF
> feature gating" above), the Windows binary may be absent from a given
> release. Check the GitHub Release page for the Windows zip; if missing,
> the build failed and only Linux artifacts shipped for that tag.

## macOS code signing & notarization

Releases are produced unsigned by the GitHub Actions workflow. To sign
and notarize a macOS binary so users do not see "macOS cannot verify the
developer", the foundation operator runs the following one-time-per-release
flow on a macOS machine with the foundation Apple Developer ID Application
certificate installed:

```bash
# 1. Download the unsigned archive
gh release download v0.2.0 \
    --repo hypermesh-online/hypermesh \
    --pattern 'hypermesh-v0.2.0-aarch64-apple-darwin.tar.gz' \
    --pattern 'hypermesh-v0.2.0-x86_64-apple-darwin.tar.gz'

# 2. Extract, sign, re-archive
for triple in aarch64-apple-darwin x86_64-apple-darwin; do
    tar xzf "hypermesh-v0.2.0-${triple}.tar.gz"
    cd "hypermesh-v0.2.0-${triple}"

    # Sign with hardened runtime + timestamp (required for notarization)
    codesign --force --options runtime --timestamp \
        --sign "Developer ID Application: Hypermesh Foundation (TEAMID)" \
        hypermesh

    # Verify
    codesign --verify --strict --verbose=2 hypermesh
    spctl --assess --type execute --verbose hypermesh   # may fail until notarized

    cd ..
    tar czf "hypermesh-v0.2.0-${triple}.tar.gz" "hypermesh-v0.2.0-${triple}"
done

# 3. Notarize (one ZIP per arch, Apple's notary service is per-binary)
for triple in aarch64-apple-darwin x86_64-apple-darwin; do
    # notarytool wants a .zip, not .tar.gz
    cd "hypermesh-v0.2.0-${triple}"
    zip -r "../notarize-${triple}.zip" hypermesh
    cd ..

    xcrun notarytool submit "notarize-${triple}.zip" \
        --apple-id "ops@hypermesh.online" \
        --team-id "TEAMID" \
        --password "@keychain:hm-notary" \
        --wait
done

# 4. Re-upload signed archives, replacing originals
gh release upload v0.2.0 \
    --repo hypermesh-online/hypermesh \
    --clobber \
    hypermesh-v0.2.0-{aarch64,x86_64}-apple-darwin.tar.gz

# 5. Recompute and re-sign release-manifest.json (binary hashes changed!)
#    See "Sign offline" section above. The signing payload changed, so the
#    foundation FALCON-1024 signature must also be regenerated.
```

The notary password lives in macOS Keychain (`security add-generic-password
-s hm-notary -a ops@hypermesh.online -w <app-specific-password>`); it is
**not** stored in the repo. The Apple Developer ID certificate and notary
credentials are foundation-operator concerns; they are not wired into the
GitHub Actions workflow because that would require uploading those secrets
to GitHub, which is contrary to the offline-key model used for the
FALCON-1024 release manifest signing.

### Without code signing

If a release ships unsigned (the default from the GitHub Actions workflow),
macOS users will see "macOS cannot verify the developer" on first launch.
They can either:

```bash
# Strip the quarantine attribute set by Gatekeeper
xattr -d com.apple.quarantine /usr/local/bin/hypermesh
```

or right-click → Open in Finder, then click "Open" in the dialog (one-time
override). The `install.sh` script does not currently strip the quarantine
attribute automatically; doing so would require either elevated privileges
or running inside the user's shell context, both of which complicate the
"pipe curl to bash" UX.

## Desktop shell bundles (Phase C.3)

Phase C.3 added a Tauri 2 desktop shell under `desktop/`. The release
workflow's `desktop` matrix job builds platform bundles in parallel
with the bare-binary matrix:

| Runner | Bundles produced |
|--------|------------------|
| `ubuntu-latest` | `.AppImage`, `.deb` |
| `macos-latest` (x86_64) | `.dmg`, `.app.tar.gz` |
| `macos-latest` (aarch64) | `.dmg`, `.app.tar.gz` |
| `windows-latest` | `.msi`, NSIS `.exe` |

The desktop matrix is currently `continue-on-error: true` because:

1. Tauri requires platform GUI sysdeps (libwebkit2gtk-4.1, GTK3,
   AyatanaAppIndicator on Linux; WebView2 on Windows; Xcode CLT on
   macOS). Workflow installs them on Linux automatically; the macOS
   and Windows runners ship them.
2. The desktop bundle attempts to embed the daemon binary as a Tauri
   sidecar by downloading the `hypermesh-{target}` artifact produced
   by the bare-binary `build` job. If that artifact is missing (for
   experimental targets that didn't compile), the bundle falls back
   to PATH lookup at runtime.
3. Real platform icons (`icons/*.{png,icns,ico}`) are placeholders in
   the alpha. Bundles will use Tauri's default icon until they're
   replaced — `cargo tauri icon path/to/source-1024.png` regenerates
   the full set.

To build a desktop bundle locally:

```bash
# Linux sysdeps (Debian/Ubuntu)
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev libssl-dev

# Tooling
corepack enable && corepack prepare pnpm@9 --activate
cargo install tauri-cli --version "^2"

# Build
cd ui/frontend && pnpm install && pnpm build
cd ../../desktop && cargo tauri build
# Bundles emerge under desktop/target/release/bundle/
```

The desktop project is intentionally *not* a workspace member of
`core/Cargo.toml`. `cargo check --workspace` does not pull in Tauri
or webview deps. See `desktop/README.md` for the full developer guide.

## Mobile shell bundles (Phase C.4)

Phase C.4 added a React Native + Expo mobile shell under `mobile/`. iOS
and Android builds are produced via **Expo Application Services (EAS)**
**outside** of GitHub Actions. The Apple Developer signing cert and
Google Play upload key are operator-controlled secrets that must not
live in CI.

| Profile (`eas.json`) | Output | Distribution |
|----------------------|--------|--------------|
| `development` | iOS simulator + Android `.apk` | local devices |
| `preview` | iOS device `.ipa` (ad-hoc) + Android `.apk` | TestFlight internal / Play Console internal |
| `production` | iOS `.ipa` (App Store) + Android `.aab` (Play Store) | store submission |

### One-time operator setup

1. **Apple Developer account** ($99/yr). Get the team ID; an App Store
   Connect API key is recommended for unattended `eas submit`.
2. **Google Play Developer account** ($25 once). Create the application
   record; download the service-account JSON for `eas submit`.
3. **EAS account.** `npm install -g eas-cli && eas login`. The
   foundation EAS account owner runs every build.
4. **App identifiers.** `app.json` ships
   `online.hypermesh.app` for both `ios.bundleIdentifier` and
   `android.package`. If you fork, change both before first build —
   they cannot be rotated post-submission without a re-review.

### Cutting a mobile build

```bash
cd mobile

# Sanity
npm install
npx tsc --noEmit

# Internal-test builds for both platforms
eas build --profile preview --platform all

# Store submission builds
eas build --profile production --platform ios
eas build --profile production --platform android

# Submit (requires eas.json `submit.production` filled in)
eas submit --profile production --platform ios --latest
eas submit --profile production --platform android --latest
```

The first invocation of `eas build` for each platform runs `expo
prebuild`, which materialises the `mobile/ios/` and `mobile/android/`
folders. Both directories are gitignored — they are deterministic
output of `app.json` + the Expo CLI.

### What's intentionally not in CI

`release.yml` does **not** include a mobile job. Three reasons:

1. **Signing keys.** The Apple `.p8` API key and Google Play service
   account JSON would have to live in GitHub secrets. The threat model
   for those keys (loss of control of the entire app surface) does not
   match the threat model for foundation CI (loss of one release
   binary's signature).
2. **Cost.** Each EAS build consumes credits. CI on every push would
   burn credits fast.
3. **Cadence.** Mobile releases are slower than daemon releases. Apple
   review takes 24-72h; Play takes a few hours. They are tagged and
   submitted by the operator, not automated.

If a foundation operator wants to tighten this loop later, the path is:
add an opt-in `workflow_dispatch` job that runs `eas build` against an
ephemeral build profile (no signing) — sufficient to catch type errors
in CI without ever touching production credentials.

### Mobile project layout

```
mobile/
├── App.tsx                       # entry, QueryClient + RootNavigator
├── app.json                      # Expo config: ids, plugins, permissions
├── eas.json                      # build/submit profiles
├── package.json                  # @hypermesh/sdk via file:..
├── tsconfig.json                 # strict
├── babel.config.js / metro.config.js / tailwind.config.js / global.css
├── src/api/client.ts             # MobileApiClient — bootstrap, connect, invoke, reauth
├── src/auth/                     # DeviceFingerprint, ChallengeSign, TokenStore
├── src/screens/                  # Connect, Dashboard, Assets, Share, Messages, DNS
├── src/navigation/               # RootNavigator (bottom-tabs + reauth bounce)
└── README.md
```

The mobile project is intentionally *not* a workspace member of
`core/Cargo.toml`. `cargo check --workspace` does not pull in Node.js,
React Native, or Expo. The TS SDK at `sdk/typescript/` is the only
in-repo dependency, consumed via `file:../sdk/typescript` in
`mobile/package.json`.

### What's deferred to C.4.5

- **FALCON-1024 keygen on device** (currently WebCrypto P-256
  placeholder). Requires UniFFI binding under `hypermesh-ffi/` exposing
  FALCON to JNI/Swift.
- **Real platform icons.** `assets/` is gitignored; replace with
  `icon.png` (1024×1024), `splash.png`, `adaptive-icon.png` before
  store submission.
- **Native folders** (`mobile/ios/`, `mobile/android/`). Generated by
  `expo prebuild` on first build. Both are gitignored.

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
Check the build job for that target. As of Phase C.2, `libbpf-sys` is no
longer in the non-Linux dep tree (target-gated), so a `libbpf-sys` link
error on macOS/Windows would now be a regression. The remaining known gap
is `blockmatrix`'s unconditional `libc::mmap` usage in
`assets/proxy/nat_translation/translation.rs` and unimplemented non-Linux
branches in `os_integration/`/`container/`/`metrics/hardware.rs` — those
are tracked under Phase C.2.5. Any other failure is a real bug — file an
issue.
