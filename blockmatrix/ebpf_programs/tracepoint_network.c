// Tracepoint Network Monitor - Track network events for HyperMesh
// This program monitors network socket operations

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Network event types
enum net_event_type {
    NET_EVENT_CONNECT = 1,
    NET_EVENT_ACCEPT = 2,
    NET_EVENT_SEND = 3,
    NET_EVENT_RECV = 4,
    NET_EVENT_CLOSE = 5,
};

// Structure for network events
struct net_event {
    __u32 pid;
    __u32 event_type;
    __u64 bytes;  // For send/recv events
    __u64 timestamp;
};

// Per-CPU array for aggregating stats
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 5);  // One per event type
    __type(key, __u32);
    __type(value, __u64);
} net_stats SEC(".maps");

// Ring buffer for detailed events
struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 512 * 1024);  // 512KB buffer
} net_events SEC(".maps");

// Helper to record event
static __always_inline void record_event(__u32 event_type, __u64 bytes)
{
    struct net_event *event;
    __u64 *stat;
    __u32 key = event_type - 1;  // Convert to 0-based index

    // Update statistics
    stat = bpf_map_lookup_elem(&net_stats, &key);
    if (stat)
        __sync_fetch_and_add(stat, 1);

    // Record detailed event
    event = bpf_ringbuf_reserve(&net_events, sizeof(*event), 0);
    if (!event)
        return;

    event->pid = bpf_get_current_pid_tgid() >> 32;
    event->event_type = event_type;
    event->bytes = bytes;
    event->timestamp = bpf_ktime_get_ns();

    bpf_ringbuf_submit(event, 0);
}

SEC("tracepoint/syscalls/sys_enter_connect")
int trace_connect_enter(void *ctx)
{
    record_event(NET_EVENT_CONNECT, 0);
    return 0;
}

SEC("tracepoint/syscalls/sys_enter_accept")
int trace_accept_enter(void *ctx)
{
    record_event(NET_EVENT_ACCEPT, 0);
    return 0;
}

SEC("tracepoint/syscalls/sys_exit_sendto")
int trace_sendto_exit(struct trace_event_raw_sys_exit *ctx)
{
    if (ctx->ret > 0)
        record_event(NET_EVENT_SEND, ctx->ret);
    return 0;
}

SEC("tracepoint/syscalls/sys_exit_recvfrom")
int trace_recvfrom_exit(struct trace_event_raw_sys_exit *ctx)
{
    if (ctx->ret > 0)
        record_event(NET_EVENT_RECV, ctx->ret);
    return 0;
}

SEC("tracepoint/syscalls/sys_enter_close")
int trace_close_enter(void *ctx)
{
    // Note: This catches all close() calls, not just sockets
    // In production, would filter for socket FDs only
    record_event(NET_EVENT_CLOSE, 0);
    return 0;
}

char _license[] SEC("license") = "GPL";