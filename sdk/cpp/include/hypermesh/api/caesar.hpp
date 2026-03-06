// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <string>
#include <hypermesh/http_client.hpp>
#include <hypermesh/types.hpp>

namespace hypermesh {

/// Caesar EVP operations (wallet, transactions, rewards, routing, governor).
class CaesarApi {
public:
    explicit CaesarApi(const HttpClient& http) : http_(http) {}

    /// Get wallet info.
    CaesarWalletInfo wallet() const;

    /// Get balance.
    CaesarBalance balance() const;

    /// List transactions. Pass 0 for no limit.
    CaesarTransactionList transactions(uint32_t limit = 0) const;

    /// Get reward info.
    CaesarRewardInfo rewards() const;

    /// Route an EVP packet to the given destination.
    CaesarRouteResult route_packet(const std::string& destination,
                                   double amount_grams) const;

    /// Get governor parameters.
    CaesarGovernorParams governor_params() const;

private:
    const HttpClient& http_;
};

} // namespace hypermesh
