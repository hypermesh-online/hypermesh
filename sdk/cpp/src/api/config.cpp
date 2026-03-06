// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/config.hpp>

namespace hypermesh {

ConfigValue ConfigApi::show() const {
    auto j = http_.get("/api/v1/config/show");
    return ConfigValue{j};
}

ConfigValue ConfigApi::get(const std::string& key) const {
    auto j = http_.get("/api/v1/config/get/" + key);
    return ConfigValue{j};
}

} // namespace hypermesh
