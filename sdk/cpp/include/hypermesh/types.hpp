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

// ── Caesar ──

struct CaesarWalletInfo {
    double balance_grams = 0.0;
    double balance_usd = 0.0;
    std::string tier;
    std::string node_id;
};

inline void from_json(const nlohmann::json& j, CaesarWalletInfo& w) {
    j.at("balance_grams").get_to(w.balance_grams);
    j.at("balance_usd").get_to(w.balance_usd);
    j.at("tier").get_to(w.tier);
    j.at("node_id").get_to(w.node_id);
}

inline void to_json(nlohmann::json& j, const CaesarWalletInfo& w) {
    j = nlohmann::json{
        {"balance_grams", w.balance_grams},
        {"balance_usd", w.balance_usd},
        {"tier", w.tier},
        {"node_id", w.node_id}
    };
}

struct CaesarBalance {
    double gold_grams = 0.0;
    double usd_equivalent = 0.0;
    std::string tier;
};

inline void from_json(const nlohmann::json& j, CaesarBalance& b) {
    j.at("gold_grams").get_to(b.gold_grams);
    j.at("usd_equivalent").get_to(b.usd_equivalent);
    j.at("tier").get_to(b.tier);
}

inline void to_json(nlohmann::json& j, const CaesarBalance& b) {
    j = nlohmann::json{
        {"gold_grams", b.gold_grams},
        {"usd_equivalent", b.usd_equivalent},
        {"tier", b.tier}
    };
}

struct CaesarTransaction {
    std::string id;
    std::string from;
    std::string to;
    double amount_grams = 0.0;
    double fee = 0.0;
    std::string status;
    uint64_t timestamp = 0;
};

inline void from_json(const nlohmann::json& j, CaesarTransaction& t) {
    j.at("id").get_to(t.id);
    j.at("from").get_to(t.from);
    j.at("to").get_to(t.to);
    j.at("amount_grams").get_to(t.amount_grams);
    j.at("fee").get_to(t.fee);
    j.at("status").get_to(t.status);
    j.at("timestamp").get_to(t.timestamp);
}

inline void to_json(nlohmann::json& j, const CaesarTransaction& t) {
    j = nlohmann::json{
        {"id", t.id}, {"from", t.from}, {"to", t.to},
        {"amount_grams", t.amount_grams}, {"fee", t.fee},
        {"status", t.status}, {"timestamp", t.timestamp}
    };
}

struct CaesarTransactionList {
    uint64_t count = 0;
    std::vector<CaesarTransaction> transactions;
};

inline void from_json(const nlohmann::json& j, CaesarTransactionList& l) {
    j.at("count").get_to(l.count);
    j.at("transactions").get_to(l.transactions);
}

inline void to_json(nlohmann::json& j, const CaesarTransactionList& l) {
    j = nlohmann::json{{"count", l.count}, {"transactions", l.transactions}};
}

struct CaesarRewardInfo {
    double total_earned = 0.0;
    double pending = 0.0;
    double tier_multiplier = 0.0;
};

inline void from_json(const nlohmann::json& j, CaesarRewardInfo& r) {
    j.at("total_earned").get_to(r.total_earned);
    j.at("pending").get_to(r.pending);
    j.at("tier_multiplier").get_to(r.tier_multiplier);
}

inline void to_json(nlohmann::json& j, const CaesarRewardInfo& r) {
    j = nlohmann::json{
        {"total_earned", r.total_earned},
        {"pending", r.pending},
        {"tier_multiplier", r.tier_multiplier}
    };
}

struct CaesarRouteResult {
    std::string packet_id;
    std::string status;
    double fee = 0.0;
};

inline void from_json(const nlohmann::json& j, CaesarRouteResult& r) {
    j.at("packet_id").get_to(r.packet_id);
    j.at("status").get_to(r.status);
    j.at("fee").get_to(r.fee);
}

inline void to_json(nlohmann::json& j, const CaesarRouteResult& r) {
    j = nlohmann::json{
        {"packet_id", r.packet_id},
        {"status", r.status},
        {"fee", r.fee}
    };
}

struct CaesarGovernorParams {
    double velocity = 0.0;
    double fee_rate = 0.0;
    double demurrage_rate = 0.0;
};

inline void from_json(const nlohmann::json& j, CaesarGovernorParams& p) {
    j.at("velocity").get_to(p.velocity);
    j.at("fee_rate").get_to(p.fee_rate);
    j.at("demurrage_rate").get_to(p.demurrage_rate);
}

inline void to_json(nlohmann::json& j, const CaesarGovernorParams& p) {
    j = nlohmann::json{
        {"velocity", p.velocity},
        {"fee_rate", p.fee_rate},
        {"demurrage_rate", p.demurrage_rate}
    };
}

// ── TrustChain ──

struct TrustChainCertificate {
    std::string id;
    std::string subject;
    std::string scope;
    std::string valid_from;
    std::string valid_to;
    std::string pem;
};

inline void from_json(const nlohmann::json& j, TrustChainCertificate& c) {
    j.at("id").get_to(c.id);
    j.at("subject").get_to(c.subject);
    j.at("scope").get_to(c.scope);
    j.at("valid_from").get_to(c.valid_from);
    j.at("valid_to").get_to(c.valid_to);
    j.at("pem").get_to(c.pem);
}

inline void to_json(nlohmann::json& j, const TrustChainCertificate& c) {
    j = nlohmann::json{
        {"id", c.id}, {"subject", c.subject}, {"scope", c.scope},
        {"valid_from", c.valid_from}, {"valid_to", c.valid_to}, {"pem", c.pem}
    };
}

struct TrustChainCertificateList {
    uint64_t count = 0;
    std::vector<TrustChainCertificate> certificates;
};

inline void from_json(const nlohmann::json& j, TrustChainCertificateList& l) {
    j.at("count").get_to(l.count);
    j.at("certificates").get_to(l.certificates);
}

inline void to_json(nlohmann::json& j, const TrustChainCertificateList& l) {
    j = nlohmann::json{{"count", l.count}, {"certificates", l.certificates}};
}

struct TrustChainValidationResult {
    bool valid = false;
    std::vector<std::string> errors;
    bool chain_valid = false;
};

inline void from_json(const nlohmann::json& j, TrustChainValidationResult& v) {
    j.at("valid").get_to(v.valid);
    if (j.contains("errors") && !j["errors"].is_null()) {
        j["errors"].get_to(v.errors);
    }
    j.at("chain_valid").get_to(v.chain_valid);
}

inline void to_json(nlohmann::json& j, const TrustChainValidationResult& v) {
    j = nlohmann::json{
        {"valid", v.valid},
        {"errors", v.errors},
        {"chain_valid", v.chain_valid}
    };
}

struct TrustChainRevokeResult {
    bool revoked = false;
    std::string cert_id;
};

inline void from_json(const nlohmann::json& j, TrustChainRevokeResult& r) {
    j.at("revoked").get_to(r.revoked);
    j.at("cert_id").get_to(r.cert_id);
}

inline void to_json(nlohmann::json& j, const TrustChainRevokeResult& r) {
    j = nlohmann::json{{"revoked", r.revoked}, {"cert_id", r.cert_id}};
}

struct TrustChainDnsZone {
    std::string name;
    uint64_t records = 0;
};

inline void from_json(const nlohmann::json& j, TrustChainDnsZone& z) {
    j.at("name").get_to(z.name);
    j.at("records").get_to(z.records);
}

inline void to_json(nlohmann::json& j, const TrustChainDnsZone& z) {
    j = nlohmann::json{{"name", z.name}, {"records", z.records}};
}

struct TrustChainDnsZoneList {
    uint64_t count = 0;
    std::vector<TrustChainDnsZone> zones;
};

inline void from_json(const nlohmann::json& j, TrustChainDnsZoneList& l) {
    j.at("count").get_to(l.count);
    j.at("zones").get_to(l.zones);
}

inline void to_json(nlohmann::json& j, const TrustChainDnsZoneList& l) {
    j = nlohmann::json{{"count", l.count}, {"zones", l.zones}};
}

// ── Engauge ──

struct EngaugeCapacityMetrics {
    uint64_t bytes_served = 0;
    double compute_delivered = 0.0;
    uint64_t storage = 0;
    double bandwidth = 0.0;
    double uptime = 0.0;
};

inline void from_json(const nlohmann::json& j, EngaugeCapacityMetrics& m) {
    j.at("bytes_served").get_to(m.bytes_served);
    j.at("compute_delivered").get_to(m.compute_delivered);
    j.at("storage").get_to(m.storage);
    j.at("bandwidth").get_to(m.bandwidth);
    j.at("uptime").get_to(m.uptime);
}

inline void to_json(nlohmann::json& j, const EngaugeCapacityMetrics& m) {
    j = nlohmann::json{
        {"bytes_served", m.bytes_served},
        {"compute_delivered", m.compute_delivered},
        {"storage", m.storage},
        {"bandwidth", m.bandwidth},
        {"uptime", m.uptime}
    };
}

struct EngaugeTrafficMetrics {
    double organic_ratio = 0.0;
    double speculative_ratio = 0.0;
    uint64_t total_requests = 0;
};

inline void from_json(const nlohmann::json& j, EngaugeTrafficMetrics& m) {
    j.at("organic_ratio").get_to(m.organic_ratio);
    j.at("speculative_ratio").get_to(m.speculative_ratio);
    j.at("total_requests").get_to(m.total_requests);
}

inline void to_json(nlohmann::json& j, const EngaugeTrafficMetrics& m) {
    j = nlohmann::json{
        {"organic_ratio", m.organic_ratio},
        {"speculative_ratio", m.speculative_ratio},
        {"total_requests", m.total_requests}
    };
}

struct EngaugeListing {
    std::string id;
    std::string resource_type;
    double price = 0.0;
};

inline void from_json(const nlohmann::json& j, EngaugeListing& l) {
    j.at("id").get_to(l.id);
    j.at("resource_type").get_to(l.resource_type);
    j.at("price").get_to(l.price);
}

inline void to_json(nlohmann::json& j, const EngaugeListing& l) {
    j = nlohmann::json{
        {"id", l.id},
        {"resource_type", l.resource_type},
        {"price", l.price}
    };
}

struct EngaugeListingList {
    uint64_t count = 0;
    std::vector<EngaugeListing> listings;
};

inline void from_json(const nlohmann::json& j, EngaugeListingList& l) {
    j.at("count").get_to(l.count);
    j.at("listings").get_to(l.listings);
}

inline void to_json(nlohmann::json& j, const EngaugeListingList& l) {
    j = nlohmann::json{{"count", l.count}, {"listings", l.listings}};
}

struct EngaugeNodeMetrics {
    double activity_score = 0.0;
    uint64_t receipts = 0;
    double bandwidth = 0.0;
};

inline void from_json(const nlohmann::json& j, EngaugeNodeMetrics& m) {
    j.at("activity_score").get_to(m.activity_score);
    j.at("receipts").get_to(m.receipts);
    j.at("bandwidth").get_to(m.bandwidth);
}

inline void to_json(nlohmann::json& j, const EngaugeNodeMetrics& m) {
    j = nlohmann::json{
        {"activity_score", m.activity_score},
        {"receipts", m.receipts},
        {"bandwidth", m.bandwidth}
    };
}

struct EngaugeLease {
    std::string id;
    std::string resource_type;
    std::string status;
};

inline void from_json(const nlohmann::json& j, EngaugeLease& l) {
    j.at("id").get_to(l.id);
    j.at("resource_type").get_to(l.resource_type);
    j.at("status").get_to(l.status);
}

inline void to_json(nlohmann::json& j, const EngaugeLease& l) {
    j = nlohmann::json{
        {"id", l.id},
        {"resource_type", l.resource_type},
        {"status", l.status}
    };
}

struct EngaugeLeaseList {
    uint64_t count = 0;
    std::vector<EngaugeLease> leases;
};

inline void from_json(const nlohmann::json& j, EngaugeLeaseList& l) {
    j.at("count").get_to(l.count);
    j.at("leases").get_to(l.leases);
}

inline void to_json(nlohmann::json& j, const EngaugeLeaseList& l) {
    j = nlohmann::json{{"count", l.count}, {"leases", l.leases}};
}

// ── Catalog ──

struct CatalogPackage {
    std::string name;
    std::string version;
    std::string description;
    std::string author;
    uint64_t downloads = 0;
};

inline void from_json(const nlohmann::json& j, CatalogPackage& p) {
    j.at("name").get_to(p.name);
    j.at("version").get_to(p.version);
    j.at("description").get_to(p.description);
    j.at("author").get_to(p.author);
    j.at("downloads").get_to(p.downloads);
}

inline void to_json(nlohmann::json& j, const CatalogPackage& p) {
    j = nlohmann::json{
        {"name", p.name}, {"version", p.version},
        {"description", p.description}, {"author", p.author},
        {"downloads", p.downloads}
    };
}

struct CatalogPackageList {
    uint64_t count = 0;
    std::vector<CatalogPackage> packages;
};

inline void from_json(const nlohmann::json& j, CatalogPackageList& l) {
    j.at("count").get_to(l.count);
    j.at("packages").get_to(l.packages);
}

inline void to_json(nlohmann::json& j, const CatalogPackageList& l) {
    j = nlohmann::json{{"count", l.count}, {"packages", l.packages}};
}

struct CatalogSearchResult {
    std::string name;
    std::string version;
    std::string description;
    double relevance = 0.0;
};

inline void from_json(const nlohmann::json& j, CatalogSearchResult& r) {
    j.at("name").get_to(r.name);
    j.at("version").get_to(r.version);
    j.at("description").get_to(r.description);
    j.at("relevance").get_to(r.relevance);
}

inline void to_json(nlohmann::json& j, const CatalogSearchResult& r) {
    j = nlohmann::json{
        {"name", r.name}, {"version", r.version},
        {"description", r.description}, {"relevance", r.relevance}
    };
}

struct CatalogSearchResults {
    uint64_t count = 0;
    std::vector<CatalogSearchResult> results;
};

inline void from_json(const nlohmann::json& j, CatalogSearchResults& r) {
    j.at("count").get_to(r.count);
    j.at("results").get_to(r.results);
}

inline void to_json(nlohmann::json& j, const CatalogSearchResults& r) {
    j = nlohmann::json{{"count", r.count}, {"results", r.results}};
}

struct CatalogPackageInfo {
    std::string name;
    std::string version;
    std::string description;
    std::string author;
    uint64_t downloads = 0;
};

inline void from_json(const nlohmann::json& j, CatalogPackageInfo& p) {
    j.at("name").get_to(p.name);
    j.at("version").get_to(p.version);
    j.at("description").get_to(p.description);
    j.at("author").get_to(p.author);
    j.at("downloads").get_to(p.downloads);
}

inline void to_json(nlohmann::json& j, const CatalogPackageInfo& p) {
    j = nlohmann::json{
        {"name", p.name}, {"version", p.version},
        {"description", p.description}, {"author", p.author},
        {"downloads", p.downloads}
    };
}

struct CatalogRegistryStats {
    uint64_t package_count = 0;
    uint64_t publisher_count = 0;
    uint64_t total_downloads = 0;
};

inline void from_json(const nlohmann::json& j, CatalogRegistryStats& s) {
    j.at("package_count").get_to(s.package_count);
    j.at("publisher_count").get_to(s.publisher_count);
    j.at("total_downloads").get_to(s.total_downloads);
}

inline void to_json(nlohmann::json& j, const CatalogRegistryStats& s) {
    j = nlohmann::json{
        {"package_count", s.package_count},
        {"publisher_count", s.publisher_count},
        {"total_downloads", s.total_downloads}
    };
}

} // namespace hypermesh
