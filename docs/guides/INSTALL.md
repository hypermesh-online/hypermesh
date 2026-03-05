<!-- Copyright © 2026 Hypermesh Foundation. All rights reserved. -->

# Installing HyperMesh

This guide covers installing and running a HyperMesh node on Linux.

## Prerequisites

- **OS**: Linux x86\_64 with systemd
- **Kernel**: 5.15 or later (required for eBPF/XDP support)
- **Network ports**:
  - `8443` — HTTP/3 gateway (public-facing)
  - `8444` — TrustChain STOQ (internal)
  - `9292` — BlockMatrix STOQ
  - `9294` — Caesar STOQ
  - `9295` — Catalog STOQ
  - `9296` — Engauge STOQ

Verify your kernel version:

```bash
uname -r
# Must be >= 5.15
```

## Quick Install

Run the one-line installer:

```bash
curl -sSf https://raw.githubusercontent.com/hypermesh-online/core/main/scripts/install.sh | bash
```

This downloads the latest release, installs binaries, creates configuration files, and sets up systemd units. Skip to [Verifying the Installation](#verifying-the-installation) once it completes.

## Manual Install

### 1. Download the release

Download the latest tarball from GitHub Releases:

```bash
VERSION="0.1.0"
curl -LO "https://github.com/hypermesh-online/core/releases/download/v${VERSION}/hypermesh-v${VERSION}-linux-x86_64.tar.gz"
```

### 2. Extract and install binaries

```bash
tar xzf hypermesh-v${VERSION}-linux-x86_64.tar.gz
sudo cp hypermesh-v${VERSION}/bin/* /usr/local/bin/
```

The tarball contains five binaries:

| Binary | Purpose |
|--------|---------|
| `gateway` | HTTP/3 gateway — bridges clearnet to internal STOQ services |
| `trustchain_ca` | Certificate authority — issues FALCON-1024 certificates |
| `hypermesh` | BlockMatrix node — runs the blockchain, manages assets |
| `catalog-server` | Asset registry — package distribution over STOQ |
| `engauge-server` | Analytics — engagement metrics and routing intelligence |

### 3. Install systemd units

```bash
sudo cp hypermesh-v${VERSION}/systemd/*.service /etc/systemd/system/
sudo systemctl daemon-reload
```

### 4. Create configuration directory

```bash
sudo mkdir -p /etc/hypermesh
sudo cp hypermesh-v${VERSION}/config/*.toml /etc/hypermesh/
```

## Configuration

Configuration files live in `/etc/hypermesh/`:

| File | Purpose |
|------|---------|
| `gateway.toml` | Gateway ports, TLS certificate source, rate limits |
| `trustchain.toml` | CA settings, certificate rotation, OCSP/CRL endpoints |
| `hypermesh.toml` | BlockMatrix node identity, matrix position, privacy mode |
| `catalog.toml` | Catalog bind address, cache size, storage backend |
| `engauge.toml` | Metrics collection, privacy filters, streaming config |

At minimum, review `gateway.toml` to confirm port bindings and `trustchain.toml` to set your certificate storage path.

## Starting Services

Start services in dependency order. TrustChain must come first (other services need certificates), and the gateway must come last (it routes to the other services):

```bash
sudo systemctl enable --now trustchain
sudo systemctl enable --now hypermesh
sudo systemctl enable --now catalog
sudo systemctl enable --now engauge
sudo systemctl enable --now gateway
```

Or as a single command:

```bash
sudo systemctl enable --now trustchain hypermesh catalog engauge gateway
```

Systemd respects the ordering constraints defined in the unit files, but enabling them in this order ensures correct startup sequencing on first boot.

## Verifying the Installation

### Check service status

```bash
sudo systemctl status trustchain hypermesh catalog engauge gateway
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
| 9296 | STOQ | Engauge | Internal — analytics and metrics |

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
