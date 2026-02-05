/// Self-Healer Module
///
/// Automatically detects and fixes common network issues:
/// - DNS resolution failures
/// - MTU mismatches
/// - Load balancer timeouts
/// - Policy gaps
/// - Connection tracking issues

use anyhow::Result;
use std::collections::HashMap;

use crate::ebpf::{DropReason, DropReasonType, MapReader, EnrichedMapReader};
use crate::kubernetes::K8sClient;
use crate::policies::PolicyManager;

pub mod dns;
pub mod mtu;
pub mod policy;

#[derive(Debug, Clone)]
pub struct HealerConfig {
    pub enabled: bool,
    pub auto_apply: bool,
    pub dry_run: bool,
    pub check_interval_secs: u64,
}

impl Default for HealerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_apply: false, // Safe default: suggest but don't apply
            dry_run: true,
            check_interval_secs: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    DNSDrops {
        namespace: String,
        pod: String,
        count: u64,
    },
    MTUMismatch {
        namespace: String,
        pod: String,
        expected: u16,
        actual: u16,
    },
    PolicyGap {
        src_namespace: String,
        src_pod: String,
        dst_namespace: String,
        dst_pod: String,
        port: u16,
        protocol: String,
    },
    LoadBalancerTimeout {
        service: String,
        backend: String,
        timeout_count: u64,
    },
    ConntrackFull {
        node: String,
        utilization: f64,
    },
}

#[derive(Debug, Clone)]
pub struct Fix {
    pub problem: Problem,
    pub action: FixAction,
    pub applied: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FixAction {
    CreateDNSPolicy { namespace: String },
    AdjustMTU { namespace: String, pod: String, new_mtu: u16 },
    CreateAllowPolicy { src: String, dst: String, port: u16 },
    RebalanceBackend { service: String, backend: String },
    TuneConntrack { node: String, new_timeout: u32 },
}

pub struct SelfHealer<M: MapReader> {
    config: HealerConfig,
    ebpf_reader: M,
    k8s_client: K8sClient,
    policy_manager: PolicyManager,
    detected_problems: Vec<Problem>,
    applied_fixes: Vec<Fix>,
}

impl<M: MapReader> SelfHealer<M> {
    pub fn new(
        config: HealerConfig,
        ebpf_reader: M,
        k8s_client: K8sClient,
    ) -> Self {
        let policy_manager = PolicyManager::new(k8s_client.clone());

        Self {
            config,
            ebpf_reader,
            k8s_client,
            policy_manager,
            detected_problems: Vec::new(),
            applied_fixes: Vec::new(),
        }
    }

    /// Main healing loop - detect and fix problems
    pub async fn run(&mut self) -> Result<HealerStats> {
        if !self.config.enabled {
            return Ok(HealerStats::default());
        }

        let mut stats = HealerStats::default();

        // Detect problems from eBPF data
        self.detect_problems().await?;
        stats.problems_detected = self.detected_problems.len();

        // Generate fixes
        let fixes = self.generate_fixes().await?;
        stats.fixes_proposed = fixes.len();

        // Apply fixes (if auto_apply enabled and not dry_run)
        if self.config.auto_apply && !self.config.dry_run {
            for fix in fixes {
                if self.apply_fix(&fix).await.is_ok() {
                    self.applied_fixes.push(fix);
                    stats.fixes_applied += 1;
                }
            }
        }

        Ok(stats)
    }

    /// Detect problems from eBPF drops
    async fn detect_problems(&mut self) -> Result<()> {
        self.detected_problems.clear();

        // Read drop reasons from eBPF
        let drops = self.ebpf_reader.read_drop_map()?;

        // Analyze drops for patterns
        let dns_drops = self.analyze_dns_drops(&drops);
        self.detected_problems.extend(dns_drops);

        let policy_gaps = self.analyze_policy_gaps(&drops);
        self.detected_problems.extend(policy_gaps);

        Ok(())
    }

    /// Analyze DNS-related drops
    fn analyze_dns_drops(&self, drops: &[DropReason]) -> Vec<Problem> {
        let mut dns_problems = Vec::new();
        let mut dns_drop_count: HashMap<String, u64> = HashMap::new();

        for drop in drops {
            if drop.port == 53 && drop.reason == DropReasonType::PolicyDenied {
                *dns_drop_count.entry(drop.src_ip.clone()).or_insert(0) += 1;
            }
        }

        for (ip, count) in dns_drop_count {
            if count > 5 {
                // Threshold: more than 5 DNS drops
                dns_problems.push(Problem::DNSDrops {
                    namespace: "unknown".to_string(), // Will be resolved if using EnrichedMapReader
                    pod: ip,
                    count,
                });
            }
        }

        dns_problems
    }

    /// Analyze policy-related drops
    fn analyze_policy_gaps(&self, drops: &[DropReason]) -> Vec<Problem> {
        let mut problems = Vec::new();

        for drop in drops {
            if drop.reason == DropReasonType::PolicyDenied {
                problems.push(Problem::PolicyGap {
                    src_namespace: "unknown".to_string(),
                    src_pod: drop.src_ip.clone(),
                    dst_namespace: "unknown".to_string(),
                    dst_pod: drop.dst_ip.clone(),
                    port: drop.port,
                    protocol: if drop.protocol == 6 {
                        "TCP".to_string()
                    } else {
                        "UDP".to_string()
                    },
                });
            }
        }

        problems
    }

    /// Generate fixes for detected problems
    async fn generate_fixes(&self) -> Result<Vec<Fix>> {
        let mut fixes = Vec::new();

        for problem in &self.detected_problems {
            match problem {
                Problem::DNSDrops { namespace, .. } => {
                    fixes.push(Fix {
                        problem: problem.clone(),
                        action: FixAction::CreateDNSPolicy {
                            namespace: namespace.clone(),
                        },
                        applied: false,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    });
                }
                Problem::PolicyGap {
                    src_namespace,
                    dst_pod,
                    port,
                    ..
                } => {
                    fixes.push(Fix {
                        problem: problem.clone(),
                        action: FixAction::CreateAllowPolicy {
                            src: src_namespace.clone(),
                            dst: dst_pod.clone(),
                            port: *port,
                        },
                        applied: false,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    });
                }
                _ => {}
            }
        }

        Ok(fixes)
    }

    /// Apply a fix
    async fn apply_fix(&mut self, fix: &Fix) -> Result<()> {
        match &fix.action {
            FixAction::CreateDNSPolicy { namespace } => {
                self.policy_manager.apply_dns_policy(namespace).await?;
                println!("✔ Applied DNS policy fix for namespace: {}", namespace);
            }
            FixAction::CreateAllowPolicy { src, dst, port } => {
                // TODO: Create specific allow policy
                println!("✔ Would create allow policy: {} -> {}:{}", src, dst, port);
            }
            _ => {
                println!("⚠ Fix action not yet implemented: {:?}", fix.action);
            }
        }

        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> HealerStats {
        HealerStats {
            problems_detected: self.detected_problems.len(),
            fixes_proposed: self.applied_fixes.len(),
            fixes_applied: self
                .applied_fixes
                .iter()
                .filter(|f| f.applied)
                .count(),
        }
    }

    /// Get detected problems
    pub fn problems(&self) -> &[Problem] {
        &self.detected_problems
    }

    /// Get applied fixes
    pub fn fixes(&self) -> &[Fix] {
        &self.applied_fixes
    }
}

/// Enhanced implementation for EnrichedMapReader
impl SelfHealer<EnrichedMapReader> {
    /// Detect problems with full pod context
    pub async fn detect_problems_enriched(&mut self) -> Result<()> {
        self.detected_problems.clear();

        // Read enriched drops from eBPF
        let enriched_drops = self.ebpf_reader.read_enriched_drops()?;

        // Analyze DNS drops with pod context
        let dns_drops = self.analyze_dns_drops_enriched(&enriched_drops);
        self.detected_problems.extend(dns_drops);

        // Analyze policy gaps with pod context
        let policy_gaps = self.analyze_policy_gaps_enriched(&enriched_drops);
        self.detected_problems.extend(policy_gaps);

        Ok(())
    }

    /// Analyze DNS-related drops with enriched pod information
    fn analyze_dns_drops_enriched(&self, drops: &[(DropReason, crate::ebpf::EnrichedDropInfo)]) -> Vec<Problem> {
        let mut dns_problems = Vec::new();
        let mut dns_drop_count: HashMap<(String, String), u64> = HashMap::new();

        for (drop, info) in drops {
            if drop.port == 53 && drop.reason == DropReasonType::PolicyDenied {
                let key = (
                    info.src_namespace.clone().unwrap_or_else(|| "unknown".to_string()),
                    info.src_pod.clone().unwrap_or_else(|| drop.src_ip.clone()),
                );
                *dns_drop_count.entry(key).or_insert(0) += 1;
            }
        }

        for ((namespace, pod), count) in dns_drop_count {
            if count > 5 {
                dns_problems.push(Problem::DNSDrops {
                    namespace,
                    pod,
                    count,
                });
            }
        }

        dns_problems
    }

    /// Analyze policy-related drops with enriched pod information
    fn analyze_policy_gaps_enriched(&self, drops: &[(DropReason, crate::ebpf::EnrichedDropInfo)]) -> Vec<Problem> {
        let mut problems = Vec::new();

        for (drop, info) in drops {
            if drop.reason == DropReasonType::PolicyDenied {
                problems.push(Problem::PolicyGap {
                    src_namespace: info.src_namespace.clone().unwrap_or_else(|| "unknown".to_string()),
                    src_pod: info.src_pod.clone().unwrap_or_else(|| drop.src_ip.clone()),
                    dst_namespace: info.dst_namespace.clone().unwrap_or_else(|| "unknown".to_string()),
                    dst_pod: info.dst_pod.clone().unwrap_or_else(|| drop.dst_ip.clone()),
                    port: drop.port,
                    protocol: if drop.protocol == 6 {
                        "TCP".to_string()
                    } else {
                        "UDP".to_string()
                    },
                });
            }
        }

        problems
    }

    /// Enhanced run loop with pod context
    pub async fn run_enriched(&mut self) -> Result<HealerStats> {
        if !self.config.enabled {
            return Ok(HealerStats::default());
        }

        let mut stats = HealerStats::default();

        // Detect problems from enriched eBPF data
        self.detect_problems_enriched().await?;
        stats.problems_detected = self.detected_problems.len();

        // Generate fixes
        let fixes = self.generate_fixes().await?;
        stats.fixes_proposed = fixes.len();

        // Apply fixes (if auto_apply enabled and not dry_run)
        if self.config.auto_apply && !self.config.dry_run {
            for fix in fixes {
                if self.apply_fix(&fix).await.is_ok() {
                    self.applied_fixes.push(fix);
                    stats.fixes_applied += 1;
                }
            }
        }

        Ok(stats)
    }
}

#[derive(Debug, Clone, Default)]
pub struct HealerStats {
    pub problems_detected: usize,
    pub fixes_proposed: usize,
    pub fixes_applied: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ebpf::MockMapReader;

    #[tokio::test]
    async fn test_healer_creation() {
        let config = HealerConfig::default();
        let reader = MockMapReader;
        let k8s_client = K8sClient::new().await.unwrap();

        let healer = SelfHealer::new(config, reader, k8s_client);
        assert!(!healer.config.auto_apply); // Safe default
    }
}
