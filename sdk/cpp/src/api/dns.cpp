// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/dns.hpp>

namespace hypermesh {

DnsList DnsApi::list() const {
    auto j = http_.get("/api/v1/dns/list");
    return j.get<DnsList>();
}

DnsRecord DnsApi::resolve(const std::string& name) const {
    auto j = http_.get("/api/v1/dns/resolve/" + name);
    return j.get<DnsRecord>();
}

DnsRegisterResponse DnsApi::register_record(const std::string& name,
                                             const std::string& address) const {
    auto body = nlohmann::json{{"name", name}, {"address", address}};
    auto j = http_.post("/api/v1/dns/register", body);
    return j.get<DnsRegisterResponse>();
}

} // namespace hypermesh
