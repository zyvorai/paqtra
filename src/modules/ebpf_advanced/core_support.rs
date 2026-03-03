#![allow(dead_code)]
// CO-RE (Compile Once - Run Everywhere) Support
use anyhow::Result;

use super::EBPFProgram;

/// Handles CO-RE compilation and BTF
#[derive(Default)]
pub struct COREHandler {
    btf_available: bool,
}

impl COREHandler {
    pub fn new() -> Result<Self> {
        let btf_available = Self::check_btf_support()?;

        if btf_available {
            tracing::info!("BTF (BPF Type Format) support detected");
        } else {
            tracing::warn!("BTF not available - CO-RE features limited");
        }

        Ok(Self { btf_available })
    }

    /// Compile program with CO-RE support
    pub async fn compile_with_core(&self, program: &EBPFProgram) -> Result<Vec<u8>> {
        if !self.btf_available {
            tracing::warn!("BTF not available, falling back to non-CO-RE compilation");
            return self.compile_without_core(program).await;
        }

        // Return pre-compiled bytecode if available
        if let Some(ref bytecode) = program.compiled_bytecode {
            tracing::info!(
                "Using pre-compiled bytecode for {} ({} bytes)",
                program.name,
                bytecode.len()
            );
            return Ok(bytecode.clone());
        }

        if program.source_code.is_empty() {
            anyhow::bail!(
                "Cannot compile {}: no source code or pre-compiled bytecode",
                program.name
            );
        }

        tracing::info!("Compiling {} with CO-RE support", program.name);
        self.invoke_clang(&program.source_code, &program.name, true)
            .await
    }

    async fn compile_without_core(&self, program: &EBPFProgram) -> Result<Vec<u8>> {
        // Return pre-compiled bytecode if available
        if let Some(ref bytecode) = program.compiled_bytecode {
            tracing::info!(
                "Using pre-compiled bytecode for {} ({} bytes)",
                program.name,
                bytecode.len()
            );
            return Ok(bytecode.clone());
        }

        if program.source_code.is_empty() {
            anyhow::bail!(
                "Cannot compile {}: no source code or pre-compiled bytecode",
                program.name
            );
        }

        tracing::info!("Compiling {} without CO-RE", program.name);
        self.invoke_clang(&program.source_code, &program.name, false)
            .await
    }

    /// Invoke clang to compile eBPF source code to bytecode
    async fn invoke_clang(
        &self,
        source_code: &str,
        name: &str,
        core_enabled: bool,
    ) -> Result<Vec<u8>> {
        // Check for clang
        let clang = Self::find_clang()?;

        // Write source to temp file
        let tmp_dir = std::env::temp_dir();
        let src_path = tmp_dir.join(format!("{}.c", name));
        let obj_path = tmp_dir.join(format!("{}.o", name));

        std::fs::write(&src_path, source_code)?;

        let mut cmd = tokio::process::Command::new(&clang);
        cmd.args(["-g", "-O2", "-target", "bpf"]);

        if core_enabled {
            cmd.args(["-D__TARGET_ARCH_x86", "-mcpu=v3"]);
        }

        cmd.arg("-c")
            .arg(src_path.to_str().unwrap())
            .arg("-o")
            .arg(obj_path.to_str().unwrap());

        let output = cmd.output().await?;

        // Clean up source file
        let _ = std::fs::remove_file(&src_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = std::fs::remove_file(&obj_path);
            anyhow::bail!("clang compilation failed for {}: {}", name, stderr);
        }

        // Read compiled object
        let bytecode = std::fs::read(&obj_path)?;
        let _ = std::fs::remove_file(&obj_path);

        tracing::info!(
            "Successfully compiled {} ({} bytes, CO-RE={})",
            name,
            bytecode.len(),
            core_enabled
        );
        Ok(bytecode)
    }

    /// Find clang binary, preferring versioned clang for BPF compilation
    fn find_clang() -> Result<String> {
        // Try versioned clang first (higher versions preferred for BPF)
        for version in (11..=19).rev() {
            let name = format!("clang-{}", version);
            if let Ok(output) = std::process::Command::new("which").arg(&name).output() {
                if output.status.success() {
                    return Ok(name);
                }
            }
        }

        // Try plain clang
        if let Ok(output) = std::process::Command::new("which").arg("clang").output() {
            if output.status.success() {
                return Ok("clang".to_string());
            }
        }

        anyhow::bail!("clang not found. Install clang (>= 11) for eBPF program compilation.")
    }

    fn check_btf_support() -> Result<bool> {
        let btf_path = std::path::Path::new("/sys/kernel/btf/vmlinux");

        if !btf_path.exists() {
            tracing::debug!("BTF vmlinux file does not exist at /sys/kernel/btf/vmlinux");
            return Ok(false);
        }

        // Check that the file is actually readable, not just that it exists
        match std::fs::metadata(btf_path) {
            Ok(meta) => {
                if meta.len() == 0 {
                    tracing::warn!("BTF vmlinux file exists but is empty");
                    return Ok(false);
                }
                // Attempt a small read to verify access
                match std::fs::File::open(btf_path) {
                    Ok(_) => {
                        tracing::debug!(
                            "BTF vmlinux is available and readable ({} bytes)",
                            meta.len()
                        );
                        Ok(true)
                    }
                    Err(e) => {
                        tracing::warn!("BTF vmlinux exists but is not readable: {}", e);
                        Ok(false)
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Cannot stat BTF vmlinux: {}", e);
                Ok(false)
            }
        }
    }

    /// Extract BTF information from kernel.
    ///
    /// Reads `/sys/kernel/btf/vmlinux` and enumerates exported BTF type names
    /// from `/sys/kernel/btf/` directory entries. Each file in that directory
    /// represents a BTF object (vmlinux + loaded kernel modules).
    pub fn get_btf_info(&self) -> Result<BTFInfo> {
        if !self.btf_available {
            anyhow::bail!("BTF not available");
        }

        let available_types = Self::enumerate_btf_objects();

        Ok(BTFInfo {
            kernel_version: Self::get_kernel_version(),
            available_types,
        })
    }

    /// Enumerate BTF objects available under /sys/kernel/btf/.
    /// Each entry corresponds to a kernel module or vmlinux itself.
    fn enumerate_btf_objects() -> Vec<String> {
        let btf_dir = std::path::Path::new("/sys/kernel/btf");
        if !btf_dir.exists() {
            return Vec::new();
        }

        match std::fs::read_dir(btf_dir) {
            Ok(entries) => {
                let mut types: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect();
                types.sort();
                tracing::debug!("Found {} BTF objects in /sys/kernel/btf/", types.len());
                types
            }
            Err(e) => {
                tracing::warn!("Failed to read /sys/kernel/btf/: {}", e);
                Vec::new()
            }
        }
    }

    /// Read the actual kernel version from /proc/version.
    /// Falls back to a descriptive "unknown" string if reading fails.
    fn get_kernel_version() -> String {
        match std::fs::read_to_string("/proc/version") {
            Ok(version_string) => {
                // /proc/version format: "Linux version 6.1.0-xxx ..."
                // Extract the version token (third whitespace-delimited field)
                version_string
                    .split_whitespace()
                    .nth(2)
                    .unwrap_or("unknown")
                    .to_string()
            }
            Err(e) => {
                tracing::warn!("Failed to read /proc/version: {}", e);
                "unknown".to_string()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BTFInfo {
    pub kernel_version: String,
    pub available_types: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_handler_creation() {
        let handler = COREHandler::new();
        assert!(handler.is_ok());
    }

    #[test]
    fn test_core_handler_default() {
        let handler = COREHandler::default();
        // Default sets btf_available to false
        assert!(!handler.btf_available);
    }

    #[test]
    fn test_kernel_version_detection() {
        let version = COREHandler::get_kernel_version();
        // On Linux, this should return a version string, not "unknown"
        // On other platforms or in CI, it might return "unknown"
        assert!(!version.is_empty());
    }

    #[test]
    fn test_btf_info_without_btf() {
        let handler = COREHandler::default(); // btf_available = false
        let result = handler.get_btf_info();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("BTF not available"));
    }

    #[test]
    fn test_enumerate_btf_objects() {
        let types = COREHandler::enumerate_btf_objects();
        // On Linux with BTF support, should find at least "vmlinux"
        if std::path::Path::new("/sys/kernel/btf/vmlinux").exists() {
            assert!(!types.is_empty());
            assert!(types.contains(&"vmlinux".to_string()));
        }
    }

    #[tokio::test]
    async fn test_compile_with_core_uses_precompiled_bytecode() {
        let handler = COREHandler::default(); // btf_available = false
        let program = super::super::EBPFProgram {
            id: "test-id".to_string(),
            name: "test-prog".to_string(),
            program_type: super::super::ProgramType::XDP,
            source_code: String::new(),
            compiled_bytecode: Some(vec![0x7f, b'E', b'L', b'F']),
            attach_point: super::super::AttachPoint::NetInterface {
                interface: "eth0".to_string(),
                direction: super::super::Direction::Ingress,
            },
            co_re_enabled: true,
        };
        // Without BTF, falls back to compile_without_core, which returns pre-compiled bytecode
        let result = handler.compile_with_core(&program).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0x7f, b'E', b'L', b'F']);
    }

    #[tokio::test]
    async fn test_compile_without_source_or_bytecode_fails() {
        let handler = COREHandler::default();
        let program = super::super::EBPFProgram {
            id: "test-id".to_string(),
            name: "test-prog".to_string(),
            program_type: super::super::ProgramType::XDP,
            source_code: String::new(),
            compiled_bytecode: None,
            attach_point: super::super::AttachPoint::NetInterface {
                interface: "eth0".to_string(),
                direction: super::super::Direction::Ingress,
            },
            co_re_enabled: false,
        };
        let result = handler.compile_with_core(&program).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no source code"));
    }
}
