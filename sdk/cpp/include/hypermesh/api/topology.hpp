// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Block-MATRIX topology operations.
class TopologyApi {
public:
    explicit TopologyApi(const HttpClient& http) : http_(http) {}

    /// Get this node's topology info (position, ID).
    TopologyInfo info() const;

    /// Get neighbors of this node.
    NeighborList neighbors() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
