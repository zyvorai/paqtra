use std::collections::HashMap;
use std::time::Duration;

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
