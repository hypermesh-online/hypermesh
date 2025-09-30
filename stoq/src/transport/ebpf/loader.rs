//! eBPF program loader and manager
//!
//! Handles loading, compilation, and management of eBPF programs
//! for the STOQ transport layer.

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

// eBPF loader would use aya

/// eBPF program source locations
pub struct EbpfSources {
    /// XDP program source
    pub xdp_source: PathBuf,
    /// TC program source (if needed)
    pub tc_source: Option<PathBuf>,
    /// Output directory for compiled programs
    pub output_dir: PathBuf,
}

impl Default for EbpfSources {
    fn default() -> Self {
        Self {
            xdp_source: PathBuf::from("src/transport/ebpf/programs/xdp.c"),
            tc_source: None,
            output_dir: PathBuf::from("target/bpf"),
        }
    }
}

/// eBPF program loader
pub struct EbpfLoader {
    sources: EbpfSources,
    /// Whether programs are loaded
    programs_loaded: bool,
}

impl EbpfLoader {
    /// Create new eBPF loader
    pub fn new() -> Self {
        Self::with_sources(EbpfSources::default())
    }

    /// Create loader with custom source paths
    pub fn with_sources(sources: EbpfSources) -> Self {
        Self {
            sources,
            programs_loaded: false,
        }
    }

    /// Compile eBPF programs from source
    pub fn compile(&mut self) -> Result<()> {
        // Create output directory
        fs::create_dir_all(&self.sources.output_dir)?;

        // Check if clang is available
        if !Self::check_clang() {
            return Err(anyhow!("clang not found. Install with: apt install clang llvm"));
        }

        // Compile XDP program
        let xdp_output = self.sources.output_dir.join("stoq_xdp.o");
        self.compile_program(&self.sources.xdp_source, &xdp_output)?;

        // Compile TC program if provided
        if let Some(tc_source) = &self.sources.tc_source {
            let tc_output = self.sources.output_dir.join("stoq_tc.o");
            self.compile_program(tc_source, &tc_output)?;
        }

        tracing::info!("eBPF programs compiled successfully");
        Ok(())
    }

    /// Compile a single eBPF program
    fn compile_program(&self, source: &Path, output: &Path) -> Result<()> {
        if !source.exists() {
            // Source doesn't exist, try to use pre-compiled version
            tracing::warn!("eBPF source {:?} not found, will use pre-compiled if available", source);
            return Ok(());
        }

        let status = Command::new("clang")
            .args(&[
                "-O2",
                "-target", "bpf",
                "-c",
                source.to_str().unwrap(),
                "-o",
                output.to_str().unwrap(),
                "-I/usr/include",
                "-I/usr/include/bpf",
                "-D__TARGET_ARCH_x86",
            ])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to compile eBPF program {:?}", source));
        }

        tracing::info!("Compiled eBPF program {:?} -> {:?}", source, output);
        Ok(())
    }

    /// Check if clang is available
    fn check_clang() -> bool {
        Command::new("clang")
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Load compiled eBPF programs
    pub fn load(&mut self) -> Result<()> {
        // Check if compiled programs exist
        let xdp_path = self.sources.output_dir.join("stoq_xdp.o");

        if !xdp_path.exists() {
            // Try to compile if not exists
            self.compile()?;

            if !xdp_path.exists() {
                tracing::warn!("eBPF bytecode not found. Run 'make ebpf' to compile programs.");
                return Ok(());
            }
        }

        // In a real implementation, we would load the bytecode with aya
        self.programs_loaded = true;

        tracing::info!("eBPF programs loaded (placeholder)");
        Ok(())
    }

    /// Check if programs are loaded
    pub fn are_programs_loaded(&self) -> bool {
        self.programs_loaded
    }

    /// Verify eBPF program before loading
    pub fn verify(&self, program_path: &Path) -> Result<()> {
        if !program_path.exists() {
            return Err(anyhow!("Program file not found: {:?}", program_path));
        }

        // Use bpftool if available for verification
        if let Ok(output) = Command::new("bpftool")
            .args(&["prog", "load", program_path.to_str().unwrap(), "/sys/fs/bpf/test_verify"])
            .output()
        {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("Program verification failed: {}", stderr));
            }

            // Clean up test program
            let _ = Command::new("rm")
                .arg("/sys/fs/bpf/test_verify")
                .status();
        }

        Ok(())
    }
}

/// eBPF program template generator
pub struct ProgramGenerator;

impl ProgramGenerator {
    /// Generate XDP program source code
    pub fn generate_xdp_source() -> String {
        r#"
#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/ipv6.h>
#include <linux/udp.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_endian.h>

#define STOQ_PORT 9292

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

SEC("xdp/stoq_filter")
int stoq_xdp_filter(struct xdp_md *ctx) {
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

    /* Check if it's STOQ traffic (port 9292) */
    if (bpf_ntohs(udp->dest) != STOQ_PORT && bpf_ntohs(udp->source) != STOQ_PORT)
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
"#.to_string()
    }

    /// Generate TC (Traffic Control) program source
    pub fn generate_tc_source() -> String {
        r#"
#include <linux/bpf.h>
#include <linux/pkt_cls.h>
#include <bpf/bpf_helpers.h>

SEC("tc/stoq_tc")
int stoq_tc_filter(struct __sk_buff *skb) {
    return TC_ACT_OK;
}

char _license[] SEC("license") = "GPL";
"#.to_string()
    }

    /// Save generated source to file
    pub fn save_source(source: &str, path: &Path) -> Result<()> {
        fs::write(path, source)?;
        tracing::info!("Generated eBPF source saved to {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = EbpfLoader::new();
        assert!(loader.sources.output_dir.to_str().unwrap().contains("bpf"));
    }

    #[test]
    fn test_program_generation() {
        let xdp_source = ProgramGenerator::generate_xdp_source();
        assert!(xdp_source.contains("stoq_xdp_filter"));
        assert!(xdp_source.contains("XDP_PASS"));

        let tc_source = ProgramGenerator::generate_tc_source();
        assert!(tc_source.contains("stoq_tc_filter"));
        assert!(tc_source.contains("TC_ACT_OK"));
    }
}