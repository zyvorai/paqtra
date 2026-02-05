/// eBPF reader layer for direct Cilium map access
///
/// This module provides low-level access to Cilium's eBPF maps for:
/// - Policy decisions
/// - Connection tracking
/// - Load balancing
/// - IP caching
/// - Metrics
/// - Drop reasons

use anyhow::Result;
use std::collections::HashMap;

pub mod reader;
pub mod parser;
pub mod bpf_reader;
pub mod bpf_syscall;
pub mod bpf_parser;
pub mod enriched_reader;

// Re-export for convenience
pub use bpf_reader::CiliumMapReader;
pub use bpf_syscall::{BpfToolReader, IdentityResolver, IdentityInfo};
pub use bpf_parser::{
    parse_ct_entry, parse_ipcache_entry, parse_lb_entry,
    parse_policy_entry, protocol_to_name, port_to_service,
};
pub use enriched_reader::{EnrichedMapReader, EnrichedConnectionInfo, EnrichedDropInfo};

#[cfg(feature = "ebpf")]
pub mod simulator;

/// Cilium eBPF Map Types
#[derive(Debug, Clone)]
pub enum CiliumMap {
    Policy,
    Conntrack,
    LoadBalancer,
    IPCache,
    Metrics,
    DropReason,
}

/// Policy decision from eBPF map
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub src_identity: u32,
    pub dst_identity: u32,
    pub port: u16,
    pub protocol: u8,
    pub verdict: PolicyVerdict,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PolicyVerdict {
    Allow,
    Deny,
    Redirect,
    Audit,
}

/// Connection tracking entry
#[derive(Debug, Clone)]
pub struct ConntrackEntry {
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub state: ConntrackState,
    pub packets: u64,
    pub bytes: u64,
    pub last_seen: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConntrackState {
    New,
    Established,
    Related,
    Invalid,
}

/// Load balancer entry
#[derive(Debug, Clone)]
pub struct LoadBalancerEntry {
    pub service_ip: String,
    pub service_port: u16,
    pub backend_ip: String,
    pub backend_port: u16,
    pub weight: u32,
    pub active_conns: u32,
}

/// IP cache entry (identity to IP mapping)
#[derive(Debug, Clone)]
pub struct IPCacheEntry {
    pub ip: String,
    pub identity: u32,
    pub namespace: String,
    pub labels: Vec<String>,
}

/// Drop reason from eBPF
#[derive(Debug, Clone)]
pub struct DropReason {
    pub src_ip: String,
    pub dst_ip: String,
    pub port: u16,
    pub protocol: u8,
    pub reason: DropReasonType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DropReasonType {
    PolicyDenied,
    InvalidPacket,
    NoRoute,
    UnknownL4Protocol,
    FragmentationNeeded,
    CTMapFull,
    NATMapFull,
    Other(u32),
}

impl DropReasonType {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => DropReasonType::PolicyDenied,
            2 => DropReasonType::InvalidPacket,
            3 => DropReasonType::NoRoute,
            4 => DropReasonType::UnknownL4Protocol,
            5 => DropReasonType::FragmentationNeeded,
            6 => DropReasonType::CTMapFull,
            7 => DropReasonType::NATMapFull,
            _ => DropReasonType::Other(code),
        }
    }
}

/// eBPF Map Reader trait
pub trait MapReader {
    fn read_policy_map(&self) -> Result<Vec<PolicyDecision>>;
    fn read_conntrack_map(&self) -> Result<Vec<ConntrackEntry>>;
    fn read_lb_map(&self) -> Result<Vec<LoadBalancerEntry>>;
    fn read_ipcache_map(&self) -> Result<Vec<IPCacheEntry>>;
    fn read_drop_map(&self) -> Result<Vec<DropReason>>;
}

/// Metrics from eBPF
#[derive(Debug, Clone, Default)]
pub struct eBPFMetrics {
    pub total_packets: u64,
    pub dropped_packets: u64,
    pub forwarded_packets: u64,
    pub policy_drops: u64,
    pub nat_lookups: u64,
    pub ct_lookups: u64,
}

/// Mock reader for testing without eBPF
#[derive(Clone, Copy)]
pub struct MockMapReader;

impl MapReader for MockMapReader {
    fn read_policy_map(&self) -> Result<Vec<PolicyDecision>> {
        Ok(vec![
            PolicyDecision {
                src_identity: 100,
                dst_identity: 200,
                port: 80,
                protocol: 6, // TCP
                verdict: PolicyVerdict::Allow,
            },
        ])
    }

    fn read_conntrack_map(&self) -> Result<Vec<ConntrackEntry>> {
        Ok(vec![])
    }

    fn read_lb_map(&self) -> Result<Vec<LoadBalancerEntry>> {
        Ok(vec![])
    }

    fn read_ipcache_map(&self) -> Result<Vec<IPCacheEntry>> {
        Ok(vec![])
    }

    fn read_drop_map(&self) -> Result<Vec<DropReason>> {
        Ok(vec![])
    }
}
