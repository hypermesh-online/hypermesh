# HyperMesh Python SDK Guide

## Installation

```bash
pip install hypermesh-sdk
```

The SDK uses only `urllib` from the standard library (zero dependencies).
For async support, install with httpx:

```bash
pip install hypermesh-sdk[async]
```

## Quick Start

```python
from hypermesh import HyperMeshClient

client = HyperMeshClient()  # defaults to http://localhost:9293
status = client.node.status()
print(status.node_id, status.chain_height)
```

Custom endpoint:

```python
client = HyperMeshClient("http://192.168.1.50:9293")
```

Async mode (requires httpx):

```python
from hypermesh import HyperMeshClient

client = HyperMeshClient(async_mode=True)
status = await client.node.status()
await client.close()
```

## API Reference

### Node

```python
# Get full node status
status = client.node.status()
# Returns: NodeStatus(chain_height, coordinate, node_id, peers, privacy_mode, uptime_secs)

# Ping the node
alive = client.node.ping()
# Returns: bool
```

### Blockchain

```python
# Get current chain height
h = client.blockchain.height()
# Returns: BlockchainHeight(height)

# Get a specific block by index
block = client.blockchain.block(0)
# Returns: Block(index, timestamp, data, hash, previous_hash)

# Validate blockchain integrity
result = client.blockchain.validate()
# Returns: ValidationResult(valid, errors)
```

### DNS

```python
# List all DNS records
dns = client.dns.list()
# Returns: DnsList(count, records: list[DnsRecord])
# DnsRecord(name, address)

# Resolve a name
record = client.dns.resolve("trust.hypermesh")
# Returns: DnsRecord(name, address)

# Register a new DNS record
resp = client.dns.register("mynode.hypermesh", "::1")
# Returns: dict
```

### Assets

```python
# List all registered assets
assets = client.asset.list()
# Returns: AssetList(count, assets: list[Asset])
# Asset(asset_id, asset_type, state, metadata)
```

### Domain

```python
# List registered domains
domains = client.domain.list()
# Returns: DomainList(count, domains: list[Domain])
# Domain(name, privacy, owner)

# Register a new domain
resp = client.domain.register("myapp", "Private")
# Returns: dict

# Join an existing domain
resp = client.domain.join("myapp")
# With invitation token:
resp = client.domain.join("myapp", token="invitation-token-here")
```

### Dashboard

```python
# List available dashboards
dashboards = client.dashboard.list()
# Returns: DashboardList(count, dashboards: list[Dashboard])
# Dashboard(name, scope, url)

# Get dashboard system info
info = client.dashboard.info()
# Returns: DashboardInfo(name, version, scope, metadata)
```

### Config

```python
# Show full node configuration
config = client.config.show()
# Returns: dict

# Get a specific config value
val = client.config.get("privacy_mode")
# Returns: any
```

### Network

```python
# List connected peers
peers = client.network.peers()
# Returns: PeerList(count, peers: list[Peer])
# Peer(node_id, address, connected_at)
```

### Topology

```python
# Get this node's position in the Block-MATRIX
topo = client.topology.info()
# Returns: TopologyInfo(coordinate, node_id)

# List matrix neighbors
neighbors = client.topology.neighbors()
# Returns: Neighbors(center, count, neighbors: list[Neighbor], radius)
# Neighbor(node_id, coordinate, distance)
```

## Error Handling

The SDK defines a hierarchy of exceptions:

```python
from hypermesh import HyperMeshClient, HyperMeshError, ConnectionError, NotFoundError, ApiError

client = HyperMeshClient()

try:
    block = client.blockchain.block(999999)
except NotFoundError:
    print("Block not found")
except ConnectionError as e:
    print(f"Cannot connect: {e}")
except ApiError as e:
    print(f"API error (HTTP {e.status_code}): {e}")
except HyperMeshError as e:
    print(f"SDK error: {e}")
```

Exception hierarchy:
- `HyperMeshError` -- base class (has optional `status_code`)
  - `ConnectionError` -- cannot reach the node
  - `NotFoundError` -- HTTP 404
  - `ApiError` -- other HTTP error responses

## Configuration

```python
client = HyperMeshClient(
    base_url="http://localhost:9293",  # Node API address
    async_mode=False,                   # Set True for httpx async
    timeout=30.0,                       # Request timeout in seconds
)
```
