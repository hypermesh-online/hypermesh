// Isolated kernel-XDP attribution for a source-bound STOQ/QUIC guest executable.
#define _GNU_SOURCE
#include <arpa/inet.h>
#include <bpf/bpf.h>
#include <bpf/libbpf.h>
#include <errno.h>
#include <linux/if_link.h>
#include <net/if.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <sys/resource.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <unistd.h>

struct conn_key { unsigned char src[16], dst[16]; __u32 sport, dport; };
struct conn_info { __u64 packets, bytes, last_seen; };
struct snapshot { __u64 on, off; int entries; };
struct trial { struct bpf_object *obj; int ifindex, map_fd, attached; __u32 prog_id; };

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

static int attach_project_object(struct trial *t) {
    struct bpf_program *prog;
    struct bpf_map *map;
    struct bpf_prog_info info = {0};
    struct bpf_map_info map_info = {0};
    struct rlimit limit = {RLIM_INFINITY, RLIM_INFINITY};
    __u32 len = sizeof(info), map_len = sizeof(map_info), queried = 0;
    setrlimit(RLIMIT_MEMLOCK, &limit);
    t->obj = bpf_object__open_file("/hypermesh_xdp.o", NULL);
    int rc = libbpf_get_error(t->obj);
    if (rc) { t->obj = NULL; return rc; }
    rc = bpf_object__load(t->obj);
    printf("GUEST: kernel_load=%d\n", rc);
    if (rc) return rc;
    prog = bpf_object__find_program_by_name(t->obj, "hypermesh_xdp_filter");
    map = bpf_object__find_map_by_name(t->obj, "connection_map");
    if (!prog || !map) return -ENOENT;
    t->map_fd = bpf_map__fd(map);
    rc = bpf_obj_get_info_by_fd(bpf_program__fd(prog), &info, &len);
    if (rc) return -errno;
    rc = bpf_obj_get_info_by_fd(t->map_fd, &map_info, &map_len);
    if (rc) return -errno;
    t->prog_id = info.id;
    printf("GUEST: program_id=%u type=%u connection_map_id=%u\n",
           info.id, info.type, map_info.id);
    rc = bpf_xdp_attach(t->ifindex, bpf_program__fd(prog),
                        XDP_FLAGS_SKB_MODE | XDP_FLAGS_UPDATE_IF_NOEXIST, NULL);
    printf("GUEST: attach=%d\n", rc);
    if (rc) return rc;
    t->attached = 1;
    rc = bpf_xdp_query_id(t->ifindex, XDP_FLAGS_SKB_MODE, &queried);
    printf("GUEST: attached_query=%d attached_id=%u\n", rc, queried);
    return rc ? rc : (queried == info.id ? 0 : -EIO);
}

static int snapshot_map(int map_fd, const char *phase, struct snapshot *s) {
    struct conn_key cursor = {0}, next = {0};
    struct conn_info info = {0};
    int first = 1;
    memset(s, 0, sizeof(*s));
    while (bpf_map_get_next_key(map_fd, first ? NULL : &cursor, &next) == 0) {
        if (bpf_map_lookup_elem(map_fd, &next, &info)) return -errno;
        unsigned short sport = ntohs((unsigned short)next.sport);
        unsigned short dport = ntohs((unsigned short)next.dport);
        if (sport == 9292 || dport == 9292) s->on += info.packets;
        if (sport == 9293 || dport == 9293) s->off += info.packets;
        s->entries++;
        if (s->entries > 32) return -E2BIG;
        cursor = next;
        first = 0;
    }
    if (errno != ENOENT) return -errno;
    printf("GUEST: map phase=%s entries=%d on9292=%llu off9293=%llu\n",
           phase, s->entries, (unsigned long long)s->on,
           (unsigned long long)s->off);
    return 0;
}

static int read_line(int fd, char *line, size_t size) {
    size_t used = 0;
    while (used + 1 < size) {
        struct pollfd input = { .fd = fd, .events = POLLIN };
        int ready = poll(&input, 1, 12000);
        if (ready <= 0) return ready ? -errno : -ETIMEDOUT;
        char c;
        ssize_t got = read(fd, &c, 1);
        if (got <= 0) return got ? -errno : (used ? -EPIPE : 0);
        line[used++] = c;
        if (c == '\n') break;
    }
    line[used] = '\0';
    return used + 1 == size && line[used - 1] != '\n' ? -E2BIG : (int)used;
}

static int process_stages(int from_child, int to_child, int map_fd) {
    const char *expected[] = {"ON_HANDSHAKE", "ON_PAYLOAD", "OFF_HANDSHAKE", "OFF_PAYLOAD"};
    struct snapshot snaps[4] = {0};
    char line[512];
    int phase = 0, on_echo = 0, off_echo = 0, passed = 0;
    for (;;) {
        int read_rc = read_line(from_child, line, sizeof(line));
        if (read_rc < 0) return read_rc;
        if (read_rc == 0) break;
        printf("GUEST: child %s", line);
        if (strstr(line, "STOQ_ROUNDTRIP:ON:")) on_echo = 1;
        if (strstr(line, "STOQ_ROUNDTRIP:OFF:")) off_echo = 1;
        if (strstr(line, "STOQ_RESULT:PASS")) passed = 1;
        if (strncmp(line, "STOQ_STAGE:", 11) != 0) continue;
        if (phase >= 4 || strncmp(line + 11, expected[phase], strlen(expected[phase])))
            return -EPROTO;
        if (phase == 1 && !on_echo) return -EPROTO;
        if (phase == 3 && !off_echo) return -EPROTO;
        int rc = snapshot_map(map_fd, expected[phase], &snaps[phase]);
        if (rc) return rc;
        if (write(to_child, "+", 1) != 1) return -EPIPE;
        phase++;
    }
    printf("GUEST: stages=%d child_pass=%d on_echo=%d off_echo=%d\n",
           phase, passed, on_echo, off_echo);
    if (phase != 4 || !passed || !on_echo || !off_echo) return -ENODATA;
    if (!snaps[0].on || snaps[1].on <= snaps[0].on) return -ENODATA;
    if (snaps[3].on != snaps[2].on || snaps[3].off) return -ENODATA;
    return 0;
}

static int wait_child(pid_t child) {
    int status = 0;
    for (int i = 0; i < 100; i++) {
        pid_t done = waitpid(child, &status, WNOHANG);
        if (done == child) {
            int code = WIFEXITED(status) ? WEXITSTATUS(status) : -EINTR;
            printf("GUEST: stoq_exit=%d\n", code);
            return code ? -ECHILD : 0;
        }
        if (done < 0) return -errno;
        usleep(100000);
    }
    kill(child, SIGKILL);
    waitpid(child, &status, 0);
    printf("GUEST: stoq_exit=timeout_killed\n");
    return -ETIMEDOUT;
}

static int run_stoq(int map_fd) {
    int input[2], output[2];
    if (pipe(input) || pipe(output)) return -errno;
    pid_t child = fork();
    if (child < 0) return -errno;
    if (child == 0) {
        dup2(input[0], STDIN_FILENO);
        dup2(output[1], STDOUT_FILENO);
        close(input[0]); close(input[1]); close(output[0]); close(output[1]);
        char *const args[] = {"/stoq_guest", NULL};
        execv(args[0], args);
        _exit(127);
    }
    close(input[0]); close(output[1]);
    int rc = process_stages(output[0], input[1], map_fd);
    if (rc) kill(child, SIGKILL);
    close(input[1]); close(output[0]);
    int child_rc = wait_child(child);
    return rc ? rc : child_rc;
}

static int detach_checked(struct trial *t) {
    if (!t->attached) { printf("GUEST: detach=skipped_not_attached\n"); return 0; }
    __u32 after = 999;
    int detach = bpf_xdp_detach(t->ifindex, XDP_FLAGS_SKB_MODE, NULL);
    int query = bpf_xdp_query_id(t->ifindex, XDP_FLAGS_SKB_MODE, &after);
    printf("GUEST: detach=%d post_query=%d post_id=%u\n", detach, query, after);
    return detach ? detach : (query ? query : (after ? -EIO : 0));
}

static int run_trial(void) {
    struct trial t = {0};
    mount("proc", "/proc", "proc", 0, NULL);
    mount("sysfs", "/sys", "sysfs", 0, NULL);
    mount("devtmpfs", "/dev", "devtmpfs", 0, NULL);
    int rc = loopback_up();
    printf("GUEST: loopback_up=%d\n", rc);
    if (rc) return rc;
    t.ifindex = if_nametoindex("lo");
    rc = attach_project_object(&t);
    if (!rc) rc = run_stoq(t.map_fd);
    int cleanup = detach_checked(&t);
    bpf_object__close(t.obj);
    return rc ? rc : cleanup;
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
