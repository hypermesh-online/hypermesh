// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// HyperMesh Unified XDP Filter - Kernel-level packet classification
// Originally extracted from stoq/src/transport/ebpf/loader.rs
// Renamed from stoq_xdp.c to hypermesh_xdp.c as the unified program
// for the entire HyperMesh eBPF subsystem.
//
// Honest gate (F10 reframe):
//   STOQ is standard ENCRYPTED QUIC (quinn + rustls). The kernel cannot
//   read the QUIC-encrypted payload, so it drops/admits by what it CAN
//   see in cleartext: the IPv6 source, the UDP port, and a PoS-authenticated
//   -peer allowlist that userspace populates AFTER the bilateral-PoS
//   handshake succeeds. There is NO in-payload "extension header" parse —
//   that magic (0x484D) was never emitted by any Rust code and would be
//   inside the encrypted payload the kernel cannot inspect.
//
// Capabilities:
//   - IPv6-only QUIC/UDP filtering on HYPERMESH_PORT (9292)
//   - PoS-authenticated-peer allowlist enforcement (policy_map + pos_header_map)
//   - Connection tracking with per-CPU statistics
//   - Filter rules (PASS / DROP / REDIRECT)
//   - AF_XDP zero-copy redirect via xsk_map

#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/in.h>
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

/* Policy enforcement - populated from userspace per source address.
 *
 * `requires_pos` is derived directly from the source's privacy tier:
 *   Anonymous  -> requires_pos = 0 (open, admitted without PoS)
 *   Private    -> requires_pos = 1 (must be PoS-authenticated)
 *   Public     -> requires_pos = 1 (must be PoS-authenticated)
 * (see ValidationPolicy::for_privacy_tier in policy_maps.rs). */
struct policy_value {
    __u32 requires_pos;         /* Require PoS validation */
    __u32 validate_asset_hash;  /* Validate asset hashes (userspace only) */
    __u32 check_matrix_routing; /* Check matrix routing (userspace only) */
    __u32 privacy_tier;         /* 0=ANONYMOUS, 2=PRIVATE, 3=PUBLIC */
};

/* PoS authentication state - populated from userspace on handshake success.
 *
 * `validated == 1` means userspace completed bilateral Proof-of-State
 * (FALCON-1024) verification for this source and mirrored the result into
 * the kernel via `set_peer_pos_validated` -> `update_pos_header_map`. */
struct pos_validation {
    __u8  algorithm;      /* 0x01=FALCON, 0x02=Ed25519, 0x03=ECDSA */
    __u32 difficulty;     /* Required difficulty (leading zero bits) */
    __u8  validated;      /* 1=passed validation, 0=pending */
    __u64 last_validated; /* Timestamp of last validation (ns) */
};

/* Asset hash registry entry - populated from userspace.
 *
 * Populated by `register_asset_hash` -> `update_asset_hash_map` for
 * userspace bookkeeping and future kernel use. Not consulted by the
 * honest kernel gate (the kernel cannot read the encrypted payload that
 * would carry an asset reference), but kept so the live Rust writer has a
 * map to target. */
struct asset_hash_entry {
    __u8  expected_hash[32]; /* BLAKE3 hash */
    __u32 shard_count;       /* Total shards */
    __u8  registered;        /* 1=registered on blockchain, 0=pending */
};

/* Matrix-topology forwarding rule - populated from userspace.
 *
 * Written by `update_routing_map` (set_routing_rule / set_matrix_route_active).
 * Reserved for the P8 cross-device XDP_TX delegation path; the honest gate
 * in this build does not read it in-kernel (no payload parse), but the map
 * is retained so the live Rust writer has a map to target. */
struct route_value {
    __u32 egress_ifindex; /* Interface index to forward on */
    __u32 active;         /* 1=forward via XDP_TX/redirect, 0=inactive */
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

/*
 * Policy enforcement map - keyed by the 16-byte IPv6 source address.
 *
 * Populated by userspace (`register_authenticated_peer` / `set_privacy_tier`)
 * so `requires_pos=1` is set for exactly the peers whose privacy tier demands
 * PoS. The honest kernel gate consults this UNCONDITIONALLY for HyperMesh
 * traffic.
 */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 65536);
    __type(key, __u8[16]);
    __type(value, struct policy_value);
} policy_map SEC(".maps");

/* PoS authentication map - keyed by IPv6 source address (16 bytes).
 * `validated=1` is written by userspace after bilateral PoS succeeds. */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 65536);
    __type(key, __u8[16]);
    __type(value, struct pos_validation);
} pos_header_map SEC(".maps");

/* Asset hash map - keyed by 32-byte BLAKE3 content hash. Userspace-populated
 * bookkeeping (see struct asset_hash_entry). */
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

/* Matrix-topology forwarding map - keyed by raw 12-byte matrix position
 * (3x f32 LE). Userspace-populated (see struct route_value). */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 65536);
    __type(key, __u8[12]); /* raw matrix position (3x f32) */
    __type(value, struct route_value);
} routing_map SEC(".maps");

/* ============================================================
 * Main XDP program
 * ============================================================ */

/* aya 0.12 (aya-obj 0.2.1) parses the piece after "xdp/" as an ATTACH TYPE and
 * accepts only cpumap/devmap/none — a program name there is rejected as an
 * invalid section. Use bare SEC("xdp"); aya derives the program name from the
 * C function name (hypermesh_xdp_filter), which is what manager.rs:235 loads. */
SEC("xdp")
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
     * PoS-authenticated-peer allowlist (the honest gate)
     *
     * After confirming this is HyperMesh/STOQ traffic (port 9292),
     * consult the allowlist userspace populates on handshake success.
     * The kernel CANNOT read the QUIC-encrypted payload, so this is
     * the only trustworthy signal available at wire speed:
     *   - policy_map[src].requires_pos: does this source's privacy tier
     *     demand PoS? (Anonymous -> 0, Private/Public -> 1)
     *   - pos_header_map[src].validated: did userspace complete bilateral
     *     PoS (FALCON-1024) for this source?
     *
     * Honors the privacy tier: an Anonymous/open source (no policy entry
     * or requires_pos==0) PASSES; a Private/Public source that has not
     * been PoS-authenticated is DROPPED.
     * -------------------------------------------------------- */
    __u8 src_ip[16];
    __builtin_memcpy(src_ip, &ip6->saddr, 16);

    struct policy_value *pol = bpf_map_lookup_elem(&policy_map, src_ip);
    if (pol && pol->requires_pos) {
        struct pos_validation *pv =
            bpf_map_lookup_elem(&pos_header_map, src_ip);
        if (!pv || pv->validated != 1) {
            /* Source's tier requires PoS but it is not authenticated. */
            stats->packets_dropped++;
            return XDP_DROP;
        }
    }
    /* Else: no policy entry, or an open (requires_pos==0) tier — fall
     * through to the conntrack / filter / AF_XDP path below. */

    /* Build connection key for connection tracking */
    struct conn_key conn = {};
    __builtin_memcpy(conn.src_ip, &ip6->saddr, 16);
    __builtin_memcpy(conn.dst_ip, &ip6->daddr, 16);
    conn.src_port = udp->source;
    conn.dst_port = udp->dest;

    /* --------------------------------------------------------
     * AF_XDP redirect path
     *
     * If an AF_XDP socket is bound on this queue, redirect for
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
