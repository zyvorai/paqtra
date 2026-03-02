// Many items used only with real eBPF hardware
#![allow(dead_code)]
/// BPF Map Access via syscalls and bpftool
///
/// Provides BPF map reading through multiple methods:
/// 1. Direct bpftool command execution
/// 2. Reading from pinned maps (future: via libbpf)

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::net::IpAddr;
use std::process::Command;

use super::{ConntrackEntry, IPCacheEntry, LoadBalancerEntry, PolicyDecision, DropReasonType};
use super::bpf_parser::{parse_ct_entry, parse_ct6_entry, parse_ipcache_entry, parse_lb_entry, parse_policy_entry, parse_metrics_entry};

/// BPF map accessor using bpftool
pub struct BpfToolReader {
    bpftool_path: String,
}

impl BpfToolReader {
    /// Create a new bpftool reader
    pub fn new() -> Result<Self> {
        // Try to find bpftool
        let bpftool_path = Self::find_bpftool()?;

        Ok(Self { bpftool_path })
    }

    /// Find bpftool binary
    fn find_bpftool() -> Result<String> {
        // Check common locations
        let paths = vec![
            "/usr/sbin/bpftool",
            "/usr/local/bin/bpftool",
            "/bin/bpftool",
        ];

        for path in paths {
            if std::path::Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }

        // Try which command
        if let Ok(output) = Command::new("which").arg("bpftool").output() {
            if output.status.success() {
                if let Ok(path) = String::from_utf8(output.stdout) {
                    return Ok(path.trim().to_string());
                }
            }
        }

        anyhow::bail!("bpftool not found. Please install bpftool package.")
    }

    /// List all BPF maps
    pub fn list_maps(&self) -> Result<Vec<BpfMapInfo>> {
        let output = Command::new(&self.bpftool_path)
            .args(["map", "list", "-j"])
            .output()
            .context("Failed to execute bpftool map list")?;

        if !output.status.success() {
            anyhow::bail!("bpftool map list failed");
        }

        let json_str = String::from_utf8(output.stdout)
            .context("Failed to parse bpftool output")?;

        let maps: Vec<Value> = serde_json::from_str(&json_str)
            .context("Failed to parse bpftool JSON")?;

        let mut result = Vec::new();
        for map in maps {
            if let Some(name) = map.get("name").and_then(|v| v.as_str()) {
                let id = map.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                result.push(BpfMapInfo {
                    id: id as u32,
                    name: name.to_string(),
                    map_type: map.get("type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                });
            }
        }

        Ok(result)
    }

    /// Find map by name
    pub fn find_map_by_name(&self, name: &str) -> Result<Option<BpfMapInfo>> {
        let maps = self.list_maps()?;
        Ok(maps.into_iter().find(|m| m.name.contains(name)))
    }

    /// Dump map contents
    pub fn dump_map(&self, map_id: u32) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let output = Command::new(&self.bpftool_path)
            .args(["map", "dump", "id", &map_id.to_string(), "-j"])
            .output()
            .context("Failed to dump map")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8(output.stdout)
            .context("Failed to parse dump output")?;

        let entries: Vec<Value> = serde_json::from_str(&json_str)
            .unwrap_or_default();

        let mut result = Vec::new();
        for entry in entries {
            if let (Some(key), Some(value)) = (
                entry.get("key"),
                entry.get("value"),
            ) {
                // Parse hex arrays to bytes
                if let (Ok(key_bytes), Ok(val_bytes)) = (
                    Self::parse_hex_array(key),
                    Self::parse_hex_array(value),
                ) {
                    result.push((key_bytes, val_bytes));
                }
            }
        }

        Ok(result)
    }

    /// Parse hex array from JSON ["0x01", "0x02", ...]
    fn parse_hex_array(value: &Value) -> Result<Vec<u8>> {
        if let Some(arr) = value.as_array() {
            let mut bytes = Vec::new();
            for item in arr {
                if let Some(hex_str) = item.as_str() {
                    let cleaned = hex_str.trim_start_matches("0x");
                    if let Ok(byte) = u8::from_str_radix(cleaned, 16) {
                        bytes.push(byte);
                    }
                }
            }
            Ok(bytes)
        } else {
            Ok(Vec::new())
        }
    }

    /// Read Cilium conntrack map
    pub fn read_cilium_ct_map(&self) -> Result<Vec<ConntrackEntry>> {
        // Find CT4 or CT6 map
        let (map, is_ipv6) = if let Some(m) = self.find_map_by_name("cilium_ct4_global")? {
            (m, false)
        } else if let Some(m) = self.find_map_by_name("cilium_ct6_global")? {
            (m, true)
        } else {
            return Ok(Vec::new());
        };

        let entries = self.dump_map(map.id)?;

        // Parse entries using proper parser
        let mut ct_entries = Vec::new();

        for (key, value) in entries {
            let entry = if is_ipv6 {
                parse_ct6_entry(&key, &value)
            } else {
                parse_ct_entry(&key, &value)
            };

            if let Ok(ct) = entry {
                ct_entries.push(ct);
            }
        }

        Ok(ct_entries)
    }

    /// Read Cilium IP cache
    pub fn read_cilium_ipcache(&self) -> Result<Vec<IPCacheEntry>> {
        let map = match self.find_map_by_name("cilium_ipcache")? {
            Some(m) => m,
            None => return Ok(Vec::new()),
        };

        let entries = self.dump_map(map.id)?;

        let mut ipcache_entries = Vec::new();

        for (key_bytes, value_bytes) in entries {
            if let Ok(entry) = parse_ipcache_entry(&key_bytes, &value_bytes) {
                ipcache_entries.push(entry);
            }
        }

        Ok(ipcache_entries)
    }

    /// Read Cilium load balancer map
    pub fn read_cilium_lb_map(&self) -> Result<Vec<LoadBalancerEntry>> {
        let map = match self.find_map_by_name("cilium_lb4_services")? {
            Some(m) => m,
            None => return Ok(Vec::new()),
        };

        let entries = self.dump_map(map.id)?;

        let mut lb_entries = Vec::new();

        for (key, value) in entries {
            if let Ok(entry) = parse_lb_entry(&key, &value) {
                lb_entries.push(entry);
            }
        }

        Ok(lb_entries)
    }

    /// Read Cilium policy map
    pub fn read_cilium_policy_map(&self) -> Result<Vec<PolicyDecision>> {
        let map = match self.find_map_by_name("cilium_policy")? {
            Some(m) => m,
            None => return Ok(Vec::new()),
        };

        let entries = self.dump_map(map.id)?;

        let mut policy_entries = Vec::new();

        for (key, value) in entries {
            if let Ok(entry) = parse_policy_entry(&key, &value) {
                policy_entries.push(entry);
            }
        }

        Ok(policy_entries)
    }

    /// Read Cilium metrics/drop counters
    pub fn read_cilium_metrics(&self) -> Result<HashMap<DropReasonType, u64>> {
        let map = match self.find_map_by_name("cilium_metrics")? {
            Some(m) => m,
            None => return Ok(HashMap::new()),
        };

        let entries = self.dump_map(map.id)?;

        let mut metrics = HashMap::new();

        for (key, value) in entries {
            if let Ok((reason, count)) = parse_metrics_entry(&key, &value) {
                *metrics.entry(reason).or_insert(0) += count;
            }
        }

        Ok(metrics)
    }

    /// Check if bpftool is available
    pub fn is_available(&self) -> bool {
        std::path::Path::new(&self.bpftool_path).exists()
    }
}

/// BPF Map Information
#[derive(Debug, Clone)]
pub struct BpfMapInfo {
    pub id: u32,
    pub name: String,
    #[allow(dead_code)]
    pub map_type: String,
}

/// Identity information
#[derive(Debug, Clone)]
pub struct IdentityInfo {
    pub identity: u32,
    pub labels: Vec<String>,
    pub namespace: String,
    pub pod_name: String,
}

/// Identity resolver - maps Cilium identities to K8s labels
#[allow(dead_code)]
pub struct IdentityResolver {
    identity_cache: HashMap<u32, IdentityInfo>,
}

#[allow(dead_code)]
impl IdentityResolver {
    pub fn new() -> Self {
        Self {
            identity_cache: HashMap::new(),
        }
    }

    /// Resolve identity to labels (from K8s API or Cilium API)
    pub async fn resolve_identity(&mut self, identity: u32) -> Option<&IdentityInfo> {
        // In real implementation:
        // 1. Query Cilium API /v1/identity/{id}
        // 2. Or parse from K8s pod labels
        // 3. Cache the result

        self.identity_cache.get(&identity)
    }

    /// Resolve IP to identity (from ipcache)
    pub fn resolve_ip(&self, _ip: &IpAddr) -> Option<u32> {
        // Query ipcache map
        // Return identity if found
        None
    }

    /// Add identity to cache
    pub fn add_identity(&mut self, info: IdentityInfo) {
        self.identity_cache.insert(info.identity, info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_bpftool() {
        // This test will fail if bpftool is not installed
        // which is expected in CI environments
        match BpfToolReader::find_bpftool() {
            Ok(path) => {
                assert!(!path.is_empty());
                assert!(path.contains("bpftool"));
            }
            Err(_) => {
                // bpftool not available, skip test
                println!("bpftool not available, skipping test");
            }
        }
    }

    #[test]
    fn test_parse_hex_array() {
        let json_val = serde_json::json!(["0x01", "0x02", "0x0a", "0xff"]);
        let bytes = BpfToolReader::parse_hex_array(&json_val).unwrap();
        assert_eq!(bytes, vec![0x01, 0x02, 0x0a, 0xff]);
    }

    #[test]
    fn test_identity_resolver() {
        let mut resolver = IdentityResolver::new();

        resolver.add_identity(IdentityInfo {
            identity: 100,
            labels: vec!["app=web".to_string()],
            namespace: "default".to_string(),
            pod_name: "web-pod-123".to_string(),
        });

        // Note: resolve_identity is async, so we can't test it here without tokio
        assert_eq!(resolver.identity_cache.len(), 1);
    }
}
