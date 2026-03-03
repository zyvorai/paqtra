#![allow(dead_code)]
/// Native BPF Map Reader using Aya
///
/// Reads Cilium's pinned BPF maps directly via `aya::maps` without
/// shelling out to bpftool. Reuses parsers from `bpf_parser` for
/// binary→struct conversion.
use anyhow::Result;
use std::path::PathBuf;

#[cfg(feature = "aya-ebpf")]
use super::bpf_parser;
use super::{
    ConntrackEntry, DropReason, IPCacheEntry, LoadBalancerEntry, MapReader, PolicyDecision,
};

/// Default path where Cilium pins its BPF maps.
const CILIUM_BPF_PATH: &str = "/sys/fs/bpf/tc/globals";

/// Map names used by Cilium.
const MAP_POLICY: &str = "cilium_policy";
const MAP_CT4: &str = "cilium_ct4_global";
const MAP_CT6: &str = "cilium_ct6_global";
const MAP_LB4: &str = "cilium_lb4_services_v2";
const MAP_IPCACHE: &str = "cilium_ipcache";
const MAP_METRICS: &str = "cilium_metrics";

/// Native Aya-based map reader for Cilium BPF maps.
///
/// Opens pinned maps from the BPF filesystem and iterates entries
/// using raw key/value byte arrays, then delegates parsing to
/// `bpf_parser` functions.
pub struct AyaMapReader {
    bpf_path: PathBuf,
}

impl AyaMapReader {
    /// Create a new reader using the default Cilium BPF path.
    pub fn new() -> Result<Self> {
        Self::with_path(PathBuf::from(CILIUM_BPF_PATH))
    }

    /// Create a reader with a custom BPF path.
    pub fn with_path(bpf_path: PathBuf) -> Result<Self> {
        if !bpf_path.exists() {
            anyhow::bail!("BPF path {:?} does not exist. Is Cilium running?", bpf_path);
        }
        Ok(Self { bpf_path })
    }

    /// Check if a specific map is pinned at the expected path.
    pub fn has_map(&self, name: &str) -> bool {
        self.bpf_path.join(name).exists()
    }

    /// Get the full path for a pinned map.
    fn map_path(&self, name: &str) -> PathBuf {
        self.bpf_path.join(name)
    }

    /// Read all entries from a pinned hash map using Aya.
    ///
    /// Uses fixed-size key/value arrays (Aya requires `Pod` types which
    /// must be `Copy + 'static`). The const generics K and V specify
    /// the key and value sizes for the specific map type.
    #[cfg(feature = "aya-ebpf")]
    fn read_typed_hash_map<const K: usize, const V: usize>(
        &self,
        map_name: &str,
    ) -> Vec<([u8; K], [u8; V])> {
        let path = self.map_path(map_name);
        if !path.exists() {
            tracing::debug!(map = map_name, "Pinned map not found, skipping");
            return Vec::new();
        }

        match Self::open_and_read_hash_map::<K, V>(&path) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!(map = map_name, error = %e, "Failed to read pinned map via Aya");
                Vec::new()
            }
        }
    }

    /// Open a pinned hash map and iterate all entries with fixed-size types.
    #[cfg(feature = "aya-ebpf")]
    fn open_and_read_hash_map<const K: usize, const V: usize>(
        path: &std::path::Path,
    ) -> Result<Vec<([u8; K], [u8; V])>> {
        use aya::maps::{HashMap, Map, MapData};

        let map_data = MapData::from_pin(path)?;
        let map = Map::HashMap(map_data);
        let map: HashMap<_, [u8; K], [u8; V]> = HashMap::try_from(map)?;

        let mut entries = Vec::new();
        for result in map.iter() {
            match result {
                Ok((key, value)) => {
                    entries.push((key, value));
                }
                Err(e) => {
                    tracing::trace!(error = %e, "Skipping map entry due to iteration error");
                }
            }
        }

        Ok(entries)
    }
}

#[allow(clippy::needless_return)]
impl MapReader for AyaMapReader {
    fn read_policy_map(&self) -> Result<Vec<PolicyDecision>> {
        #[cfg(feature = "aya-ebpf")]
        {
            // Policy map: key=12 bytes, value=24 bytes
            let entries = self.read_typed_hash_map::<12, 24>(MAP_POLICY);
            let mut results = Vec::with_capacity(entries.len());
            for (key, value) in &entries {
                match bpf_parser::parse_policy_entry(key, value) {
                    Ok(entry) => results.push(entry),
                    Err(e) => tracing::trace!(error = %e, "Skipping malformed policy entry"),
                }
            }
            return Ok(results);
        }
        #[cfg(not(feature = "aya-ebpf"))]
        Ok(Vec::new())
    }

    fn read_conntrack_map(&self) -> Result<Vec<ConntrackEntry>> {
        #[cfg(feature = "aya-ebpf")]
        {
            let mut results = Vec::new();

            // CT4 map: key=14 bytes, value=48 bytes
            let ct4_entries = self.read_typed_hash_map::<14, 48>(MAP_CT4);
            for (key, value) in &ct4_entries {
                match bpf_parser::parse_ct_entry(key, value) {
                    Ok(entry) => results.push(entry),
                    Err(e) => tracing::trace!(error = %e, "Skipping malformed CT4 entry"),
                }
            }

            // CT6 map: key=38 bytes, value=48 bytes
            let ct6_entries = self.read_typed_hash_map::<38, 48>(MAP_CT6);
            for (key, value) in &ct6_entries {
                match bpf_parser::parse_ct6_entry(key, value) {
                    Ok(entry) => results.push(entry),
                    Err(e) => tracing::trace!(error = %e, "Skipping malformed CT6 entry"),
                }
            }

            return Ok(results);
        }
        #[cfg(not(feature = "aya-ebpf"))]
        Ok(Vec::new())
    }

    fn read_lb_map(&self) -> Result<Vec<LoadBalancerEntry>> {
        #[cfg(feature = "aya-ebpf")]
        {
            // LB4 map: key=11 bytes, value=8 bytes
            let entries = self.read_typed_hash_map::<11, 8>(MAP_LB4);
            let mut results = Vec::with_capacity(entries.len());
            for (key, value) in &entries {
                match bpf_parser::parse_lb_entry(key, value) {
                    Ok(entry) => results.push(entry),
                    Err(e) => tracing::trace!(error = %e, "Skipping malformed LB entry"),
                }
            }
            return Ok(results);
        }
        #[cfg(not(feature = "aya-ebpf"))]
        Ok(Vec::new())
    }

    fn read_ipcache_map(&self) -> Result<Vec<IPCacheEntry>> {
        #[cfg(feature = "aya-ebpf")]
        {
            // IPCache as hash map fallback: key=24 bytes, value=8 bytes
            let entries = self.read_typed_hash_map::<24, 8>(MAP_IPCACHE);
            let mut results = Vec::with_capacity(entries.len());
            for (key, value) in &entries {
                match bpf_parser::parse_ipcache_entry(key, value) {
                    Ok(entry) => results.push(entry),
                    Err(e) => tracing::trace!(error = %e, "Skipping malformed ipcache entry"),
                }
            }
            return Ok(results);
        }
        #[cfg(not(feature = "aya-ebpf"))]
        Ok(Vec::new())
    }

    fn read_drop_map(&self) -> Result<Vec<DropReason>> {
        #[cfg(feature = "aya-ebpf")]
        {
            // Metrics map: key=4 bytes, value=16 bytes
            let entries = self.read_typed_hash_map::<4, 16>(MAP_METRICS);
            let mut results = Vec::new();
            for (key, value) in &entries {
                match bpf_parser::parse_metrics_entry(key, value) {
                    Ok((reason, count)) if count > 0 => {
                        results.push(DropReason {
                            src_ip: "0.0.0.0".to_string(),
                            dst_ip: "0.0.0.0".to_string(),
                            port: 0,
                            protocol: 0,
                            reason,
                            timestamp: 0,
                        });
                    }
                    Ok(_) => {}
                    Err(e) => tracing::trace!(error = %e, "Skipping malformed metrics entry"),
                }
            }
            return Ok(results);
        }
        #[cfg(not(feature = "aya-ebpf"))]
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_aya_reader_missing_path() {
        let result = AyaMapReader::with_path(PathBuf::from("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_aya_reader_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let reader = AyaMapReader::with_path(temp_dir.path().to_path_buf());
        assert!(reader.is_ok());
    }

    #[test]
    fn test_has_map_false_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let reader = AyaMapReader::with_path(temp_dir.path().to_path_buf()).unwrap();
        assert!(!reader.has_map("cilium_policy"));
    }

    #[test]
    fn test_has_map_true_when_present() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("cilium_policy"), b"").unwrap();
        let reader = AyaMapReader::with_path(temp_dir.path().to_path_buf()).unwrap();
        assert!(reader.has_map("cilium_policy"));
    }

    #[test]
    fn test_read_empty_maps() {
        let temp_dir = TempDir::new().unwrap();
        let reader = AyaMapReader::with_path(temp_dir.path().to_path_buf()).unwrap();

        // Without actual pinned maps, all reads should return empty
        assert!(reader.read_policy_map().unwrap().is_empty());
        assert!(reader.read_conntrack_map().unwrap().is_empty());
        assert!(reader.read_lb_map().unwrap().is_empty());
        assert!(reader.read_ipcache_map().unwrap().is_empty());
        assert!(reader.read_drop_map().unwrap().is_empty());
    }
}
