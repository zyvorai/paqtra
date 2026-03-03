#![allow(dead_code)]
/// BPF Capability Detection
///
/// Probes the runtime environment to determine what level of BPF access
/// is available: full program loading, read-only map access, bpftool CLI,
/// or no BPF access at all.
use std::path::Path;

/// Level of BPF access available on this system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfCapability {
    /// Root / CAP_BPF: load programs, read/write maps, perf events
    Full,
    /// /sys/fs/bpf/ readable: native map reads only
    ReadOnly,
    /// bpftool binary available: CLI-based map reads
    BpftoolOnly,
    /// No BPF access: MockMapReader fallback
    None,
}

/// Detected BPF capabilities for the current process.
#[derive(Debug, Clone)]
pub struct BpfCapabilities {
    pub capability: BpfCapability,
    pub bpf_fs_available: bool,
    pub bpf_fs_writable: bool,
    pub bpftool_available: bool,
    pub kernel_version: Option<String>,
}

impl BpfCapabilities {
    /// Probe the current system for BPF capabilities.
    pub fn detect() -> Self {
        let bpf_fs_available = Path::new("/sys/fs/bpf").exists();
        let bpf_fs_writable = bpf_fs_available && Self::check_bpf_fs_writable();
        let bpftool_available = Self::check_bpftool();
        let kernel_version = Self::read_kernel_version();

        let capability = if bpf_fs_writable && Self::check_cap_bpf() {
            BpfCapability::Full
        } else if bpf_fs_available {
            BpfCapability::ReadOnly
        } else if bpftool_available {
            BpfCapability::BpftoolOnly
        } else {
            BpfCapability::None
        };

        tracing::info!(
            capability = ?capability,
            bpf_fs_available,
            bpf_fs_writable,
            bpftool_available,
            kernel = ?kernel_version,
            "BPF capabilities detected"
        );

        Self {
            capability,
            bpf_fs_available,
            bpf_fs_writable,
            bpftool_available,
            kernel_version,
        }
    }

    /// Check whether we can write to the BPF filesystem (indicates full access).
    fn check_bpf_fs_writable() -> bool {
        // Try to stat the tc/globals directory — if it's writable we likely have
        // sufficient privileges to pin/unpin maps.
        let globals = Path::new("/sys/fs/bpf/tc/globals");
        if !globals.exists() {
            return false;
        }
        // A lightweight writability check: attempt to create and immediately
        // remove a temporary file. If we can do this, we have write access.
        let probe_path = globals.join(".cilium_tui_probe");
        match std::fs::write(&probe_path, b"") {
            Ok(()) => {
                let _ = std::fs::remove_file(&probe_path);
                true
            }
            Err(_) => false,
        }
    }

    /// Check if the current process has CAP_BPF or is running as root.
    fn check_cap_bpf() -> bool {
        // Fast check: are we root?
        if unsafe { libc::geteuid() } == 0 {
            return true;
        }

        // Check /proc/self/status for CapEff containing the CAP_BPF bit (bit 39).
        // CapEff is a hex-encoded capability mask.
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if let Some(hex) = line.strip_prefix("CapEff:\t") {
                    if let Ok(cap_eff) = u64::from_str_radix(hex.trim(), 16) {
                        // CAP_BPF = 39, CAP_SYS_ADMIN = 21
                        let cap_bpf = 1u64 << 39;
                        let cap_sys_admin = 1u64 << 21;
                        return (cap_eff & cap_bpf) != 0 || (cap_eff & cap_sys_admin) != 0;
                    }
                }
            }
        }

        false
    }

    /// Check if bpftool is available on the system.
    fn check_bpftool() -> bool {
        std::process::Command::new("bpftool")
            .arg("version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    /// Read the kernel version string.
    fn read_kernel_version() -> Option<String> {
        std::fs::read_to_string("/proc/version")
            .ok()
            .and_then(|v| v.split_whitespace().nth(2).map(String::from))
    }

    /// Whether Aya-based operations should be attempted.
    pub fn can_use_aya(&self) -> bool {
        matches!(
            self.capability,
            BpfCapability::Full | BpfCapability::ReadOnly
        )
    }

    /// Whether program loading is possible.
    pub fn can_load_programs(&self) -> bool {
        self.capability == BpfCapability::Full
    }

    /// Whether map reads are possible (via any method).
    pub fn can_read_maps(&self) -> bool {
        self.capability != BpfCapability::None
    }

    /// Whether map writes are possible.
    pub fn can_write_maps(&self) -> bool {
        self.capability == BpfCapability::Full
    }
}

impl Default for BpfCapabilities {
    fn default() -> Self {
        Self::detect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_levels() {
        // Verify enum ordering / equality
        assert_ne!(BpfCapability::Full, BpfCapability::None);
        assert_ne!(BpfCapability::ReadOnly, BpfCapability::BpftoolOnly);
        assert_eq!(BpfCapability::Full, BpfCapability::Full);
    }

    #[test]
    fn test_detect_returns_valid_capability() {
        let caps = BpfCapabilities::detect();
        // Should always return some capability level
        assert!(matches!(
            caps.capability,
            BpfCapability::Full
                | BpfCapability::ReadOnly
                | BpfCapability::BpftoolOnly
                | BpfCapability::None
        ));
    }

    #[test]
    fn test_can_use_aya() {
        let mut caps = BpfCapabilities {
            capability: BpfCapability::Full,
            bpf_fs_available: true,
            bpf_fs_writable: true,
            bpftool_available: true,
            kernel_version: None,
        };
        assert!(caps.can_use_aya());
        assert!(caps.can_load_programs());
        assert!(caps.can_read_maps());
        assert!(caps.can_write_maps());

        caps.capability = BpfCapability::ReadOnly;
        assert!(caps.can_use_aya());
        assert!(!caps.can_load_programs());
        assert!(caps.can_read_maps());
        assert!(!caps.can_write_maps());

        caps.capability = BpfCapability::BpftoolOnly;
        assert!(!caps.can_use_aya());
        assert!(!caps.can_load_programs());
        assert!(caps.can_read_maps());

        caps.capability = BpfCapability::None;
        assert!(!caps.can_use_aya());
        assert!(!caps.can_load_programs());
        assert!(!caps.can_read_maps());
        assert!(!caps.can_write_maps());
    }

    #[test]
    fn test_kernel_version_read() {
        // On Linux this should return something; on non-Linux it may be None
        let version = BpfCapabilities::read_kernel_version();
        if cfg!(target_os = "linux") {
            assert!(version.is_some());
        }
    }
}
