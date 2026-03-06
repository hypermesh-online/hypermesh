# @hypermesh/sdk

TypeScript SDK for the HyperMesh node HTTP REST API.

## Installation

```bash
npm install @hypermesh/sdk
```

## Usage

```typescript
import { HyperMeshClient } from "@hypermesh/sdk";

const client = new HyperMeshClient("http://localhost:9293");

// Node
const status = await client.node.status();
const pong = await client.node.ping();

// Blockchain
const { height } = await client.blockchain.height();
const block = await client.blockchain.block(0);
const validation = await client.blockchain.validate();

// DNS
const records = await client.dns.list();
const resolved = await client.dns.resolve("my-service");
await client.dns.register("my-service", "fd00::1");

// Network
const peers = await client.network.peers();

// Topology
const info = await client.topology.info();
const neighbors = await client.topology.neighbors();

// Assets
const assets = await client.asset.list();

// Dashboards
const dashboards = await client.dashboard.list();
const dashboardInfo = await client.dashboard.info();

// Config
const config = await client.config.show();
const value = await client.config.get("privacy_mode");

// Domains
const domains = await client.domain.list();
await client.domain.register("my-domain", "Public");
await client.domain.join("other-domain", "optional-token");
```

## Error Handling

```typescript
import { HyperMeshClient, HyperMeshError } from "@hypermesh/sdk";

const client = new HyperMeshClient();

try {
  await client.node.status();
} catch (err) {
  if (err instanceof HyperMeshError) {
    console.error(`HTTP ${err.status}: ${err.message}`);
    console.error(`Response body: ${err.body}`);
  }
}
```

## Requirements

- Node.js 18+ (uses native `fetch`)
- Zero runtime dependencies
- Works in browser and Node.js
