# HyperMesh C++ SDK

C++17 SDK for the HyperMesh node HTTP REST API.

## Dependencies

- C++17 compiler
- CMake 3.14+
- nlohmann/json (fetched automatically via CMake FetchContent)

No external HTTP library required -- uses POSIX sockets directly.

## Build

```bash
mkdir build && cd build
cmake ..
cmake --build .
```

## Usage

```cpp
#include <hypermesh/client.hpp>
#include <iostream>

int main() {
    try {
        auto client = hypermesh::HyperMeshClient("http://localhost:9293");

        // Node
        auto status = client.node().status();
        std::cout << "Node: " << status.node_id << " height=" << status.chain_height << "\n";

        auto ping = client.node().ping();
        std::cout << "Pong: " << ping.pong << "\n";

        // Blockchain
        auto height = client.blockchain().height();
        std::cout << "Height: " << height.height << "\n";

        auto block = client.blockchain().block(0);
        std::cout << "Genesis hash: " << block.hash << "\n";

        auto validation = client.blockchain().validate();
        std::cout << "Valid: " << validation.valid << "\n";

        // DNS
        auto records = client.dns().list();
        std::cout << "DNS records: " << records.count << "\n";

        auto resolved = client.dns().resolve("my-service");
        std::cout << "Resolved: " << resolved.address << "\n";

        client.dns().register_record("my-service", "fd00::1");

        // Network
        auto peers = client.network().peers();
        std::cout << "Peers: " << peers.count << "\n";

        // Topology
        auto topo = client.topology().info();
        std::cout << "Position: (" << topo.coordinate.x << ", "
                  << topo.coordinate.y << ", " << topo.coordinate.z << ")\n";

        auto neighbors = client.topology().neighbors();
        std::cout << "Neighbors: " << neighbors.count << "\n";

        // Assets
        auto assets = client.asset().list();
        std::cout << "Assets: " << assets.count << "\n";

        // Dashboard
        auto dashboards = client.dashboard().list();
        std::cout << "Dashboards: " << dashboards.count << "\n";

        auto dash_info = client.dashboard().info();

        // Config
        auto config = client.config().show();
        auto value = client.config().get("privacy_mode");

        // Domain
        auto domains = client.domain().list();
        std::cout << "Domains: " << domains.count << "\n";

        auto reg = client.domain().register_domain("my-domain", "Public");
        std::cout << "Registered: " << reg.domain << "\n";

        client.domain().join("other-domain", "invite-token");

    } catch (const hypermesh::HyperMeshError& e) {
        std::cerr << "Error: " << e.what()
                  << " (status=" << e.status_code << ")\n";
        return 1;
    }
}
```

## API Reference

| Method | Endpoint | Return Type |
|--------|----------|-------------|
| `node().status()` | `GET /api/v1/status` | `NodeStatus` |
| `node().ping()` | `GET /api/v1/ping` | `PingResponse` |
| `blockchain().height()` | `GET /api/v1/blockchain/height` | `BlockchainHeight` |
| `blockchain().block(index)` | `GET /api/v1/blockchain/block/:index` | `Block` |
| `blockchain().validate()` | `GET /api/v1/blockchain/validate` | `ValidationResult` |
| `dns().list()` | `GET /api/v1/dns/list` | `DnsList` |
| `dns().resolve(name)` | `GET /api/v1/dns/resolve/:name` | `DnsRecord` |
| `dns().register_record(name, addr)` | `POST /api/v1/dns/register` | `DnsRegisterResponse` |
| `network().peers()` | `GET /api/v1/network/peers` | `PeerList` |
| `topology().info()` | `GET /api/v1/topology/info` | `TopologyInfo` |
| `topology().neighbors()` | `GET /api/v1/topology/neighbors` | `NeighborList` |
| `asset().list()` | `GET /api/v1/asset/list` | `AssetList` |
| `dashboard().list()` | `GET /api/v1/dashboard/list` | `DashboardList` |
| `dashboard().info()` | `GET /api/v1/dashboard/info` | `DashboardInfo` |
| `config().show()` | `GET /api/v1/config/show` | `ConfigValue` |
| `config().get(key)` | `GET /api/v1/config/get/:key` | `ConfigValue` |
| `domain().list()` | `GET /api/v1/domain/list` | `DomainList` |
| `domain().register_domain(name, privacy)` | `POST /api/v1/domain/register` | `DomainRegisterResponse` |
| `domain().join(name, token?)` | `POST /api/v1/domain/join` | `DomainJoinResponse` |

## Error Handling

All methods throw `hypermesh::HyperMeshError` (inherits `std::runtime_error`) on failure.
The exception includes `status_code` (HTTP status, 0 for connection errors) and `response_body`.
