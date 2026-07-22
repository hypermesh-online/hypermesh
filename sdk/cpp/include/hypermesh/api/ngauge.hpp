// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// NGauge operations (capacity, traffic, marketplace, node metrics, leases).
class NGaugeApi {
public:
    explicit NGaugeApi(const HttpClient& http) : http_(http) {}

    /// Get capacity metrics.
    NGaugeCapacityMetrics capacity() const;

    /// Get traffic metrics.
    NGaugeTrafficMetrics traffic() const;

    /// List marketplace listings.
    NGaugeListingList marketplace_listings() const;

    /// Get node metrics.
    NGaugeNodeMetrics node_metrics() const;

    /// List leases.
    NGaugeLeaseList leases() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
