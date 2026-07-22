// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/ngauge.hpp>

namespace hypermesh {

NGaugeCapacityMetrics NGaugeApi::capacity() const {
    auto j = http_.get("/api/v1/ngauge/capacity");
    return j.get<NGaugeCapacityMetrics>();
}

NGaugeTrafficMetrics NGaugeApi::traffic() const {
    auto j = http_.get("/api/v1/ngauge/traffic");
    return j.get<NGaugeTrafficMetrics>();
}

NGaugeListingList NGaugeApi::marketplace_listings() const {
    auto j = http_.get("/api/v1/ngauge/marketplace/listings");
    return j.get<NGaugeListingList>();
}

NGaugeNodeMetrics NGaugeApi::node_metrics() const {
    auto j = http_.get("/api/v1/ngauge/node/metrics");
    return j.get<NGaugeNodeMetrics>();
}

NGaugeLeaseList NGaugeApi::leases() const {
    auto j = http_.get("/api/v1/ngauge/leases");
    return j.get<NGaugeLeaseList>();
}

} // namespace hypermesh
