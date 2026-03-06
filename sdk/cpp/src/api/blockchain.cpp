// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/blockchain.hpp>

namespace hypermesh {

BlockchainHeight BlockchainApi::height() const {
    auto j = http_.get("/api/v1/blockchain/height");
    return j.get<BlockchainHeight>();
}

Block BlockchainApi::block(uint64_t index) const {
    auto j = http_.get("/api/v1/blockchain/block/" + std::to_string(index));
    return j.get<Block>();
}

ValidationResult BlockchainApi::validate() const {
    auto j = http_.get("/api/v1/blockchain/validate");
    return j.get<ValidationResult>();
}

} // namespace hypermesh
