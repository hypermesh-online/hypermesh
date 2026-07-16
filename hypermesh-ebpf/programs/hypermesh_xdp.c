// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// HyperMesh Unified XDP Filter - Kernel-level packet classification
// Originally extracted from stoq/src/transport/ebpf/loader.rs
// Renamed from stoq_xdp.c to hypermesh_xdp.c as the unified program
// for the entire HyperMesh eBPF subsystem.
//
// A4-CORRECT (restore + wire the documented substrate, papers/HYPERMESH.md
// §5.1-5.7):
//   The primary XDP program parses the HyperMesh extension header (magic
//   0x484D, 'HM') carried as a PLAINTEXT PREFIX ahead of the encrypted QUIC
//   payload. STOQ emits this header on the send path (apply_extensions), so
//   the kernel CAN read the PoS / asset / matrix fields at the UDP-payload
//   offset at wire speed. The QUIC-encrypted body follows the header and is
//   never inspected in-kernel.
//
//   Two-tier PoS (§5.4): the kernel performs FAST structural pre-validation
//   (algorithm indicator + PoW difficulty + cache/TTL), and userspace performs
//   deep FALCON-1024 verification (TrustChain), feeding the result back into
//   pos_header_map / policy_map via set_peer_pos_validated.
//
// SAFETY INVARIANT (permanent incident fix — an earlier build DROPPED IPv4):
//   The program acts ONLY on HyperMesh traffic. Non-IPv6, non-UDP,
//   non-port-9292, or non-0x484D (no HyperMesh header present) ALWAYS returns
//   XDP_PASS — NEVER XDP_DROP. IPv4 (SSH/ICMP/clearnet), non-HyperMesh IPv6,
//   and malformed frames are handed to the kernel stack untouched. Only a
//   HyperMesh 0x484D packet whose PoS FAILS kernel pre-validation (or an
//   allowlist/filter-map DROP) returns XDP_DROP.
//
// Capabilities:
//   - IPv6-only QUIC/UDP filtering on HYPERMESH_PORT (9292)
//   - PoS-authenticated-peer allowlist enforcement (policy_map + pos_header_map)
//   - HyperMesh extension-header parse + 4-way decision (PASS/REDIRECT/TX/DROP)
//   - Kernel-side PoS structural pre-validation (pos_config_map)
//   - Matrix-topology XDP_TX forwarding skeleton (routing_map)
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

/* Connection-tracking key.
 *
 * Ports are __u32 (not __u16): a narrow 16-bit store into an on-stack key can
 * alias a spilled map_value pointer and trip the BPF verifier with
 * "corrupt spilled pointer". The key is built ONLY with wide, aligned stores
 * (16-byte memcpys for the IPs, single 4-byte stores for the ports) inside the
 * hm_conntrack_update() helper's own frame. connection_map has ZERO in-kernel
 * or userspace readers, so widening the port fields is layout-safe. */
struct conn_key {
    __u8  src_ip[16];
    __u8  dst_ip[16];
    __u32 src_port;
    __u32 dst_port;
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
    __u32 validate_asset_hash;  /* Validate asset hashes */
    __u32 check_matrix_routing; /* Check matrix routing */
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

/* Asset hash registry entry - populated from userspace by
 * `register_asset_hash` -> `update_asset_hash_map`. Consulted in-kernel when
 * a packet carries an ASSET extension header and policy->validate_asset_hash
 * is set. */
struct asset_hash_entry {
    __u8  expected_hash[32]; /* BLAKE3 hash */
    __u32 shard_count;       /* Total shards */
    __u8  registered;        /* 1=registered on blockchain, 0=pending */
};

/* Matrix-topology forwarding rule - populated from userspace by
 * `update_routing_map` (set_routing_rule / set_matrix_route_active). Consulted
 * in-kernel for the XDP_TX delegation branch when a packet carries a MATRIX
 * extension header. */
struct route_value {
    __u32 egress_ifindex; /* Interface index to forward on */
    __u32 active;         /* 1=forward via XDP_TX/redirect, 0=inactive */
};

/* Kernel-side PoS pre-validation configuration (index 0 of pos_config_map).
 *
 * Layout matches Rust `KernelPosConfig::to_bytes` (32 bytes, natural
 * alignment; u64 fields are 8-byte aligned):
 *   [0..4]   min_difficulty       (u32 LE)
 *   [4..8]   (padding)
 *   [8..16]  max_timestamp_skew_ns(u64 LE)
 *   [16..24] validation_ttl_ns    (u64 LE)
 *   [24..28] enabled              (u32 LE, 1=enforce, 0=pass-through)
 *   [28..32] (padding)
 */
struct pos_config {
    __u32 min_difficulty;        /* Minimum leading zero bits required */
    __u64 max_timestamp_skew_ns; /* Max clock skew (nanoseconds) */
    __u64 validation_ttl_ns;     /* How long a cached validation is valid */
    __u32 enabled;               /* 1=enforce kernel checks, 0=pass-through */
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
 * PoS. The kernel gate consults this UNCONDITIONALLY for HyperMesh traffic.
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
 * (see struct asset_hash_entry). Consulted for in-kernel asset integrity. */
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

/* Kernel-side PoS structural pre-validation config (single entry, index 0). */
struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct pos_config);
} pos_config_map SEC(".maps");

/* ============================================================
 * HyperMesh extension header definitions
 *
 * These headers are a PLAINTEXT PREFIX in the UDP payload, immediately
 * after the UDP header and AHEAD of the QUIC-encrypted body. STOQ emits
 * them on the send path (apply_extensions). The kernel reads them to
 * classify HyperMesh protocol metadata before passing to userspace.
 * ============================================================ */

#define HMESH_HDR_MAGIC   0x484D  /* 'HM' in network byte order */
#define HMESH_HDR_POS     0x01    /* PoS (Proof of State) header */
#define HMESH_HDR_ASSET   0x02    /* Asset hash header */
#define HMESH_HDR_MATRIX  0x03    /* Matrix routing header */
#define HMESH_HDR_PRIVACY 0x04    /* Privacy tier header */

/* Common header preceding every extension. 4 bytes, no padding:
 *   [0..2] magic  (u16, network byte order; bytes 0x48 0x4D)
 *   [2]    type   (HMESH_HDR_*)
 *   [3]    length (payload length following this header) */
struct hmesh_header {
    __u16 magic;     /* Must be HMESH_HDR_MAGIC (0x484D) */
    __u8  type;      /* One of HMESH_HDR_* constants */
    __u8  length;    /* Payload length following this header */
};

/* PoS proof summary carried in-band.
 *
 * NATURAL alignment (NOT packed) — the u32 forces a 4-byte boundary, so the
 * layout is:
 *   [0]     algorithm  (u8)
 *   [1..4]  (padding)
 *   [4..8]  difficulty (u32)
 *   [8..40] hash[32]
 * sizeof == 40. Rust `WirePosHeader::to_bytes` produces these exact 40 bytes
 * (see hypermesh_headers.rs). The kernel reads `algorithm` @0 and `hash` @8;
 * `difficulty` is carried but not consulted in-kernel. */
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
 * count_leading_zero_bits_kernel - Count leading zero bits in a byte array.
 *
 * Walks up to 32 bytes (bounded for the BPF verifier). For each fully
 * zero byte, adds 8.  For the first non-zero byte, uses a binary-search
 * approach to count the leading zeros within that byte, then stops.
 */
static __always_inline __u32 count_leading_zero_bits_kernel(__u8 *hash, __u32 len)
{
    __u32 total = 0;
    __u32 max = len < 32 ? len : 32; /* BPF verifier bound */

    /* Explicit bound for BPF verifier - max 32 iterations */
    for (__u32 i = 0; i < 32; i++) {
        if (i >= max)
            break;
        __u8 byte = hash[i];
        if (byte == 0) {
            total += 8;
            continue;
        }
        /* Count leading zeros in non-zero byte using binary search */
        if (!(byte & 0xF0)) { total += 4; byte <<= 4; }
        if (!(byte & 0xC0)) { total += 2; byte <<= 2; }
        if (!(byte & 0x80)) { total += 1; }
        break;
    }
    return total;
}

/*
 * validate_pos_algorithm - Check that the algorithm indicator byte is
 * one of the three supported signing algorithms.
 *
 * NOTE: This is a STRUCTURAL check only.  Actual signature verification
 * (FALCON-1024 lattice sigs, Ed25519 curve ops, ECDSA point math)
 * requires asymmetric crypto operations that the BPF instruction set
 * does not support, so those MUST run in userspace.
 *
 * Returns 1 if valid, 0 otherwise.
 */
static __always_inline int validate_pos_algorithm(__u8 algorithm)
{
    return (algorithm == 0x01 ||  /* FALCON-1024 */
            algorithm == 0x02 ||  /* Ed25519 */
            algorithm == 0x03);   /* ECDSA */
}

/*
 * validate_pos_for_source - Check if source IP has a valid PoS entry
 * (cache-only check without TTL enforcement).
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
 * validate_pos_enhanced - Enhanced PoS validation with structural checks.
 *
 * When kernel-side PoS validation is enabled (via pos_config_map), this
 * function performs non-cryptographic structural checks before falling
 * back to the cache-based lookup:
 *
 *   1. Algorithm indicator must be a known value (0x01/0x02/0x03)
 *   2. PoW hash must meet minimum difficulty (leading zero bits)
 *   3. Cached validation entry must not be stale (TTL check)
 *
 * Asymmetric crypto verification (FALCON-1024 lattice signatures,
 * Ed25519 curve arithmetic, ECDSA point operations) is intentionally
 * NOT performed here.  The BPF instruction set has no helpers for
 * public-key cryptography, so full signature verification MUST remain
 * in the Rust userspace validation layer (src/validation.rs), which then
 * mirrors the result back via set_peer_pos_validated.
 *
 * @src_ip:   16-byte IPv6 source address (key for pos_header_map)
 * @pos_hdr:  Parsed PoS extension header, or NULL if packet does not
 *            carry one (in which case only cache+TTL is checked)
 *
 * Returns 1 if the packet should be allowed, 0 if it should be dropped.
 */
static __always_inline int validate_pos_enhanced(
    __u8 src_ip[16],
    struct hmesh_pos_header *pos_hdr)
{
    __u32 config_key = 0;
    struct pos_config *cfg = bpf_map_lookup_elem(&pos_config_map, &config_key);

    /* If no config loaded or kernel checks disabled, fall back to
     * the cache-only allowlist check (backward compatible). */
    if (!cfg || !cfg->enabled)
        return validate_pos_for_source(src_ip);

    /* ---------- Structural checks on the in-band PoS header ---------- */

    if (pos_hdr) {
        /* Check 1: Algorithm indicator must be a known value.
         * Unknown algorithm bytes mean the packet is malformed or
         * from an incompatible protocol version -> drop immediately. */
        if (!validate_pos_algorithm(pos_hdr->algorithm))
            return 0;

        /* Check 2: PoW difficulty must meet the configured minimum.
         * This catches trivially weak proofs (e.g., no work done)
         * before they reach the expensive userspace crypto path. */
        if (cfg->min_difficulty > 0) {
            __u32 lz = count_leading_zero_bits_kernel(
                pos_hdr->hash, 32);
            if (lz < cfg->min_difficulty)
                return 0;
        }
    }

    /* ---------- Cache lookup with TTL enforcement ---------- */

    struct pos_validation *val =
        bpf_map_lookup_elem(&pos_header_map, src_ip);

    if (val && val->validated == 1) {
        /* Check 3: Cached validation must not be stale.
         * last_validated is set by userspace using bpf_ktime_get_ns()
         * (kernel monotonic clock) at the time of successful
         * cryptographic verification. */
        if (cfg->validation_ttl_ns > 0) {
            __u64 now = bpf_ktime_get_ns();
            if (now - val->last_validated > cfg->validation_ttl_ns)
                return 0; /* Stale - force re-validation */
        }
        return 1; /* Valid and fresh */
    }

    return 0; /* No cached validation for this source */
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

/*
 * hm_conntrack_update - Update the connection-tracking map for one packet.
 *
 * Builds the `struct conn_key` in ITS OWN stack frame using only wide,
 * aligned stores (16-byte memcpys for the IPs, single 4-byte stores for the
 * u32 ports). This is the verifier-safe construction pattern: NO narrow
 * (u8/u16) stores into the on-stack key that could alias a spilled
 * map_value pointer and trip "corrupt spilled pointer".
 *
 * `static __noinline` so it gets its own verifier subprogram frame (≤5 args).
 * Byte-length telemetry is intentionally omitted (the kernel cannot see the
 * encrypted QUIC bytes, and `data_end - data` pointer subtraction spilled
 * across a call is a verifier hazard).
 */
static __noinline void hm_conntrack_update(const __u8 *src_ip16,
                                           const __u8 *dst_ip16,
                                           __u32 sport,
                                           __u32 dport)
{
    struct conn_key key = {};
    __builtin_memcpy(key.src_ip, src_ip16, 16);
    __builtin_memcpy(key.dst_ip, dst_ip16, 16);
    key.src_port = sport;
    key.dst_port = dport;

    struct conn_info *info = bpf_map_lookup_elem(&connection_map, &key);
    if (info) {
        info->packets++;
        info->last_seen = bpf_ktime_get_ns();
    } else {
        struct conn_info new_info = {
            .packets = 1,
            .bytes = 0, /* telemetry-only; kernel cannot see encrypted bytes */
            .last_seen = bpf_ktime_get_ns(),
        };
        bpf_map_update_elem(&connection_map, &key, &new_info, BPF_ANY);
    }
}

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

    /* ---- SAFETY: every non-HyperMesh / malformed path returns XDP_PASS ----
     * A bounds-check failure or protocol mismatch means the frame is NOT
     * HyperMesh traffic we can classify. Hand it to the kernel stack. Dropping
     * here caused a production outage (IPv4/SSH/ICMP blackholed). NEVER drop
     * a non-HyperMesh packet. */

    /* Ethernet header bounds — malformed frame passes to the stack. */
    if (data + sizeof(*eth) > data_end)
        return XDP_PASS;

    /* Only classify IPv6 packets (HyperMesh is IPv6-only).
     * IPv4 (SSH, ICMP, clearnet-to-gateway) MUST pass untouched. */
    if (bpf_ntohs(eth->h_proto) != ETH_P_IPV6)
        return XDP_PASS;

    struct ipv6hdr *ip6 = data + sizeof(*eth);
    if (data + sizeof(*eth) + sizeof(*ip6) > data_end)
        return XDP_PASS; /* truncated IPv6 header -> pass to stack */

    /* Only classify UDP (QUIC over UDP). Other IPv6 next-headers pass. */
    if (ip6->nexthdr != IPPROTO_UDP)
        return XDP_PASS;

    struct udphdr *udp = data + sizeof(*eth) + sizeof(*ip6);
    if (data + sizeof(*eth) + sizeof(*ip6) + sizeof(*udp) > data_end)
        return XDP_PASS; /* truncated UDP header -> pass to stack */

    /* Only classify HyperMesh/STOQ traffic (port 9292). Other UDP passes. */
    if (bpf_ntohs(udp->dest) != HYPERMESH_PORT &&
        bpf_ntohs(udp->source) != HYPERMESH_PORT)
        return XDP_PASS;

    /* --------------------------------------------------------
     * PoS-authenticated-peer allowlist (fast pre-filter)
     *
     * After confirming this is HyperMesh/STOQ traffic (port 9292),
     * consult the allowlist userspace populates on handshake success.
     *   - policy_map[src].requires_pos: does this source's privacy tier
     *     demand PoS? (Anonymous -> 0, Private/Public -> 1)
     *   - pos_header_map[src].validated: did userspace complete bilateral
     *     PoS (FALCON-1024) for this source?
     *
     * Honors the privacy tier: an Anonymous/open source (no policy entry
     * or requires_pos==0) is admitted; a Private/Public source that has not
     * been PoS-authenticated is DROPPED. This is an allowlist DROP on
     * HyperMesh-port traffic — permitted by the safety invariant.
     * -------------------------------------------------------- */
    __u8 src_ip[16];
    __builtin_memcpy(src_ip, &ip6->saddr, 16);
    __u8 dst_ip[16];
    __builtin_memcpy(dst_ip, &ip6->daddr, 16);

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
     * through to the deep header classification below. */

    /* --------------------------------------------------------
     * HyperMesh extension header parsing (deep classification)
     *
     * The UDP payload begins with the HyperMesh magic (0x484D) as a
     * PLAINTEXT prefix ahead of the QUIC-encrypted body. If the magic is
     * NOT present, this is a HyperMesh-port packet WITHOUT a HyperMesh
     * header (e.g. a bare QUIC probe) -> do NOT drop; fall through to PASS.
     * Only a 0x484D packet that FAILS PoS pre-validation is dropped.
     * -------------------------------------------------------- */
    void *payload = data + sizeof(*eth) + sizeof(*ip6) + sizeof(*udp);

    int policy_checked = 0;
    int policy_passed  = 1; /* assume pass unless a check fails */

    if (payload + sizeof(struct hmesh_header) <= data_end) {
        struct hmesh_header hdr = {};
        __builtin_memcpy(&hdr, payload, sizeof(struct hmesh_header));

        if (bpf_ntohs(hdr.magic) == HMESH_HDR_MAGIC) {
            /* HyperMesh header present — look up policy for this SOURCE. */
            struct policy_value *policy =
                bpf_map_lookup_elem(&policy_map, src_ip);

            if (policy) {
                policy_checked = 1;
                void *ext_payload = payload + sizeof(struct hmesh_header);

                /* --- PoS validation (two-tier §5.4) ---
                 * When a PoS extension header is present, run kernel-side
                 * structural checks (algorithm, difficulty) PLUS the cache
                 * lookup with TTL. When absent, do a cache-only check.
                 * Deep FALCON-1024 verification stays in userspace. */
                if (policy->requires_pos) {
                    if (hdr.type == HMESH_HDR_POS &&
                        ext_payload + sizeof(struct hmesh_pos_header) <= data_end) {
                        struct hmesh_pos_header parsed_pos = {};
                        __builtin_memcpy(&parsed_pos, ext_payload,
                                         sizeof(struct hmesh_pos_header));

                        if (!validate_pos_enhanced(src_ip, &parsed_pos))
                            policy_passed = 0;
                    } else {
                        /* Policy requires PoS but header is a different type
                         * or truncated — cache+TTL check only. */
                        if (!validate_pos_enhanced(src_ip, NULL))
                            policy_passed = 0;
                    }
                }

                /* --- Asset hash validation --- */
                if (policy->validate_asset_hash) {
                    if (hdr.type == HMESH_HDR_ASSET &&
                        ext_payload + sizeof(struct hmesh_asset_header) <= data_end) {
                        struct hmesh_asset_header ahdr = {};
                        __builtin_memcpy(&ahdr, ext_payload,
                                         sizeof(struct hmesh_asset_header));

                        if (!validate_asset_registered(ahdr.asset_id))
                            policy_passed = 0;
                    }
                    /* Non-ASSET header type: skip (carries a different ext). */
                }

                /* --- Matrix routing validation --- */
                if (policy->check_matrix_routing) {
                    if (hdr.type == HMESH_HDR_MATRIX &&
                        ext_payload + sizeof(struct hmesh_matrix_header) <= data_end) {
                        struct hmesh_matrix_header mhdr = {};
                        __builtin_memcpy(&mhdr, ext_payload,
                                         sizeof(struct hmesh_matrix_header));

                        if (!validate_matrix_position(mhdr.position))
                            policy_passed = 0;
                    }
                    /* Non-MATRIX header type: skip. */
                }
            }
            /* No policy entry for this source -> HyperMesh traffic is
             * admitted (open policy), header parse is informational. */
        }
        /* magic != 0x484D -> no HyperMesh header; policy_checked stays 0,
         * packet falls through to PASS (never dropped here). */
    }

    /* A 0x484D packet whose PoS/asset/matrix pre-validation failed is the
     * ONLY header-parse DROP. Everything else falls through. */
    if (policy_checked && !policy_passed) {
        stats->packets_dropped++;
        return XDP_DROP;
    }

    /* --------------------------------------------------------
     * Matrix-topology forwarding (XDP_TX delegation skeleton, §5.1 FORWARD)
     *
     * If the packet carries a MATRIX routing extension header whose raw
     * 12-byte position resolves to an active routing_map entry, this node is
     * a relay: return XDP_TX to hand the packet back out toward the next hop.
     * (Cross-device next-hop rewrite via bpf_redirect + L2 MAC rewrite is
     * deferred; egress_ifindex is carried in the map for that future swap.)
     * -------------------------------------------------------- */
    if (payload + sizeof(struct hmesh_header) <= data_end) {
        struct hmesh_header rhdr = {};
        __builtin_memcpy(&rhdr, payload, sizeof(struct hmesh_header));
        if (bpf_ntohs(rhdr.magic) == HMESH_HDR_MAGIC &&
            rhdr.type == HMESH_HDR_MATRIX) {
            void *rext = payload + sizeof(struct hmesh_header);
            if (rext + sizeof(struct hmesh_matrix_header) <= data_end) {
                struct hmesh_matrix_header rmh = {};
                __builtin_memcpy(&rmh, rext,
                                 sizeof(struct hmesh_matrix_header));
                struct route_value *rv =
                    bpf_map_lookup_elem(&routing_map, rmh.position);
                if (rv && rv->active) {
                    stats->packets_redirected++;
                    return XDP_TX;
                }
            }
        }
    }

    /* --------------------------------------------------------
     * AF_XDP redirect path (§5.1 REDIRECT, §5.2 zero-copy)
     *
     * If an AF_XDP socket is bound on this queue, redirect for zero-copy
     * processing in userspace.
     * -------------------------------------------------------- */
    __u32 queue_idx = ctx->rx_queue_index;
    __u32 *xsk_entry = bpf_map_lookup_elem(&xsk_map, &queue_idx);
    if (xsk_entry) {
        stats->packets_redirected++;
        return bpf_redirect_map(&xsk_map, queue_idx, XDP_PASS);
    }

    /* --------------------------------------------------------
     * Connection tracking update (verifier-safe helper frame)
     * -------------------------------------------------------- */
    hm_conntrack_update(src_ip, dst_ip,
                        (__u32)udp->source, (__u32)udp->dest);

    /* --------------------------------------------------------
     * Filter rules check (explicit user DROP/REDIRECT are permitted)
     * -------------------------------------------------------- */
    struct filter_key filter = {};
    __builtin_memcpy(filter.src_ip, src_ip, 16);
    __builtin_memcpy(filter.dst_ip, dst_ip, 16);

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

    /* Pass packet to kernel stack (byte-length telemetry intentionally
     * omitted — the kernel cannot see the encrypted QUIC payload). */
    stats->packets_passed++;

    return XDP_PASS;
}

char _license[] SEC("license") = "GPL";
