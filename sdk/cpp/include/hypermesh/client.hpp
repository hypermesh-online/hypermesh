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
#include <hypermesh/api/caesar.hpp>
#include <hypermesh/api/trustchain.hpp>
#include <hypermesh/api/ngauge.hpp>
#include <hypermesh/api/catalog.hpp>

namespace hypermesh {

/// Main entry point for the HyperMesh C++ SDK.
///
/// Usage:
///   auto client = hypermesh::HyperMeshClient("https://localhost:8443");
///   auto status = client.node().status();
///   auto height = client.blockchain().height();
///   auto wallet = client.caesar().wallet();
class HyperMeshClient {
public:
    /// Construct a client targeting the given base URL.
    /// Default: https://localhost:8443 (Gateway)
    explicit HyperMeshClient(
        const std::string& base_url = "https://localhost:8443",
        const std::string& caesar_url = "https://localhost:8443",
        const std::string& trustchain_url = "https://localhost:8443",
        const std::string& ngauge_url = "https://localhost:8443",
        const std::string& catalog_url = "https://localhost:8443");

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

    /// Access the Caesar EVP API.
    CaesarApi caesar() const;

    /// Access the TrustChain API.
    TrustChainApi trustchain() const;

    /// Access the NGauge API.
    NGaugeApi ngauge() const;

    /// Access the Catalog API.
    CatalogApi catalog() const;

    /// Get the underlying HTTP client (for advanced use).
    const HttpClient& http() const;

private:
    HttpClient http_;
    HttpClient caesar_http_;
    HttpClient trustchain_http_;
    HttpClient ngauge_http_;
    HttpClient catalog_http_;
};

} // namespace hypermesh
