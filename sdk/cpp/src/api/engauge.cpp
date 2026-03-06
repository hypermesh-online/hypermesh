// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/engauge.hpp>

namespace hypermesh {

EngaugeCapacityMetrics EngaugeApi::capacity() const {
    auto j = http_.get("/api/v1/engauge/capacity");
    return j.get<EngaugeCapacityMetrics>();
}

EngaugeTrafficMetrics EngaugeApi::traffic() const {
    auto j = http_.get("/api/v1/engauge/traffic");
    return j.get<EngaugeTrafficMetrics>();
}

EngaugeListingList EngaugeApi::marketplace_listings() const {
    auto j = http_.get("/api/v1/engauge/marketplace/listings");
    return j.get<EngaugeListingList>();
}

EngaugeNodeMetrics EngaugeApi::node_metrics() const {
    auto j = http_.get("/api/v1/engauge/node/metrics");
    return j.get<EngaugeNodeMetrics>();
}

EngaugeLeaseList EngaugeApi::leases() const {
    auto j = http_.get("/api/v1/engauge/leases");
    return j.get<EngaugeLeaseList>();
}

} // namespace hypermesh
