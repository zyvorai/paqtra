#![allow(dead_code)]
/// AutoPolicy Module - Zero-Trust Policy Learning
///
/// Automatically learns traffic patterns from eBPF and generates
/// minimal privilege network policies.
///
/// Features:
/// - Traffic pattern learning
/// - Communication graph building
/// - Minimal privilege policy generation
/// - Audit mode for safe testing
/// - Policy refinement over time

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::ebpf::{ConntrackEntry, MapReader, EnrichedMapReader};
use crate::kubernetes::K8sClient;
use crate::policies::PolicyManager;

pub mod learner;
pub mod generator;
pub mod analyzer;
pub mod confidence;

/// LabelSet - A hashable, ordered set of labels
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LabelSet {
    labels: Vec<(String, String)>,
}

impl LabelSet {
    pub fn new(labels: HashMap<String, String>) -> Self {
        let mut sorted: Vec<_> = labels.into_iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        Self { labels: sorted }
    }

    pub fn to_hashmap(&self) -> HashMap<String, String> {
        self.labels.iter().cloned().collect()
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.labels.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, String)> {
        self.labels.iter()
    }
}

impl From<HashMap<String, String>> for LabelSet {
    fn from(map: HashMap<String, String>) -> Self {
        Self::new(map)
    }
}

/// AutoPolicy configuration
#[derive(Debug, Clone)]
pub struct AutoPolicyConfig {
    /// Enable learning
    pub enabled: bool,

    /// Learning duration (e.g., 7 days)
    pub learning_duration: Duration,

    /// Minimum observations before creating policy
    pub min_observations: u64,

    /// Generate policies automatically after learning
    pub auto_generate: bool,

    /// Apply policies automatically (dangerous!)
    pub auto_apply: bool,

    /// Enable audit mode (log but don't enforce)
    pub audit_mode: bool,

    /// Namespaces to learn from (empty = all)
    pub target_namespaces: Vec<String>,

    /// Update interval for learning
    pub update_interval_secs: u64,
}

impl Default for AutoPolicyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            learning_duration: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            min_observations: 10,
            auto_generate: false,
            auto_apply: false,
            audit_mode: true, // Safe default
            target_namespaces: vec![],
            update_interval_secs: 300, // 5 minutes
        }
    }
}

/// Learning state
#[derive(Debug, Clone, PartialEq)]
pub enum LearningState {
    NotStarted,
    Learning { started_at: u64, progress: f32 },
    Completed { learned_at: u64 },
    Generating,
    Generated { count: usize },
    Applied { count: usize },
}

/// Learned traffic pattern
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TrafficPattern {
    pub src_namespace: String,
    pub src_labels: LabelSet,
    pub dst_namespace: String,
    pub dst_labels: LabelSet,
    pub port: u16,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Protocol {
    TCP,
    UDP,
    ICMP,
    Other(u8),
}

impl Protocol {
    pub fn from_number(proto: u8) -> Self {
        match proto {
            6 => Protocol::TCP,
            17 => Protocol::UDP,
            1 => Protocol::ICMP,
            _ => Protocol::Other(proto),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Protocol::TCP => "TCP".to_string(),
            Protocol::UDP => "UDP".to_string(),
            Protocol::ICMP => "ICMP".to_string(),
            Protocol::Other(n) => format!("{}", n),
        }
    }
}

/// Traffic observation
#[derive(Debug, Clone)]
pub struct TrafficObservation {
    pub pattern: TrafficPattern,
    pub count: u64,
    pub first_seen: u64,
    pub last_seen: u64,
    pub bytes_transferred: u64,
}

/// Generated policy
#[derive(Debug, Clone)]
pub struct GeneratedPolicy {
    pub name: String,
    pub namespace: String,
    pub yaml: String,
    pub patterns: Vec<TrafficPattern>,
    pub confidence: f32, // 0.0 to 1.0
}

/// AutoPolicy Engine
pub struct AutoPolicy<M: MapReader> {
    config: AutoPolicyConfig,
    ebpf_reader: M,
    k8s_client: K8sClient,
    policy_manager: PolicyManager,

    state: LearningState,
    observations: HashMap<TrafficPattern, TrafficObservation>,
    generated_policies: Vec<GeneratedPolicy>,

    learning_start: Option<u64>,
}

impl<M: MapReader> AutoPolicy<M> {
    pub fn new(
        config: AutoPolicyConfig,
        ebpf_reader: M,
        k8s_client: K8sClient,
    ) -> Self {
        let policy_manager = PolicyManager::new(k8s_client.clone());

        Self {
            config,
            ebpf_reader,
            k8s_client,
            policy_manager,
            state: LearningState::NotStarted,
            observations: HashMap::new(),
            generated_policies: Vec::new(),
            learning_start: None,
        }
    }

    /// Start learning phase
    pub async fn start_learning(&mut self) -> Result<()> {
        if !self.config.enabled {
            anyhow::bail!("AutoPolicy is not enabled");
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        self.learning_start = Some(now);
        self.state = LearningState::Learning {
            started_at: now,
            progress: 0.0,
        };

        println!("🔍 AutoPolicy learning started");
        println!("   Duration: {} days", self.config.learning_duration.as_secs() / 86400);
        println!("   Mode: {}", if self.config.audit_mode { "Audit" } else { "Enforce" });

        Ok(())
    }

    /// Update learning with current traffic
    pub async fn update(&mut self) -> Result<LearningStats> {
        if !self.config.enabled {
            return Ok(LearningStats::default());
        }

        // Read current connections from eBPF
        let connections = self.ebpf_reader.read_conntrack_map()?;

        let mut stats = LearningStats::default();
        stats.connections_observed = connections.len();

        // Process each connection
        for conn in &connections {
            if let Some(pattern) = self.connection_to_pattern(conn).await? {
                self.record_observation(pattern);
                stats.patterns_learned += 1;
            }
        }

        // Update learning progress
        self.update_progress()?;

        // Check if learning is complete
        if self.is_learning_complete()? {
            println!("✅ Learning phase completed!");
            self.complete_learning()?;
        }

        stats.unique_patterns = self.observations.len();
        Ok(stats)
    }

    /// Resolve an IP address to namespace and labels via IPCache
    fn resolve_ip(&self, ip: &str) -> (String, HashMap<String, String>) {
        if let Ok(ipcache) = self.ebpf_reader.read_ipcache_map() {
            if let Some(entry) = ipcache.iter().find(|e| e.ip == ip) {
                let namespace = if entry.namespace.is_empty() {
                    "default".to_string()
                } else {
                    entry.namespace.clone()
                };
                let labels: HashMap<String, String> = entry.labels.iter()
                    .filter_map(|l| {
                        let parts: Vec<&str> = l.splitn(2, '=').collect();
                        if parts.len() == 2 {
                            Some((parts[0].to_string(), parts[1].to_string()))
                        } else {
                            None
                        }
                    })
                    .collect();
                return (namespace, labels);
            }
        }
        ("unknown".to_string(), HashMap::new())
    }

    /// Convert connection to traffic pattern
    async fn connection_to_pattern(&self, conn: &ConntrackEntry) -> Result<Option<TrafficPattern>> {
        let (src_namespace, src_labels) = self.resolve_ip(&conn.src_ip);
        let (dst_namespace, dst_labels) = self.resolve_ip(&conn.dst_ip);

        let pattern = TrafficPattern {
            src_namespace,
            src_labels: LabelSet::new(src_labels),
            dst_namespace,
            dst_labels: LabelSet::new(dst_labels),
            port: conn.dst_port,
            protocol: Protocol::from_number(conn.protocol),
        };

        Ok(Some(pattern))
    }

    /// Record traffic observation
    fn record_observation(&mut self, pattern: TrafficPattern) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.observations
            .entry(pattern.clone())
            .and_modify(|obs| {
                obs.count += 1;
                obs.last_seen = now;
            })
            .or_insert(TrafficObservation {
                pattern,
                count: 1,
                first_seen: now,
                last_seen: now,
                bytes_transferred: 0,
            });
    }

    /// Update learning progress
    fn update_progress(&mut self) -> Result<()> {
        if let Some(start) = self.learning_start {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs();

            let elapsed = now - start;
            let total = self.config.learning_duration.as_secs();
            let progress = (elapsed as f32 / total as f32).min(1.0);

            self.state = LearningState::Learning {
                started_at: start,
                progress,
            };
        }

        Ok(())
    }

    /// Check if learning phase is complete
    fn is_learning_complete(&self) -> Result<bool> {
        if let Some(start) = self.learning_start {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs();

            let elapsed = now - start;
            Ok(elapsed >= self.config.learning_duration.as_secs())
        } else {
            Ok(false)
        }
    }

    /// Complete learning phase
    fn complete_learning(&mut self) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        self.state = LearningState::Completed { learned_at: now };

        // Auto-generate if configured
        if self.config.auto_generate {
            self.generate_policies()?;
        }

        Ok(())
    }

    /// Generate policies from learned patterns
    pub fn generate_policies(&mut self) -> Result<Vec<GeneratedPolicy>> {
        self.state = LearningState::Generating;

        let mut policies = Vec::new();

        // Group patterns by source namespace/labels
        let mut by_source: HashMap<(String, LabelSet), Vec<&TrafficObservation>> = HashMap::new();

        for obs in self.observations.values() {
            // Only include patterns with sufficient observations
            if obs.count >= self.config.min_observations {
                let key = (obs.pattern.src_namespace.clone(), obs.pattern.src_labels.clone());
                by_source.entry(key).or_insert_with(Vec::new).push(obs);
            }
        }

        // Generate one policy per source
        for ((namespace, labels), observations) in by_source {
            let policy = self.generate_policy_for_source(&namespace, &labels, observations)?;
            policies.push(policy);
        }

        self.generated_policies = policies.clone();
        self.state = LearningState::Generated {
            count: policies.len(),
        };

        println!("✅ Generated {} policies", policies.len());

        Ok(policies)
    }

    /// Generate policy for a specific source
    fn generate_policy_for_source(
        &self,
        namespace: &str,
        labels: &LabelSet,
        observations: Vec<&TrafficObservation>,
    ) -> Result<GeneratedPolicy> {
        // Build egress rules
        let mut egress_rules = Vec::new();

        // Group by destination
        let mut by_dest: HashMap<(String, LabelSet), Vec<(u16, Protocol)>> = HashMap::new();

        for obs in &observations {
            let key = (
                obs.pattern.dst_namespace.clone(),
                obs.pattern.dst_labels.clone(),
            );
            by_dest
                .entry(key)
                .or_insert_with(Vec::new)
                .push((obs.pattern.port, obs.pattern.protocol));
        }

        // Create egress rules
        for ((dst_ns, dst_labels), ports) in by_dest {
            let rule = self.format_egress_rule(&dst_ns, &dst_labels, &ports);
            egress_rules.push(rule);
        }

        // Generate policy name
        let policy_name = if let Some(app) = labels.get("app") {
            format!("auto-policy-{}", app)
        } else {
            format!("auto-policy-{}", namespace)
        };

        // Build complete YAML
        let yaml = format!(
            r#"apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: {}
  namespace: {}
  labels:
    generated-by: cilium-vision
    autopolicy: "true"
spec:
  endpointSelector:
    matchLabels:
{}
  egress:
{}
"#,
            policy_name,
            namespace,
            self.format_labels(labels, 6),
            egress_rules.join("\n"),
        );

        let confidence = self.calculate_confidence(&observations);

        Ok(GeneratedPolicy {
            name: policy_name,
            namespace: namespace.to_string(),
            yaml,
            patterns: observations.iter().map(|o| o.pattern.clone()).collect(),
            confidence,
        })
    }

    /// Format egress rule
    fn format_egress_rule(
        &self,
        dst_namespace: &str,
        dst_labels: &LabelSet,
        ports: &[(u16, Protocol)],
    ) -> String {
        let mut rule = String::new();

        // Add destination selector
        rule.push_str("    - toEndpoints:\n");
        rule.push_str("        - matchLabels:\n");

        if !dst_labels.is_empty() {
            for (k, v) in dst_labels.iter() {
                rule.push_str(&format!("            {}: \"{}\"\n", k, v));
            }
        } else {
            rule.push_str(&format!("            k8s:io.kubernetes.pod.namespace: \"{}\"\n", dst_namespace));
        }

        // Add ports
        if !ports.is_empty() {
            rule.push_str("      toPorts:\n");

            // Group by protocol
            let mut tcp_ports = Vec::new();
            let mut udp_ports = Vec::new();

            for (port, proto) in ports {
                match proto {
                    Protocol::TCP => tcp_ports.push(*port),
                    Protocol::UDP => udp_ports.push(*port),
                    _ => {}
                }
            }

            if !tcp_ports.is_empty() {
                rule.push_str("        - ports:\n");
                for port in tcp_ports {
                    rule.push_str(&format!("            - port: \"{}\"\n", port));
                    rule.push_str("              protocol: TCP\n");
                }
            }

            if !udp_ports.is_empty() {
                rule.push_str("        - ports:\n");
                for port in udp_ports {
                    rule.push_str(&format!("            - port: \"{}\"\n", port));
                    rule.push_str("              protocol: UDP\n");
                }
            }
        }

        rule
    }

    /// Format labels for YAML
    fn format_labels(&self, labels: &LabelSet, indent: usize) -> String {
        let spaces = " ".repeat(indent);
        labels
            .iter()
            .map(|(k, v)| format!("{}{}: \"{}\"", spaces, k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Calculate confidence score
    fn calculate_confidence(&self, observations: &[&TrafficObservation]) -> f32 {
        let total_obs: u64 = observations.iter().map(|o| o.count).sum();

        // More observations = higher confidence
        let obs_score = (total_obs as f32 / (self.config.min_observations as f32 * 10.0)).min(1.0);

        // More patterns = lower confidence (more complex)
        let pattern_score = 1.0 / (observations.len() as f32).sqrt();

        // Combined score
        (obs_score * 0.7 + pattern_score * 0.3).min(1.0)
    }

    /// Apply generated policies
    pub async fn apply_policies(&mut self) -> Result<usize> {
        if !self.config.auto_apply {
            return Ok(0);
        }

        let mut applied = 0;

        for policy in &self.generated_policies {
            // Apply via kubectl
            if self.apply_policy_yaml(&policy.yaml).await.is_ok() {
                applied += 1;
                println!("✅ Applied policy: {} (confidence: {:.0}%)",
                    policy.name, policy.confidence * 100.0);
            }
        }

        self.state = LearningState::Applied { count: applied };

        Ok(applied)
    }

    /// Apply policy YAML
    async fn apply_policy_yaml(&self, yaml: &str) -> Result<()> {
        self.k8s_client.apply_custom_resource(None, yaml).await
    }

    /// Get current state
    pub fn state(&self) -> &LearningState {
        &self.state
    }

    /// Get observations
    pub fn observations(&self) -> &HashMap<TrafficPattern, TrafficObservation> {
        &self.observations
    }

    /// Get generated policies
    pub fn policies(&self) -> &[GeneratedPolicy] {
        &self.generated_policies
    }

    /// Get statistics
    pub fn stats(&self) -> AutoPolicyStats {
        AutoPolicyStats {
            state: self.state.clone(),
            total_observations: self.observations.values().map(|o| o.count).sum(),
            unique_patterns: self.observations.len(),
            policies_generated: self.generated_policies.len(),
            avg_confidence: if !self.generated_policies.is_empty() {
                self.generated_policies.iter().map(|p| p.confidence).sum::<f32>()
                    / self.generated_policies.len() as f32
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LearningStats {
    pub connections_observed: usize,
    pub patterns_learned: usize,
    pub unique_patterns: usize,
}

#[derive(Debug, Clone)]
pub struct AutoPolicyStats {
    pub state: LearningState,
    pub total_observations: u64,
    pub unique_patterns: usize,
    pub policies_generated: usize,
    pub avg_confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ebpf::MockMapReader;

    #[tokio::test]
    async fn test_autopolicy_creation() {
        let config = AutoPolicyConfig::default();
        let reader = MockMapReader;
        let k8s_client = K8sClient::new().await.unwrap();

        let autopolicy = AutoPolicy::new(config, reader, k8s_client);
        assert_eq!(autopolicy.state, LearningState::NotStarted);
    }

    #[tokio::test]
    async fn test_start_learning() {
        let config = AutoPolicyConfig::default();
        let reader = MockMapReader;
        let k8s_client = K8sClient::new().await.unwrap();

        let mut autopolicy = AutoPolicy::new(config, reader, k8s_client);
        autopolicy.start_learning().await.unwrap();

        assert!(matches!(autopolicy.state, LearningState::Learning { .. }));
    }

    #[test]
    fn test_protocol_conversion() {
        assert_eq!(Protocol::from_number(6), Protocol::TCP);
        assert_eq!(Protocol::from_number(17), Protocol::UDP);
        assert_eq!(Protocol::from_number(1), Protocol::ICMP);
    }
}

/// Enhanced implementation for EnrichedMapReader
impl AutoPolicy<EnrichedMapReader> {
    /// Update learning with enriched traffic (includes pod labels)
    pub async fn update_enriched(&mut self) -> Result<LearningStats> {
        if !self.config.enabled {
            return Ok(LearningStats::default());
        }

        // Read enriched connections from eBPF
        let enriched_connections = self.ebpf_reader.read_enriched_connections()?;

        let mut stats = LearningStats::default();
        stats.connections_observed = enriched_connections.len();

        // Process each enriched connection
        for (conn, info) in &enriched_connections {
            if let Some(pattern) = self.enriched_connection_to_pattern(conn, info)? {
                self.record_observation(pattern);
                stats.patterns_learned += 1;
            }
        }

        // Update learning progress
        self.update_progress()?;

        // Check if learning is complete
        if self.is_learning_complete()? {
            println!("✅ Learning phase completed!");
            self.complete_learning()?;
        }

        stats.unique_patterns = self.observations.len();
        Ok(stats)
    }

    /// Convert enriched connection to traffic pattern with real labels
    fn enriched_connection_to_pattern(
        &self,
        conn: &ConntrackEntry,
        info: &crate::ebpf::EnrichedConnectionInfo,
    ) -> Result<Option<TrafficPattern>> {
        // Extract labels into HashMap
        let src_labels_map = self.labels_vec_to_map(&info.src_labels);
        let dst_labels_map = self.labels_vec_to_map(&info.dst_labels);

        let pattern = TrafficPattern {
            src_namespace: info.src_namespace.clone().unwrap_or_else(|| "unknown".to_string()),
            src_labels: LabelSet::new(src_labels_map),
            dst_namespace: info.dst_namespace.clone().unwrap_or_else(|| "unknown".to_string()),
            dst_labels: LabelSet::new(dst_labels_map),
            port: conn.dst_port,
            protocol: Protocol::from_number(conn.protocol),
        };

        Ok(Some(pattern))
    }

    /// Convert label vector to HashMap
    fn labels_vec_to_map(&self, labels: &[String]) -> HashMap<String, String> {
        labels
            .iter()
            .filter_map(|label| {
                let parts: Vec<&str> = label.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect()
    }
}

