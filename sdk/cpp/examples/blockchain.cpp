// Blockchain operations with the HyperMesh C++ SDK.
//
// Build: g++ -std=c++17 -I../include blockchain.cpp ../src/*.cpp ../src/api/*.cpp -o blockchain
// Run:   ./blockchain

#include <hypermesh/client.hpp>
#include <iostream>

int main() {
    hypermesh::HyperMeshClient client;

    // Get chain height
    auto h = client.blockchain().height();
    std::cout << "Blockchain height: " << h.height << "\n";

    // Fetch the genesis block
    auto genesis = client.blockchain().block(0);
    std::cout << "\nGenesis block:\n";
    std::cout << "  Index: " << genesis.index << "\n";
    std::cout << "  Hash: " << genesis.hash << "\n";
    std::cout << "  Previous hash: " << genesis.previous_hash << "\n";
    std::cout << "  Timestamp: " << genesis.timestamp << "\n";

    // Fetch the latest block
    if (h.height > 0) {
        auto latest = client.blockchain().block(h.height - 1);
        std::cout << "\nLatest block (index " << latest.index << "):\n";
        std::cout << "  Hash: " << latest.hash << "\n";
    }

    // Validate the chain
    auto result = client.blockchain().validate();
    std::cout << "\nBlockchain valid: " << (result.valid ? "true" : "false") << "\n";
    std::cout << "Blocks checked: " << result.blocks_checked << "\n";
    if (!result.errors.empty()) {
        std::cout << "Errors:\n";
        for (const auto& err : result.errors) {
            std::cout << "  - " << err << "\n";
        }
    }

    return 0;
}
