// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Asset management operations.
class AssetApi {
public:
    explicit AssetApi(const HttpClient& http) : http_(http) {}

    /// List all registered assets.
    AssetList list() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
