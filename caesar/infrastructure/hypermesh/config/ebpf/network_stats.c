// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// eBPF program for network statistics collection
// Gateway Coin hypermesh infrastructure monitoring

#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/tcp.h>
#include <linux/udp.h>
#include <linux/in.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_endian.h>

// License required for eBPF
char _license[] SEC("license") = "GPL";

// Statistics structure
struct network_stats {
    __u64 packet_count;
    __u64 byte_count;
    __u64 tcp_packets;
    __u64 udp_packets;
    __u64 quic_packets;
    __u64 connection_count;
    __u64 tcp_retransmissions;
    __u64 udp_drops;
    __u64 invalid_packets;
    __u64 timestamp;
};

// Connection tracking structure
struct connection_key {
    __u32 src_ip;
    __u32 dst_ip;
    __u16 src_port;
    __u16 dst_port;
    __u8 protocol;
};

// Maps for statistics storage
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct network_stats);
} stats_map SEC(".maps");

struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, struct connection_key);
    __type(value, __u64);
} connection_map SEC(".maps");

// QUIC packet detection (UDP on port 4001)
static __always_inline int is_quic_packet(__u16 port) {
    return (port == 4001);
}

// Update network statistics
static __always_inline void update_stats(__u64 bytes, __u8 protocol, __u16 dst_port) {
    __u32 key = 0;
    struct network_stats *stats = bpf_map_lookup_elem(&stats_map, &key);
    
    if (stats) {
        __sync_fetch_and_add(&stats->packet_count, 1);
        __sync_fetch_and_add(&stats->byte_count, bytes);
        
        if (protocol == IPPROTO_TCP) {
            __sync_fetch_and_add(&stats->tcp_packets, 1);
        } else if (protocol == IPPROTO_UDP) {
            __sync_fetch_and_add(&stats->udp_packets, 1);
            if (is_quic_packet(dst_port)) {
                __sync_fetch_and_add(&stats->quic_packets, 1);
            }
        }
        
        stats->timestamp = bpf_ktime_get_ns();
    }
}

// Track connections
static __always_inline void track_connection(__u32 src_ip, __u32 dst_ip, 
                                           __u16 src_port, __u16 dst_port, 
                                           __u8 protocol) {
    struct connection_key key = {
        .src_ip = src_ip,
        .dst_ip = dst_ip,
        .src_port = src_port,
        .dst_port = dst_port,
        .protocol = protocol
    };
    
    __u64 *count = bpf_map_lookup_elem(&connection_map, &key);
    if (count) {
        __sync_fetch_and_add(count, 1);
    } else {
        __u64 new_count = 1;
        bpf_map_update_elem(&connection_map, &key, &new_count, BPF_ANY);
    }
}

// XDP program for packet processing
SEC("xdp")
int network_stats_xdp(struct xdp_md *ctx) {
    void *data_end = (void *)(long)ctx->data_end;
    void *data = (void *)(long)ctx->data;
    
    struct ethhdr *eth = data;
    
    // Bounds checking
    if ((void *)(eth + 1) > data_end) {
        return XDP_PASS;
    }
    
    // Only process IPv4 packets
    if (eth->h_proto != bpf_htons(ETH_P_IP)) {
        return XDP_PASS;
    }
    
    struct iphdr *ip = (struct iphdr *)(eth + 1);
    if ((void *)(ip + 1) > data_end) {
        return XDP_PASS;
    }
    
    // Validate IP header
    if (ip->version != 4 || ip->ihl < 5) {
        __u32 key = 0;
        struct network_stats *stats = bpf_map_lookup_elem(&stats_map, &key);
        if (stats) {
            __sync_fetch_and_add(&stats->invalid_packets, 1);
        }
        return XDP_PASS;
    }
    
    __u64 packet_size = data_end - data;
    __u16 src_port = 0, dst_port = 0;
    
    // Extract port information based on protocol
    if (ip->protocol == IPPROTO_TCP) {
        struct tcphdr *tcp = (struct tcphdr *)((char *)ip + (ip->ihl * 4));
        if ((void *)(tcp + 1) > data_end) {
            return XDP_PASS;
        }
        src_port = bpf_ntohs(tcp->source);
        dst_port = bpf_ntohs(tcp->dest);
        
        // Track TCP retransmissions (simplified detection)
        if (tcp->syn && tcp->ack) {
            // This is a connection establishment, track it
            track_connection(ip->saddr, ip->daddr, src_port, dst_port, IPPROTO_TCP);
        }
    } else if (ip->protocol == IPPROTO_UDP) {
        struct udphdr *udp = (struct udphdr *)((char *)ip + (ip->ihl * 4));
        if ((void *)(udp + 1) > data_end) {
            return XDP_PASS;
        }
        src_port = bpf_ntohs(udp->source);
        dst_port = bpf_ntohs(udp->dest);
        
        // Track UDP "connections" for QUIC
        if (is_quic_packet(dst_port) || is_quic_packet(src_port)) {
            track_connection(ip->saddr, ip->daddr, src_port, dst_port, IPPROTO_UDP);
        }
    }
    
    // Update statistics
    update_stats(packet_size, ip->protocol, dst_port);
    
    return XDP_PASS;
}

// TC program for egress traffic
SEC("tc")
int network_stats_tc_egress(struct __sk_buff *skb) {
    void *data_end = (void *)(long)skb->data_end;
    void *data = (void *)(long)skb->data;
    
    struct ethhdr *eth = data;
    
    if ((void *)(eth + 1) > data_end) {
        return TC_ACT_OK;
    }
    
    if (eth->h_proto != bpf_htons(ETH_P_IP)) {
        return TC_ACT_OK;
    }
    
    struct iphdr *ip = (struct iphdr *)(eth + 1);
    if ((void *)(ip + 1) > data_end) {
        return TC_ACT_OK;
    }
    
    __u64 packet_size = skb->len;
    __u16 dst_port = 0;
    
    if (ip->protocol == IPPROTO_UDP) {
        struct udphdr *udp = (struct udphdr *)((char *)ip + (ip->ihl * 4));
        if ((void *)(udp + 1) > data_end) {
            return TC_ACT_OK;
        }
        dst_port = bpf_ntohs(udp->dest);
    }
    
    // Update egress statistics
    update_stats(packet_size, ip->protocol, dst_port);
    
    return TC_ACT_OK;
}

// Socket filter for connection tracking
SEC("socket")
int network_stats_socket(struct __sk_buff *skb) {
    // Simple socket-level statistics
    __u32 key = 0;
    struct network_stats *stats = bpf_map_lookup_elem(&stats_map, &key);
    
    if (stats) {
        __sync_fetch_and_add(&stats->connection_count, 1);
        stats->timestamp = bpf_ktime_get_ns();
    }
    
    return 0;
}

// Helper function to get current statistics (called from userspace)
SEC("kprobe/tcp_retransmit_skb")
int trace_tcp_retransmit(struct pt_regs *ctx) {
    __u32 key = 0;
    struct network_stats *stats = bpf_map_lookup_elem(&stats_map, &key);
    
    if (stats) {
        __sync_fetch_and_add(&stats->tcp_retransmissions, 1);
    }
    
    return 0;
}