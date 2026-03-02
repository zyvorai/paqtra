// Many items used only with real eBPF hardware
#![allow(dead_code)]
/// Real Cilium BPF Map Reader
///
/// Reads data directly from Cilium's pinned BPF maps in /sys/fs/bpf/

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    ConntrackEntry, DropReason, IPCacheEntry, LoadBalancerEntry, MapReader, PolicyDecision,
};
use super::bpf_syscall::BpfToolReader;

/// Cilium BPF Map Paths
const BPF_FS_PATH: &str = "/sys/fs/bpf";
const CILIUM_PATH: &str = "tc/globals";

/// Cilium BPF Map Reader
pub struct CiliumMapReader {
    #[allow(dead_code)]
    bpf_path: PathBuf,
    available_maps: Vec<String>,
    bpftool: Option<BpfToolReader>,
}

impl CiliumMapReader {
    /// Create a new Cilium map reader
    pub fn new() -> Result<Self> {
        let bpf_path = PathBuf::from(BPF_FS_PATH).join(CILIUM_PATH);

        // Try to initialize bpftool reader
        let bpftool = BpfToolReader::new().ok();

        // Check if BPF filesystem is mounted or bpftool is available
        let available_maps = if bpf_path.exists() {
            Self::discover_maps(&bpf_path)?
        } else if let Some(ref tool) = bpftool {
            // Use bpftool to list maps
            tool.list_maps()
                .unwrap_or_default()
                .into_iter()
                .map(|m| m.name)
                .collect()
        } else {
            Vec::new()
        };

        if available_maps.is_empty() && bpftool.is_none() {
            eprintln!(
                "Warning: BPF filesystem not found at {:?} and bpftool not available. \
                 Some features will be limited.",
                bpf_path
            );
        }

        Ok(Self {
            bpf_path,
            available_maps,
            bpftool,
        })
    }

    /// Create a reader with custom BPF path (for testing)
    pub fn with_path(bpf_path: PathBuf) -> Result<Self> {
        let available_maps = if bpf_path.exists() {
            Self::discover_maps(&bpf_path)?
        } else {
            Vec::new()
        };

        Ok(Self {
            bpf_path,
            available_maps,
            bpftool: None,
        })
    }

    /// Discover available BPF maps
    fn discover_maps(path: &Path) -> Result<Vec<String>> {
        let mut maps = Vec::new();

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with("cilium_") {
                        maps.push(name);
                    }
                }
            }
        }

        Ok(maps)
    }

    /// Check if a map exists
    fn has_map(&self, name: &str) -> bool {
        self.available_maps.contains(&name.to_string())
    }

    /// Get path to a specific map
    fn map_path(&self, name: &str) -> PathBuf {
        self.bpf_path.join(name)
    }

    /// Read raw entries from a map
    fn read_map_entries(&self, map_name: &str) -> Result<Vec<Vec<u8>>> {
        let map_path = self.map_path(map_name);

        if !map_path.exists() {
            return Ok(Vec::new());
        }

        // In a real implementation, we would use libbpf or bpf syscalls
        // to iterate through map entries. For now, return empty.
        // This is a placeholder for the actual BPF map iteration logic.

        Ok(Vec::new())
    }

    /// Get list of available maps
    pub fn available_maps(&self) -> &[String] {
        &self.available_maps
    }

    /// Check if Cilium is running (maps are available)
    pub fn is_cilium_available(&self) -> bool {
        !self.available_maps.is_empty()
    }
}

impl MapReader for CiliumMapReader {
    fn read_policy_map(&self) -> Result<Vec<PolicyDecision>> {
        // Use bpftool if available (preferred method)
        if let Some(tool) = &self.bpftool {
            return tool.read_cilium_policy_map();
        }

        // Fallback: Not implemented for direct filesystem access yet
        // This would require parsing pinned maps directly
        Ok(Vec::new())
    }

    fn read_conntrack_map(&self) -> Result<Vec<ConntrackEntry>> {
        // Use bpftool if available (preferred method)
        if let Some(tool) = &self.bpftool {
            return tool.read_cilium_ct_map();
        }

        // Fallback: Not implemented for direct filesystem access yet
        Ok(Vec::new())
    }

    fn read_lb_map(&self) -> Result<Vec<LoadBalancerEntry>> {
        // Use bpftool if available (preferred method)
        if let Some(tool) = &self.bpftool {
            return tool.read_cilium_lb_map();
        }

        // Fallback: Not implemented for direct filesystem access yet
        Ok(Vec::new())
    }

    fn read_ipcache_map(&self) -> Result<Vec<IPCacheEntry>> {
        // Use bpftool if available (preferred method)
        if let Some(tool) = &self.bpftool {
            return tool.read_cilium_ipcache();
        }

        // Fallback: Not implemented for direct filesystem access yet
        Ok(Vec::new())
    }

    fn read_drop_map(&self) -> Result<Vec<DropReason>> {
        // Use bpftool if available (preferred method)
        if let Some(tool) = &self.bpftool {
            // Read metrics map and convert to drop reasons
            let metrics = tool.read_cilium_metrics()?;

            let mut drops = Vec::new();
            for (reason, count) in metrics {
                // Create a drop reason entry for each type with count > 0
                if count > 0 {
                    drops.push(DropReason {
                        src_ip: "0.0.0.0".to_string(),  // Metrics map doesn't have flow info
                        dst_ip: "0.0.0.0".to_string(),
                        port: 0,
                        protocol: 0,
                        reason,
                        timestamp: 0,
                    });
                }
            }

            return Ok(drops);
        }

        // Fallback: Not implemented for direct filesystem access yet
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discover_maps() {
        let temp_dir = TempDir::new().unwrap();
        let bpf_path = temp_dir.path();

        // Create mock map files
        fs::write(bpf_path.join("cilium_policy"), b"").unwrap();
        fs::write(bpf_path.join("cilium_ct4_global"), b"").unwrap();
        fs::write(bpf_path.join("other_file"), b"").unwrap();

        let maps = CiliumMapReader::discover_maps(bpf_path).unwrap();

        assert_eq!(maps.len(), 2);
        assert!(maps.contains(&"cilium_policy".to_string()));
        assert!(maps.contains(&"cilium_ct4_global".to_string()));
        assert!(!maps.contains(&"other_file".to_string()));
    }

    #[test]
    fn test_reader_with_empty_path() {
        let temp_dir = TempDir::new().unwrap();
        let reader = CiliumMapReader::with_path(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(reader.available_maps().len(), 0);
        assert!(!reader.is_cilium_available());
    }

    #[test]
    fn test_has_map() {
        let temp_dir = TempDir::new().unwrap();
        let bpf_path = temp_dir.path();

        fs::write(bpf_path.join("cilium_policy"), b"").unwrap();

        let reader = CiliumMapReader::with_path(bpf_path.to_path_buf()).unwrap();

        assert!(reader.has_map("cilium_policy"));
        assert!(!reader.has_map("cilium_nonexistent"));
    }
}
