/// Native BPF Map Writer using Aya
///
/// Writes to Cilium's pinned BPF maps (policy, LB, ipcache, metrics)
/// via `aya::maps` without shelling out to bpftool.
use anyhow::Result;
use std::path::PathBuf;

use super::MapWriter;

/// Default path where Cilium pins its BPF maps.
const CILIUM_BPF_PATH: &str = "/sys/fs/bpf/tc/globals";

/// Aya-based map writer for Cilium BPF maps.
pub struct AyaMapWriter {
    bpf_path: PathBuf,
}

impl AyaMapWriter {
    /// Create a new writer using the default Cilium BPF path.
    pub fn new() -> Result<Self> {
        Self::with_path(PathBuf::from(CILIUM_BPF_PATH))
    }

    /// Create a writer with a custom BPF path.
    pub fn with_path(bpf_path: PathBuf) -> Result<Self> {
        if !bpf_path.exists() {
            anyhow::bail!("BPF path {:?} does not exist. Is Cilium running?", bpf_path);
        }
        Ok(Self { bpf_path })
    }

    /// Get the full path for a pinned map.
    fn map_path(&self, name: &str) -> PathBuf {
        self.bpf_path.join(name)
    }

    /// Write a typed key-value pair to a pinned hash map.
    #[cfg(feature = "aya-ebpf")]
    fn write_typed_entry<const K: usize, const V: usize>(
        &self,
        map_name: &str,
        key: &[u8; K],
        value: &[u8; V],
    ) -> Result<()> {
        use aya::maps::{HashMap, Map, MapData};

        let path = self.map_path(map_name);
        if !path.exists() {
            anyhow::bail!("Map {} not found at {:?}", map_name, path);
        }

        let map_data = MapData::from_pin(&path)?;
        let map = Map::HashMap(map_data);
        let mut map: HashMap<_, [u8; K], [u8; V]> = HashMap::try_from(map)?;
        map.insert(*key, *value, 0)?;

        tracing::debug!(map = map_name, key_len = K, "Entry written");
        Ok(())
    }

    #[cfg(not(feature = "aya-ebpf"))]
    fn write_typed_entry<const K: usize, const V: usize>(
        &self,
        map_name: &str,
        _key: &[u8; K],
        _value: &[u8; V],
    ) -> Result<()> {
        tracing::warn!(
            map = map_name,
            "Cannot write map entry: aya-ebpf feature not enabled"
        );
        anyhow::bail!("aya-ebpf feature not enabled: cannot write to BPF maps")
    }

    /// Delete a typed key from a pinned hash map.
    #[cfg(feature = "aya-ebpf")]
    fn delete_typed_entry<const K: usize, const V: usize>(
        &self,
        map_name: &str,
        key: &[u8; K],
    ) -> Result<()> {
        use aya::maps::{HashMap, Map, MapData};

        let path = self.map_path(map_name);
        if !path.exists() {
            anyhow::bail!("Map {} not found at {:?}", map_name, path);
        }

        let map_data = MapData::from_pin(&path)?;
        let map = Map::HashMap(map_data);
        let mut map: HashMap<_, [u8; K], [u8; V]> = HashMap::try_from(map)?;
        map.remove(key)?;

        tracing::debug!(map = map_name, key_len = K, "Entry deleted");
        Ok(())
    }

    #[cfg(not(feature = "aya-ebpf"))]
    fn delete_typed_entry<const K: usize, const V: usize>(
        &self,
        map_name: &str,
        _key: &[u8; K],
    ) -> Result<()> {
        tracing::warn!(
            map = map_name,
            "Cannot delete map entry: aya-ebpf feature not enabled"
        );
        anyhow::bail!("aya-ebpf feature not enabled: cannot write to BPF maps")
    }

    /// Build a policy map key from identity, port, and protocol.
    fn build_policy_key(src_identity: u32, dst_port: u16, protocol: u8, egress: bool) -> [u8; 12] {
        let mut key = [0u8; 12];
        key[0..4].copy_from_slice(&src_identity.to_le_bytes());
        key[4..6].copy_from_slice(&dst_port.to_be_bytes());
        key[6] = protocol;
        key[7] = if egress { 1 } else { 0 };
        key
    }

    /// Build an LB4 service key from IP, port, slot, and protocol.
    fn build_lb4_key(ip: std::net::Ipv4Addr, port: u16, slot: u16, proto: u8) -> [u8; 11] {
        let mut key = [0u8; 11];
        key[0..4].copy_from_slice(&ip.octets());
        key[4..6].copy_from_slice(&port.to_be_bytes());
        key[6..8].copy_from_slice(&slot.to_be_bytes());
        key[8] = proto;
        key
    }
}

impl MapWriter for AyaMapWriter {
    fn write_policy_entry(
        &self,
        src_identity: u32,
        dst_port: u16,
        protocol: u8,
        allow: bool,
    ) -> Result<()> {
        let key = Self::build_policy_key(src_identity, dst_port, protocol, false);

        // Value: packets(u64) + bytes(u64) + proxy_port(u16) + pad
        let mut value = [0u8; 24];
        if allow {
            value[0] = 1; // packets = 1 → Allow verdict
        }

        self.write_typed_entry::<12, 24>("cilium_policy", &key, &value)
    }

    fn delete_policy_entry(&self, src_identity: u32, dst_port: u16, protocol: u8) -> Result<()> {
        let key = Self::build_policy_key(src_identity, dst_port, protocol, false);
        self.delete_typed_entry::<12, 24>("cilium_policy", &key)
    }

    fn write_lb_entry(
        &self,
        service_ip: std::net::Ipv4Addr,
        service_port: u16,
        backend_ip: std::net::Ipv4Addr,
        backend_port: u16,
        slot: u16,
        protocol: u8,
    ) -> Result<()> {
        let key = Self::build_lb4_key(service_ip, service_port, slot, protocol);

        let mut value = [0u8; 8];
        value[0..4].copy_from_slice(&backend_ip.octets());
        value[4..6].copy_from_slice(&backend_port.to_be_bytes());

        self.write_typed_entry::<11, 8>("cilium_lb4_services_v2", &key, &value)
    }

    fn delete_lb_entry(
        &self,
        service_ip: std::net::Ipv4Addr,
        service_port: u16,
        slot: u16,
        protocol: u8,
    ) -> Result<()> {
        let key = Self::build_lb4_key(service_ip, service_port, slot, protocol);
        self.delete_typed_entry::<11, 8>("cilium_lb4_services_v2", &key)
    }

    fn write_ipcache_entry(
        &self,
        ip: std::net::Ipv4Addr,
        identity: u32,
        prefix_len: u32,
    ) -> Result<()> {
        // Build key: prefix_len(u32) + cluster_id(u16) + family(u8) + pad(u8) + ip(4B)
        let mut key = [0u8; 12];
        key[0..4].copy_from_slice(&prefix_len.to_le_bytes());
        key[6] = 2; // AF_INET
        key[8..12].copy_from_slice(&ip.octets());

        let mut value = [0u8; 8];
        value[0..4].copy_from_slice(&identity.to_le_bytes());

        self.write_typed_entry::<12, 8>("cilium_ipcache", &key, &value)
    }

    fn clear_metrics(&self) -> Result<()> {
        #[cfg(feature = "aya-ebpf")]
        {
            use aya::maps::{HashMap, Map, MapData};

            let path = self.map_path("cilium_metrics");
            if !path.exists() {
                tracing::debug!("Metrics map not found, nothing to clear");
                return Ok(());
            }

            let map_data = MapData::from_pin(&path)?;
            let map = Map::HashMap(map_data);
            let mut map: HashMap<_, [u8; 4], [u8; 16]> = HashMap::try_from(map)?;

            // Collect all keys first, then zero their values
            let keys: Vec<[u8; 4]> = map.keys().filter_map(|r| r.ok()).collect();

            let zero_value = [0u8; 16]; // count(u64) + bytes(u64)
            for key in keys {
                if let Err(e) = map.insert(key, zero_value, 0) {
                    tracing::trace!(error = %e, "Failed to zero metrics entry");
                }
            }

            tracing::info!("Metrics map cleared");
        }

        #[cfg(not(feature = "aya-ebpf"))]
        {
            anyhow::bail!("Cannot clear metrics: aya-ebpf feature not enabled");
        }

        #[cfg(feature = "aya-ebpf")]
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_writer_missing_path() {
        let result = AyaMapWriter::with_path(PathBuf::from("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_writer_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let writer = AyaMapWriter::with_path(temp_dir.path().to_path_buf());
        assert!(writer.is_ok());
    }

    #[test]
    fn test_build_policy_key() {
        let key = AyaMapWriter::build_policy_key(100, 80, 6, false);
        assert_eq!(key.len(), 12);
        assert_eq!(&key[0..4], &100u32.to_le_bytes());
        assert_eq!(&key[4..6], &80u16.to_be_bytes());
        assert_eq!(key[6], 6);
        assert_eq!(key[7], 0);
    }

    #[test]
    fn test_build_policy_key_egress() {
        let key = AyaMapWriter::build_policy_key(200, 443, 6, true);
        assert_eq!(key[7], 1);
    }

    #[test]
    fn test_build_lb4_key() {
        let ip = std::net::Ipv4Addr::new(10, 96, 0, 1);
        let key = AyaMapWriter::build_lb4_key(ip, 443, 1, 6);
        assert_eq!(key.len(), 11);
        assert_eq!(&key[0..4], &[10, 96, 0, 1]);
        assert_eq!(&key[4..6], &443u16.to_be_bytes());
        assert_eq!(&key[6..8], &1u16.to_be_bytes());
        assert_eq!(key[8], 6);
    }

    #[test]
    fn test_map_writer_trait_without_feature() {
        let temp_dir = TempDir::new().unwrap();
        let writer = AyaMapWriter::with_path(temp_dir.path().to_path_buf()).unwrap();

        // Without aya-ebpf feature, write/delete operations should return errors
        let result = writer.write_policy_entry(100, 80, 6, true);
        assert!(result.is_err());

        let result = writer.clear_metrics();
        // Without aya-ebpf feature, clear_metrics should return an error
        assert!(result.is_err());
    }
}
