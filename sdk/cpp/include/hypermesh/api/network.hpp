// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Network peer management operations.
class NetworkApi {
public:
    explicit NetworkApi(const HttpClient& http) : http_(http) {}

    /// List all connected peers.
    PeerList peers() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
