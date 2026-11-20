// Isolated guest-only diagnostic for the tracked HyperMesh XDP C program.
#define _GNU_SOURCE
#include <arpa/inet.h>
#include <bpf/bpf.h>
#include <bpf/libbpf.h>
#include <errno.h>
#include <linux/if_link.h>
#include <net/if.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <sys/resource.h>
#include <sys/socket.h>
#include <unistd.h>

struct conn_key { unsigned char src[16], dst[16]; __u32 sport, dport; };
struct conn_info { __u64 packets, bytes, last_seen; };

static int loopback_up(void) {
    struct ifreq iface = {0};
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd < 0) return -errno;
    strcpy(iface.ifr_name, "lo");
    int rc = ioctl(fd, SIOCGIFFLAGS, &iface);
    if (!rc) { iface.ifr_flags |= IFF_UP; rc = ioctl(fd, SIOCSIFFLAGS, &iface); }
    int result = rc ? -errno : 0;
    close(fd);
    return result;
}

static int receiver(unsigned short port) {
    int fd = socket(AF_INET6, SOCK_DGRAM, 0);
    if (fd < 0) return -errno;
    struct sockaddr_in6 addr = { .sin6_family = AF_INET6, .sin6_port = htons(port) };
    addr.sin6_addr = in6addr_loopback;
    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr))) {
        int error = -errno;
        close(fd);
        return error;
    }
    return fd;
}

static int send_once(int fd, unsigned short port) {
    struct sockaddr_in6 dst = { .sin6_family = AF_INET6, .sin6_port = htons(port) };
    dst.sin6_addr = in6addr_loopback;
    static const char payload[] = "guest-only-udp-control";
    ssize_t sent = sendto(fd, payload, sizeof(payload), 0,
                          (struct sockaddr *)&dst, sizeof(dst));
    return sent == sizeof(payload) ? 0 : (sent < 0 ? -errno : -EIO);
}

static int send_pair(void) {
    int on = receiver(9292), off = receiver(9293);
    if (on < 0) return on;
    if (off < 0) { close(on); return off; }
    int tx = socket(AF_INET6, SOCK_DGRAM, 0);
    if (tx < 0) { close(on); close(off); return -errno; }
    int on_rc = send_once(tx, 9292);
    int off_rc = send_once(tx, 9293);
    printf("GUEST: udp9292_send=%d udp9293_send=%d\n", on_rc, off_rc);
    usleep(10000);
    close(tx); close(on); close(off);
    return on_rc ? on_rc : off_rc;
}

static int inspect_connections(int fd) {
    struct conn_key cursor = {0}, next = {0};
    struct conn_info info = {0};
    int first = 1, on = 0, off = 0, entries = 0;
    __u64 on_packets = 0;
    while (bpf_map_get_next_key(fd, first ? NULL : &cursor, &next) == 0) {
        if (bpf_map_lookup_elem(fd, &next, &info)) return -errno;
        unsigned short port = ntohs((unsigned short)next.dport);
        if (port == 9292) { on++; on_packets += info.packets; }
        if (port == 9293) off++;
        entries++;
        if (entries > 16) return -E2BIG;
        cursor = next;
        first = 0;
    }
    if (errno != ENOENT) return -errno;
    printf("GUEST: connection_map_entries=%d on9292=%d on_packets=%llu off9293=%d\n",
           entries, on, (unsigned long long)on_packets, off);
    return on == 1 && on_packets >= 1 && off == 0 ? 0 : -ENODATA;
}

static int detach_checked(int ifindex) {
    __u32 after = 999;
    int detach = bpf_xdp_detach(ifindex, XDP_FLAGS_SKB_MODE, NULL);
    int query = bpf_xdp_query_id(ifindex, XDP_FLAGS_SKB_MODE, &after);
    printf("GUEST: detach=%d post_query=%d post_id=%u\n", detach, query, after);
    return detach ? detach : (query ? query : (after ? -EIO : 0));
}

static int run_trial(void) {
    struct bpf_object *obj = NULL;
    struct bpf_program *prog;
    struct bpf_map *map;
    struct bpf_prog_info info = {0};
    struct rlimit limit = {RLIM_INFINITY, RLIM_INFINITY};
    __u32 len = sizeof(info), attached_id = 0;
    int ifindex, result, attached = 0;
    mount("proc", "/proc", "proc", 0, NULL);
    mount("sysfs", "/sys", "sysfs", 0, NULL);
    result = loopback_up();
    printf("GUEST: loopback_up=%d\n", result);
    if (result) return result;
    ifindex = if_nametoindex("lo");
    setrlimit(RLIMIT_MEMLOCK, &limit);
    obj = bpf_object__open_file("/hypermesh_xdp.o", NULL);
    result = libbpf_get_error(obj);
    if (result) { obj = NULL; goto cleanup; }
    result = bpf_object__load(obj);
    printf("GUEST: kernel_load=%d\n", result);
    if (result) goto cleanup;
    prog = bpf_object__find_program_by_name(obj, "hypermesh_xdp_filter");
    map = bpf_object__find_map_by_name(obj, "connection_map");
    if (!prog || !map) { result = -ENOENT; goto cleanup; }
    result = bpf_obj_get_info_by_fd(bpf_program__fd(prog), &info, &len);
    if (result) goto cleanup;
    printf("GUEST: program_id=%u type=%u map_fd=%d\n", info.id, info.type, bpf_map__fd(map));
    result = bpf_xdp_attach(ifindex, bpf_program__fd(prog),
                            XDP_FLAGS_SKB_MODE | XDP_FLAGS_UPDATE_IF_NOEXIST, NULL);
    printf("GUEST: attach=%d\n", result);
    if (result) goto cleanup;
    attached = 1;
    result = bpf_xdp_query_id(ifindex, XDP_FLAGS_SKB_MODE, &attached_id);
    printf("GUEST: attached_query=%d attached_id=%u\n", result, attached_id);
    if (!result && attached_id != info.id) result = -EIO;
    if (!result) result = send_pair();
    if (!result) result = inspect_connections(bpf_map__fd(map));
cleanup:
    if (attached) { int det = detach_checked(ifindex); if (!result) result = det; }
    else printf("GUEST: detach=skipped_not_attached\n");
    bpf_object__close(obj);
    return result;
}

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    printf("GUEST: trial_start\n");
    int result = run_trial();
    printf("GUEST: result=%s code=%d\n", result ? "FAIL" : "PASS", result);
    sync();
    reboot(RB_POWER_OFF);
    return 1;
}
