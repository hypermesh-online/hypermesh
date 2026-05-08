// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#include <hypermesh/http_client.hpp>
#include <hypermesh/error.hpp>

#include <algorithm>
#include <array>
#include <cerrno>
#include <cstring>
#include <sstream>
#include <string>

#include <arpa/inet.h>
#include <netdb.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

namespace hypermesh {

namespace {

/// RAII wrapper for a file descriptor.
class SocketGuard {
public:
    explicit SocketGuard(int fd) : fd_(fd) {}
    ~SocketGuard() {
        if (fd_ >= 0) {
            ::close(fd_);
        }
    }
    SocketGuard(const SocketGuard&) = delete;
    SocketGuard& operator=(const SocketGuard&) = delete;
    int fd() const { return fd_; }

private:
    int fd_;
};

/// Send all bytes through a socket. Throws on failure.
void send_all(int fd, const std::string& data) {
    size_t sent = 0;
    while (sent < data.size()) {
        auto n = ::send(fd, data.data() + sent, data.size() - sent, 0);
        if (n < 0) {
            throw HyperMeshError(
                std::string("send failed: ") + std::strerror(errno));
        }
        sent += static_cast<size_t>(n);
    }
}

/// Read all available response data from a socket until the connection closes.
std::string recv_all(int fd) {
    std::string result;
    std::array<char, 4096> buf{};
    for (;;) {
        auto n = ::recv(fd, buf.data(), buf.size(), 0);
        if (n < 0) {
            throw HyperMeshError(
                std::string("recv failed: ") + std::strerror(errno));
        }
        if (n == 0) {
            break;
        }
        result.append(buf.data(), static_cast<size_t>(n));

        // Check if we have received the full response by looking for
        // Content-Length or end of chunked encoding.
        // For simplicity, check if headers are complete and body is fully read.
        auto header_end = result.find("\r\n\r\n");
        if (header_end != std::string::npos) {
            auto body_start = header_end + 4;
            // Look for Content-Length
            auto cl_pos = result.find("Content-Length: ");
            if (cl_pos == std::string::npos) {
                cl_pos = result.find("content-length: ");
            }
            if (cl_pos != std::string::npos &&
                cl_pos < header_end) {
                auto cl_end = result.find("\r\n", cl_pos);
                auto cl_val = result.substr(cl_pos + 16,
                                            cl_end - cl_pos - 16);
                auto content_length =
                    static_cast<size_t>(std::stoul(cl_val));
                if (result.size() - body_start >= content_length) {
                    break;
                }
            }
        }
    }
    return result;
}

/// Parse an HTTP response into status code and body.
HttpClient::HttpResponse parse_http_response(const std::string& raw) {
    HttpClient::HttpResponse resp{};

    auto header_end = raw.find("\r\n\r\n");
    if (header_end == std::string::npos) {
        throw HyperMeshError("Malformed HTTP response: no header terminator");
    }

    // Parse status line: "HTTP/1.1 200 OK\r\n"
    auto first_line_end = raw.find("\r\n");
    auto status_line = raw.substr(0, first_line_end);

    // Find the status code (after first space)
    auto space1 = status_line.find(' ');
    if (space1 == std::string::npos) {
        throw HyperMeshError("Malformed HTTP status line");
    }
    auto space2 = status_line.find(' ', space1 + 1);
    auto code_str = status_line.substr(
        space1 + 1, (space2 != std::string::npos ? space2 : status_line.size())
                     - space1 - 1);
    resp.status_code = std::stoi(code_str);

    resp.body = raw.substr(header_end + 4);
    return resp;
}

} // namespace

HttpClient::HttpClient(const std::string& base_url) {
    parse_url(base_url, scheme_, host_, port_);
}

void HttpClient::parse_url(const std::string& url, std::string& scheme,
                           std::string& host, int& port) {
    // Extract scheme
    auto scheme_end = url.find("://");
    if (scheme_end == std::string::npos) {
        scheme = "http";
        scheme_end = 0;
    } else {
        scheme = url.substr(0, scheme_end);
        scheme_end += 3;
    }

    auto rest = url.substr(scheme_end);

    // Strip trailing slashes and path
    auto slash_pos = rest.find('/');
    if (slash_pos != std::string::npos) {
        rest = rest.substr(0, slash_pos);
    }

    // Check for port
    auto colon_pos = rest.rfind(':');
    // Handle IPv6 addresses in brackets
    auto bracket_close = rest.rfind(']');
    if (colon_pos != std::string::npos &&
        (bracket_close == std::string::npos || colon_pos > bracket_close)) {
        host = rest.substr(0, colon_pos);
        port = std::stoi(rest.substr(colon_pos + 1));
    } else {
        host = rest;
        port = (scheme == "https") ? 443 : 80;
    }

    // Strip brackets from IPv6
    if (!host.empty() && host.front() == '[' && host.back() == ']') {
        host = host.substr(1, host.size() - 2);
    }
}

HttpClient::HttpResponse HttpClient::send_request(
    const std::string& method, const std::string& path,
    const std::string& body) const {

    // Resolve host
    struct addrinfo hints{};
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;

    struct addrinfo* result = nullptr;
    auto port_str = std::to_string(port_);
    int rv = ::getaddrinfo(host_.c_str(), port_str.c_str(), &hints, &result);
    if (rv != 0) {
        throw HyperMeshError(
            std::string("DNS resolution failed for ") + host_ + ": " +
            gai_strerror(rv));
    }

    // RAII for addrinfo
    struct AddrInfoGuard {
        struct addrinfo* info;
        ~AddrInfoGuard() { ::freeaddrinfo(info); }
    } addr_guard{result};

    int sock_fd = -1;
    for (auto* rp = result; rp != nullptr; rp = rp->ai_next) {
        sock_fd = ::socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol);
        if (sock_fd < 0) {
            continue;
        }
        if (::connect(sock_fd, rp->ai_addr, rp->ai_addrlen) == 0) {
            break;
        }
        ::close(sock_fd);
        sock_fd = -1;
    }

    if (sock_fd < 0) {
        throw HyperMeshError(
            "Connection failed to " + host_ + ":" + port_str);
    }

    SocketGuard guard(sock_fd);

    // Build HTTP/1.1 request
    std::ostringstream req;
    req << method << " " << path << " HTTP/1.1\r\n";
    req << "Host: " << host_;
    if ((scheme_ == "http" && port_ != 80) ||
        (scheme_ == "https" && port_ != 443)) {
        req << ":" << port_;
    }
    req << "\r\n";
    req << "Accept: application/json\r\n";
    req << "Connection: close\r\n";

    if (!body.empty()) {
        req << "Content-Type: application/json\r\n";
        req << "Content-Length: " << body.size() << "\r\n";
    }

    // Phase K.2 — capability token (alpha-default inert when empty).
    if (!capability_token_.empty()) {
        req << kCapabilityTokenHeader << ": " << capability_token_ << "\r\n";
    }

    req << "\r\n";
    if (!body.empty()) {
        req << body;
    }

    send_all(guard.fd(), req.str());
    auto raw = recv_all(guard.fd());
    return parse_http_response(raw);
}

nlohmann::json HttpClient::get(const std::string& path) const {
    auto resp = send_request("GET", path);

    if (resp.status_code < 200 || resp.status_code >= 300) {
        throw HyperMeshError(
            "HTTP " + std::to_string(resp.status_code),
            resp.status_code, resp.body);
    }

    if (resp.body.empty()) {
        return nlohmann::json::object();
    }
    return nlohmann::json::parse(resp.body);
}

nlohmann::json HttpClient::post(const std::string& path,
                                const nlohmann::json& body) const {
    auto body_str = body.dump();
    auto resp = send_request("POST", path, body_str);

    if (resp.status_code < 200 || resp.status_code >= 300) {
        throw HyperMeshError(
            "HTTP " + std::to_string(resp.status_code),
            resp.status_code, resp.body);
    }

    if (resp.body.empty()) {
        return nlohmann::json::object();
    }
    return nlohmann::json::parse(resp.body);
}

} // namespace hypermesh
