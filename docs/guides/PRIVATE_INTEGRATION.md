<!-- Copyright © 2026 Hypermesh Foundation. All rights reserved. -->

# Private Network Integration (Federation)

This guide covers bridging two private HyperMesh networks so they can share specific data without exposing their full networks to each other.

## Overview

Federation lets two independent private networks exchange selected assets through a controlled bridge. Each network retains full sovereignty — only explicitly shared assets cross the boundary, and both sides validate every transfer independently.

Real-world examples:

- A bank shares quarterly audit reports with a regulator
- A supplier shares inventory data with a manufacturer
- Two departments in an organization exchange approved documents
- A research lab shares published datasets with a partner institution

Federation does not merge networks. Each network keeps its own blockchain, its own certificate authority, and its own privacy boundary. The bridge is a narrow, policy-controlled channel between two gateway nodes.

## How Federation Works

Federation operates through four components:

1. **Gateway nodes**: Each network designates a node as its federation gateway
2. **STOQ connection**: Gateways establish a STOQ link with a defined trust level
3. **Sharing policies**: Rules that control which assets can cross the bridge and in which direction
4. **Dual Proof of State**: Every transfer requires valid PoS proof in both the source and destination chains

```
Network A                              Network B
+-----------+                          +-----------+
| Node A1   |                          | Node B1   |
| Node A2   |                          | Node B2   |
| Gateway A |------- STOQ bridge ------| Gateway B |
| Node A3   |  (Conditional trust)     | Node B3   |
+-----------+                          +-----------+
```

Only Gateway A and Gateway B communicate directly. Internal nodes never see the other network.

## Trust Levels

Federation connections are configured with one of three trust levels:

### Full Trust

Complete data sharing between networks. Both gateways can access any asset on the other network without policy restrictions.

Use this only for tightly coupled networks where full transparency is acceptable (e.g., two departments within the same organization that share everything).

```toml
[[federation.peers]]
address = "[fd98:7654:321b::1]:8444"
trust_level = "full"
```

### Conditional Trust (Recommended)

Policy-gated sharing. Only assets matching explicitly defined criteria cross the bridge. This is the most common and recommended configuration.

```toml
[[federation.peers]]
address = "[fd98:7654:321b::1]:8444"
trust_level = "conditional"

[federation.peers.policy]
allowed_asset_tags = ["shared", "audit-report"]
allowed_asset_types = ["Library"]
direction = "bidirectional"
max_transfer_size_bytes = 104857600
```

### Untrusted

No data sharing. Gateways exchange only routing information for mesh connectivity purposes (e.g., matrix topology hints). No assets cross the bridge.

```toml
[[federation.peers]]
address = "[fd98:7654:321b::1]:8444"
trust_level = "untrusted"
```

This level is useful when two networks want to be aware of each other for routing optimization without sharing any actual data.

## Setup Steps

### Step 1: Designate gateway nodes

On each network, choose which node will serve as the federation gateway. This node must:

- Be running the `gateway` service
- Have STOQ port 8444 reachable from the other network's gateway
- Have a valid TrustChain certificate

Any node can be a gateway, but it is common to use a dedicated machine at the network edge.

### Step 2: Exchange gateway certificates

Each gateway needs the other gateway's TrustChain certificate to authenticate the STOQ connection.

On Network A's gateway:

```bash
hypermesh certs export --format pem > gateway-a.pem
```

On Network B's gateway:

```bash
hypermesh certs export --format pem > gateway-b.pem
```

Transfer these certificates through a trusted channel (in-person USB exchange, encrypted email, existing secure connection). Then import:

On Network A's gateway:

```bash
hypermesh certs import gateway-b.pem --trust-level conditional
```

On Network B's gateway:

```bash
hypermesh certs import gateway-a.pem --trust-level conditional
```

### Step 3: Configure federation peers

On Network A's gateway, edit `/etc/hypermesh/gateway.toml`:

```toml
[gateway]
mode = "inter_network"

[[federation.peers]]
name = "Network B"
address = "[fd98:7654:321b::1]:8444"
trust_level = "conditional"

[federation.peers.policy]
allowed_asset_tags = ["shared"]
direction = "bidirectional"
max_transfer_size_bytes = 104857600
require_pos_validation = true
```

On Network B's gateway, add the reciprocal configuration pointing to Network A's gateway address.

### Step 4: Define sharing policies

Policies control exactly what crosses the bridge. Configure them on both sides.

#### By asset tag

```toml
[federation.peers.policy]
allowed_asset_tags = ["audit-report", "quarterly-data"]
```

Only assets explicitly tagged with these labels can be shared.

#### By asset type

```toml
[federation.peers.policy]
allowed_asset_types = ["Library"]
```

Only assets of the specified types can cross the bridge.

#### By direction

```toml
[federation.peers.policy]
direction = "outbound"  # This network sends, does not receive
```

Options: `bidirectional`, `outbound` (send only), `inbound` (receive only).

#### Combined policies

All policy fields are AND conditions. An asset must match all specified criteria to cross the bridge.

### Step 5: Test with a sample transfer

On Network A, publish an asset and tag it for sharing:

```bash
hypermesh catalog publish ./test-package/
hypermesh catalog tag test-package --add shared
```

On Network B, verify the asset is discoverable:

```bash
hypermesh catalog search --federation "test-package"
```

Then install it:

```bash
hypermesh catalog install --federation test-package
```

The federation flag tells the Catalog to query federated peers in addition to the local network.

## Security Guarantees

### Transport encryption

All federation traffic uses STOQ with FALCON-1024 post-quantum signatures. The gateway-to-gateway connection is encrypted end-to-end.

### Asset encryption

Assets transferred across the federation bridge are encrypted with Kyber-1024 quantum-resistant encryption. The receiving gateway decrypts only after validating Proof of State.

### Dual Proof of State

Every cross-network transfer requires valid PoS proof in both chains:

1. **Source chain validation**: The asset must have valid PoSpace (storage), PoStake (ownership), PoWork (computation), and PoTime (temporal ordering) proofs in the source network's blockchain
2. **Destination chain registration**: The receiving gateway creates a new PoS proof set and registers the asset in the destination network's blockchain
3. **Both proofs recorded**: The transfer is recorded on both chains, creating a bilateral audit trail

### Audit trail

Every cross-network transfer is recorded in both Network blockchains. The source chain records the outbound transfer (asset hash, destination, timestamp). The destination chain records the inbound transfer (asset hash, source, validation result, timestamp).

This creates a bilateral, immutable audit trail. Both networks can independently verify the history of any shared asset.

### No side channels

Federation provides no implicit access. Specifically:

- Only explicitly tagged/typed assets are visible to the other network
- Internal network topology is not exposed (only the gateway address)
- Node identities within each network are not disclosed
- Blockchain state beyond the shared assets is not accessible
- The gateway does not cache or store data from the other network beyond active transfers

## Real-World Example: Bank and Regulator

A bank needs to share quarterly audit reports with a financial regulator (e.g., the IRS).

### Bank's network setup

```toml
# /etc/hypermesh/gateway.toml on the bank's gateway
[[federation.peers]]
name = "IRS Audit Network"
address = "[fd00:irs:audit::1]:8444"
trust_level = "conditional"

[federation.peers.policy]
allowed_asset_tags = ["audit-report"]
allowed_asset_types = ["Library"]
direction = "outbound"
require_pos_validation = true
```

The bank publishes audit reports to their private Catalog:

```bash
hypermesh catalog publish ./q4-2025-audit/
hypermesh catalog tag q4-2025-audit --add audit-report
```

### Regulator's network setup

```toml
# /etc/hypermesh/gateway.toml on the IRS gateway
[[federation.peers]]
name = "Bank Audit Feed"
address = "[fd00:bank:gw::1]:8444"
trust_level = "conditional"

[federation.peers.policy]
allowed_asset_tags = ["audit-report"]
direction = "inbound"
require_pos_validation = true
```

The IRS gateway automatically discovers new assets tagged `audit-report` from the bank's gateway, validates the PoS proof, and ingests the report into the IRS network's blockchain.

### What stays private

- The bank's internal transactions, customer data, and other assets are invisible to the IRS
- The IRS's internal systems, investigation data, and other network activity are invisible to the bank
- Only assets tagged `audit-report` cross the bridge
- The bridge is outbound-only from the bank's perspective — the IRS cannot push data to the bank

## Monitoring Federation

### Check federation status

```bash
hypermesh federation status
```

Shows:

- Connected federation peers
- Trust levels
- Active sharing policies
- Recent transfers (count, direction, status)
- Connection health

### View transfer history

```bash
hypermesh federation transfers --peer "Network B"
```

### Check for policy violations

```bash
hypermesh federation audit --peer "Network B"
```

Lists any transfer attempts that were blocked by policy, helping you identify misconfigured tags or overly restrictive policies.

## Troubleshooting

### Gateways fail to connect

- Verify both gateways can reach each other on port 8444 (STOQ)
- Confirm certificates were imported correctly: `hypermesh certs list`
- Check that both gateways have `mode = "inter_network"` in their config

### Asset not visible to federated peer

- Verify the asset is tagged with a tag that matches the policy: `hypermesh catalog info <package>`
- Confirm the policy direction allows the transfer (outbound from source, inbound at destination)
- Check that both gateways have matching `allowed_asset_tags` in their policies

### Transfer fails with "PoS validation error"

- The asset's PoS proof may have expired. Re-publish or update the asset.
- Ensure both networks' blockchains are healthy: `hypermesh status --blockchain`
- Verify TrustChain is running on both gateways: `sudo systemctl status trustchain`

### Transfer succeeds but asset integrity check fails

- The content hash recorded in the source blockchain must match the reconstructed asset on the destination
- This typically indicates network corruption during transfer. Retry the transfer.
- If persistent, check shard integrity: `hypermesh catalog verify <package>`

## Next Steps

- [Setting Up a Private HyperMesh Network](PRIVATE_NETWORK.md) — prerequisite for federation
- [Running a Private Catalog](PRIVATE_CATALOG.md) — publish assets for federation sharing
- [Exposing Services via Public Gateway](PUBLIC_ENDPOINTS.md) — alternative to federation for public access
