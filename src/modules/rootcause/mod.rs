#![allow(dead_code)]
/// Root-Cause Engine Module
///
/// Analyzes packet drops and provides human-readable explanations
/// with actionable fixes.
///
/// Features:
/// - Drop reason analysis from eBPF
/// - Policy correlation
/// - Human-readable explanations
/// - Actionable fix suggestions
/// - Historical pattern analysis

use anyhow::Result;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ebpf::{DropReason as EbpfDropReason, DropReasonType, MapReader, PolicyDecision};
use crate::kubernetes::K8sClient;
use crate::policies::PolicyManager;

pub mod analyzer;
pub mod explainer;
pub mod correlator;

/// Root-cause engine configuration
#[derive(Debug, Clone)]
pub struct RootCauseConfig {
    /// Enable root-cause analysis
    pub enabled: bool,

    /// Analyze only recent drops (seconds)
    pub analysis_window_secs: u64,

    /// Minimum drop count to trigger analysis
    pub min_drop_count: u64,

    /// Enable automatic correlation with policies
    pub auto_correlate: bool,

    /// Enable historical pattern analysis
    pub pattern_analysis: bool,
}

impl Default for RootCauseConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            analysis_window_secs: 300, // 5 minutes
            min_drop_count: 5,
            auto_correlate: true,
            pattern_analysis: true,
        }
    }
}

/// Drop event from eBPF
#[derive(Debug, Clone)]
pub struct DropEvent {
    pub timestamp: u64,
    pub src_ip: IpAddr,
    pub dst_ip: IpAddr,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub reason: DropReason,
    pub identity_src: u32,
    pub identity_dst: u32,
    pub namespace: Option<String>,
    pub pod_name: Option<String>,
}

/// Drop reason (enhanced from eBPF)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DropReason {
    /// Policy denied the packet
    PolicyDenied,

    /// Invalid source IP
    InvalidSourceIP,

    /// Invalid packet (malformed)
    InvalidPacket,

    /// Connection tracking state mismatch
    CTStateMismatch,

    /// Port not allowed by policy
    PortNotAllowed,

    /// Unknown L3 protocol
    UnknownL3Protocol,

    /// Unknown L4 protocol
    UnknownL4Protocol,

    /// Unsupported L3 protocol
    UnsupportedL3Protocol,

    /// No mapping for destination
    NoMapping,

    /// Destination identity unknown
    UnknownDestination,

    /// Load balancer error
    LBError,

    /// Service not found
    ServiceNotFound,

    /// No healthy backend
    NoBackend,

    /// Fragment needed (MTU issue)
    FragNeeded,

    /// TTL exceeded
    TTLExceeded,

    /// Other reason
    Other(u8),
}

impl DropReason {
    pub fn from_ebpf(reason: &DropReasonType) -> Self {
        match reason {
            DropReasonType::PolicyDenied => DropReason::PolicyDenied,
            DropReasonType::InvalidPacket => DropReason::InvalidPacket,
            DropReasonType::NoRoute => DropReason::NoMapping,
            DropReasonType::UnknownL4Protocol => DropReason::UnknownL4Protocol,
            DropReasonType::FragmentationNeeded => DropReason::FragNeeded,
            DropReasonType::CTMapFull => DropReason::CTStateMismatch,
            DropReasonType::NATMapFull => DropReason::LBError,
            DropReasonType::InvalidSourceIP => DropReason::InvalidSourceIP,
            DropReasonType::InvalidDestIP => DropReason::UnknownDestination,
            DropReasonType::UnsupportedL3Protocol => DropReason::UnsupportedL3Protocol,
            DropReasonType::MissedTailCall => DropReason::Other(133),
            DropReasonType::ErrorWritingToPacket => DropReason::Other(134),
            DropReasonType::UnknownL4ICMPType => DropReason::Other(135),
            DropReasonType::UnknownICMPv6Type => DropReason::Other(136),
            DropReasonType::UnknownICMPv6Code => DropReason::Other(137),
            DropReasonType::ServiceBackendNotFound => DropReason::NoBackend,
            DropReasonType::NoTunnelEndpoint => DropReason::NoMapping,
            DropReasonType::HostUnreachable => DropReason::UnknownDestination,
            DropReasonType::StaleOrUnroutable => DropReason::NoMapping,
            DropReasonType::ConnectionTrackingInvalid => DropReason::CTStateMismatch,
            DropReasonType::AuthRequired => DropReason::PolicyDenied,
            DropReasonType::NATNotNeeded => DropReason::Other(184),
            DropReasonType::IsClusterIP => DropReason::Other(185),
            DropReasonType::Other(code) => DropReason::Other(*code as u8),
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            DropReason::PolicyDenied => "Policy Denied",
            DropReason::InvalidSourceIP => "Invalid Source IP",
            DropReason::InvalidPacket => "Invalid Packet",
            DropReason::CTStateMismatch => "Connection Tracking State Mismatch",
            DropReason::PortNotAllowed => "Port Not Allowed",
            DropReason::UnknownL3Protocol => "Unknown L3 Protocol",
            DropReason::UnknownL4Protocol => "Unknown L4 Protocol",
            DropReason::UnsupportedL3Protocol => "Unsupported L3 Protocol",
            DropReason::NoMapping => "No Mapping",
            DropReason::UnknownDestination => "Unknown Destination",
            DropReason::LBError => "Load Balancer Error",
            DropReason::ServiceNotFound => "Service Not Found",
            DropReason::NoBackend => "No Healthy Backend",
            DropReason::FragNeeded => "Fragment Needed (MTU)",
            DropReason::TTLExceeded => "TTL Exceeded",
            DropReason::Other(_) => "Other",
        }
    }
}

/// Root-cause analysis result
#[derive(Debug, Clone)]
pub struct RootCauseAnalysis {
    /// The drop event being analyzed
    pub event: DropEvent,

    /// Human-readable explanation
    pub explanation: String,

    /// Likely root cause
    pub likely_cause: String,

    /// Suggested fix
    pub suggested_fix: SuggestedFix,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Related policy (if applicable)
    pub related_policy: Option<String>,

    /// Additional context
    pub context: Vec<String>,
}

/// Suggested fix for the drop
#[derive(Debug, Clone)]
pub enum SuggestedFix {
    /// Add a policy rule
    AddPolicyRule {
        namespace: String,
        from_labels: HashMap<String, String>,
        to_labels: HashMap<String, String>,
        port: u16,
        protocol: String,
        yaml: String,
    },

    /// Update MTU settings
    UpdateMTU {
        interface: String,
        current_mtu: u16,
        suggested_mtu: u16,
        command: String,
    },

    /// Fix DNS configuration
    FixDNS {
        namespace: String,
        issue: String,
        command: String,
    },

    /// Check connection tracking
    CheckConntrack {
        issue: String,
        commands: Vec<String>,
    },

    /// Add service endpoint
    AddServiceEndpoint {
        service: String,
        namespace: String,
        reason: String,
    },

    /// Fix load balancer
    FixLoadBalancer {
        service: String,
        namespace: String,
        issue: String,
    },

    /// Manual investigation needed
    ManualInvestigation {
        reason: String,
        steps: Vec<String>,
    },
}

/// Drop pattern for historical analysis
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct DropPattern {
    pub reason: DropReason,
    pub src_identity: u32,
    pub dst_identity: u32,
    pub dst_port: u16,
    pub protocol: u8,
}

/// Drop statistics
#[derive(Debug, Clone)]
pub struct DropStats {
    pub total_drops: u64,
    pub by_reason: HashMap<DropReason, u64>,
    pub by_namespace: HashMap<String, u64>,
    pub top_patterns: Vec<(DropPattern, u64)>,
}

/// Root-Cause Engine
pub struct RootCauseEngine<M: MapReader> {
    config: RootCauseConfig,
    ebpf_reader: M,
    k8s_client: K8sClient,
    policy_manager: PolicyManager,

    /// Historical drop events
    drop_history: Vec<DropEvent>,

    /// Drop pattern counts
    pattern_counts: HashMap<DropPattern, u64>,
}

impl<M: MapReader> RootCauseEngine<M> {
    pub fn new(
        config: RootCauseConfig,
        ebpf_reader: M,
        k8s_client: K8sClient,
    ) -> Self {
        let policy_manager = PolicyManager::new(k8s_client.clone());

        Self {
            config,
            ebpf_reader,
            k8s_client,
            policy_manager,
            drop_history: Vec::new(),
            pattern_counts: HashMap::new(),
        }
    }

    /// Analyze current drops
    pub async fn analyze_drops(&mut self) -> Result<Vec<RootCauseAnalysis>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        // Read drops from eBPF
        let ebpf_drops = self.ebpf_reader.read_drop_map()?;

        // Convert to drop events
        let mut new_events = Vec::new();
        for ebpf_drop in &ebpf_drops {
            let event = self.ebpf_drop_to_event(ebpf_drop).await?;
            new_events.push(event);
        }

        // Update history
        self.update_history(new_events.clone());

        // Analyze each significant drop pattern
        let mut analyses = Vec::new();
        let significant_drops = self.find_significant_drops();

        for drop in significant_drops {
            let analysis = self.analyze_single_drop(&drop).await?;
            analyses.push(analysis);
        }

        Ok(analyses)
    }

    /// Resolve an IP address to identity and namespace via IPCache
    fn resolve_ip_info(&self, ip: &str) -> (u32, Option<String>) {
        if let Ok(ipcache) = self.ebpf_reader.read_ipcache_map() {
            if let Some(entry) = ipcache.iter().find(|e| e.ip == ip) {
                let namespace = if entry.namespace.is_empty() {
                    None
                } else {
                    Some(entry.namespace.clone())
                };
                return (entry.identity, namespace);
            }
        }
        (0, None)
    }

    /// Convert eBPF drop to drop event
    async fn ebpf_drop_to_event(&self, ebpf_drop: &EbpfDropReason) -> Result<DropEvent> {
        // Parse IP addresses
        let src_ip: IpAddr = ebpf_drop.src_ip.parse()
            .unwrap_or_else(|_| IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));
        let dst_ip: IpAddr = ebpf_drop.dst_ip.parse()
            .unwrap_or_else(|_| IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));

        // Resolve IPs to pod info via IPCache
        let (identity_src, src_ns) = self.resolve_ip_info(&ebpf_drop.src_ip);
        let (identity_dst, dst_ns) = self.resolve_ip_info(&ebpf_drop.dst_ip);

        let event = DropEvent {
            timestamp: ebpf_drop.timestamp,
            src_ip,
            dst_ip,
            src_port: 0,
            dst_port: ebpf_drop.port,
            protocol: ebpf_drop.protocol,
            reason: DropReason::from_ebpf(&ebpf_drop.reason),
            identity_src,
            identity_dst,
            namespace: src_ns.or(dst_ns),
            pod_name: None,
        };

        Ok(event)
    }

    /// Update drop history
    fn update_history(&mut self, new_events: Vec<DropEvent>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Add new events
        self.drop_history.extend(new_events);

        // Update pattern counts
        for event in &self.drop_history {
            let pattern = DropPattern {
                reason: event.reason.clone(),
                src_identity: event.identity_src,
                dst_identity: event.identity_dst,
                dst_port: event.dst_port,
                protocol: event.protocol,
            };
            *self.pattern_counts.entry(pattern).or_insert(0) += 1;
        }

        // Clean old events
        let cutoff = now - self.config.analysis_window_secs;
        self.drop_history.retain(|e| e.timestamp >= cutoff);
    }

    /// Find significant drops (above threshold)
    fn find_significant_drops(&self) -> Vec<DropEvent> {
        let mut significant = Vec::new();

        for (pattern, count) in &self.pattern_counts {
            if *count >= self.config.min_drop_count {
                // Find a representative event for this pattern
                if let Some(event) = self.drop_history.iter().find(|e| {
                    e.reason == pattern.reason
                        && e.identity_src == pattern.src_identity
                        && e.identity_dst == pattern.dst_identity
                        && e.dst_port == pattern.dst_port
                        && e.protocol == pattern.protocol
                }) {
                    significant.push(event.clone());
                }
            }
        }

        significant
    }

    /// Analyze a single drop event
    async fn analyze_single_drop(&self, event: &DropEvent) -> Result<RootCauseAnalysis> {
        use explainer::DropExplainer;
        use correlator::PolicyCorrelator;

        // Get human-readable explanation
        let explanation = DropExplainer::explain(event);

        // Correlate with policies if enabled
        let (related_policy, policy_context) = if self.config.auto_correlate {
            PolicyCorrelator::correlate_with_policy(
                event,
                &self.ebpf_reader,
            ).await?
        } else {
            (None, Vec::new())
        };

        // Determine likely cause
        let likely_cause = self.determine_likely_cause(event, &policy_context);

        // Generate suggested fix
        let suggested_fix = self.generate_fix(event, &related_policy).await?;

        // Calculate confidence
        let confidence = self.calculate_confidence(event);

        Ok(RootCauseAnalysis {
            event: event.clone(),
            explanation,
            likely_cause,
            suggested_fix,
            confidence,
            related_policy,
            context: policy_context,
        })
    }

    /// Determine likely root cause
    fn determine_likely_cause(&self, event: &DropEvent, context: &[String]) -> String {
        match &event.reason {
            DropReason::PolicyDenied => {
                if context.is_empty() {
                    "No CiliumNetworkPolicy allows this traffic".to_string()
                } else {
                    format!("Policy exists but doesn't match: {}", context.join(", "))
                }
            }
            DropReason::PortNotAllowed => {
                format!("Port {} is not allowed by any policy", event.dst_port)
            }
            DropReason::CTStateMismatch => {
                "Connection tracking table is out of sync or full".to_string()
            }
            DropReason::NoBackend => {
                "Service has no healthy backend pods".to_string()
            }
            DropReason::ServiceNotFound => {
                "Service does not exist or is not registered".to_string()
            }
            DropReason::FragNeeded => {
                "MTU mismatch - packet too large for network path".to_string()
            }
            DropReason::UnknownDestination => {
                "Destination identity not found in identity map".to_string()
            }
            _ => {
                format!("Drop reason: {}", event.reason.to_string())
            }
        }
    }

    /// Generate suggested fix
    async fn generate_fix(
        &self,
        event: &DropEvent,
        _related_policy: &Option<String>,
    ) -> Result<SuggestedFix> {
        match &event.reason {
            DropReason::PolicyDenied | DropReason::PortNotAllowed => {
                // Suggest adding a policy rule
                let protocol = match event.protocol {
                    6 => "TCP",
                    17 => "UDP",
                    _ => "UNKNOWN",
                };

                let namespace = event.namespace.clone().unwrap_or_else(|| "default".to_string());

                // TODO: Resolve labels from identities
                let from_labels = HashMap::from([
                    ("security.identity".to_string(), event.identity_src.to_string()),
                ]);
                let to_labels = HashMap::from([
                    ("security.identity".to_string(), event.identity_dst.to_string()),
                ]);

                let yaml = format!(
                    r#"apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: allow-traffic-fix
  namespace: {}
spec:
  endpointSelector:
    matchLabels:
      security.identity: "{}"
  egress:
  - toEndpoints:
    - matchLabels:
        security.identity: "{}"
    toPorts:
    - ports:
      - port: "{}"
        protocol: {}
"#,
                    namespace,
                    event.identity_src,
                    event.identity_dst,
                    event.dst_port,
                    protocol
                );

                Ok(SuggestedFix::AddPolicyRule {
                    namespace,
                    from_labels,
                    to_labels,
                    port: event.dst_port,
                    protocol: protocol.to_string(),
                    yaml,
                })
            }

            DropReason::FragNeeded => {
                Ok(SuggestedFix::UpdateMTU {
                    interface: "cilium_host".to_string(),
                    current_mtu: 1500,
                    suggested_mtu: 1450,
                    command: "ip link set cilium_host mtu 1450".to_string(),
                })
            }

            DropReason::CTStateMismatch => {
                Ok(SuggestedFix::CheckConntrack {
                    issue: "Connection tracking state mismatch".to_string(),
                    commands: vec![
                        "cilium bpf ct list global".to_string(),
                        "cilium bpf ct flush".to_string(),
                    ],
                })
            }

            DropReason::NoBackend => {
                let service = event.namespace.clone().unwrap_or_else(|| "unknown".to_string());
                Ok(SuggestedFix::AddServiceEndpoint {
                    service: format!("service-on-port-{}", event.dst_port),
                    namespace: service,
                    reason: "Service has no healthy backends".to_string(),
                })
            }

            DropReason::ServiceNotFound => {
                let service = event.namespace.clone().unwrap_or_else(|| "unknown".to_string());
                Ok(SuggestedFix::FixLoadBalancer {
                    service: format!("service-on-port-{}", event.dst_port),
                    namespace: service,
                    issue: "Service not found in load balancer map".to_string(),
                })
            }

            _ => {
                Ok(SuggestedFix::ManualInvestigation {
                    reason: format!("Uncommon drop reason: {}", event.reason.to_string()),
                    steps: vec![
                        "Check cilium monitor output".to_string(),
                        "Review Hubble flows".to_string(),
                        "Inspect eBPF maps directly".to_string(),
                    ],
                })
            }
        }
    }

    /// Calculate confidence in the analysis
    fn calculate_confidence(&self, event: &DropEvent) -> f32 {
        let pattern = DropPattern {
            reason: event.reason.clone(),
            src_identity: event.identity_src,
            dst_identity: event.identity_dst,
            dst_port: event.dst_port,
            protocol: event.protocol,
        };

        let count = self.pattern_counts.get(&pattern).copied().unwrap_or(0);

        // More occurrences = higher confidence
        let frequency_score = (count as f32 / 100.0).min(1.0);

        // Well-known reasons = higher confidence
        let reason_score = match event.reason {
            DropReason::PolicyDenied | DropReason::PortNotAllowed => 1.0,
            DropReason::NoBackend | DropReason::ServiceNotFound => 0.9,
            DropReason::FragNeeded | DropReason::CTStateMismatch => 0.8,
            _ => 0.5,
        };

        // Combined confidence
        (frequency_score * 0.4 + reason_score * 0.6).min(1.0)
    }

    /// Get drop statistics
    pub fn get_stats(&self) -> DropStats {
        let total_drops = self.drop_history.len() as u64;

        let mut by_reason: HashMap<DropReason, u64> = HashMap::new();
        for event in &self.drop_history {
            *by_reason.entry(event.reason.clone()).or_insert(0) += 1;
        }

        let mut by_namespace: HashMap<String, u64> = HashMap::new();
        for event in &self.drop_history {
            if let Some(ns) = &event.namespace {
                *by_namespace.entry(ns.clone()).or_insert(0) += 1;
            }
        }

        let mut top_patterns: Vec<_> = self.pattern_counts.iter()
            .map(|(p, c)| (p.clone(), *c))
            .collect();
        top_patterns.sort_by(|a, b| b.1.cmp(&a.1));
        top_patterns.truncate(10);

        DropStats {
            total_drops,
            by_reason,
            by_namespace,
            top_patterns,
        }
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.drop_history.clear();
        self.pattern_counts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ebpf::MockMapReader;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_rootcause_creation() {
        let config = RootCauseConfig::default();
        let reader = MockMapReader;
        let k8s_client = K8sClient::new().await.unwrap();

        let engine = RootCauseEngine::new(config, reader, k8s_client);
        assert_eq!(engine.drop_history.len(), 0);
    }

    #[test]
    fn test_drop_reason_conversion() {
        let ebpf_reason = DropReasonType::PolicyDenied;
        let reason = DropReason::from_ebpf(&ebpf_reason);
        assert_eq!(reason, DropReason::PolicyDenied);
    }

    #[test]
    fn test_drop_reason_to_string() {
        assert_eq!(DropReason::PolicyDenied.to_string(), "Policy Denied");
        assert_eq!(DropReason::FragNeeded.to_string(), "Fragment Needed (MTU)");
    }
}
