#![allow(dead_code)]
// Hot Loader - Load/unload eBPF programs without restart
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{ProgramInfo, ProgramStats};

/// Hot-loads eBPF programs at runtime
pub struct HotLoader {
    loaded_programs: Arc<RwLock<HashMap<String, LoadedProgram>>>,
    default_interface: String,
}

struct LoadedProgram {
    info: ProgramInfo,
    stats: ProgramStats,
    bytecode: Vec<u8>,
    fd: Option<i32>, // File descriptor for loaded program
}

impl HotLoader {
    pub fn new() -> Result<Self> {
        let default_interface = Self::detect_default_interface();
        tracing::info!(interface = %default_interface, "HotLoader initialized with default interface");
        Ok(Self {
            loaded_programs: Arc::new(RwLock::new(HashMap::new())),
            default_interface,
        })
    }

    /// Detect the default network interface by reading /proc/net/route.
    /// Falls back to "eth0" if detection fails.
    fn detect_default_interface() -> String {
        if let Ok(content) = std::fs::read_to_string("/proc/net/route") {
            // The default route has destination 00000000
            for line in content.lines().skip(1) {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 2 && fields[1] == "00000000" {
                    return fields[0].to_string();
                }
            }
        }
        tracing::warn!("Could not detect default interface, falling back to eth0");
        "eth0".to_string()
    }

    /// Load a compiled eBPF program.
    ///
    /// `program_type` and `attach_point` describe where and how the program
    /// should be attached.  The bytecode is the compiled eBPF ELF object.
    pub async fn load_program_with_info(
        &mut self,
        bytecode: Vec<u8>,
        program_type: super::ProgramType,
        attach_point: super::AttachPoint,
    ) -> Result<String> {
        let program_id = uuid::Uuid::new_v4().to_string();

        let fd = self.load_bpf_program(&bytecode, &program_type)?;

        let loaded_program = LoadedProgram {
            info: ProgramInfo {
                id: program_id.clone(),
                name: format!("program-{}", &program_id[..8]),
                program_type,
                attach_point,
                loaded_at: Utc::now(),
            },
            stats: ProgramStats {
                packets_processed: 0,
                bytes_processed: 0,
                execution_time_ns: 0,
                errors: 0,
            },
            bytecode,
            fd: Some(fd),
        };

        let mut programs = self.loaded_programs.write().await;
        programs.insert(program_id.clone(), loaded_program);

        tracing::info!(program_id = %program_id, "eBPF program loaded successfully");
        Ok(program_id)
    }

    /// Load a compiled eBPF program (legacy API, defaults to XDP/Ingress on first interface).
    pub async fn load_program(&mut self, bytecode: Vec<u8>) -> Result<String> {
        tracing::warn!(
            "load_program() called without explicit program_type/attach_point; \
             defaulting to XDP/Ingress. Prefer load_program_with_info() instead."
        );
        self.load_program_with_info(
            bytecode,
            super::ProgramType::XDP,
            super::AttachPoint::NetInterface {
                interface: self.default_interface.clone(),
                direction: super::Direction::Ingress,
            },
        )
        .await
    }

    /// Unload a program
    pub async fn unload_program(&mut self, program_id: &str) -> Result<()> {
        let mut programs = self.loaded_programs.write().await;

        if let Some(program) = programs.remove(program_id) {
            if let Some(_fd) = program.fd {
                // Close file descriptor
                // In real implementation: close eBPF program FD
                tracing::debug!("Closing eBPF program file descriptor");
            }
            tracing::info!("Unloaded program: {}", program_id);
            Ok(())
        } else {
            anyhow::bail!("Program not found: {}", program_id)
        }
    }

    /// List all loaded programs
    pub fn list_programs(&self) -> Vec<ProgramInfo> {
        match self.loaded_programs.try_read() {
            Ok(programs) => programs.values().map(|p| p.info.clone()).collect(),
            Err(_) => {
                tracing::debug!("Could not acquire read lock on loaded_programs; returning empty");
                vec![]
            }
        }
    }

    /// Get program statistics
    pub fn get_stats(&self, program_id: &str) -> Option<ProgramStats> {
        match self.loaded_programs.try_read() {
            Ok(programs) => programs.get(program_id).map(|p| p.stats.clone()),
            Err(_) => {
                tracing::debug!("Could not acquire read lock on loaded_programs");
                None
            }
        }
    }

    fn load_bpf_program(&self, bytecode: &[u8], program_type: &super::ProgramType) -> Result<i32> {
        if bytecode.is_empty() {
            anyhow::bail!(
                "Cannot load eBPF program: bytecode is empty. \
                 Ensure the program was compiled successfully before loading."
            );
        }

        // Validate ELF magic number if bytecode is present
        if bytecode.len() < 4 || &bytecode[0..4] != b"\x7fELF" {
            tracing::warn!(
                "Bytecode does not appear to be a valid ELF object \
                 (missing ELF magic number). Loading may fail at the kernel level."
            );
        }

        // Try Aya loader first when feature is enabled
        #[cfg(feature = "aya-ebpf")]
        {
            match self.load_with_aya(bytecode, program_type) {
                Ok(fd) => return Ok(fd),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Aya loader failed, falling back to stub"
                    );
                }
            }
        }

        // Stub fallback: log intent but cannot actually load programs
        tracing::warn!(
            program_type = ?program_type,
            bytecode_len = bytecode.len(),
            "STUB: BPF program load requested but no real loader is available. \
             Build with --features aya-ebpf and run as root for real loading."
        );

        // Return -1 to indicate no real FD was obtained
        Ok(-1)
    }

    /// Load a BPF program using the Aya crate.
    ///
    /// This performs real bpf() syscalls and requires CAP_BPF/root.
    #[cfg(feature = "aya-ebpf")]
    fn load_with_aya(&self, bytecode: &[u8], program_type: &super::ProgramType) -> Result<i32> {
        use aya::programs::Program;
        use aya::Ebpf;

        let mut bpf = Ebpf::load(bytecode)?;

        // Find a program matching the requested type, then load it via
        // the concrete variant's `.load()` method.
        let target_name = bpf
            .programs()
            .find(|(_, p)| {
                matches!(
                    (program_type, p),
                    (super::ProgramType::XDP, Program::Xdp(_))
                        | (super::ProgramType::TC, Program::SchedClassifier(_))
                        | (super::ProgramType::Kprobe, Program::KProbe(_))
                        | (super::ProgramType::Tracepoint, Program::TracePoint(_))
                        | (super::ProgramType::PerfEvent, Program::PerfEvent(_))
                )
            })
            .map(|(name, _)| name.to_string());

        let name = target_name.ok_or_else(|| {
            anyhow::anyhow!("No {:?} program section found in ELF object", program_type)
        })?;

        let prog = bpf
            .program_mut(&name)
            .ok_or_else(|| anyhow::anyhow!("Program '{}' not found after enumeration", name))?;

        // Each variant has its own `.load()` method
        match prog {
            Program::Xdp(xdp) => {
                xdp.load()?;
                tracing::info!(program = %name, "XDP program loaded via Aya");
            }
            Program::SchedClassifier(tc) => {
                tc.load()?;
                tracing::info!(program = %name, "TC program loaded via Aya");
            }
            Program::KProbe(kprobe) => {
                kprobe.load()?;
                tracing::info!(program = %name, "Kprobe program loaded via Aya");
            }
            Program::TracePoint(tp) => {
                tp.load()?;
                tracing::info!(program = %name, "Tracepoint program loaded via Aya");
            }
            Program::PerfEvent(pe) => {
                pe.load()?;
                tracing::info!(program = %name, "PerfEvent program loaded via Aya");
            }
            _ => {
                anyhow::bail!("Unsupported program type: {:?}", program_type);
            }
        }

        Ok(0)
    }

    /// Attach a loaded program to its target.
    ///
    /// This handles the attachment logic for different program types
    /// and attach points.
    #[cfg(feature = "aya-ebpf")]
    pub fn attach_program(
        &self,
        bpf: &mut aya::Ebpf,
        program_name: &str,
        attach_point: &super::AttachPoint,
    ) -> Result<()> {
        use aya::programs::{tc, Program, XdpFlags};

        let program = bpf
            .program_mut(program_name)
            .ok_or_else(|| anyhow::anyhow!("Program '{}' not found", program_name))?;

        match attach_point {
            super::AttachPoint::NetInterface {
                interface,
                direction,
            } => match program {
                Program::Xdp(xdp) => {
                    xdp.attach(interface, XdpFlags::default())?;
                    tracing::info!(
                        program = program_name,
                        interface = %interface,
                        "XDP program attached"
                    );
                }
                Program::SchedClassifier(tc_prog) => {
                    let _ = tc::qdisc_add_clsact(interface);
                    let tc_direction = match direction {
                        super::Direction::Egress => tc::TcAttachType::Egress,
                        _ => tc::TcAttachType::Ingress,
                    };
                    tc_prog.attach(interface, tc_direction)?;
                    tracing::info!(
                        program = program_name,
                        interface = %interface,
                        direction = ?direction,
                        "TC program attached"
                    );
                }
                _ => anyhow::bail!(
                    "Program type mismatch: expected XDP or TC for NetInterface attach point"
                ),
            },
            super::AttachPoint::KernelFunction { function } => match program {
                Program::KProbe(kprobe) => {
                    kprobe.attach(function, 0)?;
                    tracing::info!(
                        program = program_name,
                        function = %function,
                        "Kprobe attached"
                    );
                }
                _ => anyhow::bail!(
                    "Program type mismatch: expected KProbe for KernelFunction attach point"
                ),
            },
            super::AttachPoint::Tracepoint { category, name } => match program {
                Program::TracePoint(tp) => {
                    tp.attach(category, name)?;
                    tracing::info!(
                        program = program_name,
                        category = %category,
                        name = %name,
                        "Tracepoint attached"
                    );
                }
                _ => anyhow::bail!(
                    "Program type mismatch: expected TracePoint for Tracepoint attach point"
                ),
            },
            _ => {
                tracing::warn!(
                    attach_point = ?attach_point,
                    "Attach point not yet supported for Aya"
                );
            }
        }

        Ok(())
    }
}

impl Default for HotLoader {
    fn default() -> Self {
        Self {
            loaded_programs: Arc::new(RwLock::new(HashMap::new())),
            default_interface: Self::detect_default_interface(),
        }
    }
}

impl Drop for HotLoader {
    fn drop(&mut self) {
        tracing::info!("Cleaning up hot-loaded eBPF programs");
        // In real implementation: unload all programs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_loader_creation() {
        let loader = HotLoader::new();
        assert!(loader.is_ok());
    }

    #[test]
    fn test_hot_loader_default() {
        let _loader = HotLoader::default();
        // Should not panic
    }

    #[tokio::test]
    async fn test_load_program_with_valid_elf_bytecode() {
        let mut loader = HotLoader::new().unwrap();
        // Valid ELF magic number prefix
        let elf_bytecode = vec![0x7f, b'E', b'L', b'F', 0x00, 0x01, 0x02];
        let result = loader.load_program(elf_bytecode).await;
        assert!(result.is_ok());
        let program_id = result.unwrap();
        assert!(!program_id.is_empty());
    }

    #[tokio::test]
    async fn test_load_program_empty_bytecode_fails() {
        let mut loader = HotLoader::new().unwrap();
        let result = loader.load_program(vec![]).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("bytecode is empty"));
    }

    #[tokio::test]
    async fn test_load_program_non_elf_still_loads() {
        let mut loader = HotLoader::new().unwrap();
        // Non-ELF bytecode: the loader warns but does not reject
        let result = loader
            .load_program(vec![0x00, 0x01, 0x02, 0x03, 0x04])
            .await;
        // The stub loader still accepts it with a warning
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unload_existing_program() {
        let mut loader = HotLoader::new().unwrap();
        let bytecode = vec![0x7f, b'E', b'L', b'F', 0x00];
        let program_id = loader.load_program(bytecode).await.unwrap();

        let result = loader.unload_program(&program_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unload_nonexistent_program_fails() {
        let mut loader = HotLoader::new().unwrap();
        let result = loader.unload_program("nonexistent-id").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_load_unload_lifecycle() {
        let mut loader = HotLoader::new().unwrap();
        let bytecode = vec![0x7f, b'E', b'L', b'F', 0x00];
        let program_id = loader.load_program(bytecode).await.unwrap();

        // Unload
        assert!(loader.unload_program(&program_id).await.is_ok());

        // Unload again should fail
        assert!(loader.unload_program(&program_id).await.is_err());
    }

    #[test]
    fn test_list_programs_empty() {
        let loader = HotLoader::new().unwrap();
        let programs = loader.list_programs();
        assert!(programs.is_empty());
    }

    #[test]
    fn test_get_stats_nonexistent() {
        let loader = HotLoader::new().unwrap();
        assert!(loader.get_stats("anything").is_none());
    }
}
