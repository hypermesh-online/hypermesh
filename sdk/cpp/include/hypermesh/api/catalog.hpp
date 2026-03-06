// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <string>
#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Catalog operations (browse, search, package info, registry stats).
class CatalogApi {
public:
    explicit CatalogApi(const HttpClient& http) : http_(http) {}

    /// Browse packages with optional query and page.
    CatalogPackageList browse(const std::string& query = "",
                              uint32_t page = 0) const;

    /// Search packages.
    CatalogSearchResults search(const std::string& query) const;

    /// Get detailed info about a named package.
    CatalogPackageInfo package_info(const std::string& name) const;

    /// Get aggregate registry statistics.
    CatalogRegistryStats registry_stats() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
