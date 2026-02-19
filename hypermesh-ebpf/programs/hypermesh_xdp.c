// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// HyperMesh Unified XDP Filter - Kernel-level packet classification
// Originally extracted from stoq/src/transport/ebpf/loader.rs
// Renamed from stoq_xdp.c to hypermesh_xdp.c as the unified program
// for the entire HyperMesh eBPF subsystem.
//
// Capabilities:
//   - IPv6-only QUIC/UDP filtering on HYPERMESH_PORT (9292)
//   - Connection tracking with per-CPU statistics
//   - Filter rules (PASS / DROP / REDIRECT)
//   - HyperMesh extension header parsing (PoS, Asset, Matrix, Privacy)
//   - Policy-based validation (PoS, asset hash, matrix routing)
//   - AF_XDP zero-copy redirect via xsk_map

#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/ipv6.h>
#include <linux/udp.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_endian.h>

#define HYPERMESH_PORT 9292

/* ============================================================
 * Struct definitions
 * ============================================================ */

struct conn_key {
    __u8 src_ip[16];
    __u8 dst_ip[16];
    __u16 src_port;
    __u16 dst_port;
};

struct conn_info {
    __u64 packets;
    __u64 bytes;
    __u64 last_seen;
};

struct xdp_stats {
    __u64 packets_passed;
    __u64 packets_dropped;
    __u64 packets_redirected;
    __u64 bytes_processed;
};

struct filter_key {
    __u8 src_ip[16];
    __u8 dst_ip[16];
};

/* Policy enforcement - populated from userspace per connection */
struct policy_value {
    __u32 requires_pos;         /* Require PoS validation */
    __u32 validate_asset_hash;  /* Validate asset hashes */
    __u32 check_matrix_routing; /* Check matrix routing */
    __u32 privacy_tier;         /* 0=ANONYMOUS, 2=PRIVATE, 3=PUBLIC */
};

/* PoS header validation state - populated/updated from userspace */
struct pos_validation {
    __u8  algorithm;      /* 0x01=FALCON, 0x02=Ed25519, 0x03=ECDSA */
    __u32 difficulty;     /* Required difficulty (leading zero bits) */
    __u8  validated;      /* 1=passed validation, 0=pending */
    __u64 last_validated; /* Timestamp of last validation (ns) */
};

/* Asset hash registry entry - populated from userspace */
struct asset_hash_entry {
    __u8  expected_hash[32]; /* BLAKE3 hash */
    __u32 shard_count;       /* Total shards */
    __u8  registered;        /* 1=registered on blockchain, 0=pending */
};

/* ============================================================
 * BPF maps
 * ============================================================ */

/* Connection tracking map - keyed by 4-tuple */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 65536);
    __type(key, struct conn_key);
    __type(value, struct conn_info);
} connection_map SEC(".maps");

/* Per-CPU statistics */
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct xdp_stats);
} stats_map SEC(".maps");

/* Filter rules map - keyed by src/dst IP pair, value is action */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, struct filter_key);
    __type(value, __u32);
} filter_map SEC(".maps");

/* Policy enforcement map - keyed by connection, value is policy flags */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 65536);
    __type(key, struct conn_key);
    __type(value, struct policy_value);
} policy_map SEC(".maps");

/* PoS header validation map - keyed by IPv6 source address (16 bytes) */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 65536);
    __type(key, __u8[16]);
    __type(value, struct pos_validation);
} pos_header_map SEC(".maps");

/* Asset hash validation map - keyed by asset ID (32 bytes) */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 100000);
    __type(key, __u8[32]);
    __type(value, struct asset_hash_entry);
} asset_hash_map SEC(".maps");

/* XSK map for AF_XDP zero-copy socket redirect */
struct {
    __uint(type, BPF_MAP_TYPE_XSKMAP);
    __uint(max_entries, 64);  /* Up to 64 queues */
    __type(key, __u32);
    __type(value, __u32);
} xsk_map SEC(".maps");

/* ============================================================
 * HyperMesh extension header definitions
 *
 * These headers are embedded in the QUIC/UDP payload immediately
 * after the UDP header. They allow kernel-level inspection of
 * HyperMesh protocol metadata before passing to userspace.
 * ============================================================ */

#define HMESH_HDR_MAGIC   0x484D  /* 'HM' in network byte order */
#define HMESH_HDR_POS     0x01    /* PoS (Proof of State) header */
#define HMESH_HDR_ASSET   0x02    /* Asset hash header */
#define HMESH_HDR_MATRIX  0x03    /* Matrix routing header */
#define HMESH_HDR_PRIVACY 0x04    /* Privacy tier header */

/* Common header preceding every extension */
struct hmesh_header {
    __u16 magic;     /* Must be HMESH_HDR_MAGIC (0x484D) */
    __u8  type;      /* One of HMESH_HDR_* constants */
    __u8  length;    /* Payload length following this header */
};

/* PoS proof summary carried in-band */
struct hmesh_pos_header {
    __u8  algorithm;   /* 0x01=FALCON, 0x02=Ed25519, 0x03=ECDSA */
    __u32 difficulty;  /* Required difficulty (leading zero bits) */
    __u8  hash[32];    /* Work hash (first 32 bytes of proof) */
};

/* Asset identification and integrity header */
struct hmesh_asset_header {
    __u8  asset_id[32]; /* Asset identifier (ContentHash) */
    __u8  hash[32];     /* Content hash (BLAKE3) */
    __u16 shard_index;  /* Current shard index */
    __u16 shard_total;  /* Total shard count */
};

/* Block-MATRIX topology header (3x float32 position + hop count) */
struct hmesh_matrix_header {
    __u8  position[12]; /* 3x f32: x, y, z packed */
    __u8  hop_count;    /* Routing hop count */
};

/* ============================================================
 * Inline helpers
 * ============================================================ */

/*
 * validate_pos_for_source - Check if source IP has a valid PoS entry
 *
 * Returns 1 if validated, 0 otherwise.
 */
static __always_inline int validate_pos_for_source(__u8 src_ip[16])
{
    struct pos_validation *val;

    val = bpf_map_lookup_elem(&pos_header_map, src_ip);
    if (!val)
        return 0; /* No entry -> not validated */

    return val->validated == 1;
}

/*
 * validate_asset_registered - Check if an asset ID is registered
 *
 * Returns 1 if registered, 0 otherwise.
 */
static __always_inline int validate_asset_registered(__u8 asset_id[32])
{
    struct asset_hash_entry *entry;

    entry = bpf_map_lookup_elem(&asset_hash_map, asset_id);
    if (!entry)
        return 0; /* Unknown asset */

    return entry->registered == 1;
}

/*
 * validate_matrix_position - Check that matrix position is non-zero
 *
 * A zero position (all 12 bytes == 0) means the node hasn't been
 * placed in the matrix yet, which is invalid for routed traffic.
 * Returns 1 if valid (non-zero), 0 if invalid (all zeros).
 */
static __always_inline int validate_matrix_position(__u8 position[12])
{
    /* Check each 4-byte word; if any is non-zero the position is valid */
    __u32 a = 0, b = 0, c = 0;

    __builtin_memcpy(&a, position,     4);
    __builtin_memcpy(&b, position + 4, 4);
    __builtin_memcpy(&c, position + 8, 4);

    return (a | b | c) != 0;
}

/* ============================================================
 * Main XDP program
 * ============================================================ */

SEC("xdp/hypermesh_filter")
int hypermesh_xdp_filter(struct xdp_md *ctx)
{
    void *data     = (void *)(long)ctx->data;
    void *data_end = (void *)(long)ctx->data_end;

    struct ethhdr *eth = data;
    struct xdp_stats *stats;
    __u32 key = 0;

    /* Get per-CPU stats */
    stats = bpf_map_lookup_elem(&stats_map, &key);
    if (!stats)
        return XDP_PASS;

    /* Verify ethernet header bounds */
    if (data + sizeof(*eth) > data_end)
        return XDP_DROP;

    /* Only process IPv6 packets (HyperMesh is IPv6-only) */
    if (bpf_ntohs(eth->h_proto) != ETH_P_IPV6) {
        stats->packets_dropped++;
        return XDP_DROP;
    }

    struct ipv6hdr *ip6 = data + sizeof(*eth);
    if (data + sizeof(*eth) + sizeof(*ip6) > data_end)
        return XDP_DROP;

    /* Only process UDP packets (QUIC runs over UDP) */
    if (ip6->nexthdr != IPPROTO_UDP)
        return XDP_PASS;

    struct udphdr *udp = data + sizeof(*eth) + sizeof(*ip6);
    if (data + sizeof(*eth) + sizeof(*ip6) + sizeof(*udp) > data_end)
        return XDP_DROP;

    /* Check if it's HyperMesh/STOQ traffic (port 9292) */
    if (bpf_ntohs(udp->dest) != HYPERMESH_PORT &&
        bpf_ntohs(udp->source) != HYPERMESH_PORT)
        return XDP_PASS;

    /* --------------------------------------------------------
     * HyperMesh extension header parsing
     *
     * After the UDP header, look for the HyperMesh magic bytes
     * (0x484D). If present, parse extension headers and apply
     * policy-based validation before forwarding.
     * -------------------------------------------------------- */

    /* Pointer to start of UDP payload (after UDP header) */
    void *payload = data + sizeof(*eth) + sizeof(*ip6) + sizeof(*udp);

    /* Build connection key for policy and connection tracking */
    struct conn_key conn = {};
    __builtin_memcpy(conn.src_ip, &ip6->saddr, 16);
    __builtin_memcpy(conn.dst_ip, &ip6->daddr, 16);
    conn.src_port = udp->source;
    conn.dst_port = udp->dest;

    /* Track whether we found HyperMesh headers and if policy passed */
    int policy_checked = 0;
    int policy_passed  = 1; /* Assume pass unless policy says otherwise */

    /* Check if payload contains at least a HyperMesh common header */
    if (payload + sizeof(struct hmesh_header) <= data_end) {
        struct hmesh_header hdr = {};
        __builtin_memcpy(&hdr, payload, sizeof(struct hmesh_header));

        /* Verify HyperMesh magic bytes */
        if (bpf_ntohs(hdr.magic) == HMESH_HDR_MAGIC) {
            /* Found HyperMesh header - look up policy for this connection */
            struct policy_value *policy;
            policy = bpf_map_lookup_elem(&policy_map, &conn);

            if (policy) {
                policy_checked = 1;
                void *ext_payload = payload + sizeof(struct hmesh_header);

                /* --- PoS validation --- */
                if (policy->requires_pos) {
                    if (hdr.type == HMESH_HDR_POS &&
                        ext_payload + sizeof(struct hmesh_pos_header) <= data_end) {
                        /*
                         * Packet carries a PoS header; verify
                         * the source IP has been validated in the
                         * pos_header_map (populated by userspace
                         * after full cryptographic verification).
                         */
                        if (!validate_pos_for_source(conn.src_ip)) {
                            policy_passed = 0;
                        }
                    } else {
                        /*
                         * Policy requires PoS but this packet
                         * either has a different header type or
                         * the PoS header is truncated.  Check if
                         * the source has a prior validation.
                         */
                        if (!validate_pos_for_source(conn.src_ip)) {
                            policy_passed = 0;
                        }
                    }
                }

                /* --- Asset hash validation --- */
                if (policy->validate_asset_hash) {
                    if (hdr.type == HMESH_HDR_ASSET &&
                        ext_payload + sizeof(struct hmesh_asset_header) <= data_end) {
                        /*
                         * Extract the asset_id from the header
                         * and verify it is registered in the
                         * asset_hash_map.
                         */
                        struct hmesh_asset_header ahdr = {};
                        __builtin_memcpy(&ahdr, ext_payload,
                                         sizeof(struct hmesh_asset_header));

                        if (!validate_asset_registered(ahdr.asset_id)) {
                            policy_passed = 0;
                        }
                    }
                    /* If header type is not ASSET, skip this check
                     * (the packet may carry a different extension). */
                }

                /* --- Matrix routing validation --- */
                if (policy->check_matrix_routing) {
                    if (hdr.type == HMESH_HDR_MATRIX &&
                        ext_payload + sizeof(struct hmesh_matrix_header) <= data_end) {
                        /*
                         * Verify the matrix position is non-zero.
                         * A zero position means the sender hasn't
                         * been placed in the Block-MATRIX topology.
                         */
                        struct hmesh_matrix_header mhdr = {};
                        __builtin_memcpy(&mhdr, ext_payload,
                                         sizeof(struct hmesh_matrix_header));

                        if (!validate_matrix_position(mhdr.position)) {
                            policy_passed = 0;
                        }
                    }
                    /* If header type is not MATRIX, skip this check. */
                }
            }
            /* If no policy entry exists for this connection, all
             * HyperMesh traffic is allowed through (open policy). */
        }
    }

    /* If policy enforcement was applied and failed, drop the packet */
    if (policy_checked && !policy_passed) {
        stats->packets_dropped++;
        return XDP_DROP;
    }

    /* --------------------------------------------------------
     * AF_XDP redirect path
     *
     * If policy passed (or no policy was configured) and an
     * AF_XDP socket is bound on this queue, redirect for
     * zero-copy processing in userspace.
     * -------------------------------------------------------- */
    __u32 queue_idx = ctx->rx_queue_index;
    __u32 *xsk_entry = bpf_map_lookup_elem(&xsk_map, &queue_idx);
    if (xsk_entry) {
        stats->packets_redirected++;
        return bpf_redirect_map(&xsk_map, queue_idx, XDP_PASS);
    }

    /* --------------------------------------------------------
     * Connection tracking update
     * -------------------------------------------------------- */
    struct conn_info *info = bpf_map_lookup_elem(&connection_map, &conn);
    if (info) {
        info->packets++;
        info->bytes += data_end - data;
        info->last_seen = bpf_ktime_get_ns();
    } else {
        struct conn_info new_info = {
            .packets = 1,
            .bytes = data_end - data,
            .last_seen = bpf_ktime_get_ns()
        };
        bpf_map_update_elem(&connection_map, &conn, &new_info, BPF_ANY);
    }

    /* --------------------------------------------------------
     * Filter rules check
     * -------------------------------------------------------- */
    struct filter_key filter = {};
    __builtin_memcpy(filter.src_ip, &ip6->saddr, 16);
    __builtin_memcpy(filter.dst_ip, &ip6->daddr, 16);

    __u32 *action = bpf_map_lookup_elem(&filter_map, &filter);
    if (action) {
        switch (*action) {
            case 1: /* DROP */
                stats->packets_dropped++;
                return XDP_DROP;
            case 3: /* REDIRECT to AF_XDP */
                stats->packets_redirected++;
                return bpf_redirect_map(&xsk_map, ctx->rx_queue_index, XDP_PASS);
            default:
                break;
        }
    }

    /* Update statistics and pass packet to kernel stack */
    stats->packets_passed++;
    stats->bytes_processed += data_end - data;

    return XDP_PASS;
}

char _license[] SEC("license") = "GPL";
