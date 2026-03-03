<!-- Copyright © 2026 Hypermesh Foundation. All rights reserved. -->

# Setting Up a Private HyperMesh Network

This guide covers connecting multiple devices into a private HyperMesh network for resource sharing without any clearnet dependency.

## Overview

A private network lets you share resources across your own devices — family computers, home servers, company workstations — without relying on external services or the public internet.

Every HyperMesh node runs a **Device blockchain** locally from the moment it boots. This chain operates independently and requires no network connectivity. When nodes join a private network, they synchronize a shared **Network blockchain** on top of their local Device chains.

The privacy mode for private networks is **Private** (bounded, tracked within the group). All participants know each other's identities and contributions, but nothing is visible outside the network.

## Step 1: Install on All Devices

Install HyperMesh on every device that will join the network. Follow the [Installation Guide](INSTALL.md) on each machine.

Minimum device requirements:

- 1 Mb/s network connectivity (between devices)
- 50 GB available storage
- 4 GB RAM
- 2-core CPU at 1 GHz

These are protocol minimums. More resources allow the node to contribute more to the mesh and handle larger assets.

## Step 2: Configure Privacy Mode

On each node, set the privacy mode to `private`. Edit `/etc/hypermesh/hypermesh.toml`:

```toml
[node]
privacy_mode = "private"
```

Or pass it as a flag when starting the service:

```bash
hypermesh --privacy private
```

In private mode, TrustChain operates locally (`local://trustchain`). Each node acts as its own certificate authority — no external CA is contacted. Certificates are generated using FALCON-1024 post-quantum signatures.

## Step 3: Network Discovery

### Local network (same subnet)

Nodes discover each other automatically via IPv6 multicast on the local network. No configuration is needed. When a new node comes online, it announces itself and existing nodes respond with their identities and blockchain state.

### Remote devices (across subnets or VPN)

For devices that are not on the same local network, add peer addresses manually in `/etc/hypermesh/hypermesh.toml`:

```toml
[network]
peers = [
    "[fd12:3456:789a::1]:9292",
    "[fd12:3456:789a::2]:9292",
]
```

Use IPv6 addresses. If your devices connect over a VPN, use the VPN-assigned IPv6 addresses.

### Mixed environments

You can combine both methods. Local devices discover each other automatically while remote devices are configured as static peers. The mesh treats all peers equally regardless of how they were discovered.

## Step 4: Verify the Mesh

Once all devices are running and configured, verify the mesh is healthy.

### Check connected peers

```bash
hypermesh status
```

This shows:

- Your node's identity and matrix position
- Connected peers and their status
- Device blockchain height (local)
- Network blockchain sync status (shared)

### Check blockchain synchronization

```bash
hypermesh status --blockchain
```

Verify that:

- The Device chain is running (it always should be)
- The Network chain shows the correct peer count
- Block heights are synchronized across peers

### Check from another node

Run `hypermesh status` on a different device in the network. The peer lists should be consistent and blockchain heights should match (within a few blocks during active sync).

## Certificate Management

In private mode, certificate management is automatic:

1. **Node startup**: TrustChain generates a self-signed FALCON-1024 certificate for the node
2. **First connection**: When two nodes connect, they exchange certificates over STOQ
3. **Trust establishment**: Each node stores the peer's certificate locally
4. **Ongoing**: Certificates are rotated automatically. Re-keying happens without interrupting the mesh.

There is no central certificate authority in a private network. Each node trusts the certificates it has received directly from peers. This is the P2P trust model (`local://trustchain`).

### Inspecting certificates

```bash
hypermesh certs list
```

This shows all trusted peer certificates, their expiration dates, and the FALCON-1024 public key fingerprints.

### Revoking a peer

If a device is lost or compromised:

```bash
hypermesh certs revoke <peer-node-id>
```

This removes the peer's certificate from your trust store and broadcasts the revocation to other nodes on the Network blockchain. The revoked node will be disconnected from the mesh.

## No Clearnet Required

A private HyperMesh network operates entirely without clearnet access:

- **No DNS**: Nodes address each other by IPv6 directly
- **No external CA**: TrustChain runs locally on each node
- **No gateway registration**: The public gateway at `trust.hypermesh.online` is not contacted
- **No public blockchain**: The Network blockchain is shared only among your private peers
- **No CAESAR rewards**: Private networks do not participate in the public reward system

If you later want to bridge your private network to another network or the public mesh, see the [Private Network Integration](PRIVATE_INTEGRATION.md) guide.

## Troubleshooting

### Nodes cannot discover each other on the local network

- Verify IPv6 is enabled on all interfaces
- Check that multicast is not blocked by your firewall
- Ensure port 9292/udp is open between devices

### Remote peers fail to connect

- Verify the peer address and port are correct
- Confirm STOQ port 9292 is reachable from the remote device
- Check that both nodes are running with `--privacy private`

### Blockchain sync stalls

- Run `hypermesh status --blockchain` to check block heights
- Restart the lagging node: `sudo systemctl restart hypermesh`
- Check logs for errors: `journalctl -u hypermesh --no-pager -n 50`

## Next Steps

- [Running a Private Catalog](PRIVATE_CATALOG.md) — distribute packages within your private network
- [Private Network Integration](PRIVATE_INTEGRATION.md) — bridge data between two private networks
