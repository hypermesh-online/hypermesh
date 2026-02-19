// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// HyperMesh Unified XDP Filter - Kernel-level packet classification
// Originally extracted from stoq/src/transport/ebpf/loader.rs
// Renamed from stoq_xdp.c to hypermesh_xdp.c as the unified program
// for the entire HyperMesh eBPF subsystem.

#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/ipv6.h>
#include <linux/udp.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_endian.h>

#define HYPERMESH_PORT 9292

/* Connection tracking map */
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

/* Filter rules map */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, struct filter_key);
    __type(value, __u32);
} filter_map SEC(".maps");

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

SEC("xdp/hypermesh_filter")
int hypermesh_xdp_filter(struct xdp_md *ctx) {
    void *data = (void *)(long)ctx->data;
    void *data_end = (void *)(long)ctx->data_end;

    struct ethhdr *eth = data;
    struct xdp_stats *stats;
    __u32 key = 0;

    /* Get per-CPU stats */
    stats = bpf_map_lookup_elem(&stats_map, &key);
    if (!stats)
        return XDP_PASS;

    /* Verify ethernet header */
    if (data + sizeof(*eth) > data_end)
        return XDP_DROP;

    /* Only process IPv6 packets */
    if (bpf_ntohs(eth->h_proto) != ETH_P_IPV6) {
        stats->packets_dropped++;
        return XDP_DROP;
    }

    struct ipv6hdr *ip6 = data + sizeof(*eth);
    if (data + sizeof(*eth) + sizeof(*ip6) > data_end)
        return XDP_DROP;

    /* Only process UDP packets */
    if (ip6->nexthdr != IPPROTO_UDP)
        return XDP_PASS;

    struct udphdr *udp = data + sizeof(*eth) + sizeof(*ip6);
    if (data + sizeof(*eth) + sizeof(*ip6) + sizeof(*udp) > data_end)
        return XDP_DROP;

    /* Check if it's HyperMesh/STOQ traffic (port 9292) */
    if (bpf_ntohs(udp->dest) != HYPERMESH_PORT && bpf_ntohs(udp->source) != HYPERMESH_PORT)
        return XDP_PASS;

    /* Update connection tracking */
    struct conn_key conn = {};
    __builtin_memcpy(conn.src_ip, &ip6->saddr, 16);
    __builtin_memcpy(conn.dst_ip, &ip6->daddr, 16);
    conn.src_port = udp->source;
    conn.dst_port = udp->dest;

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

    /* Check filter rules */
    struct filter_key filter = {};
    __builtin_memcpy(filter.src_ip, &ip6->saddr, 16);
    __builtin_memcpy(filter.dst_ip, &ip6->daddr, 16);

    __u32 *action = bpf_map_lookup_elem(&filter_map, &filter);
    if (action) {
        switch (*action) {
            case 1: /* DROP */
                stats->packets_dropped++;
                return XDP_DROP;
            case 3: /* REDIRECT */
                stats->packets_redirected++;
                return XDP_REDIRECT;
            default:
                break;
        }
    }

    /* Update statistics and pass packet */
    stats->packets_passed++;
    stats->bytes_processed += data_end - data;

    return XDP_PASS;
}

char _license[] SEC("license") = "GPL";
