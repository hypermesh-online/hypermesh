// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/api/caesar.hpp>

namespace hypermesh {

CaesarWalletInfo CaesarApi::wallet() const {
    auto j = http_.get("/api/v1/caesar/wallet");
    return j.get<CaesarWalletInfo>();
}

CaesarBalance CaesarApi::balance() const {
    auto j = http_.get("/api/v1/caesar/balance");
    return j.get<CaesarBalance>();
}

CaesarTransactionList CaesarApi::transactions(uint32_t limit) const {
    std::string path = "/api/v1/caesar/transactions";
    if (limit > 0) {
        path += "?limit=" + std::to_string(limit);
    }
    auto j = http_.get(path);
    return j.get<CaesarTransactionList>();
}

CaesarRewardInfo CaesarApi::rewards() const {
    auto j = http_.get("/api/v1/caesar/rewards");
    return j.get<CaesarRewardInfo>();
}

CaesarRouteResult CaesarApi::route_packet(const std::string& destination,
                                           double amount_grams) const {
    auto body = nlohmann::json{
        {"destination", destination},
        {"amount_grams", amount_grams}
    };
    auto j = http_.post("/api/v1/caesar/route", body);
    return j.get<CaesarRouteResult>();
}

CaesarGovernorParams CaesarApi::governor_params() const {
    auto j = http_.get("/api/v1/caesar/governor/params");
    return j.get<CaesarGovernorParams>();
}

} // namespace hypermesh
