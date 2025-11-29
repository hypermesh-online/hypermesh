// XDP Packet Counter - Basic eBPF program for HyperMesh
// This program counts packets without dropping them

#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <bpf/bpf_helpers.h>

// Map to store packet and byte counters
struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __uint(max_entries, 4);
    __type(key, __u32);
    __type(value, __u64);
} packet_stats SEC(".maps");

// Counter indexes
#define PACKETS_TOTAL    0
#define BYTES_TOTAL      1
#define PACKETS_IPV4     2
#define PACKETS_IPV6     3

SEC("xdp")
int xdp_packet_counter(struct xdp_md *ctx)
{
    void *data_end = (void *)(long)ctx->data_end;
    void *data = (void *)(long)ctx->data;
    struct ethhdr *eth = data;
    __u32 key;
    __u64 *value;
    __u64 packet_size;

    // Check packet has at least Ethernet header
    if (data + sizeof(*eth) > data_end)
        return XDP_PASS;

    // Calculate packet size
    packet_size = data_end - data;

    // Increment total packet counter
    key = PACKETS_TOTAL;
    value = bpf_map_lookup_elem(&packet_stats, &key);
    if (value)
        __sync_fetch_and_add(value, 1);

    // Increment total bytes counter
    key = BYTES_TOTAL;
    value = bpf_map_lookup_elem(&packet_stats, &key);
    if (value)
        __sync_fetch_and_add(value, packet_size);

    // Count IPv4 vs IPv6
    if (eth->h_proto == __constant_htons(ETH_P_IP)) {
        key = PACKETS_IPV4;
        value = bpf_map_lookup_elem(&packet_stats, &key);
        if (value)
            __sync_fetch_and_add(value, 1);
    } else if (eth->h_proto == __constant_htons(ETH_P_IPV6)) {
        key = PACKETS_IPV6;
        value = bpf_map_lookup_elem(&packet_stats, &key);
        if (value)
            __sync_fetch_and_add(value, 1);
    }

    // Always pass packets through (no drops)
    return XDP_PASS;
}

char _license[] SEC("license") = "GPL";