// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/catalog.hpp>

namespace hypermesh {

CatalogPackageList CatalogApi::browse(const std::string& query,
                                       uint32_t page) const {
    std::string path = "/api/v1/catalog/browse";
    std::string sep = "?";
    if (!query.empty()) {
        path += sep + "query=" + query;
        sep = "&";
    }
    if (page > 0) {
        path += sep + "page=" + std::to_string(page);
    }
    auto j = http_.get(path);
    return j.get<CatalogPackageList>();
}

CatalogSearchResults CatalogApi::search(const std::string& query) const {
    auto j = http_.get("/api/v1/catalog/search?query=" + query);
    return j.get<CatalogSearchResults>();
}

CatalogPackageInfo CatalogApi::package_info(const std::string& name) const {
    auto j = http_.get("/api/v1/catalog/package/" + name);
    return j.get<CatalogPackageInfo>();
}

CatalogRegistryStats CatalogApi::registry_stats() const {
    auto j = http_.get("/api/v1/catalog/registry/stats");
    return j.get<CatalogRegistryStats>();
}

} // namespace hypermesh
