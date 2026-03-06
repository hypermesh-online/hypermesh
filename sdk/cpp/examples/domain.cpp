// Domain, dashboard, config, asset, and topology operations.
//
// Build: g++ -std=c++17 -I../include domain.cpp ../src/*.cpp ../src/api/*.cpp -o domain
// Run:   ./domain

#include <hypermesh/client.hpp>
#include <iostream>

int main() {
    hypermesh::HyperMeshClient client;

    // List registered domains
    auto domains = client.domain().list();
    std::cout << "Domains: " << domains.count << "\n";
    for (const auto& d : domains.domains) {
        std::cout << "  " << d.domain
                  << " (privacy: " << d.privacy
                  << ", owner: " << d.owner << ")\n";
    }

    // Register a new domain
    std::cout << "\nRegistering domain 'testapp' with Private privacy...\n";
    auto reg = client.domain().register_domain("testapp", "Private");
    std::cout << "Registered: " << reg.domain
              << " network_id=" << reg.network_id
              << " status=" << reg.status << "\n";

    // Join a domain
    std::cout << "\nJoining domain 'testapp'...\n";
    auto join = client.domain().join("testapp");
    std::cout << "Join result: " << join.data.dump() << "\n";

    // Dashboard
    auto dash_info = client.dashboard().info();
    std::cout << "\nDashboard info: " << dash_info.data.dump() << "\n";

    auto dashboards = client.dashboard().list();
    std::cout << "Dashboards: " << dashboards.count << "\n";
    for (const auto& d : dashboards.dashboards) {
        std::cout << "  " << d.name << " v" << d.version << "\n";
    }

    // Config
    auto config = client.config().show();
    std::cout << "\nConfig: " << config.data.dump(2) << "\n";

    // Assets
    auto assets = client.asset().list();
    std::cout << "\nAssets: " << assets.count << "\n";
    for (const auto& a : assets.assets) {
        std::cout << "  [" << a.category << "] " << a.content_hash
                  << " (scope: " << a.scope << ")\n";
    }

    // Topology
    auto topo = client.topology().info();
    std::cout << "\nTopology:\n";
    std::cout << "  Node: " << topo.node_id << "\n";
    std::cout << "  Position: ("
              << topo.coordinate.x << ", "
              << topo.coordinate.y << ", "
              << topo.coordinate.z << ")\n";

    auto neighbors = client.topology().neighbors();
    std::cout << "  Neighbors: " << neighbors.count
              << " (radius: " << neighbors.radius << ")\n";

    return 0;
}
