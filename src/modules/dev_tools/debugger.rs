// Interactive Debugger - Debug network requests in real-time
use anyhow::Result;

use super::DebugConfig;

/// Interactive network debugger
pub struct InteractiveDebugger {}

impl InteractiveDebugger {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub async fn start_session(&self, config: DebugConfig) -> Result<String> {
        if config.service.is_empty() {
            anyhow::bail!("service name cannot be empty");
        }
        if config.namespace.is_empty() {
            anyhow::bail!("namespace cannot be empty");
        }

        anyhow::bail!(
            "Interactive debugging is not yet implemented. \
             Would start {:?} debug session for {}/{} with {} breakpoint(s). \
             In production: attach eBPF probes to service, set up request/response \
             interception, enable breakpoints, and start trace collection.",
            config.debug_mode,
            config.namespace,
            config.service,
            config.breakpoints.len()
        )
    }
}

impl Default for InteractiveDebugger {
    fn default() -> Self {
        Self {}
    }
}
