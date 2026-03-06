// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/node.hpp>

namespace hypermesh {

NodeStatus NodeApi::status() const {
    auto j = http_.get("/api/v1/status");
    return j.get<NodeStatus>();
}

PingResponse NodeApi::ping() const {
    auto j = http_.get("/api/v1/ping");
    return j.get<PingResponse>();
}

} // namespace hypermesh
