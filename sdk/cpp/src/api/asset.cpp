// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/asset.hpp>

namespace hypermesh {

AssetList AssetApi::list() const {
    auto j = http_.get("/api/v1/asset/list");
    return j.get<AssetList>();
}

} // namespace hypermesh
