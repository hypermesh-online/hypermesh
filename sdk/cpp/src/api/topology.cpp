// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/topology.hpp>

namespace hypermesh {

TopologyInfo TopologyApi::info() const {
    auto j = http_.get("/api/v1/topology/info");
    return j.get<TopologyInfo>();
}

NeighborList TopologyApi::neighbors() const {
    auto j = http_.get("/api/v1/topology/neighbors");
    return j.get<NeighborList>();
}

} // namespace hypermesh
