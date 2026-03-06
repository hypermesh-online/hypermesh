// DNS operations with the HyperMesh C++ SDK.
//
// Build: g++ -std=c++17 -I../include dns.cpp ../src/*.cpp ../src/api/*.cpp -o dns
// Run:   ./dns

#include <hypermesh/client.hpp>
#include <hypermesh/error.hpp>
#include <iostream>

int main() {
    hypermesh::HyperMeshClient client;

    // List existing DNS records
    auto dns = client.dns().list();
    std::cout << "DNS records: " << dns.count << "\n";
    for (const auto& record : dns.records) {
        std::cout << "  " << record.name << " -> " << record.address << "\n";
    }

    // Register a new record
    std::cout << "\nRegistering example.hypermesh -> ::1\n";
    auto reg = client.dns().register_record("example.hypermesh", "::1");
    std::cout << "Register result: " << reg.data.dump() << "\n";

    // Resolve the record
    try {
        auto resolved = client.dns().resolve("example.hypermesh");
        std::cout << "Resolved: " << resolved.name << " -> " << resolved.address << "\n";
    } catch (const hypermesh::HyperMeshError& e) {
        std::cout << "Resolve failed: " << e.what() << "\n";
    }

    // List records after registration
    auto updated = client.dns().list();
    std::cout << "\nDNS records after registration: " << updated.count << "\n";
    for (const auto& record : updated.records) {
        std::cout << "  " << record.name << " -> " << record.address << "\n";
    }

    return 0;
}
