// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/ffi_client.hpp>
#include <utility>

namespace hypermesh {

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

void FfiClient::require_connected() const {
    if (client_ == nullptr) {
        throw FfiError("FfiClient is not connected");
    }
}

std::string FfiClient::take_result(char* result) {
    if (result == nullptr) {
        const char* err = hypermesh_last_error(client_);
        std::string msg = err ? err : "unknown FFI error";
        throw FfiError(msg);
    }
    std::string value(result);
    hypermesh_free_string(result);
    return value;
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

FfiClient::FfiClient(const std::string& socket_path) {
    const char* path = socket_path.empty() ? nullptr : socket_path.c_str();
    client_ = hypermesh_connect(path);
    if (client_ == nullptr) {
        throw FfiError("failed to connect to HyperMesh daemon");
    }
}

FfiClient::~FfiClient() {
    if (client_ != nullptr) {
        hypermesh_disconnect(client_);
        client_ = nullptr;
    }
}

FfiClient::FfiClient(FfiClient&& other) noexcept
    : client_(other.client_) {
    other.client_ = nullptr;
}

FfiClient& FfiClient::operator=(FfiClient&& other) noexcept {
    if (this != &other) {
        if (client_ != nullptr) {
            hypermesh_disconnect(client_);
        }
        client_ = other.client_;
        other.client_ = nullptr;
    }
    return *this;
}

void FfiClient::disconnect() {
    if (client_ != nullptr) {
        hypermesh_disconnect(client_);
        client_ = nullptr;
    }
}

bool FfiClient::is_connected() const {
    return client_ != nullptr;
}

// ---------------------------------------------------------------------------
// Raw RPC
// ---------------------------------------------------------------------------

std::string FfiClient::call(const std::string& method,
                            const std::string& params_json) {
    require_connected();
    return take_result(
        hypermesh_call(client_, method.c_str(), params_json.c_str()));
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

std::string FfiClient::status() {
    require_connected();
    return take_result(hypermesh_status(client_));
}

// ---------------------------------------------------------------------------
// DNS
// ---------------------------------------------------------------------------

std::string FfiClient::dns_resolve(const std::string& name) {
    require_connected();
    return take_result(hypermesh_dns_resolve(client_, name.c_str()));
}

std::string FfiClient::dns_list() {
    require_connected();
    return take_result(hypermesh_dns_list(client_));
}

std::string FfiClient::dns_register(const std::string& name,
                                    const std::string& addr) {
    require_connected();
    return take_result(
        hypermesh_dns_register(client_, name.c_str(), addr.c_str()));
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

std::string FfiClient::peers() {
    require_connected();
    return take_result(hypermesh_peers(client_));
}

// ---------------------------------------------------------------------------
// Blockchain
// ---------------------------------------------------------------------------

std::string FfiClient::blockchain_height() {
    require_connected();
    return take_result(hypermesh_blockchain_height(client_));
}

std::string FfiClient::blockchain_block(uint64_t index) {
    require_connected();
    return take_result(hypermesh_blockchain_block(client_, index));
}

// ---------------------------------------------------------------------------
// Topology
// ---------------------------------------------------------------------------

std::string FfiClient::topology_info() {
    require_connected();
    return take_result(hypermesh_topology_info(client_));
}

// ---------------------------------------------------------------------------
// Assets
// ---------------------------------------------------------------------------

std::string FfiClient::asset_list() {
    require_connected();
    return take_result(hypermesh_asset_list(client_));
}

std::string FfiClient::asset_store(const std::string& file_path) {
    require_connected();
    return take_result(hypermesh_asset_store(client_, file_path.c_str()));
}

std::string FfiClient::asset_fetch(const std::string& asset_id,
                                   const std::string& output_path) {
    require_connected();
    return take_result(
        hypermesh_asset_fetch(client_, asset_id.c_str(), output_path.c_str()));
}

// ---------------------------------------------------------------------------
// Domains
// ---------------------------------------------------------------------------

std::string FfiClient::domain_list() {
    require_connected();
    return take_result(hypermesh_domain_list(client_));
}

std::string FfiClient::domain_register(const std::string& name,
                                       const std::string& privacy) {
    require_connected();
    return take_result(
        hypermesh_domain_register(client_, name.c_str(), privacy.c_str()));
}

// ---------------------------------------------------------------------------
// Dashboards
// ---------------------------------------------------------------------------

std::string FfiClient::dashboard_list() {
    require_connected();
    return take_result(hypermesh_dashboard_list(client_));
}

std::string FfiClient::dashboard_deploy(const std::string& path) {
    require_connected();
    return take_result(hypermesh_dashboard_deploy(client_, path.c_str()));
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

std::string FfiClient::config_show() {
    require_connected();
    return take_result(hypermesh_config_show(client_));
}

std::string FfiClient::config_get(const std::string& key) {
    require_connected();
    return take_result(hypermesh_config_get(client_, key.c_str()));
}

// ---------------------------------------------------------------------------
// Caesar EVP
// ---------------------------------------------------------------------------

std::string FfiClient::caesar_wallet() {
    require_connected();
    return take_result(hypermesh_caesar_wallet(client_));
}

std::string FfiClient::caesar_balance() {
    require_connected();
    return take_result(hypermesh_caesar_balance(client_));
}

std::string FfiClient::caesar_transactions(uint32_t limit) {
    require_connected();
    return take_result(hypermesh_caesar_transactions(client_, limit));
}

std::string FfiClient::caesar_rewards() {
    require_connected();
    return take_result(hypermesh_caesar_rewards(client_));
}

std::string FfiClient::caesar_route_packet(const std::string& destination,
                                           double amount_grams) {
    require_connected();
    return take_result(
        hypermesh_caesar_route_packet(client_, destination.c_str(),
                                     amount_grams));
}

std::string FfiClient::caesar_governor_params() {
    require_connected();
    return take_result(hypermesh_caesar_governor_params(client_));
}

// ---------------------------------------------------------------------------
// TrustChain
// ---------------------------------------------------------------------------

std::string FfiClient::trustchain_certificates() {
    require_connected();
    return take_result(hypermesh_trustchain_certificates(client_));
}

std::string FfiClient::trustchain_issue(const std::string& subject,
                                        const std::string& scope) {
    require_connected();
    return take_result(
        hypermesh_trustchain_issue(client_, subject.c_str(), scope.c_str()));
}

std::string FfiClient::trustchain_validate(const std::string& cert_pem) {
    require_connected();
    return take_result(
        hypermesh_trustchain_validate(client_, cert_pem.c_str()));
}

std::string FfiClient::trustchain_revoke(const std::string& cert_id) {
    require_connected();
    return take_result(
        hypermesh_trustchain_revoke(client_, cert_id.c_str()));
}

std::string FfiClient::trustchain_dns_zones() {
    require_connected();
    return take_result(hypermesh_trustchain_dns_zones(client_));
}

// ---------------------------------------------------------------------------
// Engauge Analytics
// ---------------------------------------------------------------------------

std::string FfiClient::engauge_capacity() {
    require_connected();
    return take_result(hypermesh_engauge_capacity(client_));
}

std::string FfiClient::engauge_traffic() {
    require_connected();
    return take_result(hypermesh_engauge_traffic(client_));
}

std::string FfiClient::engauge_marketplace() {
    require_connected();
    return take_result(hypermesh_engauge_marketplace(client_));
}

std::string FfiClient::engauge_node_metrics() {
    require_connected();
    return take_result(hypermesh_engauge_node_metrics(client_));
}

std::string FfiClient::engauge_leases() {
    require_connected();
    return take_result(hypermesh_engauge_leases(client_));
}

// ---------------------------------------------------------------------------
// Catalog Registry
// ---------------------------------------------------------------------------

std::string FfiClient::catalog_browse(const std::string& query,
                                      uint32_t page) {
    require_connected();
    const char* q = query.empty() ? nullptr : query.c_str();
    return take_result(hypermesh_catalog_browse(client_, q, page));
}

std::string FfiClient::catalog_search(const std::string& query) {
    require_connected();
    return take_result(hypermesh_catalog_search(client_, query.c_str()));
}

std::string FfiClient::catalog_package_info(const std::string& name) {
    require_connected();
    return take_result(hypermesh_catalog_package_info(client_, name.c_str()));
}

std::string FfiClient::catalog_registry_stats() {
    require_connected();
    return take_result(hypermesh_catalog_registry_stats(client_));
}

} // namespace hypermesh
