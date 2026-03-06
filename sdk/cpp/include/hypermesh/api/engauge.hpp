// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Engauge operations (capacity, traffic, marketplace, node metrics, leases).
class EngaugeApi {
public:
    explicit EngaugeApi(const HttpClient& http) : http_(http) {}

    /// Get capacity metrics.
    EngaugeCapacityMetrics capacity() const;

    /// Get traffic metrics.
    EngaugeTrafficMetrics traffic() const;

    /// List marketplace listings.
    EngaugeListingList marketplace_listings() const;

    /// Get node metrics.
    EngaugeNodeMetrics node_metrics() const;

    /// List leases.
    EngaugeLeaseList leases() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
