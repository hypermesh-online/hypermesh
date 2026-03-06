// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/trustchain.hpp>

namespace hypermesh {

TrustChainCertificateList TrustChainApi::certificates() const {
    auto j = http_.get("/api/v1/trustchain/certificates");
    return j.get<TrustChainCertificateList>();
}

TrustChainCertificate TrustChainApi::issue(const std::string& subject,
                                            const std::string& scope) const {
    auto body = nlohmann::json{{"subject", subject}, {"scope", scope}};
    auto j = http_.post("/api/v1/trustchain/issue", body);
    return j.get<TrustChainCertificate>();
}

TrustChainValidationResult TrustChainApi::validate(const std::string& cert_pem) const {
    auto body = nlohmann::json{{"cert_pem", cert_pem}};
    auto j = http_.post("/api/v1/trustchain/validate", body);
    return j.get<TrustChainValidationResult>();
}

TrustChainRevokeResult TrustChainApi::revoke(const std::string& cert_id) const {
    auto body = nlohmann::json{{"cert_id", cert_id}};
    auto j = http_.post("/api/v1/trustchain/revoke", body);
    return j.get<TrustChainRevokeResult>();
}

TrustChainDnsZoneList TrustChainApi::dns_zones() const {
    auto j = http_.get("/api/v1/trustchain/dns/zones");
    return j.get<TrustChainDnsZoneList>();
}

} // namespace hypermesh
