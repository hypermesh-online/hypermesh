<!-- Copyright (c) 2026 Hypermesh Foundation. All rights reserved. -->

# Joining the HyperMesh Network

This guide walks you through joining the public HyperMesh mesh network. By the end, your node will be connected to `trust.hypermesh.online`, exchanging blocks and participating in the mesh.

**Time**: ~5 minutes
**Requirements**: Linux x86_64, Rust toolchain (or download a release binary)

---

## 1. Get the Binary

### Option A: Download release (recommended)

```bash
# Download the latest static binary (runs on any x86-64 Linux, no dependencies)
curl -LO https://github.com/hypermesh-online/core/releases/latest/download/hypermesh-linux-x86_64
chmod +x hypermesh-linux-x86_64
sudo mv hypermesh-linux-x86_64 /usr/local/bin/hypermesh
```

### Option B: Build from source

```bash
git clone https://github.com/hypermesh-online/core.git
cd core
sudo apt install clang lld pkg-config    # Ubuntu/Debian
cargo build --release -p blockmatrix
# Binary at: target/release/hypermesh
```

For a portable static binary (no glibc dependency):

```bash
rustup target add x86_64-unknown-linux-musl
C_INCLUDE_PATH=/usr/include cargo build --release --target x86_64-unknown-linux-musl -p blockmatrix
# Binary at: target/x86_64-unknown-linux-musl/release/hypermesh
```

---

## 2. Start Your Node

### Join the public network

```bash
hypermesh --privacy public --bootstrap "[2600:1900:4001:cf7::]:9292" connect
```

That's it. Your node will:

1. Create a genesis block (your sovereign blockchain starts immediately)
2. Assess your hardware and register CPU/memory/storage/GPU as blockchain assets
3. Generate a self-signed TLS certificate (no CA dependency needed)
4. Connect to `trust.hypermesh.online` via STOQ (QUIC/IPv6)
5. Complete a bilateral Proof of State handshake with the bootstrap node
6. Start the IPC daemon (other CLI commands talk to the running daemon)
7. Begin accepting connections from other peers

You should see:

```
INFO  No persisted state found, initializing fresh node at (0, 0, 0)
INFO  Created genesis block: <hash>
INFO  Registered hardware assets in block #1
INFO  Connected to [2600:1900:4001:cf7::]:9292 with adaptive optimization
INFO  Successfully connected to node <peer_id> at (0,0,0)
INFO  Node running in Public mode
INFO  IPC server listening
INFO  Starting to accept incoming connections
```

### Disconnect

```bash
hypermesh disconnect
```

This gracefully stops the daemon, cleans up the IPC socket, and saves state.

### Run as a background service

Create a systemd unit (optional):

```bash
sudo tee /etc/systemd/system/hypermesh.service << 'EOF'
[Unit]
Description=HyperMesh Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/hypermesh --privacy public --stoq-port 9292 --bootstrap "[2600:1900:4001:cf7::]:9292" connect --foreground
Restart=on-failure
RestartSec=5
User=nobody
LimitNOFILE=65536
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable --now hypermesh
```

---

## 3. Verify Your Connection

Check your node is connected to the mesh:

```bash
# Check status (talks to daemon via IPC)
hypermesh status

# View logs (if running as a service)
journalctl -u hypermesh -f

# You should see periodic status lines:
#   INFO Connected nodes: 1
#   INFO   - Node db27f030 at (0,0,0)
```

---

## 4. Register a DNS Name

Register a name for your node on the blockchain:

```bash
hypermesh dns register my-node --addr fd00::1
```

This creates a blockchain asset with full Proof of State. The name propagates to connected peers via block sync.

```bash
# List your registered names
hypermesh dns list

# Resolve a name
hypermesh dns resolve my-node
```

---

## 5. Store and Fetch Files

Store a file through the asset pipeline (Compress -> Encrypt -> Shard -> Distribute):

```bash
# Store a file — creates shards and distributes to peers
hypermesh store /path/to/file.txt

# Output: Asset ID and shard map location
# Stored asset <asset_id> (14 shards, shard map: ~/.blockmatrix/shard_maps/<id>.json)
```

Fetch it back:

```bash
hypermesh fetch <asset_id> --output /path/to/recovered.txt
```

The pipeline uses Kyber-1024 quantum-resistant encryption and Reed-Solomon 10+4 erasure coding. You only need 10 of 14 shards to reconstruct.

---

## 6. Register a Domain

Create a domain that maps to a Network-scope blockchain:

```bash
# Register a top-level domain
hypermesh domain register my-org --privacy public

# Create a sub-domain
hypermesh domain create dev.my-org --privacy private

# List your domains
hypermesh domain list

# Invite a peer to join your domain
hypermesh domain invite my-org --peer <node_id>

# Join a domain (with invitation token)
hypermesh join my-org --invite <token>
```

Each domain creates its own Network-scope blockchain. Sub-domains derive from parent chains. Hierarchical DNS resolution walks parent domain chains right-to-left.

---

## 7. Privacy Modes

HyperMesh supports three privacy modes at the transport layer:

| Mode | Flag | Behavior |
|------|------|----------|
| **Private** | `--privacy private` | Localhost only, no network (default) |
| **Anonymous** | `--privacy anonymous` | Ephemeral certs, untracked connections |
| **Public** | `--privacy public` | Full mesh participation, discoverable |

Privacy mode controls transport behavior. Your local blockchain runs regardless of mode.

```bash
# Start in anonymous mode (encrypted but untracked)
hypermesh --privacy anonymous --bootstrap "[2600:1900:4001:cf7::]:9292" connect

# Start in private mode (no network, localhost only)
hypermesh --privacy private connect

# Change privacy mode at runtime
hypermesh set-privacy public
```

---

## 8. Node Configuration

### TOML config file

Initialize a config file:

```bash
hypermesh config init
# Creates ~/.hypermesh/config.toml
```

View and modify configuration:

```bash
# Show full config
hypermesh config show

# Get a specific value
hypermesh config get node.privacy_mode

# Set a value
hypermesh config set node.privacy_mode '"public"'
hypermesh config set node.stoq_port 9300
```

CLI flags always override config file values.

### Matrix coordinates

Position your node in the 3D matrix topology:

```bash
hypermesh --coord-x 10 --coord-y 20 --coord-z 5 --privacy public \
  --bootstrap "[2600:1900:4001:cf7::]:9292" connect
```

Coordinates affect routing — nodes closer in matrix space are preferred for shard distribution and block relay. Use coordinates that reflect your geographic or logical position.

### Custom STOQ port

```bash
hypermesh --stoq-port 9300 --privacy public \
  --bootstrap "[2600:1900:4001:cf7::]:9292" connect
```

### Data directory

```bash
hypermesh --data-dir /var/lib/hypermesh --privacy public \
  --bootstrap "[2600:1900:4001:cf7::]:9292" connect
```

Node state (genesis block, blockchain, certificates, DNS records) persists to this directory. On restart, the node resumes from where it left off.

### Reflector mode

Run as a public relay that accepts and relays connections:

```bash
hypermesh --reflector --privacy public --stoq-port 9292 connect --foreground
```

Reflector nodes join the Network scope blockchain and participate in block sync coordination.

### JSON output

All commands support `--json` for machine-parseable output:

```bash
hypermesh --json status
hypermesh --json dns list
```

---

## 9. Dashboards

Create and deploy scope-aware dashboards:

```bash
# Scaffold a new dashboard project
hypermesh dashboard init my-dashboard

# Deploy a dashboard
hypermesh dashboard deploy ./my-dashboard

# List deployed dashboards
hypermesh dashboard list

# Get dashboard info
hypermesh dashboard info my-dashboard
```

Dashboards serve different content based on scope: `public/` for anonymous visitors, `private/` for authenticated peers, `admin/` for the node owner.

---

## 10. Firewall

HyperMesh uses QUIC (UDP) over IPv6. Allow your STOQ port:

```bash
# If using ufw
sudo ufw allow 9292/udp

# If using iptables
sudo ip6tables -A INPUT -p udp --dport 9292 -j ACCEPT
```

Only expose the STOQ port if you want other nodes to connect to you. Outbound connections work without firewall changes.

---

## CLI Reference

| Command | Description |
|---------|-------------|
| `hypermesh connect` | Connect to the mesh (starts daemon) |
| `hypermesh disconnect` | Disconnect from the mesh (stops daemon) |
| `hypermesh status` | Show node status (blockchain height, connected peers) |
| `hypermesh set-privacy <mode>` | Change privacy mode at runtime |
| `hypermesh store <path>` | Store a file as a distributed, encrypted asset |
| `hypermesh fetch <id> [-o path]` | Fetch and reconstruct an asset |
| `hypermesh dns register <name>` | Register a DNS name on the blockchain |
| `hypermesh dns resolve <name>` | Resolve a blockchain DNS name |
| `hypermesh dns list` | List all registered DNS names |
| `hypermesh domain register <name>` | Register a domain (creates Network blockchain) |
| `hypermesh domain create <name>` | Create a sub-domain |
| `hypermesh domain list` | List registered domains |
| `hypermesh domain nodes <name>` | Show nodes in a domain |
| `hypermesh domain invite <name>` | Generate an invitation token |
| `hypermesh join <network>` | Join a domain network |
| `hypermesh config show` | Show current configuration |
| `hypermesh config get <key>` | Get a config value |
| `hypermesh config set <key> <val>` | Set a config value |
| `hypermesh config init` | Create default config file |
| `hypermesh dashboard init [name]` | Scaffold a dashboard project |
| `hypermesh dashboard deploy <path>` | Deploy a dashboard |
| `hypermesh dashboard list` | List deployed dashboards |
| `hypermesh dashboard info <name>` | Get dashboard details |

### Global options

| Option | Default | Description |
|--------|---------|-------------|
| `--privacy <mode>` | `private` | Privacy mode: private, anonymous, public |
| `--bootstrap <addr>` | none | Bootstrap peer address (IPv6, can repeat) |
| `--stoq-port <port>` | `9292` | STOQ listener port (UDP/QUIC) |
| `--coord-x/y/z <n>` | `0` | Matrix position coordinates |
| `--reflector` | off | Run as a public relay node |
| `--data-dir <path>` | `~/.blockmatrix` | Persistent state directory |
| `--config <path>` | `~/.hypermesh/config.toml` | Custom config file path |
| `--json` | off | Machine-parseable JSON output |
| `--debug` | off | Enable debug-level logging |

---

## How It Works

### Bootstrap sequence

1. **Genesis**: Your node creates its own blockchain with a unique genesis block — no network required
2. **Hardware assessment**: CPU, memory, storage, GPU detected and registered as blockchain assets (block #1)
3. **Self-signed certificate**: Generated locally via TrustChain — no external CA needed
4. **QUIC connection**: Encrypted tunnel to bootstrap peer using self-signed cert
5. **PoS handshake**: Bilateral Proof of State exchange — your node proves its state, the peer proves theirs
6. **IPC daemon**: Unix socket server starts, enabling CLI commands to talk to the running node
7. **Mesh participation**: Blocks sync, shards distribute, DNS propagates

TLS provides encryption. Proof of State provides authentication. No certificate authority, DNS server, or external dependency is required to bootstrap trust.

### What gets persisted

| Data | Location | Survives restart |
|------|----------|-----------------|
| Genesis block | `<data-dir>/node_*/blockchain/` | Yes |
| Blockchain (all blocks) | `<data-dir>/node_*/blockchain/` | Yes |
| Certificate | `<data-dir>/node_*/certificate.json` | Yes |
| DNS records | `<data-dir>/node_*/dns_records.json` | Yes |
| Domain registrations | `<data-dir>/node_*/domains.json` | Yes |
| Shard maps | `<data-dir>/shard_maps/` | Yes |
| Shard data | `<data-dir>/shards/` | Yes |
| Config | `~/.hypermesh/config.toml` | Yes |

### Network architecture

```
Your Node                          trust.hypermesh.online
+------------------+               +------------------+
| Device Blockchain|               | Device Blockchain|
| (sovereign)      |               | (sovereign)      |
|                  |   STOQ/QUIC   |                  |
| STOQ Transport   |<=============>| STOQ Transport   |
|  port 9300       |   IPv6 only   |  port 9292       |
|                  |               |  (reflector)     |
| PoS Handshake    |  bilateral    | PoS Handshake    |
| Block Sync       |<=============>| Block Sync       |
| Shard Transport  |               | Shard Transport  |
| IPC Daemon       |               | IPC Daemon       |
+------------------+               +------------------+
```

Every node maintains its own sovereign blockchain. Network participation is voluntary. Your data stays on your machine unless you explicitly distribute it.

---

## Troubleshooting

### "Only IPv6 addresses supported"

HyperMesh is IPv6-only. Ensure your system has IPv6 connectivity:

```bash
ip -6 addr show scope global
# Should show at least one global IPv6 address
```

If you only have IPv4, you may need to enable IPv6 on your network or use a tunnel.

### Connection timeout to bootstrap node

Check that UDP port 9292 is reachable:

```bash
# Test IPv6 connectivity
ping6 2600:1900:4001:cf7::
```

If behind a firewall, ensure outbound UDP to port 9292 is allowed.

### "Chain integrity violation" on restart

This happens when persisted blockchain data is incompatible with the current binary (e.g., after a breaking update). Clear the data directory:

```bash
rm -rf ~/.blockmatrix/node_*
```

The node will create a fresh genesis block on next start.

### Node connects but shows 0 peers

Your node connected at the QUIC level but the PoS handshake may have failed. Check logs with `--debug`:

```bash
hypermesh --debug --privacy public --bootstrap "[2600:1900:4001:cf7::]:9292" connect
```

---

## Next Steps

- [Setting Up a Private Network](PRIVATE_NETWORK.md) — connect your own devices
- [Running a Private Catalog](PRIVATE_CATALOG.md) — distribute packages within your mesh
- [Exposing Public Endpoints](PUBLIC_ENDPOINTS.md) — make services accessible via gateway
- [Full Installation Guide](INSTALL.md) — deploy all 5 services with systemd
