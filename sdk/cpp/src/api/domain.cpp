// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/domain.hpp>

namespace hypermesh {

DomainList DomainApi::list() const {
    auto j = http_.get("/api/v1/domain/list");
    return j.get<DomainList>();
}

DomainRegisterResponse DomainApi::register_domain(
    const std::string& name, const std::string& privacy) const {
    auto body = nlohmann::json{{"name", name}, {"privacy", privacy}};
    auto j = http_.post("/api/v1/domain/register", body);
    return j.get<DomainRegisterResponse>();
}

DomainJoinResponse DomainApi::join(const std::string& name,
                                   const std::string& token) const {
    auto body = nlohmann::json{{"name", name}};
    if (!token.empty()) {
        body["token"] = token;
    }
    auto j = http_.post("/api/v1/domain/join", body);
    return j.get<DomainJoinResponse>();
}

} // namespace hypermesh
