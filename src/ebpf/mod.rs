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

pub mod bpf_parser;
pub mod bpf_reader;
pub mod bpf_syscall;
pub mod enriched_reader;
pub mod parser;
pub mod reader;

// Re-export for convenience
pub use bpf_reader::CiliumMapReader;
pub use bpf_syscall::IdentityInfo;
pub use enriched_reader::{EnrichedConnectionInfo, EnrichedDropInfo, EnrichedMapReader};

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
#[allow(dead_code)]
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

/// Drop reason types from Cilium kernel eBPF programs.
/// See: https://docs.cilium.io/en/stable/operations/metrics/#drop-reasons
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DropReasonType {
    PolicyDenied,
    InvalidPacket,
    NoRoute,
    UnknownL4Protocol,
    FragmentationNeeded,
    CTMapFull,
    NATMapFull,
    InvalidSourceIP,
    InvalidDestIP,
    UnsupportedL3Protocol,
    MissedTailCall,
    ErrorWritingToPacket,
    UnknownL4ICMPType,
    UnknownICMPv6Type,
    UnknownICMPv6Code,
    ServiceBackendNotFound,
    NoTunnelEndpoint,
    HostUnreachable,
    StaleOrUnroutable,
    ConnectionTrackingInvalid,
    AuthRequired,
    NATNotNeeded,
    IsClusterIP,
    Other(u32),
}

impl DropReasonType {
    pub fn from_code(code: u32) -> Self {
        match code {
            0 => DropReasonType::PolicyDenied,
            1 => DropReasonType::PolicyDenied,
            2 => DropReasonType::InvalidPacket,
            3 => DropReasonType::NoRoute,
            4 => DropReasonType::UnknownL4Protocol,
            5 => DropReasonType::FragmentationNeeded,
            6 => DropReasonType::CTMapFull,
            7 => DropReasonType::NATMapFull,
            130 => DropReasonType::InvalidSourceIP,
            131 => DropReasonType::InvalidDestIP,
            132 => DropReasonType::UnsupportedL3Protocol,
            133 => DropReasonType::MissedTailCall,
            134 => DropReasonType::ErrorWritingToPacket,
            135 => DropReasonType::UnknownL4ICMPType,
            136 => DropReasonType::UnknownICMPv6Type,
            137 => DropReasonType::UnknownICMPv6Code,
            140 => DropReasonType::ServiceBackendNotFound,
            141 => DropReasonType::NoTunnelEndpoint,
            148 => DropReasonType::HostUnreachable,
            152 => DropReasonType::StaleOrUnroutable,
            153 => DropReasonType::ConnectionTrackingInvalid,
            181 => DropReasonType::AuthRequired,
            184 => DropReasonType::NATNotNeeded,
            185 => DropReasonType::IsClusterIP,
            _ => DropReasonType::Other(code),
        }
    }

    #[allow(dead_code)]
    pub fn description(&self) -> &str {
        match self {
            DropReasonType::PolicyDenied => "Policy denied",
            DropReasonType::InvalidPacket => "Invalid packet",
            DropReasonType::NoRoute => "No route",
            DropReasonType::UnknownL4Protocol => "Unknown L4 protocol",
            DropReasonType::FragmentationNeeded => "Fragmentation needed",
            DropReasonType::CTMapFull => "Connection tracking map full",
            DropReasonType::NATMapFull => "NAT map full",
            DropReasonType::InvalidSourceIP => "Invalid source IP",
            DropReasonType::InvalidDestIP => "Invalid destination IP",
            DropReasonType::UnsupportedL3Protocol => "Unsupported L3 protocol",
            DropReasonType::MissedTailCall => "Missed tail call",
            DropReasonType::ErrorWritingToPacket => "Error writing to packet",
            DropReasonType::UnknownL4ICMPType => "Unknown L4 ICMP type",
            DropReasonType::UnknownICMPv6Type => "Unknown ICMPv6 type",
            DropReasonType::UnknownICMPv6Code => "Unknown ICMPv6 code",
            DropReasonType::ServiceBackendNotFound => "Service backend not found",
            DropReasonType::NoTunnelEndpoint => "No tunnel endpoint",
            DropReasonType::HostUnreachable => "Host unreachable",
            DropReasonType::StaleOrUnroutable => "Stale or unroutable",
            DropReasonType::ConnectionTrackingInvalid => "Connection tracking invalid",
            DropReasonType::AuthRequired => "Authentication required",
            DropReasonType::NATNotNeeded => "NAT not needed",
            DropReasonType::IsClusterIP => "Is ClusterIP",
            DropReasonType::Other(code) => {
                let _ = code;
                "Unknown drop reason"
            }
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
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct EbpfMetrics {
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
        Ok(vec![PolicyDecision {
            src_identity: 100,
            dst_identity: 200,
            port: 80,
            protocol: 6, // TCP
            verdict: PolicyVerdict::Allow,
        }])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drop_reason_type_from_code() {
        assert_eq!(DropReasonType::from_code(1), DropReasonType::PolicyDenied);
        assert_eq!(DropReasonType::from_code(2), DropReasonType::InvalidPacket);
        assert_eq!(DropReasonType::from_code(3), DropReasonType::NoRoute);
        assert_eq!(
            DropReasonType::from_code(4),
            DropReasonType::UnknownL4Protocol
        );
        assert_eq!(
            DropReasonType::from_code(5),
            DropReasonType::FragmentationNeeded
        );
        assert_eq!(DropReasonType::from_code(6), DropReasonType::CTMapFull);
        assert_eq!(DropReasonType::from_code(7), DropReasonType::NATMapFull);
        assert_eq!(
            DropReasonType::from_code(130),
            DropReasonType::InvalidSourceIP
        );
        assert_eq!(
            DropReasonType::from_code(131),
            DropReasonType::InvalidDestIP
        );
        assert_eq!(
            DropReasonType::from_code(140),
            DropReasonType::ServiceBackendNotFound
        );
        assert_eq!(DropReasonType::from_code(181), DropReasonType::AuthRequired);
    }

    #[test]
    fn test_drop_reason_type_unknown_code() {
        assert_eq!(DropReasonType::from_code(99), DropReasonType::Other(99));
        assert_eq!(DropReasonType::from_code(255), DropReasonType::Other(255));
    }

    #[test]
    fn test_drop_reason_description() {
        assert_eq!(DropReasonType::PolicyDenied.description(), "Policy denied");
        assert_eq!(DropReasonType::NoRoute.description(), "No route");
        assert_eq!(
            DropReasonType::AuthRequired.description(),
            "Authentication required"
        );
        assert_eq!(
            DropReasonType::Other(99).description(),
            "Unknown drop reason"
        );
    }

    #[test]
    fn test_mock_reader_policy_map() {
        let reader = MockMapReader;
        let policies = reader.read_policy_map().unwrap();
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].src_identity, 100);
        assert_eq!(policies[0].dst_identity, 200);
        assert_eq!(policies[0].port, 80);
        assert_eq!(policies[0].protocol, 6);
        assert_eq!(policies[0].verdict, PolicyVerdict::Allow);
    }

    #[test]
    fn test_mock_reader_empty_maps() {
        let reader = MockMapReader;
        assert!(reader.read_conntrack_map().unwrap().is_empty());
        assert!(reader.read_lb_map().unwrap().is_empty());
        assert!(reader.read_ipcache_map().unwrap().is_empty());
        assert!(reader.read_drop_map().unwrap().is_empty());
    }

    #[test]
    fn test_policy_verdict_equality() {
        assert_eq!(PolicyVerdict::Allow, PolicyVerdict::Allow);
        assert_eq!(PolicyVerdict::Deny, PolicyVerdict::Deny);
        assert_ne!(PolicyVerdict::Allow, PolicyVerdict::Deny);
        assert_ne!(PolicyVerdict::Allow, PolicyVerdict::Redirect);
    }

    #[test]
    fn test_conntrack_state_equality() {
        assert_eq!(ConntrackState::New, ConntrackState::New);
        assert_eq!(ConntrackState::Established, ConntrackState::Established);
        assert_ne!(ConntrackState::New, ConntrackState::Established);
    }

    #[test]
    fn test_ebpf_metrics_default() {
        let metrics = EbpfMetrics::default();
        assert_eq!(metrics.total_packets, 0);
        assert_eq!(metrics.dropped_packets, 0);
        assert_eq!(metrics.forwarded_packets, 0);
        assert_eq!(metrics.policy_drops, 0);
    }
}
