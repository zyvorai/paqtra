/// Integration tests for cross-module interactions.
///
/// Validates that multiple modules can be instantiated with the same
/// mock infrastructure and that their data structures interoperate
/// correctly. Tests interactions between RootCause, Healer, AutoPolicy,
/// Simulator, and Replay modules.

use cilium_tui::ebpf::{
    DropReasonType, MapReader, MockMapReader, PolicyVerdict,
};
use cilium_tui::kubernetes::K8sClient;
use cilium_tui::modules::autopolicy::{AutoPolicy, AutoPolicyConfig, LearningState};
use cilium_tui::modules::healer::{HealerConfig, SelfHealer};
use cilium_tui::modules::replay::{ReplayConfig, ReplayEngine};
use cilium_tui::modules::rootcause::{DropReason as RootCauseDropReason, RootCauseConfig, RootCauseEngine};
use cilium_tui::modules::simulator::{RiskLevel, Simulator, SimulatorConfig};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn mock_k8s_client() -> K8sClient {
    K8sClient::mock()
}

// ---------------------------------------------------------------------------
// Tests: Multiple modules share the same MockMapReader type
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_modules_share_mock_reader() {
    // All modules should be constructible with MockMapReader.
    // This verifies the MapReader trait bound is satisfied uniformly.
    let k8s = mock_k8s_client();

    let healer = SelfHealer::new(HealerConfig::default(), MockMapReader, k8s.clone());
    let autopolicy = AutoPolicy::new(AutoPolicyConfig::default(), MockMapReader, k8s.clone());
    let rootcause = RootCauseEngine::new(RootCauseConfig::default(), MockMapReader, k8s.clone());
    let simulator = Simulator::new(SimulatorConfig::default(), MockMapReader, k8s.clone());
    let replay = ReplayEngine::new(ReplayConfig::default(), MockMapReader, k8s);

    // Verify each module starts in a clean state
    assert!(healer.problems().is_empty());
    assert_eq!(*autopolicy.state(), LearningState::NotStarted);
    assert_eq!(rootcause.get_stats().total_drops, 0);
    assert_eq!(simulator.stats().flow_history_size, 0);
    assert_eq!(replay.stats().total_recordings, 0);
}

// ---------------------------------------------------------------------------
// Tests: RootCause and Healer analyze from the same mock data source
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_rootcause_and_healer_both_start_empty() {
    let k8s = mock_k8s_client();
    let mut healer = SelfHealer::new(HealerConfig::default(), MockMapReader, k8s.clone());
    let rootcause = RootCauseEngine::new(RootCauseConfig::default(), MockMapReader, k8s);

    // Both modules read from MockMapReader, which returns empty drop maps.
    // Run healer's public API
    let healer_stats = healer.run().await.unwrap();
    assert_eq!(
        healer_stats.problems_detected, 0,
        "Healer should detect no problems from empty mock drops"
    );
    assert!(healer.problems().is_empty());

    // RootCause stats should also be empty
    let stats = rootcause.get_stats();
    assert_eq!(stats.total_drops, 0);
}

#[tokio::test]
async fn test_rootcause_and_healer_consistent_run_on_mock() {
    let k8s = mock_k8s_client();

    // Run healer
    let mut healer = SelfHealer::new(HealerConfig::default(), MockMapReader, k8s.clone());
    let healer_stats = healer.run().await.unwrap();

    // Run rootcause analysis
    let mut rootcause = RootCauseEngine::new(RootCauseConfig::default(), MockMapReader, k8s);
    let analyses = rootcause.analyze_drops().await.unwrap();

    // Both should find nothing from empty mock data
    assert_eq!(healer_stats.problems_detected, 0);
    assert!(analyses.is_empty());
}

// ---------------------------------------------------------------------------
// Tests: DropReason mapping consistency between eBPF and RootCause
// ---------------------------------------------------------------------------

#[test]
fn test_drop_reason_mapping_consistency() {
    // Verify that all major DropReasonType variants produce valid
    // RootCauseDropReason conversions without panicking.
    let reason_types = vec![
        DropReasonType::PolicyDenied,
        DropReasonType::InvalidPacket,
        DropReasonType::NoRoute,
        DropReasonType::UnknownL4Protocol,
        DropReasonType::FragmentationNeeded,
        DropReasonType::CTMapFull,
        DropReasonType::NATMapFull,
        DropReasonType::InvalidSourceIP,
        DropReasonType::InvalidDestIP,
        DropReasonType::ServiceBackendNotFound,
        DropReasonType::ConnectionTrackingInvalid,
        DropReasonType::AuthRequired,
        DropReasonType::Other(42),
    ];

    for rt in &reason_types {
        let converted = RootCauseDropReason::from_ebpf(rt);
        let display = converted.to_string();
        assert!(
            !display.is_empty(),
            "DropReason from {:?} should have a non-empty display string",
            rt
        );
    }
}

#[test]
fn test_policy_denied_maps_consistently() {
    // PolicyDenied in eBPF should map to PolicyDenied in RootCause
    let ebpf_reason = DropReasonType::PolicyDenied;
    let rootcause_reason = RootCauseDropReason::from_ebpf(&ebpf_reason);
    assert_eq!(rootcause_reason, RootCauseDropReason::PolicyDenied);
}

#[test]
fn test_auth_required_maps_to_policy_denied() {
    // AuthRequired in eBPF maps to PolicyDenied in RootCause
    let rootcause_reason = RootCauseDropReason::from_ebpf(&DropReasonType::AuthRequired);
    assert_eq!(rootcause_reason, RootCauseDropReason::PolicyDenied);
}

#[test]
fn test_fragmentation_maps_to_frag_needed() {
    let rootcause_reason = RootCauseDropReason::from_ebpf(&DropReasonType::FragmentationNeeded);
    assert_eq!(rootcause_reason, RootCauseDropReason::FragNeeded);
}

#[test]
fn test_service_backend_not_found_maps_to_no_backend() {
    let rootcause_reason = RootCauseDropReason::from_ebpf(&DropReasonType::ServiceBackendNotFound);
    assert_eq!(rootcause_reason, RootCauseDropReason::NoBackend);
}

#[test]
fn test_ct_map_full_maps_to_ct_state_mismatch() {
    let rootcause_reason = RootCauseDropReason::from_ebpf(&DropReasonType::CTMapFull);
    assert_eq!(rootcause_reason, RootCauseDropReason::CTStateMismatch);
}

// ---------------------------------------------------------------------------
// Tests: Modules can run concurrently on mock infrastructure
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_healer_and_autopolicy_run_independently() {
    let k8s = mock_k8s_client();

    let mut healer = SelfHealer::new(HealerConfig::default(), MockMapReader, k8s.clone());
    let mut autopolicy = AutoPolicy::new(AutoPolicyConfig::default(), MockMapReader, k8s);

    // Run healer
    let healer_stats = healer.run().await.unwrap();
    assert_eq!(healer_stats.problems_detected, 0);

    // Start autopolicy learning
    autopolicy.start_learning().await.unwrap();
    assert!(matches!(autopolicy.state(), LearningState::Learning { .. }));

    // Both should still be in valid states
    assert!(healer.problems().is_empty());
    assert!(autopolicy.observations().is_empty());
}

#[tokio::test]
async fn test_simulator_and_replay_run_independently() {
    let k8s = mock_k8s_client();

    let mut simulator = Simulator::new(SimulatorConfig::default(), MockMapReader, k8s.clone());
    let mut replay = ReplayEngine::new(ReplayConfig::default(), MockMapReader, k8s);

    // Load simulator history
    let loaded = simulator.load_history().await.unwrap();
    assert_eq!(loaded, 0);

    // Start replay recording
    let rec_id = replay.start_recording("cross-test".to_string()).await.unwrap();
    assert!(!rec_id.is_empty());

    // Both modules operate independently
    assert_eq!(simulator.stats().flow_history_size, 0);
    assert!(replay.stats().recording_in_progress);
}

#[tokio::test]
async fn test_rootcause_and_autopolicy_independent() {
    let k8s = mock_k8s_client();

    let mut rootcause = RootCauseEngine::new(RootCauseConfig::default(), MockMapReader, k8s.clone());
    let mut autopolicy = AutoPolicy::new(AutoPolicyConfig::default(), MockMapReader, k8s);

    // Analyze drops via rootcause
    let analyses = rootcause.analyze_drops().await.unwrap();
    assert!(analyses.is_empty());

    // Start autopolicy learning
    autopolicy.start_learning().await.unwrap();
    assert!(matches!(autopolicy.state(), LearningState::Learning { .. }));

    // Update autopolicy
    let stats = autopolicy.update().await.unwrap();
    assert_eq!(stats.connections_observed, 0);

    // rootcause is still in valid state
    assert_eq!(rootcause.get_stats().total_drops, 0);
}

// ---------------------------------------------------------------------------
// Tests: Risk level from simulator applies to policy decisions
// ---------------------------------------------------------------------------

#[test]
fn test_risk_level_informs_policy_verdict_decisions() {
    // Low risk changes are safe to apply
    let low = RiskLevel::from_score(1);
    assert_eq!(low, RiskLevel::Low);

    // Critical risk changes should be blocked
    let critical = RiskLevel::from_score(10);
    assert_eq!(critical, RiskLevel::Critical);

    // These are the thresholds that would be used in cross-module decisions
    assert!(RiskLevel::Low < RiskLevel::Medium);
    assert!(RiskLevel::High < RiskLevel::Critical);
}

#[test]
fn test_risk_level_boundary_values() {
    // Exact boundaries
    assert_eq!(RiskLevel::from_score(2), RiskLevel::Low);
    assert_eq!(RiskLevel::from_score(3), RiskLevel::Medium);
    assert_eq!(RiskLevel::from_score(5), RiskLevel::Medium);
    assert_eq!(RiskLevel::from_score(6), RiskLevel::High);
    assert_eq!(RiskLevel::from_score(8), RiskLevel::High);
    assert_eq!(RiskLevel::from_score(9), RiskLevel::Critical);
}

// ---------------------------------------------------------------------------
// Tests: MockMapReader returns consistent data across modules
// ---------------------------------------------------------------------------

#[test]
fn test_mock_map_reader_consistent_across_reads() {
    let reader = MockMapReader;

    // Policy map should return the same data every time
    let policies1 = reader.read_policy_map().unwrap();
    let policies2 = reader.read_policy_map().unwrap();
    assert_eq!(policies1.len(), policies2.len());
    assert_eq!(policies1[0].src_identity, policies2[0].src_identity);
    assert_eq!(policies1[0].verdict, policies2[0].verdict);

    // Drop map should consistently be empty
    let drops1 = reader.read_drop_map().unwrap();
    let drops2 = reader.read_drop_map().unwrap();
    assert_eq!(drops1.len(), drops2.len());
    assert!(drops1.is_empty());
    assert!(drops2.is_empty());
}

#[test]
fn test_mock_reader_all_maps_readable() {
    let reader = MockMapReader;

    assert!(reader.read_policy_map().is_ok());
    assert!(reader.read_conntrack_map().is_ok());
    assert!(reader.read_lb_map().is_ok());
    assert!(reader.read_ipcache_map().is_ok());
    assert!(reader.read_drop_map().is_ok());
}

#[test]
fn test_mock_reader_policy_map_has_allow_verdict() {
    let reader = MockMapReader;
    let decisions = reader.read_policy_map().unwrap();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].verdict, PolicyVerdict::Allow);
}

// ---------------------------------------------------------------------------
// Tests: K8sClient::mock() is reusable across modules
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_k8s_mock_cloneable_across_modules() {
    let k8s = mock_k8s_client();

    // Clone the client for each module (mimicking real usage)
    let _healer = SelfHealer::new(HealerConfig::default(), MockMapReader, k8s.clone());
    let _autopolicy = AutoPolicy::new(AutoPolicyConfig::default(), MockMapReader, k8s.clone());
    let _rootcause = RootCauseEngine::new(RootCauseConfig::default(), MockMapReader, k8s.clone());
    let _simulator = Simulator::new(SimulatorConfig::default(), MockMapReader, k8s.clone());
    let _replay = ReplayEngine::new(ReplayConfig::default(), MockMapReader, k8s);

    // If we get here, all modules accepted the cloned K8sClient
}

// ---------------------------------------------------------------------------
// Tests: Full lifecycle via public APIs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_healer_then_rootcause_lifecycle() {
    let k8s = mock_k8s_client();

    // Step 1: Healer runs and finds no problems on mock data
    let mut healer = SelfHealer::new(HealerConfig::default(), MockMapReader, k8s.clone());
    let healer_stats = healer.run().await.unwrap();
    assert_eq!(healer_stats.problems_detected, 0);
    assert!(healer.problems().is_empty());
    assert!(healer.fixes().is_empty());

    // Step 2: RootCause analyzes the same mock data and finds no drops
    let mut rootcause = RootCauseEngine::new(RootCauseConfig::default(), MockMapReader, k8s);
    let analyses = rootcause.analyze_drops().await.unwrap();
    assert!(analyses.is_empty());
    assert_eq!(rootcause.get_stats().total_drops, 0);
}

#[tokio::test]
async fn test_autopolicy_to_simulator_lifecycle() {
    let k8s = mock_k8s_client();

    // Step 1: AutoPolicy starts learning
    let mut autopolicy = AutoPolicy::new(AutoPolicyConfig::default(), MockMapReader, k8s.clone());
    autopolicy.start_learning().await.unwrap();
    assert!(matches!(autopolicy.state(), LearningState::Learning { .. }));

    // Step 2: Update (no data from mock)
    let learning_stats = autopolicy.update().await.unwrap();
    assert_eq!(learning_stats.connections_observed, 0);

    // Step 3: Generate policies (none, since no observations)
    let policies = autopolicy.generate_policies().unwrap();
    assert!(policies.is_empty());

    // Step 4: Simulator can be used to test the (empty) policies
    let mut simulator = Simulator::new(SimulatorConfig::default(), MockMapReader, k8s);
    let loaded = simulator.load_history().await.unwrap();
    assert_eq!(loaded, 0);
}

#[tokio::test]
async fn test_replay_to_rootcause_lifecycle() {
    let k8s = mock_k8s_client();

    // Step 1: Start a recording
    let mut replay = ReplayEngine::new(ReplayConfig::default(), MockMapReader, k8s.clone());
    let rec_id = replay.start_recording("lifecycle-test".to_string()).await.unwrap();
    assert!(!rec_id.is_empty());

    // Step 2: Capture (no data from mock)
    let captured = replay.capture().await.unwrap();
    assert_eq!(captured, 0);

    // Step 3: Stop recording
    let recording = replay.stop_recording().await.unwrap();
    assert_eq!(recording.name, "lifecycle-test");
    assert_eq!(recording.flow_count, 0);

    // Step 4: RootCause can analyze drops (empty in mock)
    let mut rootcause = RootCauseEngine::new(RootCauseConfig::default(), MockMapReader, k8s);
    let analyses = rootcause.analyze_drops().await.unwrap();
    assert!(analyses.is_empty());
}

// ---------------------------------------------------------------------------
// Tests: All five modules created simultaneously
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_all_five_modules_coexist() {
    let k8s = mock_k8s_client();

    let mut healer = SelfHealer::new(HealerConfig::default(), MockMapReader, k8s.clone());
    let mut autopolicy = AutoPolicy::new(AutoPolicyConfig::default(), MockMapReader, k8s.clone());
    let mut rootcause = RootCauseEngine::new(RootCauseConfig::default(), MockMapReader, k8s.clone());
    let mut simulator = Simulator::new(SimulatorConfig::default(), MockMapReader, k8s.clone());
    let mut replay = ReplayEngine::new(ReplayConfig::default(), MockMapReader, k8s);

    // Run all modules
    let healer_stats = healer.run().await.unwrap();
    autopolicy.start_learning().await.unwrap();
    let _learning_stats = autopolicy.update().await.unwrap();
    let analyses = rootcause.analyze_drops().await.unwrap();
    let loaded = simulator.load_history().await.unwrap();
    let rec_id = replay.start_recording("all-modules-test".to_string()).await.unwrap();

    // Verify all are in expected states
    assert_eq!(healer_stats.problems_detected, 0);
    assert!(matches!(autopolicy.state(), LearningState::Learning { .. }));
    assert!(analyses.is_empty());
    assert_eq!(loaded, 0);
    assert!(!rec_id.is_empty());
    assert!(replay.stats().recording_in_progress);
}
