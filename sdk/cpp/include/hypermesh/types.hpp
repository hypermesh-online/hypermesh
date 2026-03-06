// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

#pragma once

#include <cstdint>
#include <string>
#include <vector>
#include <nlohmann/json.hpp>

namespace hypermesh {

// ── Shared ──

struct Coordinate {
    double x = 0.0;
    double y = 0.0;
    double z = 0.0;
};

inline void from_json(const nlohmann::json& j, Coordinate& c) {
    j.at("x").get_to(c.x);
    j.at("y").get_to(c.y);
    j.at("z").get_to(c.z);
}

inline void to_json(nlohmann::json& j, const Coordinate& c) {
    j = nlohmann::json{{"x", c.x}, {"y", c.y}, {"z", c.z}};
}

// ── Node ──

struct NodeStatus {
    uint64_t chain_height = 0;
    Coordinate coordinate;
    std::string node_id;
    uint64_t peers = 0;
    std::string privacy_mode;
    uint64_t uptime_secs = 0;
};

inline void from_json(const nlohmann::json& j, NodeStatus& s) {
    j.at("chain_height").get_to(s.chain_height);
    j.at("coordinate").get_to(s.coordinate);
    j.at("node_id").get_to(s.node_id);
    j.at("peers").get_to(s.peers);
    j.at("privacy_mode").get_to(s.privacy_mode);
    j.at("uptime_secs").get_to(s.uptime_secs);
}

inline void to_json(nlohmann::json& j, const NodeStatus& s) {
    j = nlohmann::json{
        {"chain_height", s.chain_height},
        {"coordinate", s.coordinate},
        {"node_id", s.node_id},
        {"peers", s.peers},
        {"privacy_mode", s.privacy_mode},
        {"uptime_secs", s.uptime_secs}
    };
}

struct PingResponse {
    bool pong = false;
};

inline void from_json(const nlohmann::json& j, PingResponse& p) {
    j.at("pong").get_to(p.pong);
}

inline void to_json(nlohmann::json& j, const PingResponse& p) {
    j = nlohmann::json{{"pong", p.pong}};
}

// ── Blockchain ──

struct BlockchainHeight {
    uint64_t height = 0;
};

inline void from_json(const nlohmann::json& j, BlockchainHeight& h) {
    j.at("height").get_to(h.height);
}

inline void to_json(nlohmann::json& j, const BlockchainHeight& h) {
    j = nlohmann::json{{"height", h.height}};
}

struct Block {
    uint64_t index = 0;
    uint64_t timestamp = 0;
    std::string hash;
    std::string previous_hash;
    nlohmann::json extra;  // additional fields
};

inline void from_json(const nlohmann::json& j, Block& b) {
    j.at("index").get_to(b.index);
    j.at("timestamp").get_to(b.timestamp);
    j.at("hash").get_to(b.hash);
    j.at("previous_hash").get_to(b.previous_hash);
    b.extra = j;
}

inline void to_json(nlohmann::json& j, const Block& b) {
    j = b.extra;
    j["index"] = b.index;
    j["timestamp"] = b.timestamp;
    j["hash"] = b.hash;
    j["previous_hash"] = b.previous_hash;
}

struct ValidationResult {
    bool valid = false;
    std::vector<std::string> errors;
    uint64_t blocks_checked = 0;
};

inline void from_json(const nlohmann::json& j, ValidationResult& v) {
    j.at("valid").get_to(v.valid);
    if (j.contains("errors") && !j["errors"].is_null()) {
        j["errors"].get_to(v.errors);
    }
    if (j.contains("blocks_checked") && !j["blocks_checked"].is_null()) {
        j["blocks_checked"].get_to(v.blocks_checked);
    }
}

inline void to_json(nlohmann::json& j, const ValidationResult& v) {
    j = nlohmann::json{
        {"valid", v.valid},
        {"errors", v.errors},
        {"blocks_checked", v.blocks_checked}
    };
}

// ── DNS ──

struct DnsRecord {
    std::string name;
    std::string address;
};

inline void from_json(const nlohmann::json& j, DnsRecord& r) {
    j.at("name").get_to(r.name);
    j.at("address").get_to(r.address);
}

inline void to_json(nlohmann::json& j, const DnsRecord& r) {
    j = nlohmann::json{{"name", r.name}, {"address", r.address}};
}

struct DnsList {
    uint64_t count = 0;
    std::vector<DnsRecord> records;
};

inline void from_json(const nlohmann::json& j, DnsList& l) {
    j.at("count").get_to(l.count);
    j.at("records").get_to(l.records);
}

inline void to_json(nlohmann::json& j, const DnsList& l) {
    j = nlohmann::json{{"count", l.count}, {"records", l.records}};
}

struct DnsRegisterResponse {
    nlohmann::json data;
};

inline void from_json(const nlohmann::json& j, DnsRegisterResponse& r) {
    r.data = j;
}

inline void to_json(nlohmann::json& j, const DnsRegisterResponse& r) {
    j = r.data;
}

// ── Network ──

struct Peer {
    nlohmann::json data;
};

inline void from_json(const nlohmann::json& j, Peer& p) {
    p.data = j;
}

inline void to_json(nlohmann::json& j, const Peer& p) {
    j = p.data;
}

struct PeerList {
    uint64_t count = 0;
    std::vector<Peer> peers;
};

inline void from_json(const nlohmann::json& j, PeerList& l) {
    j.at("count").get_to(l.count);
    j.at("peers").get_to(l.peers);
}

inline void to_json(nlohmann::json& j, const PeerList& l) {
    j = nlohmann::json{{"count", l.count}, {"peers", l.peers}};
}

// ── Topology ──

struct TopologyInfo {
    Coordinate coordinate;
    std::string node_id;
};

inline void from_json(const nlohmann::json& j, TopologyInfo& t) {
    j.at("coordinate").get_to(t.coordinate);
    j.at("node_id").get_to(t.node_id);
}

inline void to_json(nlohmann::json& j, const TopologyInfo& t) {
    j = nlohmann::json{{"coordinate", t.coordinate}, {"node_id", t.node_id}};
}

struct Neighbor {
    nlohmann::json data;
};

inline void from_json(const nlohmann::json& j, Neighbor& n) {
    n.data = j;
}

inline void to_json(nlohmann::json& j, const Neighbor& n) {
    j = n.data;
}

struct NeighborList {
    Coordinate center;
    uint64_t count = 0;
    std::vector<Neighbor> neighbors;
    double radius = 0.0;
};

inline void from_json(const nlohmann::json& j, NeighborList& l) {
    j.at("center").get_to(l.center);
    j.at("count").get_to(l.count);
    j.at("neighbors").get_to(l.neighbors);
    j.at("radius").get_to(l.radius);
}

inline void to_json(nlohmann::json& j, const NeighborList& l) {
    j = nlohmann::json{
        {"center", l.center},
        {"count", l.count},
        {"neighbors", l.neighbors},
        {"radius", l.radius}
    };
}

// ── Asset ──

struct Asset {
    uint64_t block_index = 0;
    std::string category;
    std::string content_hash;
    std::string scope;
};

inline void from_json(const nlohmann::json& j, Asset& a) {
    j.at("block_index").get_to(a.block_index);
    j.at("category").get_to(a.category);
    j.at("content_hash").get_to(a.content_hash);
    j.at("scope").get_to(a.scope);
}

inline void to_json(nlohmann::json& j, const Asset& a) {
    j = nlohmann::json{
        {"block_index", a.block_index},
        {"category", a.category},
        {"content_hash", a.content_hash},
        {"scope", a.scope}
    };
}

struct AssetList {
    uint64_t count = 0;
    std::vector<Asset> assets;
};

inline void from_json(const nlohmann::json& j, AssetList& l) {
    j.at("count").get_to(l.count);
    j.at("assets").get_to(l.assets);
}

inline void to_json(nlohmann::json& j, const AssetList& l) {
    j = nlohmann::json{{"count", l.count}, {"assets", l.assets}};
}

// ── Dashboard ──

struct DashboardEntry {
    uint64_t block = 0;
    std::string description;
    std::string domain;
    std::string hash;
    std::string name;
    std::string registered_at;
    std::string version;
};

inline void from_json(const nlohmann::json& j, DashboardEntry& d) {
    j.at("block").get_to(d.block);
    j.at("description").get_to(d.description);
    j.at("domain").get_to(d.domain);
    j.at("hash").get_to(d.hash);
    j.at("name").get_to(d.name);
    j.at("registered_at").get_to(d.registered_at);
    j.at("version").get_to(d.version);
}

inline void to_json(nlohmann::json& j, const DashboardEntry& d) {
    j = nlohmann::json{
        {"block", d.block},
        {"description", d.description},
        {"domain", d.domain},
        {"hash", d.hash},
        {"name", d.name},
        {"registered_at", d.registered_at},
        {"version", d.version}
    };
}

struct DashboardList {
    uint64_t count = 0;
    std::vector<DashboardEntry> dashboards;
};

inline void from_json(const nlohmann::json& j, DashboardList& l) {
    j.at("count").get_to(l.count);
    j.at("dashboards").get_to(l.dashboards);
}

inline void to_json(nlohmann::json& j, const DashboardList& l) {
    j = nlohmann::json{{"count", l.count}, {"dashboards", l.dashboards}};
}

struct DashboardInfo {
    nlohmann::json data;
};

inline void from_json(const nlohmann::json& j, DashboardInfo& d) {
    d.data = j;
}

inline void to_json(nlohmann::json& j, const DashboardInfo& d) {
    j = d.data;
}

// ── Config ──

struct ConfigValue {
    nlohmann::json data;
};

inline void from_json(const nlohmann::json& j, ConfigValue& c) {
    c.data = j;
}

inline void to_json(nlohmann::json& j, const ConfigValue& c) {
    j = c.data;
}

// ── Domain ──

struct Domain {
    std::string domain;
    std::string network_id;
    std::string owner;
    std::string privacy;
};

inline void from_json(const nlohmann::json& j, Domain& d) {
    j.at("domain").get_to(d.domain);
    j.at("network_id").get_to(d.network_id);
    j.at("owner").get_to(d.owner);
    j.at("privacy").get_to(d.privacy);
}

inline void to_json(nlohmann::json& j, const Domain& d) {
    j = nlohmann::json{
        {"domain", d.domain},
        {"network_id", d.network_id},
        {"owner", d.owner},
        {"privacy", d.privacy}
    };
}

struct DomainList {
    uint64_t count = 0;
    std::vector<Domain> domains;
};

inline void from_json(const nlohmann::json& j, DomainList& l) {
    j.at("count").get_to(l.count);
    j.at("domains").get_to(l.domains);
}

inline void to_json(nlohmann::json& j, const DomainList& l) {
    j = nlohmann::json{{"count", l.count}, {"domains", l.domains}};
}

struct DomainRegisterResponse {
    std::string domain;
    std::string network_id;
    std::string privacy;
    std::string owner;
    uint64_t block = 0;
    std::string status;
};

inline void from_json(const nlohmann::json& j, DomainRegisterResponse& r) {
    j.at("domain").get_to(r.domain);
    j.at("network_id").get_to(r.network_id);
    j.at("privacy").get_to(r.privacy);
    j.at("owner").get_to(r.owner);
    j.at("block").get_to(r.block);
    j.at("status").get_to(r.status);
}

inline void to_json(nlohmann::json& j, const DomainRegisterResponse& r) {
    j = nlohmann::json{
        {"domain", r.domain},
        {"network_id", r.network_id},
        {"privacy", r.privacy},
        {"owner", r.owner},
        {"block", r.block},
        {"status", r.status}
    };
}

struct DomainJoinResponse {
    nlohmann::json data;
};

inline void from_json(const nlohmann::json& j, DomainJoinResponse& r) {
    r.data = j;
}

inline void to_json(nlohmann::json& j, const DomainJoinResponse& r) {
    j = r.data;
}

} // namespace hypermesh
