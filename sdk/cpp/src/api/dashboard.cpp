// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/dashboard.hpp>

namespace hypermesh {

DashboardList DashboardApi::list() const {
    auto j = http_.get("/api/v1/dashboard/list");
    return j.get<DashboardList>();
}

DashboardInfo DashboardApi::info() const {
    auto j = http_.get("/api/v1/dashboard/info");
    return j.get<DashboardInfo>();
}

} // namespace hypermesh
