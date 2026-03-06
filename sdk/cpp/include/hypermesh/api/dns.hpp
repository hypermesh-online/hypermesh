// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <string>
#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// DNS registration and resolution operations.
class DnsApi {
public:
    explicit DnsApi(const HttpClient& http) : http_(http) {}

    /// List all registered DNS records.
    DnsList list() const;

    /// Resolve a DNS name to its record.
    DnsRecord resolve(const std::string& name) const;

    /// Register a new DNS record.
    DnsRegisterResponse register_record(const std::string& name,
                                        const std::string& address) const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
