// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <cstdint>
#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Blockchain query operations.
class BlockchainApi {
public:
    explicit BlockchainApi(const HttpClient& http) : http_(http) {}

    /// Get the current chain height.
    BlockchainHeight height() const;

    /// Get a block by index.
    Block block(uint64_t index) const;

    /// Validate the local blockchain integrity.
    ValidationResult validate() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
