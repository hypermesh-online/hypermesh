// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Dashboard query operations.
class DashboardApi {
public:
    explicit DashboardApi(const HttpClient& http) : http_(http) {}

    /// List all registered dashboards.
    DashboardList list() const;

    /// Get dashboard info.
    DashboardInfo info() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
