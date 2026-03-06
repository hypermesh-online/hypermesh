// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <string>
#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Configuration query operations.
class ConfigApi {
public:
    explicit ConfigApi(const HttpClient& http) : http_(http) {}

    /// Show the full configuration.
    ConfigValue show() const;

    /// Get a specific configuration key.
    ConfigValue get(const std::string& key) const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
