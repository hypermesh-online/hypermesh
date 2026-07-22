// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/client.hpp>

namespace hypermesh {

HyperMeshClient::HyperMeshClient(
    const std::string& base_url,
    const std::string& caesar_url,
    const std::string& trustchain_url,
    const std::string& ngauge_url,
    const std::string& catalog_url)
    : http_(base_url),
      caesar_http_(caesar_url),
      trustchain_http_(trustchain_url),
      ngauge_http_(ngauge_url),
      catalog_http_(catalog_url) {}

NodeApi HyperMeshClient::node() const {
    return NodeApi(http_);
}

BlockchainApi HyperMeshClient::blockchain() const {
    return BlockchainApi(http_);
}

DnsApi HyperMeshClient::dns() const {
    return DnsApi(http_);
}

NetworkApi HyperMeshClient::network() const {
    return NetworkApi(http_);
}

TopologyApi HyperMeshClient::topology() const {
    return TopologyApi(http_);
}

AssetApi HyperMeshClient::asset() const {
    return AssetApi(http_);
}

DashboardApi HyperMeshClient::dashboard() const {
    return DashboardApi(http_);
}

ConfigApi HyperMeshClient::config() const {
    return ConfigApi(http_);
}

DomainApi HyperMeshClient::domain() const {
    return DomainApi(http_);
}

CaesarApi HyperMeshClient::caesar() const {
    return CaesarApi(caesar_http_);
}

TrustChainApi HyperMeshClient::trustchain() const {
    return TrustChainApi(trustchain_http_);
}

NGaugeApi HyperMeshClient::ngauge() const {
    return NGaugeApi(ngauge_http_);
}

CatalogApi HyperMeshClient::catalog() const {
    return CatalogApi(catalog_http_);
}

const HttpClient& HyperMeshClient::http() const {
    return http_;
}

} // namespace hypermesh
