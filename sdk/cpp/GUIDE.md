# HyperMesh C++ SDK Guide

## Requirements

- C++17 or later
- [nlohmann/json](https://github.com/nlohmann/json) (header-only)
- POSIX sockets (Linux/macOS)

## Building

Using CMake:

```bash
mkdir build && cd build
cmake ..
make
```

Or add to your CMakeLists.txt:

```cmake
add_subdirectory(path/to/hypermesh-sdk)
target_link_libraries(your_target PRIVATE hypermesh_sdk)
```

## Quick Start

```cpp
#include <hypermesh/client.hpp>
#include <iostream>

int main() {
    hypermesh::HyperMeshClient client; // defaults to http://localhost:9293
    auto status = client.node().status();
    std::cout << status.node_id << " height=" << status.chain_height << "\n";
    return 0;
}
```

Custom endpoint:

```cpp
hypermesh::HyperMeshClient client("http://192.168.1.50:9293");
```

## API Reference

All methods are synchronous and return value types. On failure, they throw
`hypermesh::HyperMeshError`.

### Node

```cpp
// Get full node status
auto status = client.node().status();
// Fields: chain_height, coordinate, node_id, peers, privacy_mode, uptime_secs

// Ping the node
auto pong = client.node().ping();
// Fields: pong (bool)
```

### Blockchain

```cpp
// Get current chain height
auto h = client.blockchain().height();
// Fields: height (uint64_t)

// Get a specific block by index
auto block = client.blockchain().block(0);
// Fields: index, timestamp, hash, previous_hash, extra (nlohmann::json)

// Validate blockchain integrity
auto result = client.blockchain().validate();
// Fields: valid, errors (vector<string>), blocks_checked
```

### DNS

```cpp
// List all DNS records
auto dns = client.dns().list();
// Fields: count, records (vector<DnsRecord>)
// DnsRecord: name, address

// Resolve a name
auto record = client.dns().resolve("trust.hypermesh");
// Fields: name, address

// Register a new DNS record
auto resp = client.dns().register_record("mynode.hypermesh", "::1");
// Fields: data (nlohmann::json)
```

Note: The method is `register_record` (not `register`) because `register` is a
C++ reserved keyword.

### Assets

```cpp
// List all registered assets
auto assets = client.asset().list();
// Fields: count, assets (vector<Asset>)
// Asset: block_index, category, content_hash, scope
```

### Domain

```cpp
// List registered domains
auto domains = client.domain().list();
// Fields: count, domains (vector<Domain>)
// Domain: domain, network_id, owner, privacy

// Register a new domain
auto reg = client.domain().register_domain("myapp", "Private");
// Fields: domain, network_id, privacy, owner, block, status

// Join an existing domain
auto join = client.domain().join("myapp");
// With invitation token:
auto join2 = client.domain().join("myapp", "token-here");
// Fields: data (nlohmann::json)
```

Note: The method is `register_domain` (not `register`) because `register` is a
C++ reserved keyword.

### Dashboard

```cpp
// List available dashboards
auto dashboards = client.dashboard().list();
// Fields: count, dashboards (vector<DashboardEntry>)
// DashboardEntry: block, description, domain, hash, name, registered_at, version

// Get dashboard info
auto info = client.dashboard().info();
// Fields: data (nlohmann::json)
```

### Config

```cpp
// Show full node configuration
auto config = client.config().show();
// Fields: data (nlohmann::json)

// Get a specific config value
auto val = client.config().get("privacy_mode");
// Fields: data (nlohmann::json)
```

### Network

```cpp
// List connected peers
auto peers = client.network().peers();
// Fields: count, peers (vector<Peer>)
```

### Topology

```cpp
// Get this node's topology position
auto topo = client.topology().info();
// Fields: coordinate (Coordinate{x,y,z}), node_id

// List matrix neighbors
auto neighbors = client.topology().neighbors();
// Fields: center, count, neighbors (vector<Neighbor>), radius
```

## Error Handling

All methods throw `hypermesh::HyperMeshError` on failure:

```cpp
#include <hypermesh/client.hpp>
#include <hypermesh/error.hpp>
#include <iostream>

int main() {
    hypermesh::HyperMeshClient client;

    try {
        auto block = client.blockchain().block(999999);
    } catch (const hypermesh::HyperMeshError& e) {
        std::cerr << "Error: " << e.what() << "\n";
        std::cerr << "Status: " << e.status_code << "\n";
        std::cerr << "Body: " << e.response_body << "\n";
    }

    return 0;
}
```

`HyperMeshError` extends `std::runtime_error` and provides:
- `what()` -- human-readable message
- `status_code` (`int`) -- HTTP status code (0 for connection errors)
- `response_body` (`std::string`) -- raw response body

## Types

All types use nlohmann/json for (de)serialization and are defined in
`<hypermesh/types.hpp>`. Key types:

| Type | Fields |
|------|--------|
| `Coordinate` | `x`, `y`, `z` (double) |
| `NodeStatus` | `chain_height`, `coordinate`, `node_id`, `peers`, `privacy_mode`, `uptime_secs` |
| `Block` | `index`, `timestamp`, `hash`, `previous_hash`, `extra` |
| `DnsRecord` | `name`, `address` |
| `Asset` | `block_index`, `category`, `content_hash`, `scope` |
| `Domain` | `domain`, `network_id`, `owner`, `privacy` |
| `DashboardEntry` | `block`, `description`, `domain`, `hash`, `name`, `registered_at`, `version` |
