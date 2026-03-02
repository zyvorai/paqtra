// Library re-exports for integration testing
//
// This file exposes internal modules so that integration tests in the
// `tests/` directory can import crate types.

pub mod ebpf;
pub mod kubernetes;
pub mod policies;
pub mod cilium;
pub mod endpoints;
pub mod hubble;
pub mod bootstrap;
pub mod integration;

// Selective module re-exports to avoid broken experimental modules.
// Each stable module is declared directly with its path.
pub mod modules {
    #[path = "../modules/healer/mod.rs"]
    pub mod healer;

    #[path = "../modules/autopolicy/mod.rs"]
    pub mod autopolicy;

    #[path = "../modules/rootcause/mod.rs"]
    pub mod rootcause;

    #[path = "../modules/simulator/mod.rs"]
    pub mod simulator;

    #[path = "../modules/replay/mod.rs"]
    pub mod replay;

    #[path = "../modules/packet_explainer/mod.rs"]
    pub mod packet_explainer;

    #[path = "../modules/chaos/mod.rs"]
    pub mod chaos;

    #[path = "../modules/canary/mod.rs"]
    pub mod canary;

    #[path = "../modules/multicluster/mod.rs"]
    pub mod multicluster;

    #[path = "../modules/dev_tools/mod.rs"]
    pub mod dev_tools;

    #[path = "../modules/ebpf_advanced/mod.rs"]
    pub mod ebpf_advanced;

    #[path = "../modules/security_compliance/mod.rs"]
    pub mod security_compliance;

    #[path = "../modules/anomaly_detection/mod.rs"]
    pub mod anomaly_detection;
}
