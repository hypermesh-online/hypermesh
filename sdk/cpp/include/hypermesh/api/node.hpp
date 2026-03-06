// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Node status and health operations.
class NodeApi {
public:
    explicit NodeApi(const HttpClient& http) : http_(http) {}

    /// Get the current node status.
    NodeStatus status() const;

    /// Ping the node.
    PingResponse ping() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
