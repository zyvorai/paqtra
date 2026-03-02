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
