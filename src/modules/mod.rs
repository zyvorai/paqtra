/// Intelligent modules for cilium-vision
///
/// Each module provides a specific capability:
/// - healer: Automatic problem detection and fixing
/// - autopolicy: Zero-trust policy learning and generation
/// - rootcause: Drop analysis and explanation
/// - replay: Traffic recording and replay
/// - simulator: What-if policy simulation
/// - packet_explainer: Interactive packet analysis and explanation
/// - profiler: Performance analysis
/// - optimizer: Cost and placement optimization

pub mod healer;
pub mod autopolicy;
pub mod rootcause;
pub mod simulator;
pub mod replay;
pub mod packet_explainer;
pub mod chaos;
pub mod canary;

// Future modules (stubs for now)
// pub mod profiler;
// pub mod optimizer;
