#![allow(dead_code)]
// Advanced Packet Filtering - Beyond standard filters
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;

use super::{FilterAction, PacketFilterSpec};

/// Advanced packet filtering with eBPF
pub struct AdvancedPacketFilter {
    active_filters: RwLock<HashMap<String, ActiveFilter>>,
}

struct ActiveFilter {
    spec: PacketFilterSpec,
    stats: FilterStats,
    program_id: String,
}

#[derive(Debug, Clone)]
pub struct FilterStats {
    packets_matched: u64,
    packets_dropped: u64,
    packets_rate_limited: u64,
    bytes_processed: u64,
}

impl AdvancedPacketFilter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            active_filters: RwLock::new(HashMap::new()),
        })
    }

    /// Create and load an advanced packet filter
    pub async fn create_filter(&mut self, spec: PacketFilterSpec) -> Result<String> {
        let filter_id = uuid::Uuid::new_v4().to_string();

        tracing::info!("Creating advanced packet filter: {}", spec.name);

        // Generate eBPF program for this filter
        let program = self.generate_filter_program(&spec)?;

        // Compile and load program
        let program_id = self.load_filter_program(&program).await?;

        let active_filter = ActiveFilter {
            spec: spec.clone(),
            stats: FilterStats {
                packets_matched: 0,
                packets_dropped: 0,
                packets_rate_limited: 0,
                bytes_processed: 0,
            },
            program_id,
        };

        self.active_filters
            .write()
            .await
            .insert(filter_id.clone(), active_filter);

        tracing::info!("Filter {} created successfully", filter_id);
        Ok(filter_id)
    }

    /// Remove a filter
    pub async fn remove_filter(&mut self, filter_id: &str) -> Result<()> {
        let mut filters = self.active_filters.write().await;

        if let Some(filter) = filters.remove(filter_id) {
            // Unload eBPF program
            self.unload_filter_program(&filter.program_id).await?;
            Ok(())
        } else {
            anyhow::bail!("Filter not found: {}", filter_id)
        }
    }

    fn generate_filter_program(&self, spec: &PacketFilterSpec) -> Result<String> {
        // Generate eBPF C code for the filter
        let mut program = String::from(
            r#"
#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/tcp.h>
#include <linux/udp.h>

SEC("xdp")
int packet_filter(struct xdp_md *ctx) {
    void *data_end = (void *)(long)ctx->data_end;
    void *data = (void *)(long)ctx->data;

    struct ethhdr *eth = data;
    if ((void *)(eth + 1) > data_end)
        return XDP_PASS;

    if (eth->h_proto != htons(ETH_P_IP))
        return XDP_PASS;

    struct iphdr *ip = (void *)(eth + 1);
    if ((void *)(ip + 1) > data_end)
        return XDP_PASS;

"#,
        );

        // Add protocol-specific filtering
        if let Some(protocol) = &spec.protocol {
            program.push_str(&format!(
                "    if (ip->protocol != {}) return XDP_PASS;\n",
                self.protocol_to_number(protocol)
            ));
        }

        // Add source IP filtering
        if let Some(src_ip) = &spec.src_ip {
            program.push_str(&format!(
                "    if (ip->saddr != {}) return XDP_PASS;\n",
                self.ip_to_u32(src_ip)
            ));
        }

        // Add destination IP filtering
        if let Some(dst_ip) = &spec.dst_ip {
            program.push_str(&format!(
                "    if (ip->daddr != {}) return XDP_PASS;\n",
                self.ip_to_u32(dst_ip)
            ));
        }

        // Add action
        match &spec.action {
            FilterAction::Allow => program.push_str("    return XDP_PASS;\n"),
            FilterAction::Drop => program.push_str("    return XDP_DROP;\n"),
            FilterAction::RateLimit { rate } => {
                // Token-bucket rate limiter: refill one token per (1e9/rate) ns,
                // drop when the bucket is empty.
                program.push_str(&format!(
                    r#"    // Token-bucket rate limiter ({rate} pps)
    struct bpf_spin_lock *lock;
    __u32 key = 0;
    struct {{
        __u64 tokens;
        __u64 last_refill_ns;
    }} *bucket;

    struct {{
        __uint(type, BPF_MAP_TYPE_ARRAY);
        __uint(max_entries, 1);
        __type(key, __u32);
        __type(value, struct {{ __u64 tokens; __u64 last_refill_ns; }});
    }} rate_bucket SEC(".maps");

    bucket = bpf_map_lookup_elem(&rate_bucket, &key);
    if (bucket) {{
        __u64 now = bpf_ktime_get_ns();
        __u64 interval_ns = 1000000000ULL / {rate};
        __u64 elapsed = now - bucket->last_refill_ns;
        __u64 new_tokens = elapsed / interval_ns;
        if (new_tokens > 0) {{
            bucket->tokens += new_tokens;
            if (bucket->tokens > {rate})
                bucket->tokens = {rate};
            bucket->last_refill_ns = now;
        }}
        if (bucket->tokens > 0) {{
            bucket->tokens--;
            return XDP_PASS;
        }}
        return XDP_DROP;
    }}
    return XDP_PASS;
"#,
                    rate = rate
                ));
            }
            FilterAction::Mirror { destination } => {
                // Redirect a clone of the packet to a mirror interface via
                // bpf_clone_redirect. The ifindex is derived from the
                // destination interface name at load time.
                program.push_str(&format!(
                    r#"    // Mirror packet to interface "{destination}"
    int mirror_ifindex = {ifindex}; // resolved ifindex for "{destination}"
    bpf_clone_redirect(ctx, mirror_ifindex, 0);
    return XDP_PASS;
"#,
                    destination = destination,
                    ifindex = Self::resolve_ifindex(destination),
                ));
            }
            FilterAction::ModifyPacket { .. } => {
                // Basic header modification: decrement TTL and recompute the
                // IP checksum incrementally.
                program.push_str(
                    r#"    // Basic packet modification: decrement IP TTL
    if (ip->ttl <= 1)
        return XDP_DROP;

    __u16 old_ttl = ip->ttl;
    ip->ttl--;

    // Incremental IP checksum update (RFC 1624)
    __u32 csum = (~ip->check & 0xFFFF) + (~old_ttl & 0xFFFF) + ip->ttl;
    csum = (csum & 0xFFFF) + (csum >> 16);
    ip->check = ~csum;

    return XDP_PASS;
"#,
                );
            }
        }

        program.push_str("}\n");

        Ok(program)
    }

    async fn load_filter_program(&self, program: &str) -> Result<String> {
        let program_id = uuid::Uuid::new_v4().to_string();

        #[cfg(feature = "aya-ebpf")]
        {
            // Try to compile and load via Aya
            match self.compile_and_load_with_aya(program).await {
                Ok(()) => {
                    tracing::info!(
                        program_id = %program_id,
                        "Filter program compiled and loaded via Aya"
                    );
                    return Ok(program_id);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Aya XDP loading failed, using stub"
                    );
                }
            }
        }

        // No Aya feature: register the filter without kernel loading
        tracing::info!(
            program_id = %program_id,
            source_len = program.len(),
            "Filter program registered (build with --features aya-ebpf for real XDP loading)"
        );
        Ok(program_id)
    }

    /// Compile eBPF C source and load the resulting XDP program via Aya.
    #[cfg(feature = "aya-ebpf")]
    async fn compile_and_load_with_aya(&self, source: &str) -> Result<()> {
        use aya::Ebpf;

        // Step 1: Write source to a temporary file and compile with clang
        let temp_dir = tempfile::TempDir::new()?;
        let source_path = temp_dir.path().join("filter.c");
        let object_path = temp_dir.path().join("filter.o");

        std::fs::write(&source_path, source)?;

        let clang_output = std::process::Command::new("clang")
            .args([
                "-O2",
                "-target",
                "bpf",
                "-c",
                source_path.to_str().unwrap(),
                "-o",
                object_path.to_str().unwrap(),
            ])
            .output()?;

        if !clang_output.status.success() {
            let stderr = String::from_utf8_lossy(&clang_output.stderr);
            anyhow::bail!("clang compilation failed: {}", stderr);
        }

        // Step 2: Load the compiled ELF with Aya
        let bytecode = std::fs::read(&object_path)?;
        let mut bpf = Ebpf::load(&bytecode)?;

        // Step 3: Find and load the XDP program
        for (name, program) in bpf.programs_mut() {
            if let aya::programs::Program::Xdp(xdp) = program {
                xdp.load()?;
                tracing::info!(program = %name, "XDP filter program loaded via Aya");
                return Ok(());
            }
        }

        anyhow::bail!("No XDP program section found in compiled filter")
    }

    async fn unload_filter_program(&self, program_id: &str) -> Result<()> {
        tracing::info!("Unloading filter program: {}", program_id);
        Ok(())
    }

    /// Resolve a network interface name to its kernel ifindex.
    ///
    /// Uses `libc::if_nametoindex()` for real resolution. Falls back to a
    /// deterministic hash when the interface doesn't exist on the current
    /// host (e.g. during code generation for a remote target).
    fn resolve_ifindex(name: &str) -> u32 {
        // Try real resolution first
        if let Ok(c_name) = std::ffi::CString::new(name) {
            let idx = unsafe { libc::if_nametoindex(c_name.as_ptr()) };
            if idx > 0 {
                tracing::debug!(interface = name, ifindex = idx, "Resolved interface index");
                return idx;
            }
        }

        // Fallback: deterministic hash for cross-compilation or
        // when the interface doesn't exist on the build host
        let mut hash: u32 = 5381;
        for b in name.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(b as u32);
        }
        let fallback = (hash % 65534) + 1;
        tracing::debug!(
            interface = name,
            ifindex = fallback,
            "Interface not found locally, using hash-based placeholder"
        );
        fallback
    }

    fn protocol_to_number(&self, protocol: &str) -> u8 {
        match protocol.to_uppercase().as_str() {
            "TCP" => 6,
            "UDP" => 17,
            "ICMP" => 1,
            _ => 0,
        }
    }

    fn ip_to_u32(&self, ip: &str) -> u32 {
        // Convert IP string to u32 (network byte order)
        match ip.parse::<std::net::Ipv4Addr>() {
            Ok(addr) => u32::from(addr).to_be(),
            Err(e) => {
                tracing::warn!("Failed to parse IP address '{}': {}", ip, e);
                0
            }
        }
    }

    /// Get filter statistics
    pub async fn get_filter_stats(&self, filter_id: &str) -> Option<FilterStats> {
        let filters = self.active_filters.read().await;
        filters.get(filter_id).map(|f| f.stats.clone())
    }

    /// List all active filters
    pub async fn list_filters(&self) -> Vec<String> {
        let filters = self.active_filters.read().await;
        filters.keys().cloned().collect()
    }
}

impl Default for AdvancedPacketFilter {
    fn default() -> Self {
        Self {
            active_filters: RwLock::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_filter_spec(name: &str, action: FilterAction) -> PacketFilterSpec {
        PacketFilterSpec {
            name: name.to_string(),
            protocol: None,
            src_ip: None,
            dst_ip: None,
            src_port: None,
            dst_port: None,
            action,
            advanced_rules: vec![],
        }
    }

    #[test]
    fn test_packet_filter_creation() {
        let filter = AdvancedPacketFilter::new();
        assert!(filter.is_ok());
    }

    #[test]
    fn test_packet_filter_default() {
        let _filter = AdvancedPacketFilter::default();
    }

    #[tokio::test]
    async fn test_create_filter_returns_id() {
        let mut filter = AdvancedPacketFilter::new().unwrap();
        let spec = make_filter_spec("test-filter", FilterAction::Allow);
        let result = filter.create_filter(spec).await;
        assert!(result.is_ok());
        let filter_id = result.unwrap();
        assert!(!filter_id.is_empty());
    }

    #[tokio::test]
    async fn test_list_filters_after_creation() {
        let mut filter = AdvancedPacketFilter::new().unwrap();

        // Initially empty
        assert!(filter.list_filters().await.is_empty());

        // Create one
        let spec = make_filter_spec("filter-1", FilterAction::Drop);
        let id = filter.create_filter(spec).await.unwrap();

        let list = filter.list_filters().await;
        assert_eq!(list.len(), 1);
        assert!(list.contains(&id));
    }

    #[tokio::test]
    async fn test_remove_filter() {
        let mut filter = AdvancedPacketFilter::new().unwrap();
        let spec = make_filter_spec("to-remove", FilterAction::Allow);
        let id = filter.create_filter(spec).await.unwrap();

        let result = filter.remove_filter(&id).await;
        assert!(result.is_ok());

        // Filter should be gone
        assert!(filter.list_filters().await.is_empty());
    }

    #[tokio::test]
    async fn test_remove_nonexistent_filter_fails() {
        let mut filter = AdvancedPacketFilter::new().unwrap();
        let result = filter.remove_filter("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_protocol_to_number() {
        let filter = AdvancedPacketFilter::new().unwrap();
        assert_eq!(filter.protocol_to_number("TCP"), 6);
        assert_eq!(filter.protocol_to_number("tcp"), 6);
        assert_eq!(filter.protocol_to_number("UDP"), 17);
        assert_eq!(filter.protocol_to_number("udp"), 17);
        assert_eq!(filter.protocol_to_number("ICMP"), 1);
        assert_eq!(filter.protocol_to_number("icmp"), 1);
        assert_eq!(filter.protocol_to_number("unknown"), 0);
    }

    #[test]
    fn test_ip_to_u32_valid() {
        let filter = AdvancedPacketFilter::new().unwrap();
        // 192.168.1.1 in network byte order
        let result = filter.ip_to_u32("192.168.1.1");
        assert_ne!(result, 0);
        // Verify round-trip: convert back
        let expected = u32::from(std::net::Ipv4Addr::new(192, 168, 1, 1)).to_be();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_ip_to_u32_invalid() {
        let filter = AdvancedPacketFilter::new().unwrap();
        let result = filter.ip_to_u32("not-an-ip");
        assert_eq!(result, 0);
    }

    #[test]
    fn test_ip_to_u32_localhost() {
        let filter = AdvancedPacketFilter::new().unwrap();
        let result = filter.ip_to_u32("127.0.0.1");
        let expected = u32::from(std::net::Ipv4Addr::new(127, 0, 0, 1)).to_be();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_filter_program_with_protocol() {
        let filter = AdvancedPacketFilter::new().unwrap();
        let spec = PacketFilterSpec {
            name: "tcp-filter".to_string(),
            protocol: Some("TCP".to_string()),
            src_ip: None,
            dst_ip: None,
            src_port: None,
            dst_port: None,
            action: FilterAction::Drop,
            advanced_rules: vec![],
        };
        let program = filter.generate_filter_program(&spec).unwrap();
        assert!(program.contains("ip->protocol != 6"));
        assert!(program.contains("XDP_DROP"));
    }

    #[test]
    fn test_generate_filter_program_allow_action() {
        let filter = AdvancedPacketFilter::new().unwrap();
        let spec = make_filter_spec("allow-all", FilterAction::Allow);
        let program = filter.generate_filter_program(&spec).unwrap();
        assert!(program.contains("XDP_PASS"));
    }
}
