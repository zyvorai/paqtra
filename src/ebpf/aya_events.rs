#![allow(dead_code)]
/// BPF Event Streaming via Aya
///
/// Provides real-time event streaming from Cilium's perf event arrays
/// and ring buffers using the Aya crate. Events include drops, traces,
/// and policy verdicts.
use anyhow::Result;
use std::path::PathBuf;

#[cfg(feature = "aya-ebpf")]
use bytes::BytesMut;

/// Default path to Cilium's pinned event maps.
const CILIUM_BPF_PATH: &str = "/sys/fs/bpf/tc/globals";
const MAP_EVENTS: &str = "cilium_events";

/// Cilium event types (from bpf/lib/common.h).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BpfEventType {
    Drop,
    Debug,
    Capture,
    Trace,
    PolicyVerdict,
    Unknown(u8),
}

impl BpfEventType {
    fn from_type_byte(b: u8) -> Self {
        match b {
            1 => BpfEventType::Drop,
            2 => BpfEventType::Debug,
            3 => BpfEventType::Capture,
            4 => BpfEventType::Trace,
            5 => BpfEventType::PolicyVerdict,
            _ => BpfEventType::Unknown(b),
        }
    }
}

/// A parsed BPF event from Cilium's perf event array.
#[derive(Debug, Clone)]
pub struct BpfEvent {
    pub event_type: BpfEventType,
    pub cpu: u32,
    pub data: BpfEventData,
}

/// Event-specific data.
#[derive(Debug, Clone)]
pub enum BpfEventData {
    Drop(DropEvent),
    Trace(TraceEvent),
    PolicyVerdict(PolicyVerdictEvent),
    Raw(Vec<u8>),
}

/// A packet drop event.
#[derive(Debug, Clone)]
pub struct DropEvent {
    pub src_label: u32,
    pub dst_label: u32,
    pub dst_id: u16,
    pub reason: u8,
    pub src_ip: String,
    pub dst_ip: String,
    pub proto: u8,
    pub src_port: u16,
    pub dst_port: u16,
}

/// A trace/debug event.
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub src_label: u32,
    pub dst_label: u32,
    pub dst_id: u16,
    pub reason: u8,
    pub observation_point: u8,
    pub src_ip: String,
    pub dst_ip: String,
}

/// A policy verdict event.
#[derive(Debug, Clone)]
pub struct PolicyVerdictEvent {
    pub src_label: u32,
    pub dst_label: u32,
    pub dst_port: u16,
    pub proto: u8,
    pub verdict: i32,
    pub direction: u8,
    pub auth_type: u8,
}

/// Streams BPF events from Cilium's perf event array.
pub struct BpfEventStream {
    bpf_path: PathBuf,
}

impl Default for BpfEventStream {
    fn default() -> Self {
        Self::new()
    }
}

impl BpfEventStream {
    pub fn new() -> Self {
        Self {
            bpf_path: PathBuf::from(CILIUM_BPF_PATH),
        }
    }

    pub fn with_path(bpf_path: PathBuf) -> Self {
        Self { bpf_path }
    }

    /// Check if the event map is available.
    pub fn is_available(&self) -> bool {
        self.bpf_path.join(MAP_EVENTS).exists()
    }

    /// Start streaming events via perf event array.
    ///
    /// Returns a receiver that yields parsed BPF events. One task is spawned
    /// per online CPU to read from the per-CPU perf buffers.
    #[cfg(feature = "aya-ebpf")]
    pub async fn start_event_stream(
        &self,
        buffer_size: usize,
    ) -> Result<tokio::sync::mpsc::Receiver<BpfEvent>> {
        use aya::maps::perf::AsyncPerfEventArray;
        use aya::maps::{Map, MapData};
        use aya::util::online_cpus;

        let path = self.bpf_path.join(MAP_EVENTS);
        if !path.exists() {
            anyhow::bail!("Event map not found at {:?}", path);
        }

        let map_data = MapData::from_pin(&path)?;
        let map = Map::PerfEventArray(map_data);
        let mut perf_array = AsyncPerfEventArray::try_from(map)?;

        let (tx, rx) = tokio::sync::mpsc::channel(buffer_size);

        let cpus =
            online_cpus().map_err(|e| anyhow::anyhow!("Failed to get online CPUs: {:?}", e))?;

        for cpu_id in cpus {
            let mut buf = perf_array.open(cpu_id, Some(buffer_size / 2))?;
            let tx = tx.clone();

            tokio::spawn(async move {
                let mut buffers = (0..10)
                    .map(|_| BytesMut::with_capacity(4096))
                    .collect::<Vec<_>>();

                loop {
                    let events = match buf.read_events(&mut buffers).await {
                        Ok(events) => events,
                        Err(e) => {
                            tracing::warn!(cpu = cpu_id, error = %e, "Error reading perf events");
                            break;
                        }
                    };

                    for buf in buffers.iter().take(events.read) {
                        let data = buf;
                        if let Some(event) = parse_perf_event(data, cpu_id) {
                            if tx.send(event).await.is_err() {
                                return; // Receiver dropped
                            }
                        }
                    }
                }
            });
        }

        tracing::info!("BPF event stream started");
        Ok(rx)
    }

    #[cfg(not(feature = "aya-ebpf"))]
    pub async fn start_event_stream(
        &self,
        buffer_size: usize,
    ) -> Result<tokio::sync::mpsc::Receiver<BpfEvent>> {
        tracing::warn!("BPF event streaming requires the aya-ebpf feature");
        let (_tx, rx) = tokio::sync::mpsc::channel(buffer_size);
        Ok(rx)
    }

    /// Start streaming via ring buffer (kernel 5.8+).
    ///
    /// Falls back to perf event array if ring buffer is not available.
    #[cfg(feature = "aya-ebpf")]
    pub async fn start_ring_buffer_stream(
        &self,
        buffer_size: usize,
    ) -> Result<tokio::sync::mpsc::Receiver<BpfEvent>> {
        use aya::maps::RingBuf;
        use aya::maps::{Map, MapData};

        let path = self.bpf_path.join(MAP_EVENTS);
        if !path.exists() {
            anyhow::bail!("Event map not found at {:?}", path);
        }

        // Try ring buffer first
        let map_result = MapData::from_pin(&path);
        match map_result {
            Ok(map_data) => match RingBuf::try_from(Map::RingBuf(map_data)) {
                Ok(mut ring_buf) => {
                    let (tx, rx) = tokio::sync::mpsc::channel(buffer_size);

                    tokio::spawn(async move {
                        loop {
                            while let Some(item) = ring_buf.next() {
                                let data = item.as_ref();
                                if let Some(event) = parse_perf_event(data, 0) {
                                    if tx.send(event).await.is_err() {
                                        return;
                                    }
                                }
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                        }
                    });

                    tracing::info!("Ring buffer event stream started");
                    return Ok(rx);
                }
                Err(_) => {
                    tracing::info!(
                        "Ring buffer not available for this map, falling back to perf array"
                    );
                }
            },
            Err(_) => {
                tracing::info!("Failed to open map for ring buffer, falling back to perf array");
            }
        }

        // Fallback to perf event array
        self.start_event_stream(buffer_size).await
    }

    #[cfg(not(feature = "aya-ebpf"))]
    pub async fn start_ring_buffer_stream(
        &self,
        buffer_size: usize,
    ) -> Result<tokio::sync::mpsc::Receiver<BpfEvent>> {
        self.start_event_stream(buffer_size).await
    }
}

/// Parse a raw perf event buffer into a typed BpfEvent.
///
/// Cilium's perf event header format:
/// - type (u8): event type discriminator
/// - subtype (u8): event subtype
/// - source (u16): source endpoint ID
/// - hash (u32): flow hash
/// - ... followed by event-specific data
fn parse_perf_event(data: &[u8], cpu: u32) -> Option<BpfEvent> {
    if data.len() < 8 {
        return None;
    }

    let event_type = BpfEventType::from_type_byte(data[0]);

    let event_data = match &event_type {
        BpfEventType::Drop => parse_drop_event(data),
        BpfEventType::Trace => parse_trace_event(data),
        BpfEventType::PolicyVerdict => parse_policy_verdict_event(data),
        _ => Some(BpfEventData::Raw(data.to_vec())),
    };

    event_data.map(|event_data| BpfEvent {
        event_type,
        cpu,
        data: event_data,
    })
}

/// Parse a drop event from raw bytes.
fn parse_drop_event(data: &[u8]) -> Option<BpfEventData> {
    // Minimum size for drop event header + IP addresses
    if data.len() < 24 {
        return Some(BpfEventData::Raw(data.to_vec()));
    }

    let src_label = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let dst_label = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    let dst_id = u16::from_le_bytes([data[16], data[17]]);
    let reason = data[18];

    // Parse IP addresses if present (offset depends on Cilium version)
    let (src_ip, dst_ip) = if data.len() >= 32 {
        let src = std::net::Ipv4Addr::new(data[24], data[25], data[26], data[27]);
        let dst = std::net::Ipv4Addr::new(data[28], data[29], data[30], data[31]);
        (src.to_string(), dst.to_string())
    } else {
        ("0.0.0.0".to_string(), "0.0.0.0".to_string())
    };

    Some(BpfEventData::Drop(DropEvent {
        src_label,
        dst_label,
        dst_id,
        reason,
        src_ip,
        dst_ip,
        proto: if data.len() > 19 { data[19] } else { 0 },
        src_port: if data.len() >= 34 {
            u16::from_be_bytes([data[32], data[33]])
        } else {
            0
        },
        dst_port: if data.len() >= 36 {
            u16::from_be_bytes([data[34], data[35]])
        } else {
            0
        },
    }))
}

/// Parse a trace event from raw bytes.
fn parse_trace_event(data: &[u8]) -> Option<BpfEventData> {
    if data.len() < 20 {
        return Some(BpfEventData::Raw(data.to_vec()));
    }

    let src_label = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let dst_label = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    let dst_id = u16::from_le_bytes([data[16], data[17]]);
    let reason = data[18];
    let observation_point = data[19];

    let (src_ip, dst_ip) = if data.len() >= 28 {
        let src = std::net::Ipv4Addr::new(data[20], data[21], data[22], data[23]);
        let dst = std::net::Ipv4Addr::new(data[24], data[25], data[26], data[27]);
        (src.to_string(), dst.to_string())
    } else {
        ("0.0.0.0".to_string(), "0.0.0.0".to_string())
    };

    Some(BpfEventData::Trace(TraceEvent {
        src_label,
        dst_label,
        dst_id,
        reason,
        observation_point,
        src_ip,
        dst_ip,
    }))
}

/// Parse a policy verdict event from raw bytes.
fn parse_policy_verdict_event(data: &[u8]) -> Option<BpfEventData> {
    if data.len() < 24 {
        return Some(BpfEventData::Raw(data.to_vec()));
    }

    let src_label = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let dst_label = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    let dst_port = u16::from_be_bytes([data[16], data[17]]);
    let proto = data[18];
    let verdict = i32::from_le_bytes([data[19], data[20], data[21], data[22]]);
    let direction = data[23];
    let auth_type = if data.len() > 24 { data[24] } else { 0 };

    Some(BpfEventData::PolicyVerdict(PolicyVerdictEvent {
        src_label,
        dst_label,
        dst_port,
        proto,
        verdict,
        direction,
        auth_type,
    }))
}

impl BpfEventData {
    fn as_ref(&self) -> &[u8] {
        match self {
            BpfEventData::Raw(data) => data,
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_from_byte() {
        assert_eq!(BpfEventType::from_type_byte(1), BpfEventType::Drop);
        assert_eq!(BpfEventType::from_type_byte(2), BpfEventType::Debug);
        assert_eq!(BpfEventType::from_type_byte(3), BpfEventType::Capture);
        assert_eq!(BpfEventType::from_type_byte(4), BpfEventType::Trace);
        assert_eq!(BpfEventType::from_type_byte(5), BpfEventType::PolicyVerdict);
        assert_eq!(BpfEventType::from_type_byte(99), BpfEventType::Unknown(99));
    }

    #[test]
    fn test_parse_drop_event() {
        let mut data = vec![0u8; 36];
        data[0] = 1; // Drop type
                     // src_label = 100
        data[8..12].copy_from_slice(&100u32.to_le_bytes());
        // dst_label = 200
        data[12..16].copy_from_slice(&200u32.to_le_bytes());
        // reason = 3
        data[18] = 3;
        // src_ip = 10.0.0.1
        data[24] = 10;
        data[25] = 0;
        data[26] = 0;
        data[27] = 1;
        // dst_ip = 10.0.0.2
        data[28] = 10;
        data[29] = 0;
        data[30] = 0;
        data[31] = 2;

        let event = parse_drop_event(&data);
        assert!(event.is_some());
        if let Some(BpfEventData::Drop(drop)) = event {
            assert_eq!(drop.src_label, 100);
            assert_eq!(drop.dst_label, 200);
            assert_eq!(drop.reason, 3);
            assert_eq!(drop.src_ip, "10.0.0.1");
            assert_eq!(drop.dst_ip, "10.0.0.2");
        } else {
            panic!("Expected DropEvent");
        }
    }

    #[test]
    fn test_parse_trace_event() {
        let mut data = vec![0u8; 28];
        data[0] = 4; // Trace type
        data[8..12].copy_from_slice(&50u32.to_le_bytes());
        data[12..16].copy_from_slice(&60u32.to_le_bytes());
        data[19] = 2; // observation point

        let event = parse_trace_event(&data);
        assert!(event.is_some());
        if let Some(BpfEventData::Trace(trace)) = event {
            assert_eq!(trace.src_label, 50);
            assert_eq!(trace.dst_label, 60);
            assert_eq!(trace.observation_point, 2);
        } else {
            panic!("Expected TraceEvent");
        }
    }

    #[test]
    fn test_parse_policy_verdict_event() {
        let mut data = vec![0u8; 25];
        data[0] = 5; // PolicyVerdict type
        data[8..12].copy_from_slice(&1000u32.to_le_bytes());
        data[12..16].copy_from_slice(&2000u32.to_le_bytes());
        data[16..18].copy_from_slice(&443u16.to_be_bytes());
        data[18] = 6; // TCP
        data[19..23].copy_from_slice(&1i32.to_le_bytes()); // verdict = allow
        data[23] = 1; // direction = egress

        let event = parse_policy_verdict_event(&data);
        assert!(event.is_some());
        if let Some(BpfEventData::PolicyVerdict(pv)) = event {
            assert_eq!(pv.src_label, 1000);
            assert_eq!(pv.dst_label, 2000);
            assert_eq!(pv.dst_port, 443);
            assert_eq!(pv.proto, 6);
            assert_eq!(pv.verdict, 1);
            assert_eq!(pv.direction, 1);
        } else {
            panic!("Expected PolicyVerdictEvent");
        }
    }

    #[test]
    fn test_parse_too_short_data() {
        // Data too short for any structured event
        let data = vec![0u8; 4];
        let result = parse_perf_event(&data, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_event_stream_availability() {
        let stream = BpfEventStream::new();
        // On a non-Cilium system, events should not be available
        // (unless we're actually running on a Cilium node)
        let _ = stream.is_available();
    }

    #[tokio::test]
    async fn test_event_stream_without_cilium() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let stream = BpfEventStream::with_path(temp_dir.path().to_path_buf());
        assert!(!stream.is_available());
    }
}
