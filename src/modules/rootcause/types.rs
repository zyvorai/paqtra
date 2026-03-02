use std::collections::HashMap;
use std::net::IpAddr;

pub use crate::ebpf::DropReasonType;

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
