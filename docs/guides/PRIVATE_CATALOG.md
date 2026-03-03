<!-- Copyright © 2026 Hypermesh Foundation. All rights reserved. -->

# Running a Private Catalog

This guide covers running a Catalog server within a private HyperMesh network to distribute asset packages (software, configurations, templates) exclusively among your nodes.

## Overview

The Catalog is HyperMesh's asset package manager. It stores, versions, and distributes packages over STOQ. In a private network, the Catalog operates in isolation — packages never leave your network and are never registered with the public HyperMesh catalog.

A private Catalog is useful for:

- Distributing internal tools and scripts across your devices
- Sharing configuration templates within a team
- Versioning and tracking internal asset definitions
- Maintaining a private registry of validated packages

## Default Behavior

The Catalog server binds to the IPv6 loopback address by default:

```
[::1]:9295
```

This means:

- The local node can access the Catalog directly
- Private network peers can access it over STOQ (port 9295)
- No clearnet clients can reach it

If you installed HyperMesh following the [Installation Guide](INSTALL.md), the Catalog is already running. Verify with:

```bash
sudo systemctl status catalog
```

## Configuration

The Catalog configuration lives at `/etc/hypermesh/catalog.toml`:

```toml
[server]
bind = "[::1]:9295"

[storage]
path = "/var/lib/hypermesh/catalog"

[cache]
# Maximum cache size in bytes (default: 256MB for private networks)
max_size = 268435456

[network]
privacy_mode = "private"
```

The `privacy_mode = "private"` setting ensures the Catalog only communicates with peers in your private network.

## Publishing Packages

Packages are published to the Catalog via STOQ. Each published package is registered as a blockchain asset with full Proof of State validation.

### Create a package manifest

Create a `package.toml` in your package directory:

```toml
[package]
name = "my-internal-tool"
version = "1.0.0"
description = "Internal monitoring script"
asset_type = "Library"

[dependencies]
# Other catalog packages this depends on

[resources]
# HyperMesh resource requirements for execution
cpu_cores = 1
memory_mb = 256
```

### Publish the package

```bash
hypermesh catalog publish ./my-internal-tool/
```

This:

1. Validates the package manifest and contents
2. Compresses the package (Brotli)
3. Encrypts the compressed blob (Kyber-1024)
4. Shards the encrypted data (Reed-Solomon erasure coding)
5. Registers the package as a blockchain asset with PoS proof
6. Distributes shards to available nodes in the private network

### List published packages

```bash
hypermesh catalog list
```

### Get package details

```bash
hypermesh catalog info my-internal-tool
```

This shows the package metadata, version history, shard distribution, and PoS validation status.

## Consuming Packages

Other nodes on the private network can query and download packages.

### Search for packages

```bash
hypermesh catalog search "monitoring"
```

### Install a package

```bash
hypermesh catalog install my-internal-tool
```

The consuming node:

1. Queries the Catalog for the package's shard map (a small instruction set, typically under 1 KB)
2. Fetches shards from nodes that hold them
3. Reconstructs the encrypted blob from shards
4. Decrypts and decompresses the package
5. Validates the content hash against the blockchain record

This is instruction-based retrieval — the node receives a small set of instructions telling it where to find shards, then pulls them from multiple peers in the mesh.

### Pin a version

```bash
hypermesh catalog install my-internal-tool@1.0.0
```

## No Clearnet Access

Private Catalog packages are completely isolated:

- Packages are sharded and stored only on nodes within your private network
- The shard map is recorded only on your private Network blockchain
- No queries are made to the public catalog at `catalog.hypermesh.online`
- No package metadata leaves your network

There is no automatic synchronization between private and public catalogs. A package published to a private Catalog does not exist from the perspective of the public mesh.

## Federation

If you need to share specific packages with another private network, configure federation between the two networks. This does not open your full catalog — only explicitly shared packages cross the bridge.

### Configure federation trust

In `/etc/hypermesh/gateway.toml` on your gateway node:

```toml
[[federation.peers]]
address = "[fd98:7654:321b::1]:8444"
trust_level = "conditional"

[federation.peers.policy]
# Only share packages tagged with "shared"
allowed_asset_tags = ["shared"]
direction = "bidirectional"
```

### Tag a package for sharing

```bash
hypermesh catalog tag my-internal-tool --add shared
```

Once tagged and the federation is active, the other network's gateway can pull the package. Both sides validate Proof of State before completing the transfer.

For full details on federation setup, see the [Private Network Integration](PRIVATE_INTEGRATION.md) guide.

## Storage Management

### Check storage usage

```bash
hypermesh catalog storage
```

### Clean up old package versions

```bash
hypermesh catalog prune --keep-versions 3
```

This removes shard data for all but the 3 most recent versions of each package. The blockchain records remain (they are immutable), but the actual shard data is garbage-collected.

## Troubleshooting

### Package publish fails with "PoS validation error"

- Ensure your node's Device blockchain is running: `hypermesh status`
- Check that TrustChain is active: `sudo systemctl status trustchain`
- Verify the node has valid certificates: `hypermesh certs list`

### Peer cannot find a published package

- Confirm both nodes are on the same private network: `hypermesh status`
- Check that the Network blockchain is synchronized on both nodes
- Verify the Catalog service is running on the publishing node

### Shard reconstruction fails

- At least `k` of `n` shards must be available (default: 10 of 14)
- Check how many nodes are online: `hypermesh status`
- If nodes have gone offline, wait for them to rejoin or re-publish the package

## Next Steps

- [Setting Up a Private HyperMesh Network](PRIVATE_NETWORK.md) — prerequisite for private Catalog
- [Private Network Integration](PRIVATE_INTEGRATION.md) — share packages across private networks via federation
