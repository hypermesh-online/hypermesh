// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <string>
#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Domain registration and membership operations.
class DomainApi {
public:
    explicit DomainApi(const HttpClient& http) : http_(http) {}

    /// List all registered domains.
    DomainList list() const;

    /// Register a new domain.
    DomainRegisterResponse register_domain(const std::string& name,
                                           const std::string& privacy) const;

    /// Join an existing domain, optionally with an invitation token.
    DomainJoinResponse join(const std::string& name,
                            const std::string& token = "") const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
