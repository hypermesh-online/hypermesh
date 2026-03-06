// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <string>
#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// TrustChain operations (certificates, issuance, validation, revocation, DNS zones).
class TrustChainApi {
public:
    explicit TrustChainApi(const HttpClient& http) : http_(http) {}

    /// List all certificates.
    TrustChainCertificateList certificates() const;

    /// Issue a new certificate.
    TrustChainCertificate issue(const std::string& subject,
                                const std::string& scope) const;

    /// Validate a certificate PEM.
    TrustChainValidationResult validate(const std::string& cert_pem) const;

    /// Revoke a certificate by ID.
    TrustChainRevokeResult revoke(const std::string& cert_id) const;

    /// List DNS zones.
    TrustChainDnsZoneList dns_zones() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
