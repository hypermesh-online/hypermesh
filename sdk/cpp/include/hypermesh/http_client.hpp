// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <string>
#include <nlohmann/json.hpp>

namespace hypermesh {

/// Minimal HTTP/1.1 client using POSIX sockets.
/// Supports GET and POST with JSON bodies. No external dependencies.
class HttpClient {
public:
    /// Construct with base URL, e.g. "https://localhost:8443".
    explicit HttpClient(const std::string& base_url);

    /// Perform a GET request. Returns parsed JSON response body.
    nlohmann::json get(const std::string& path) const;

    /// Perform a POST request with a JSON body. Returns parsed JSON response body.
    nlohmann::json post(const std::string& path,
                        const nlohmann::json& body) const;

    /// Internal HTTP response (public for implementation use only).
    struct HttpResponse {
        int status_code = 0;
        std::string body;
    };

private:
    std::string host_;
    int port_ = 80;
    std::string scheme_;

    HttpResponse send_request(const std::string& method,
                              const std::string& path,
                              const std::string& body = "") const;

    static void parse_url(const std::string& url, std::string& scheme,
                          std::string& host, int& port);
};

} // namespace hypermesh
