// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <string>
#include <stdexcept>
#include <cstdint>

// Pull in the C FFI header directly.
#include "../../../../hypermesh-ffi/include/hypermesh.h"

namespace hypermesh {

/// Exception thrown by FfiClient on FFI call failure.
class FfiError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

/// Direct FFI client that links against libhypermesh_ffi.
///
/// Unlike HyperMeshClient (HTTP-based), this class communicates with the
/// daemon over a Unix domain socket through the C FFI layer, avoiding
/// serialization overhead and network hops.
///
/// Usage:
///   auto client = hypermesh::FfiClient();            // default socket
///   auto status = client.status();                   // JSON string
///   auto addr   = client.dns_resolve("myhost");      // address string
///   client.disconnect();
///
/// Link with: -lhypermesh_ffi
class FfiClient {
public:
    /// Connect to the HyperMesh daemon.
    ///
    /// @param socket_path  Path to the Unix socket, or empty string to use
    ///                     the default 3-tier fallback.
    explicit FfiClient(const std::string& socket_path = "");

    ~FfiClient();

    // Non-copyable.
    FfiClient(const FfiClient&) = delete;
    FfiClient& operator=(const FfiClient&) = delete;

    // Movable.
    FfiClient(FfiClient&& other) noexcept;
    FfiClient& operator=(FfiClient&& other) noexcept;

    // -- Connection lifecycle ------------------------------------------------

    /// Disconnect from the daemon. Safe to call multiple times.
    void disconnect();

    /// Check whether the client holds a valid handle.
    bool is_connected() const;

    // -- Raw RPC -------------------------------------------------------------

    /// Send an arbitrary JSON-RPC call. Returns the response JSON.
    std::string call(const std::string& method,
                     const std::string& params_json);

    // -- Node ----------------------------------------------------------------

    /// Fetch current node status. Returns JSON.
    std::string status();

    // -- DNS -----------------------------------------------------------------

    /// Resolve a DNS name. Returns the address string.
    std::string dns_resolve(const std::string& name);

    /// List all registered DNS entries. Returns JSON array.
    std::string dns_list();

    /// Register a DNS name pointing to addr. Returns JSON.
    std::string dns_register(const std::string& name,
                             const std::string& addr);

    // -- Network -------------------------------------------------------------

    /// List connected peers. Returns JSON array.
    std::string peers();

    // -- Blockchain ----------------------------------------------------------

    /// Get the current blockchain height. Returns JSON.
    std::string blockchain_height();

    /// Get a block by index. Returns JSON.
    std::string blockchain_block(uint64_t index);

    // -- Topology ------------------------------------------------------------

    /// Get this node's topology info. Returns JSON.
    std::string topology_info();

    // -- Assets --------------------------------------------------------------

    /// List all stored assets. Returns JSON array.
    std::string asset_list();

    /// Store a file as a HyperMesh asset. Returns JSON with asset_id.
    std::string asset_store(const std::string& file_path);

    /// Fetch an asset by ID and write it to output_path. Returns "ok".
    std::string asset_fetch(const std::string& asset_id,
                            const std::string& output_path);

    // -- Domains -------------------------------------------------------------

    /// List registered domains. Returns JSON array.
    std::string domain_list();

    /// Register a domain with name and privacy mode. Returns JSON.
    std::string domain_register(const std::string& name,
                                const std::string& privacy);

    // -- Dashboards ----------------------------------------------------------

    /// List deployed dashboards. Returns JSON array.
    std::string dashboard_list();

    /// Deploy a dashboard from the given path. Returns JSON.
    std::string dashboard_deploy(const std::string& path);

    // -- Config --------------------------------------------------------------

    /// Show the full daemon config. Returns JSON.
    std::string config_show();

    /// Get a single config value by key. Returns JSON.
    std::string config_get(const std::string& key);

    // -- Caesar EVP ----------------------------------------------------------

    /// Fetch Caesar wallet info. Returns JSON.
    std::string caesar_wallet();

    /// Fetch Caesar balance. Returns JSON.
    std::string caesar_balance();

    /// Fetch recent Caesar transactions. 0 = default limit. Returns JSON.
    std::string caesar_transactions(uint32_t limit = 0);

    /// Fetch accumulated Caesar rewards. Returns JSON.
    std::string caesar_rewards();

    /// Route a Caesar EVP packet to a destination. Returns JSON.
    std::string caesar_route_packet(const std::string& destination,
                                    double amount_grams);

    /// Fetch current Caesar Governor parameters. Returns JSON.
    std::string caesar_governor_params();

    // -- TrustChain ----------------------------------------------------------

    /// List all TrustChain certificates. Returns JSON array.
    std::string trustchain_certificates();

    /// Issue a certificate for subject and scope. Returns JSON.
    std::string trustchain_issue(const std::string& subject,
                                 const std::string& scope);

    /// Validate a PEM-encoded certificate. Returns JSON.
    std::string trustchain_validate(const std::string& cert_pem);

    /// Revoke a certificate by ID. Returns JSON.
    std::string trustchain_revoke(const std::string& cert_id);

    /// List TrustChain DNS zones. Returns JSON array.
    std::string trustchain_dns_zones();

    // -- Engauge Analytics ---------------------------------------------------

    /// Fetch node capacity metrics. Returns JSON.
    std::string engauge_capacity();

    /// Fetch traffic statistics. Returns JSON.
    std::string engauge_traffic();

    /// Fetch marketplace resource pool info. Returns JSON.
    std::string engauge_marketplace();

    /// Fetch detailed node-level metrics. Returns JSON.
    std::string engauge_node_metrics();

    /// Fetch active resource leases. Returns JSON array.
    std::string engauge_leases();

    // -- Catalog Registry ----------------------------------------------------

    /// Browse catalog packages. query may be empty. Returns paginated JSON.
    std::string catalog_browse(const std::string& query, uint32_t page = 0);

    /// Search catalog packages by query. Returns JSON array.
    std::string catalog_search(const std::string& query);

    /// Get info about a specific catalog package. Returns JSON.
    std::string catalog_package_info(const std::string& name);

    /// Fetch catalog registry statistics. Returns JSON.
    std::string catalog_registry_stats();

private:
    hypermesh_client_t* client_ = nullptr;

    /// Require a live connection or throw.
    void require_connected() const;

    /// Take ownership of a C string result, free it, and return as
    /// std::string. Throws FfiError if result is NULL.
    std::string take_result(char* result);
};

} // namespace hypermesh
