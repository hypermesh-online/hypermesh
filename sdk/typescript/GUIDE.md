# HyperMesh TypeScript SDK Guide

## Installation

```bash
npm install @hypermesh/sdk
```

Requires Node.js 18+ (uses native `fetch`).

## Quick Start

```typescript
import { HyperMeshClient } from "@hypermesh/sdk";

const client = new HyperMeshClient(); // defaults to https://localhost:8443
const status = await client.node.status();
console.log(status.node_id, status.chain_height);
```

Custom endpoint:

```typescript
const client = new HyperMeshClient("https://192.168.1.50:8443");
```

## API Reference

### Node

```typescript
// Get full node status
const status = await client.node.status();
// Returns: NodeStatus { chain_height, coordinate, node_id, peers, privacy_mode, uptime_secs }

// Ping the node
const pong = await client.node.ping();
// Returns: PingResponse { pong: true }
```

### Blockchain

```typescript
// Get current chain height
const h = await client.blockchain.height();
// Returns: BlockchainHeight { height }

// Get a specific block by index
const block = await client.blockchain.block(0);
// Returns: Block { index, timestamp, hash, previous_hash, ... }

// Validate blockchain integrity
const result = await client.blockchain.validate();
// Returns: BlockchainValidation { valid, errors?, blocks_checked? }
```

### DNS

```typescript
// List all DNS records
const dns = await client.dns.list();
// Returns: DnsListResponse { count, records: DnsRecord[] }

// Resolve a name
const record = await client.dns.resolve("trust.hypermesh");
// Returns: DnsResolveResponse { name, address }

// Register a new DNS record
const resp = await client.dns.register("mynode.hypermesh", "::1");
// Returns: DnsRegisterResponse
```

### Assets

```typescript
// List all registered assets
const assets = await client.asset.list();
// Returns: AssetListResponse { count, assets: Asset[] }
// Asset: { block_index, category, content_hash, scope }
```

### Domain

```typescript
// List registered domains
const domains = await client.domain.list();
// Returns: DomainListResponse { count, domains: Domain[] }
// Domain: { domain, network_id, owner, privacy }

// Register a new domain (creates a network-scope blockchain)
const reg = await client.domain.register("myapp", "Private");
// Returns: DomainRegisterResponse { domain, network_id, privacy, owner, block, status }

// Join an existing domain
const join = await client.domain.join("myapp");
// With invitation token:
const joinPrivate = await client.domain.join("myapp", "invitation-token-here");
```

### Dashboard

```typescript
// List available dashboards
const dashboards = await client.dashboard.list();
// Returns: DashboardListResponse { count, dashboards: DashboardEntry[] }
// DashboardEntry: { block, description, domain, hash, name, registered_at, version }

// Get dashboard system info
const info = await client.dashboard.info();
// Returns: DashboardInfo
```

### Config

```typescript
// Show full node configuration
const config = await client.config.show();
// Returns: ConfigShowResponse (map of all config keys/values)

// Get a specific config value
const val = await client.config.get("privacy_mode");
// Returns: ConfigGetResponse
```

### Network

```typescript
// List connected peers
const peers = await client.network.peers();
// Returns: PeersResponse { count, peers: Peer[] }
```

### Topology

```typescript
// Get this node's position in the Block-MATRIX
const topo = await client.topology.info();
// Returns: TopologyInfo { coordinate: { x, y, z }, node_id }

// List matrix neighbors
const neighbors = await client.topology.neighbors();
// Returns: TopologyNeighbors { center, count, neighbors: Neighbor[], radius }
```

## Error Handling

All methods throw `HyperMeshError` on failure:

```typescript
import { HyperMeshClient, HyperMeshError } from "@hypermesh/sdk";

const client = new HyperMeshClient();

try {
  const block = await client.blockchain.block(999999);
} catch (err) {
  if (err instanceof HyperMeshError) {
    console.error("Status:", err.status);   // HTTP status code (0 for connection errors)
    console.error("Body:", err.body);       // Raw response body
    console.error("Message:", err.message); // Human-readable message
  }
}
```

## Types

All response types are exported from the package:

```typescript
import type {
  NodeStatus,
  Block,
  BlockchainHeight,
  BlockchainValidation,
  DnsRecord,
  DnsListResponse,
  DnsResolveResponse,
  Asset,
  AssetListResponse,
  Domain,
  DomainListResponse,
  DomainRegisterResponse,
  DashboardEntry,
  DashboardListResponse,
  DashboardInfo,
  ConfigShowResponse,
  ConfigGetResponse,
  PeersResponse,
  TopologyInfo,
  TopologyNeighbors,
  Coordinate,
} from "@hypermesh/sdk";
```
