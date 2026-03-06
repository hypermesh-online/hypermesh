// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/network.hpp>

namespace hypermesh {

PeerList NetworkApi::peers() const {
    auto j = http_.get("/api/v1/network/peers");
    return j.get<PeerList>();
}

} // namespace hypermesh
