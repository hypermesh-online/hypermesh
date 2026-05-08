# HyperMesh Desktop Shell (Phase C.3)

Tauri 2 native shell for the HyperMesh stack:

- **System tray** with daemon health indicator + start/stop/quit actions
- **First-run wizard** (privacy mode, identity generation, optional trustnet-test, optional foundation grant)
- **Daemon subprocess management** — spawns the bare `hypermesh` binary and watches the IPC socket
- **Embedded WebView** hosting the existing React UI from `../ui/frontend/`

This Cargo project is intentionally separate from the workspace at
`../Cargo.toml`. Daemon CI (`cargo check --workspace …`) does not pull
in webview / GUI sysdeps. The desktop shell has its own CI lane in
`.github/workflows/release.yml`.

---

## Developer prerequisites

The Rust toolchain is the same as the rest of the repo. The webview
side requires platform-specific GUI libraries; this is normal for any
Tauri 2 project:

| Platform | Sysdeps |
|----------|---------|
| Linux (Debian/Ubuntu) | `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libssl-dev`, `pkg-config` |
| Linux (Arch) | `webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator`, `librsvg`, `openssl`, `pkgconf` |
| macOS | Xcode command-line tools (`xcode-select --install`) |
| Windows | Microsoft Edge WebView2 runtime (Win10 1803+ / Win11 ships with it), MSVC toolchain |

JavaScript tooling:

```bash
# install pnpm if you don't have it
corepack enable && corepack prepare pnpm@9 --activate

# install Tauri CLI globally OR per-repo
cargo install tauri-cli --version "^2"
# (alternatively: pnpm add -g @tauri-apps/cli@next)
```

The hypermesh daemon binary is expected to be on `$PATH`. Install via
`scripts/install.sh` (Linux/macOS) or `scripts/install.ps1` (Windows)
from the repo root. To override, set `HYPERMESH_BIN` to an absolute
path before launching the shell.

---

## Local development

```bash
# 1. Install JS dependencies (only first time)
cd ../ui/frontend && pnpm install

# 2. From here, run the dev shell — Vite dev server starts on :5173,
#    Tauri opens a window pointed at it with hot reload.
cd ../../desktop
cargo tauri dev
```

If `cargo tauri` is not on PATH, install it once with
`cargo install tauri-cli --version "^2"`.

---

## Production build

```bash
# Builds the React app to ../ui/frontend/dist, then bundles per-target.
cargo tauri build

# Output bundles land under target/release/bundle/{deb,appimage,dmg,msi,nsis}/
```

Bundle targets are configured in `tauri.conf.json` under `bundle.targets`.

---

## Daemon binary placement

For C.3 alpha, the desktop shell expects `hypermesh` on `$PATH` (this
matches the install scripts shipped in C.1 / C.2). The bundled-sidecar
approach — copying the daemon binary into the app's `Resources/bin/`
directory at bundle time — is tracked as **C.3.5**:

1. The matrix-produced binary from `release.yml`'s `build` job would be
   downloaded as an artifact in the new `desktop` matrix job.
2. It would be placed at `desktop/binaries/hypermesh-{target}` (sidecar
   naming convention).
3. `tauri.conf.json` would gain an `externalBin` entry referencing it.
4. `daemon.rs` would invoke the sidecar via Tauri's shell plugin
   `Command::sidecar("hypermesh")` instead of `Command::new("hypermesh")`.

Until then, the install scripts handle binary placement.

---

## Tauri commands surface

These are the commands the React side calls via `@tauri-apps/api/core`'s
`invoke()`:

| Command | Args | Returns |
|---------|------|---------|
| `daemon_start` | `{ args?: { privacy_mode, network_id?, foreground?, extra_args? } }` | `DaemonStatus` |
| `daemon_stop` | – | `DaemonStatus` |
| `daemon_status` | – | `DaemonStatus` |
| `daemon_check_update` | – | `Object \| null` (raw `system.check_update` IPC response) |
| `wizard_should_show` | – | `bool` |
| `wizard_state` | – | `WizardState` |
| `wizard_set_privacy` | `{ mode }` | `()` |
| `wizard_set_trustnet_test` | `{ optIn }` | `()` |
| `wizard_set_foundation_grant` | `{ requested }` | `()` |
| `wizard_complete` | – | `WizardState` |

Background events emitted from Rust → React:

- `daemon-status` — payload: `DaemonStatus` (fired on state transitions)
- `update-available` — payload: `string \| null` (latest version when available)

---

## Files

```
desktop/
├── Cargo.toml          # Standalone crate — NOT a workspace member
├── build.rs            # tauri_build::build() shim
├── tauri.conf.json     # Bundle config (frontend path, identifiers, bundler targets)
├── capabilities/
│   └── default.json    # Tauri 2 permissions for the main window
├── icons/              # 32/128/256 px + .icns / .ico (placeholders included)
├── src/
│   ├── main.rs         # App entry, command registration, run-loop
│   ├── daemon.rs       # DaemonManager (spawn/stop/status/IPC ping)
│   ├── tray.rs         # System tray menu + event handler
│   └── wizard.rs       # First-run wizard state persistence
└── README.md
```

The companion React-side wizard pages live in
`../ui/frontend/components/wizard/`.

---

## Status

- ✅ Project scaffolding + tauri.conf.json
- ✅ Daemon subprocess manager (Unix; Windows named-pipe is C.3.5)
- ✅ System tray with status indicator + Start/Stop/Update/Quit
- ✅ First-run wizard backend + React page scaffolding
- ✅ Tray ↔ UpdateBanner bridge via `update-available` event
- ⏳ release.yml `desktop` matrix job — see C.3.5
- ⏳ Sidecar daemon binary bundling — see C.3.5
- ⏳ Real PNG/icns/ico icons (placeholders shipped)

---

## Quality gate

```bash
# UI standalone build (Gateway use case unchanged)
cd ../ui/frontend && pnpm build

# Desktop crate compiles (requires platform Tauri sysdeps to be installed;
# without them, `cargo check` fails at the libwebkit2gtk-4.1-dev probe).
cd ../../desktop && cargo check

# Workspace cargo check unaffected (desktop is NOT a workspace member)
cd .. && cargo check --workspace --features caesar,intelligence
```
