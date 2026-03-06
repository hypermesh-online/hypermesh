// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <string>
#include <hypermesh/http_client.hpp>
#include <hypermesh/error.hpp>
#include <hypermesh/types.hpp>
#include <hypermesh/api/node.hpp>
#include <hypermesh/api/blockchain.hpp>
#include <hypermesh/api/dns.hpp>
#include <hypermesh/api/network.hpp>
#include <hypermesh/api/topology.hpp>
#include <hypermesh/api/asset.hpp>
#include <hypermesh/api/dashboard.hpp>
#include <hypermesh/api/config.hpp>
#include <hypermesh/api/domain.hpp>

namespace hypermesh {

/// Main entry point for the HyperMesh C++ SDK.
///
/// Usage:
///   auto client = hypermesh::HyperMeshClient("http://localhost:9293");
///   auto status = client.node().status();
///   auto height = client.blockchain().height();
class HyperMeshClient {
public:
    /// Construct a client targeting the given base URL.
    /// Default: http://localhost:9293
    explicit HyperMeshClient(
        const std::string& base_url = "http://localhost:9293");

    /// Access the node status API.
    NodeApi node() const;

    /// Access the blockchain API.
    BlockchainApi blockchain() const;

    /// Access the DNS API.
    DnsApi dns() const;

    /// Access the network API.
    NetworkApi network() const;

    /// Access the topology API.
    TopologyApi topology() const;

    /// Access the asset API.
    AssetApi asset() const;

    /// Access the dashboard API.
    DashboardApi dashboard() const;

    /// Access the config API.
    ConfigApi config() const;

    /// Access the domain API.
    DomainApi domain() const;

    /// Get the underlying HTTP client (for advanced use).
    const HttpClient& http() const;

private:
    HttpClient http_;
};

} // namespace hypermesh
