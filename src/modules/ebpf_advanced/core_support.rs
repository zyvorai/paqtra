// CO-RE (Compile Once - Run Everywhere) Support
use anyhow::Result;

use super::EBPFProgram;

/// Handles CO-RE compilation and BTF
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

        tracing::info!("Compiling {} with CO-RE support", program.name);

        // In real implementation:
        // 1. Parse BTF from kernel
        // 2. Compile with clang -g -O2 -target bpf -D__TARGET_ARCH_x86
        // 3. Generate relocations for struct offsets
        // 4. Enable BTF and CO-RE features

        // For now, return stub
        Ok(vec![])
    }

    async fn compile_without_core(&self, program: &EBPFProgram) -> Result<Vec<u8>> {
        tracing::info!("Compiling {} without CO-RE", program.name);
        // Standard compilation without CO-RE
        Ok(vec![])
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

    /// Extract BTF information from kernel
    pub fn get_btf_info(&self) -> Result<BTFInfo> {
        if !self.btf_available {
            anyhow::bail!("BTF not available");
        }

        // In real implementation: parse /sys/kernel/btf/vmlinux
        Ok(BTFInfo {
            kernel_version: Self::get_kernel_version(),
            available_types: Vec::new(),
        })
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

impl Default for COREHandler {
    fn default() -> Self {
        Self {
            btf_available: false,
        }
    }
}
