// Kprobe Execve Monitor - Track process execution for HyperMesh
// This program monitors sys_execve calls to track new processes

#include <linux/bpf.h>
#include <linux/ptrace.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Structure to store process execution info
struct exec_event {
    __u32 pid;
    __u32 uid;
    char comm[16];  // Task command name
};

// Ring buffer for sending events to userspace
struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 256 * 1024);  // 256KB buffer
} exec_events SEC(".maps");

// Map to count executions per UID
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 1024);
    __type(key, __u32);    // UID
    __type(value, __u64);  // Count
} exec_counts SEC(".maps");

SEC("kprobe/sys_execve")
int trace_execve(struct pt_regs *ctx)
{
    struct exec_event *event;
    __u64 *count;
    __u32 pid, uid;

    // Get current PID and UID
    pid = bpf_get_current_pid_tgid() >> 32;
    uid = bpf_get_current_uid_gid() & 0xFFFFFFFF;

    // Reserve space in ring buffer
    event = bpf_ringbuf_reserve(&exec_events, sizeof(*event), 0);
    if (!event)
        return 0;

    // Fill event data
    event->pid = pid;
    event->uid = uid;
    bpf_get_current_comm(&event->comm, sizeof(event->comm));

    // Submit event
    bpf_ringbuf_submit(event, 0);

    // Update execution count for this UID
    count = bpf_map_lookup_elem(&exec_counts, &uid);
    if (count) {
        __sync_fetch_and_add(count, 1);
    } else {
        __u64 init_count = 1;
        bpf_map_update_elem(&exec_counts, &uid, &init_count, BPF_ANY);
    }

    return 0;
}

SEC("kretprobe/sys_execve")
int trace_execve_ret(struct pt_regs *ctx)
{
    // Could track return values here if needed
    // For now, just a placeholder
    return 0;
}

char _license[] SEC("license") = "GPL";