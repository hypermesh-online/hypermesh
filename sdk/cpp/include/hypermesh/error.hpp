// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <stdexcept>
#include <string>

namespace hypermesh {

/// Exception thrown by all SDK operations on failure.
class HyperMeshError : public std::runtime_error {
public:
    /// HTTP status code (0 if connection-level failure).
    int status_code;
    /// Raw response body, if available.
    std::string response_body;

    HyperMeshError(const std::string& message, int status = 0,
                   std::string body = "")
        : std::runtime_error(message),
          status_code(status),
          response_body(std::move(body)) {}
};

} // namespace hypermesh
