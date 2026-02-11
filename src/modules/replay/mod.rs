#![allow(dead_code)]
/// Traffic Replay Module
///
/// Records real network traffic and replays it in different contexts
/// to validate policy changes, test migrations, and compare cluster behaviors.
///
/// Features:
/// - Flow recording from eBPF/Hubble
/// - Storage and persistence
/// - Cross-cluster replay
/// - Outcome comparison
/// - Policy validation

use anyhow::Result;
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ebpf::{ConntrackEntry, MapReader, PolicyVerdict};
use crate::kubernetes::K8sClient;

pub mod recorder;
pub mod storage;
pub mod player;
pub mod comparator;

/// Replay configuration
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    /// Enable replay system
    pub enabled: bool,

    /// Recording directory
    pub recording_dir: PathBuf,

    /// Maximum recording size (bytes)
    pub max_recording_size: usize,

    /// Maximum recording duration (seconds)
    pub max_recording_duration: u64,

    /// Enable compression
    pub compress: bool,

    /// Replay rate multiplier (1.0 = real-time)
    pub replay_rate: f32,

    /// Enable detailed comparison
    pub detailed_comparison: bool,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            recording_dir: PathBuf::from("/tmp/cilium-vision/recordings"),
            max_recording_size: 100 * 1024 * 1024, // 100 MB
            max_recording_duration: 300,            // 5 minutes
            compress: true,
            replay_rate: 1.0,
            detailed_comparison: true,
        }
    }
}

/// Recorded flow
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecordedFlow {
    /// Timestamp when recorded
    pub timestamp: u64,

    /// Time offset from recording start (for replay timing)
    pub offset_ms: u64,

    /// Source IP
    pub src_ip: IpAddr,

    /// Destination IP
    pub dst_ip: IpAddr,

    /// Source port
    pub src_port: u16,

    /// Destination port
    pub dst_port: u16,

    /// Protocol (6=TCP, 17=UDP, etc.)
    pub protocol: u8,

    /// Source identity
    pub src_identity: u32,

    /// Destination identity
    pub dst_identity: u32,

    /// Source namespace
    pub src_namespace: String,

    /// Destination namespace
    pub dst_namespace: String,

    /// Source pod labels
    pub src_labels: HashMap<String, String>,

    /// Destination pod labels
    pub dst_labels: HashMap<String, String>,

    /// Policy verdict
    pub verdict: PolicyVerdict,

    /// Bytes transferred
    pub bytes: u64,

    /// Packets count
    pub packets: u64,

    /// HTTP method (if L7)
    pub http_method: Option<String>,

    /// HTTP path (if L7)
    pub http_path: Option<String>,

    /// HTTP status (if L7)
    pub http_status: Option<u16>,
}

/// Recording metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Recording {
    /// Recording ID
    pub id: String,

    /// Name/description
    pub name: String,

    /// Source cluster
    pub source_cluster: String,

    /// Recording start time
    pub start_time: u64,

    /// Recording end time
    pub end_time: u64,

    /// Total flows recorded
    pub flow_count: usize,

    /// Total bytes
    pub total_bytes: u64,

    /// Unique namespaces
    pub namespaces: Vec<String>,

    /// Unique services
    pub services: Vec<String>,

    /// File path
    pub file_path: PathBuf,

    /// Compressed
    pub compressed: bool,
}

/// Replay result
#[derive(Debug, Clone)]
pub struct ReplayResult {
    /// Recording that was replayed
    pub recording: Recording,

    /// Target cluster
    pub target_cluster: String,

    /// Flows attempted
    pub flows_attempted: usize,

    /// Flows successful
    pub flows_successful: usize,

    /// Flows failed
    pub flows_failed: usize,

    /// Comparison results
    pub comparison: Option<ComparisonResult>,

    /// Replay duration
    pub replay_duration_ms: u64,

    /// Errors encountered
    pub errors: Vec<String>,
}

/// Comparison between original and replay
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Total flows compared
    pub total_flows: usize,

    /// Flows with identical behavior
    pub identical: usize,

    /// Flows with different verdicts
    pub verdict_changed: usize,

    /// Flows with different latency
    pub latency_changed: usize,

    /// New drops (worked before, dropped now)
    pub new_drops: Vec<FlowDifference>,

    /// Fixed drops (dropped before, works now)
    pub fixed_drops: Vec<FlowDifference>,

    /// Performance differences
    pub performance: PerformanceDifference,

    /// Overall similarity score (0.0 to 1.0)
    pub similarity_score: f32,
}

/// Difference between original and replayed flow
#[derive(Debug, Clone)]
pub struct FlowDifference {
    pub flow: RecordedFlow,
    pub original_verdict: PolicyVerdict,
    pub replay_verdict: PolicyVerdict,
    pub original_latency_ms: Option<f64>,
    pub replay_latency_ms: Option<f64>,
}

/// Performance difference
#[derive(Debug, Clone)]
pub struct PerformanceDifference {
    pub avg_latency_original_ms: f64,
    pub avg_latency_replay_ms: f64,
    pub latency_delta_percent: f64,
    pub throughput_original_mbps: f64,
    pub throughput_replay_mbps: f64,
    pub throughput_delta_percent: f64,
}

/// Replay filter
#[derive(Debug, Clone)]
pub struct ReplayFilter {
    /// Filter by namespace
    pub namespaces: Option<Vec<String>>,

    /// Filter by source labels
    pub src_labels: Option<HashMap<String, String>>,

    /// Filter by destination labels
    pub dst_labels: Option<HashMap<String, String>>,

    /// Filter by ports
    pub ports: Option<Vec<u16>>,

    /// Filter by protocols
    pub protocols: Option<Vec<u8>>,

    /// Filter by verdict
    pub verdicts: Option<Vec<PolicyVerdict>>,

    /// Limit flow count
    pub limit: Option<usize>,
}

impl Default for ReplayFilter {
    fn default() -> Self {
        Self {
            namespaces: None,
            src_labels: None,
            dst_labels: None,
            ports: None,
            protocols: None,
            verdicts: None,
            limit: None,
        }
    }
}

/// Traffic Replay Engine
pub struct ReplayEngine<M: MapReader> {
    config: ReplayConfig,
    ebpf_reader: M,
    k8s_client: K8sClient,

    /// Current recording session
    current_recording: Option<RecordingSession>,

    /// Available recordings
    recordings: Vec<Recording>,
}

/// Active recording session
struct RecordingSession {
    recording: Recording,
    flows: Vec<RecordedFlow>,
    start_time: SystemTime,
}

impl<M: MapReader> ReplayEngine<M> {
    pub fn new(
        config: ReplayConfig,
        ebpf_reader: M,
        k8s_client: K8sClient,
    ) -> Self {
        // Ensure recording directory exists
        if let Err(e) = std::fs::create_dir_all(&config.recording_dir) {
            eprintln!("Failed to create recording directory: {}", e);
        }

        Self {
            config,
            ebpf_reader,
            k8s_client,
            current_recording: None,
            recordings: Vec::new(),
        }
    }

    /// Start recording traffic
    pub async fn start_recording(&mut self, name: String) -> Result<String> {
        if self.current_recording.is_some() {
            anyhow::bail!("Recording already in progress");
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let id = format!("rec-{}-{}", now, uuid::Uuid::new_v4());

        let recording = Recording {
            id: id.clone(),
            name,
            source_cluster: "default".to_string(), // TODO: Get from K8s
            start_time: now,
            end_time: 0,
            flow_count: 0,
            total_bytes: 0,
            namespaces: Vec::new(),
            services: Vec::new(),
            file_path: self.config.recording_dir.join(format!("{}.json", id)),
            compressed: self.config.compress,
        };

        self.current_recording = Some(RecordingSession {
            recording,
            flows: Vec::new(),
            start_time: SystemTime::now(),
        });

        println!("🔴 Recording started: {}", id);

        Ok(id)
    }

    /// Capture current traffic
    pub async fn capture(&mut self) -> Result<usize> {
        // Read current connections first
        let connections = self.ebpf_reader.read_conntrack_map()?;

        // Get session
        let session = self.current_recording.as_mut()
            .ok_or_else(|| anyhow::anyhow!("No recording in progress"))?;

        let start_time = session.start_time;
        let mut captured = 0;

        // Read IPCache for resolution
        let ipcache = self.ebpf_reader.read_ipcache_map().unwrap_or_default();

        for conn in &connections {
            let flow = Self::conntrack_to_recorded_flow(conn, &start_time, &ipcache).await?;
            session.flows.push(flow);
            captured += 1;

            // Check size limits
            if session.flows.len() * 1024 > self.config.max_recording_size {
                println!("⚠️  Recording size limit reached");
                break;
            }
        }

        session.recording.flow_count = session.flows.len();

        Ok(captured)
    }

    /// Stop recording and save
    pub async fn stop_recording(&mut self) -> Result<Recording> {
        let mut session = self.current_recording.take()
            .ok_or_else(|| anyhow::anyhow!("No recording in progress"))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        session.recording.end_time = now;
        session.recording.flow_count = session.flows.len();

        // Calculate total bytes
        session.recording.total_bytes = session.flows.iter()
            .map(|f| f.bytes)
            .sum();

        // Extract unique namespaces
        let mut namespaces: Vec<_> = session.flows.iter()
            .flat_map(|f| vec![f.src_namespace.clone(), f.dst_namespace.clone()])
            .collect();
        namespaces.sort();
        namespaces.dedup();
        session.recording.namespaces = namespaces;

        // Save to file
        use storage::RecordingStorage;
        let storage = RecordingStorage::new(self.config.recording_dir.clone());
        storage.save(&session.recording, &session.flows)?;

        println!("⏹️  Recording stopped: {} ({} flows)",
            session.recording.id,
            session.recording.flow_count);

        let recording = session.recording.clone();
        self.recordings.push(recording.clone());

        Ok(recording)
    }

    /// Resolve IP to identity and namespace from IPCache entries
    fn resolve_ip_from_cache(ip: &str, ipcache: &[crate::ebpf::IPCacheEntry]) -> (u32, String, HashMap<String, String>) {
        if let Some(entry) = ipcache.iter().find(|e| e.ip == ip) {
            let namespace = if entry.namespace.is_empty() {
                "unknown".to_string()
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
            return (entry.identity, namespace, labels);
        }
        (0, "unknown".to_string(), HashMap::new())
    }

    /// Convert conntrack entry to recorded flow
    async fn conntrack_to_recorded_flow(
        conn: &ConntrackEntry,
        start_time: &SystemTime,
        ipcache: &[crate::ebpf::IPCacheEntry],
    ) -> Result<RecordedFlow> {
        let now = SystemTime::now();
        let offset_ms = now.duration_since(*start_time)?.as_millis() as u64;

        let timestamp = now.duration_since(UNIX_EPOCH)?.as_secs();

        let src_ip: IpAddr = conn.src_ip.parse()
            .unwrap_or_else(|_| IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));
        let dst_ip: IpAddr = conn.dst_ip.parse()
            .unwrap_or_else(|_| IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));

        let (src_identity, src_namespace, src_labels) = Self::resolve_ip_from_cache(&conn.src_ip, ipcache);
        let (dst_identity, dst_namespace, dst_labels) = Self::resolve_ip_from_cache(&conn.dst_ip, ipcache);

        Ok(RecordedFlow {
            timestamp,
            offset_ms,
            src_ip,
            dst_ip,
            src_port: conn.src_port,
            dst_port: conn.dst_port,
            protocol: conn.protocol,
            src_identity,
            dst_identity,
            src_namespace,
            dst_namespace,
            src_labels,
            dst_labels,
            verdict: PolicyVerdict::Allow,  // Assume allowed if in conntrack
            bytes: conn.bytes,
            packets: conn.packets,
            http_method: None,
            http_path: None,
            http_status: None,
        })
    }

    /// Replay a recording
    pub async fn replay(
        &mut self,
        recording_id: &str,
        filter: Option<ReplayFilter>,
    ) -> Result<ReplayResult> {
        use player::ReplayPlayer;
        use comparator::ReplayComparator;

        // Load recording
        let recording = self.load_recording(recording_id)?;

        println!("▶️  Replaying: {} ({} flows)", recording.name, recording.flow_count);

        // Load flows
        use storage::RecordingStorage;
        let storage = RecordingStorage::new(self.config.recording_dir.clone());
        let mut flows = storage.load(&recording)?;

        // Apply filter
        if let Some(f) = &filter {
            flows = Self::apply_filter(flows, f);
        }

        let start = std::time::Instant::now();

        // Replay flows
        let player = ReplayPlayer::new(
            self.config.replay_rate,
            &self.ebpf_reader,
            &self.k8s_client,
        );

        let replay_outcomes = player.replay(&flows).await?;

        let replay_duration_ms = start.elapsed().as_millis() as u64;

        // Compare if enabled
        let comparison = if self.config.detailed_comparison {
            let comparator = ReplayComparator::new();
            Some(comparator.compare(&flows, &replay_outcomes)?)
        } else {
            None
        };

        // Count successes/failures
        let flows_successful = replay_outcomes.iter()
            .filter(|o| o.success)
            .count();

        let flows_failed = replay_outcomes.iter()
            .filter(|o| !o.success)
            .count();

        let errors = replay_outcomes.iter()
            .filter_map(|o| o.error.clone())
            .collect();

        Ok(ReplayResult {
            recording,
            target_cluster: "local".to_string(),  // TODO: Support remote clusters
            flows_attempted: flows.len(),
            flows_successful,
            flows_failed,
            comparison,
            replay_duration_ms,
            errors,
        })
    }

    /// Apply filter to flows
    fn apply_filter(flows: Vec<RecordedFlow>, filter: &ReplayFilter) -> Vec<RecordedFlow> {
        let mut filtered = flows;

        if let Some(namespaces) = &filter.namespaces {
            filtered.retain(|f| {
                namespaces.contains(&f.src_namespace) || namespaces.contains(&f.dst_namespace)
            });
        }

        if let Some(ports) = &filter.ports {
            filtered.retain(|f| ports.contains(&f.dst_port));
        }

        if let Some(protocols) = &filter.protocols {
            filtered.retain(|f| protocols.contains(&f.protocol));
        }

        if let Some(verdicts) = &filter.verdicts {
            filtered.retain(|f| verdicts.contains(&f.verdict));
        }

        if let Some(limit) = filter.limit {
            filtered.truncate(limit);
        }

        filtered
    }

    /// Load recording metadata
    fn load_recording(&mut self, recording_id: &str) -> Result<Recording> {
        // Try to find in memory
        if let Some(rec) = self.recordings.iter().find(|r| r.id == recording_id) {
            return Ok(rec.clone());
        }

        // Load from disk
        use storage::RecordingStorage;
        let storage = RecordingStorage::new(self.config.recording_dir.clone());
        let recording = storage.load_metadata(recording_id)?;

        self.recordings.push(recording.clone());

        Ok(recording)
    }

    /// List available recordings
    pub fn list_recordings(&mut self) -> Result<Vec<Recording>> {
        use storage::RecordingStorage;
        let storage = RecordingStorage::new(self.config.recording_dir.clone());
        let recordings = storage.list()?;

        self.recordings = recordings.clone();

        Ok(recordings)
    }

    /// Delete a recording
    pub fn delete_recording(&mut self, recording_id: &str) -> Result<()> {
        use storage::RecordingStorage;
        let storage = RecordingStorage::new(self.config.recording_dir.clone());
        storage.delete(recording_id)?;

        self.recordings.retain(|r| r.id != recording_id);

        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> ReplayStats {
        ReplayStats {
            total_recordings: self.recordings.len(),
            recording_in_progress: self.current_recording.is_some(),
            total_flows_recorded: self.recordings.iter().map(|r| r.flow_count).sum(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplayStats {
    pub total_recordings: usize,
    pub recording_in_progress: bool,
    pub total_flows_recorded: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ebpf::MockMapReader;

    #[tokio::test]
    async fn test_replay_engine_creation() {
        let config = ReplayConfig::default();
        let reader = MockMapReader;
        let k8s_client = K8sClient::new().await.unwrap();

        let engine = ReplayEngine::new(config, reader, k8s_client);
        assert_eq!(engine.recordings.len(), 0);
    }

    #[test]
    fn test_replay_filter_default() {
        let filter = ReplayFilter::default();
        assert!(filter.namespaces.is_none());
        assert!(filter.limit.is_none());
    }

    #[tokio::test]
    async fn test_start_recording() {
        let config = ReplayConfig::default();
        let reader = MockMapReader;
        let k8s_client = K8sClient::new().await.unwrap();

        let mut engine = ReplayEngine::new(config, reader, k8s_client);
        let id = engine.start_recording("test".to_string()).await.unwrap();

        assert!(!id.is_empty());
        assert!(engine.current_recording.is_some());
    }
}
