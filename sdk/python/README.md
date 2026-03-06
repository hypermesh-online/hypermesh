# HyperMesh Python SDK

Python client for the HyperMesh node HTTP REST API.

## Install

```bash
pip install hypermesh-sdk            # sync only (zero dependencies)
pip install hypermesh-sdk[async]     # adds httpx for async support
```

## Quick start

```python
from hypermesh import HyperMeshClient

client = HyperMeshClient("http://localhost:9293")

# Node
status = client.node.status()
print(status.node_id, status.chain_height)
client.node.ping()  # True

# Blockchain
height = client.blockchain.height()
block = client.blockchain.block(0)
result = client.blockchain.validate()

# DNS
records = client.dns.list()
record = client.dns.resolve("my-service")
client.dns.register("my-service", "fd00::1")

# Network
peers = client.network.peers()

# Topology
info = client.topology.info()
neighbors = client.topology.neighbors()

# Assets
assets = client.asset.list()

# Dashboards
dashboards = client.dashboard.list()
info = client.dashboard.info()

# Config
config = client.config.show()
value = client.config.get("privacy_mode")

# Domains
domains = client.domain.list()
client.domain.register("my-domain", "Public")
client.domain.join("other-domain", token="invite-token")
```

## Async usage

```python
import asyncio
from hypermesh import HyperMeshClient

async def main():
    client = HyperMeshClient("http://localhost:9293", async_mode=True)
    status = await client.node.status()
    print(status.node_id)
    await client.close()

asyncio.run(main())
```

## Error handling

```python
from hypermesh import HyperMeshClient, ConnectionError, NotFoundError, ApiError

client = HyperMeshClient()

try:
    block = client.blockchain.block(9999)
except NotFoundError:
    print("Block not found")
except ConnectionError:
    print("Cannot reach node")
except ApiError as e:
    print(f"API error {e.status_code}: {e}")
```

## Requirements

- Python 3.10+
- No dependencies for sync mode (uses `urllib.request`)
- Optional `httpx>=0.25` for async mode
