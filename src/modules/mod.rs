/// Escape a string for safe interpolation into YAML values.
/// Wraps the value in double quotes and escapes embedded backslashes, quotes,
/// newlines, carriage returns, and tabs.
pub fn yaml_escape(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    )
}

/// Sanitize a string for use as a Kubernetes resource name.
/// Lowercases, replaces non-alphanumeric characters (except hyphens and dots)
/// with hyphens, and trims leading/trailing hyphens.
pub fn sanitize_k8s_name(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub mod autopolicy;
pub mod canary;
pub mod chaos;
/// Intelligent modules for cilium-vision
///
/// Each module provides a specific capability:
/// - healer: Automatic problem detection and fixing
/// - autopolicy: Zero-trust policy learning and generation
/// - rootcause: Drop analysis and explanation
/// - replay: Traffic recording and replay
/// - simulator: What-if policy simulation
/// - packet_explainer: Interactive packet analysis and explanation
/// - chaos: Chaos engineering and fault injection
/// - canary: Progressive traffic shifting and canary deployments
/// - multicluster: Multi-cluster orchestration
/// - anomaly_detection: AI/ML-powered anomaly detection (experimental)
/// - ebpf_advanced: Advanced eBPF capabilities (experimental)
/// - security_compliance: Zero-trust and compliance frameworks (experimental)
/// - dev_tools: Developer experience tools (experimental)
pub mod healer;
pub mod multicluster;
pub mod packet_explainer;
pub mod replay;
pub mod rootcause;
pub mod simulator;

// Experimental cutting-edge features
pub mod anomaly_detection;
pub mod dev_tools;
pub mod ebpf_advanced;
pub mod security_compliance;
