<!-- Copyright © 2026 Hypermesh Foundation. All rights reserved. -->

# Installing HyperMesh

The bounded packaging path below uses `scripts/install/hypermesh-install.sh`.
It requires Linux and GNU utilities, including `mv -T`, `stat -c`, `mktemp`,
and `sha256sum`; POSIX shell syntax is not a cross-platform support claim.
It installs one local artifact without starting it. The platform, service and
network reference sections later in this guide are not validated by that path;
Linux/systemd/eBPF prerequisites there are not prerequisites for this offline
packaging fixture. Startup and platform support remain unverified here.

## Prerequisites

- **OS**: Linux x86\_64 with systemd
- **Kernel**: 5.15 or later (required for eBPF/XDP support)
- **Network ports**:
  - `8443` — HTTP/3 gateway (public-facing)
  - `8444` — TrustChain STOQ (internal)
  - `9292` — BlockMatrix STOQ
  - `9294` — Caesar STOQ
  - `9295` — Catalog STOQ
  - `9296` — NGauge STOQ

Verify your kernel version:

```bash
uname -r
# Must be >= 5.15
```

## Private Offline Install

The installer defaults to **private** privacy. `public` is selected only through
an explicit `HYPERMESH_PRIVACY=public`; offline installation rejects public mode.
The supported offline packaging combination requires all of these explicitly:
`HYPERMESH_OFFLINE=1`, `HYPERMESH_NO_START=1`, a readable prebuilt artifact,
its expected SHA-256, and absolute install/data directories. Invalid flags,
privacy, or incomplete/conflicting inputs fail before preparation.
Each offline root is checked independently for an absolute path; colons in
otherwise valid absolute roots are allowed, not treated as separators.

Example from the repository root, using caller-owned directories (replace the
artifact path and set `EXPECTED_SHA256` to the expected 64-hex digest):

```sh
: "${EXPECTED_SHA256:?set the expected 64-hex SHA-256 before installing}"
umask 077
mkdir -p "$HOME/.local/bin" "$HOME/.local/share/hypermesh"
HYPERMESH_PREBUILT_BINARY=/absolute/path/to/hypermesh \
HYPERMESH_PREBUILT_SHA256="$EXPECTED_SHA256" \
HYPERMESH_INSTALL_DIR="$HOME/.local/bin" \
HYPERMESH_DATA_DIR="$HOME/.local/share/hypermesh" \
HYPERMESH_PRIVACY=private \
HYPERMESH_OFFLINE=1 \
HYPERMESH_NO_START=1 \
sh scripts/install/hypermesh-install.sh
```

| Environment variable | Implemented behavior |
|----------------------|----------------------|
| `HYPERMESH_PREBUILT_BINARY` | Local readable artifact; mandatory offline. Without it, the non-offline path builds with Cargo. |
| `HYPERMESH_PREBUILT_SHA256` | Mandatory 64-hex expected SHA-256 for **every** prebuilt install, including non-offline installs; rejected without an artifact. |
| `HYPERMESH_INSTALL_DIR` | Destination for `hypermesh`; default `/usr/local/bin`, but must be explicit and absolute offline. |
| `HYPERMESH_DATA_DIR` | Caller-owned state directory; default `$HOME/.blockmatrix`, but must be explicit and absolute offline. |
| `HYPERMESH_PRIVACY` | `private` by default; accepts `private`, `public`, or `anonymous`; offline requires `private`. |
| `HYPERMESH_OFFLINE` | `0` by default; accepts only `0` or `1`. `1` requires the complete offline combination above. |
| `HYPERMESH_NO_START` | `0` by default; accepts only `0` or `1`. `1` never executes the artifact, including connect/status/version. |
| `HYPERMESH_TARGET` | Non-offline build target; defaults to `x86_64-unknown-linux-musl`. |
| `HYPERMESH_STOQ_PORT`, `HYPERMESH_NETWORK_ID` | Startup parameters only; defaults `9292` and `trustnet`. |

Offline mode never builds, downloads, or executes the artifact; it does not
invoke Cargo, rustup, sudo, services, or key generation. The installer prepares
the data directory without rewriting existing identity, assets, or configuration.
It copies into private collision-safe staging on the destination filesystem,
checks the **staged bytes**, sets executable mode, then atomically renames them.
The hash is read from staged content through standard input, so backslashes
or newlines in directory names do not change the parsed digest.
Both commit and rollback use GNU `mv -fT --` (no-target-directory semantics);
unsupported utilities fail closed, with no fallback to plain `mv`. A directory
substituted at the destination is not treated as a container for the artifact
or deleted to force restoration. If it blocks restoration, the old binary's
backup is retained and its location reported for recovery.
Handled staging failures/interruption preserve the previous installed binary;
handled postreplacement failures attempt to restore it. Failed rollback is an
error with the backup location retained. SIGKILL, power loss, persistent
filesystem errors, hostile parent-directory rewrites, and an all-powerful
same-UID actor are not recovery guarantees. The deterministic destination-
directory substitution fixture verifies refusal without deleting unrelated
directory contents or node state; it does not establish general race immunity.

An expected SHA-256 verifies byte integrity, **not** authenticated release
provenance, post-quantum certificates, or Proof of State. Hermetic mock-artifact
packaging tests are not evidence of a real node, a private-network join, or
platform support. Genuine private-node end-to-end testing and authority/crypto
adapter validation remain separate gates; no one-click authentication, DNS, or
IPv6 runtime operation is established here.

## Release Contents and Other Install Paths

The current `.github/workflows/release.yml` packaging steps (lines 237–256)
bundle **one `hypermesh` artifact** (`hypermesh.exe` on Windows); the Unix step
also attempts to copy `scripts/install.sh` as install metadata. They do not
supply the previously assumed five-service `bin/`, `systemd/`, or `config/`
layout. Do not copy nonexistent binaries, units, or configuration from that
release archive. The separate `scripts/install.sh` quick-install/startup path
is not validated by the offline packaging tests above.

The component names below are reference roles, **not** a list of delivered
release binaries:

| Binary | Purpose |
|--------|---------|
| `gateway` | HTTP/3 gateway — bridges clearnet to internal STOQ services |
| `trustchain_ca` | Certificate authority — issues FALCON-1024 certificates |
| `hypermesh` | BlockMatrix node — runs the blockchain, manages assets |
| `catalog-server` | Asset registry — package distribution over STOQ |
| `ngauge-server` | Analytics — engagement metrics and routing intelligence |

The following multi-service configuration/startup/network sections apply only
where those components, configuration and units have been supplied separately.
They are reference deployment guidance, not outcomes of the single-artifact
release or the private offline installer, and were not exercised here.

## Configuration

Configuration files live in `/etc/hypermesh/`:

| File | Purpose |
|------|---------|
| `gateway.toml` | Gateway ports, TLS certificate source, rate limits |
| `trustchain.toml` | CA settings, certificate rotation, OCSP/CRL endpoints |
| `hypermesh.toml` | BlockMatrix node identity, matrix position, privacy mode |
| `catalog.toml` | Catalog bind address, cache size, storage backend |
| `ngauge.toml` | Metrics collection, privacy filters, streaming config |

At minimum, review `gateway.toml` to confirm port bindings and `trustchain.toml` to set your certificate storage path.

## Starting Services

Start services in dependency order. TrustChain must come first (other services need certificates), and the gateway must come last (it routes to the other services):

```bash
sudo systemctl enable --now trustchain
sudo systemctl enable --now hypermesh
sudo systemctl enable --now catalog
sudo systemctl enable --now ngauge
sudo systemctl enable --now gateway
```

Or as a single command:

```bash
sudo systemctl enable --now trustchain hypermesh catalog ngauge gateway
```

Systemd respects the ordering constraints defined in the unit files, but enabling them in this order ensures correct startup sequencing on first boot.

## Verifying the Installation

### Check service status

```bash
sudo systemctl status trustchain hypermesh catalog ngauge gateway
```

All five services should show `active (running)`.

### Check the gateway health endpoint

```bash
curl -k https://localhost:8443/health
```

The `-k` flag is required during alpha because the gateway uses self-signed TLS certificates. A healthy response confirms the gateway is running and can reach internal services.

### Check individual service logs

```bash
journalctl -u hypermesh --no-pager -n 20
journalctl -u trustchain --no-pager -n 20
```

## Port Reference

| Port | Protocol | Service | Exposure |
|------|----------|---------|----------|
| 8443 | HTTP/3 (QUIC) | Gateway | Public — clearnet entry point |
| 8444 | STOQ | TrustChain | Internal — certificate operations |
| 9292 | STOQ | BlockMatrix | Internal — blockchain and asset operations |
| 9294 | STOQ | Caesar | Internal — EVP packet routing |
| 9295 | STOQ | Catalog | Internal — package registry |
| 9296 | STOQ | NGauge | Internal — analytics and metrics |

All internal ports use STOQ (QUIC-based transport with eBPF acceleration). Only port 8443 should be exposed to the public internet. Internal ports communicate over IPv6.

## Firewall Configuration

Allow only the gateway port from the public internet. Internal ports should be restricted to your mesh:

```bash
# Allow public gateway access
sudo ufw allow 8443/udp

# Allow internal STOQ between mesh nodes (adjust source range)
sudo ufw allow from fd00::/8 to any port 8444:9296 proto udp
```

## Next Steps

- [Joining the Network (Quick Start)](ONBOARDING.md) — join the public mesh in one command
- [Setting Up a Private HyperMesh Network](PRIVATE_NETWORK.md) — connect multiple devices
- [Exposing Services via Public Gateway](PUBLIC_ENDPOINTS.md) — make your node publicly accessible
- [Running a Private Catalog](PRIVATE_CATALOG.md) — distribute packages within your network
