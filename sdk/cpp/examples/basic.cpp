// Basic usage of the HyperMesh C++ SDK.
//
// Build: g++ -std=c++17 -I../include basic.cpp ../src/client.cpp ../src/http_client.cpp
//        ../src/api/node.cpp -o basic -lnlohmann_json
// Run:   ./basic

#include <hypermesh/client.hpp>
#include <hypermesh/error.hpp>
#include <iostream>

int main() {
    hypermesh::HyperMeshClient client; // http://localhost:9293

    // Ping the node
    auto pong = client.node().ping();
    std::cout << "Ping: " << (pong.pong ? "true" : "false") << "\n";

    // Get node status
    auto status = client.node().status();
    std::cout << "Node ID: " << status.node_id << "\n";
    std::cout << "Chain height: " << status.chain_height << "\n";
    std::cout << "Peers: " << status.peers << "\n";
    std::cout << "Privacy mode: " << status.privacy_mode << "\n";
    std::cout << "Uptime: " << status.uptime_secs << " seconds\n";
    std::cout << "Coordinate: ("
              << status.coordinate.x << ", "
              << status.coordinate.y << ", "
              << status.coordinate.z << ")\n";

    // List connected peers
    auto peers = client.network().peers();
    std::cout << "\nConnected peers: " << peers.count << "\n";

    // Error handling
    try {
        client.blockchain().block(999999);
    } catch (const hypermesh::HyperMeshError& e) {
        std::cout << "\nExpected error for block 999999:\n";
        std::cout << "  Status: " << e.status_code << "\n";
        std::cout << "  Message: " << e.what() << "\n";
    }

    return 0;
}
