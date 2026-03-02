/// Integration tests for TUI tab management.
///
/// Since the TUI tab system is defined inside the binary's `tui` module
/// and uses hardcoded string arrays, these tests verify the tab index
/// logic, display names, and boundary conditions by replicating the
/// exact tab configuration from `src/tui/mod.rs`.

/// The authoritative tab list from the TUI, kept in sync with
/// `src/tui/mod.rs` line ~976.
const TAB_NAMES: &[&str] = &[
    "Flows",
    "Connections",
    "Endpoints",
    "Policies",
    "Metrics",
    "Healer",
    "AutoPolicy",
    "RootCause",
    "Simulator",
    "Replay",
    "Chaos",
    "Canary",
    "MultiCluster",
];

/// Expected number of tabs in the TUI.
const EXPECTED_TAB_COUNT: usize = 13;

/// Simulated TabIndex for testing the usize-to-tab conversion logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TabIndex {
    Flows,
    Connections,
    Endpoints,
    Policies,
    Metrics,
    Healer,
    AutoPolicy,
    RootCause,
    Simulator,
    Replay,
    Chaos,
    Canary,
    MultiCluster,
}

impl TabIndex {
    /// Convert from a raw `usize` (selected_tab) to a TabIndex.
    /// Returns None if out of range.
    fn from_usize(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(TabIndex::Flows),
            1 => Some(TabIndex::Connections),
            2 => Some(TabIndex::Endpoints),
            3 => Some(TabIndex::Policies),
            4 => Some(TabIndex::Metrics),
            5 => Some(TabIndex::Healer),
            6 => Some(TabIndex::AutoPolicy),
            7 => Some(TabIndex::RootCause),
            8 => Some(TabIndex::Simulator),
            9 => Some(TabIndex::Replay),
            10 => Some(TabIndex::Chaos),
            11 => Some(TabIndex::Canary),
            12 => Some(TabIndex::MultiCluster),
            _ => None,
        }
    }

    /// Convert TabIndex to its display name (matching the TUI strings).
    fn display_name(&self) -> &'static str {
        match self {
            TabIndex::Flows => "Flows",
            TabIndex::Connections => "Connections",
            TabIndex::Endpoints => "Endpoints",
            TabIndex::Policies => "Policies",
            TabIndex::Metrics => "Metrics",
            TabIndex::Healer => "Healer",
            TabIndex::AutoPolicy => "AutoPolicy",
            TabIndex::RootCause => "RootCause",
            TabIndex::Simulator => "Simulator",
            TabIndex::Replay => "Replay",
            TabIndex::Chaos => "Chaos",
            TabIndex::Canary => "Canary",
            TabIndex::MultiCluster => "MultiCluster",
        }
    }

    /// Convert TabIndex to its integer index.
    fn to_usize(&self) -> usize {
        match self {
            TabIndex::Flows => 0,
            TabIndex::Connections => 1,
            TabIndex::Endpoints => 2,
            TabIndex::Policies => 3,
            TabIndex::Metrics => 4,
            TabIndex::Healer => 5,
            TabIndex::AutoPolicy => 6,
            TabIndex::RootCause => 7,
            TabIndex::Simulator => 8,
            TabIndex::Replay => 9,
            TabIndex::Chaos => 10,
            TabIndex::Canary => 11,
            TabIndex::MultiCluster => 12,
        }
    }
}

impl std::fmt::Display for TabIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ---------------------------------------------------------------------------
// Tests: tab names are all non-empty
// ---------------------------------------------------------------------------

#[test]
fn test_all_tab_names_are_nonempty() {
    for (i, name) in TAB_NAMES.iter().enumerate() {
        assert!(
            !name.is_empty(),
            "Tab name at index {} must not be empty",
            i
        );
    }
}

#[test]
fn test_all_tab_names_have_no_leading_trailing_whitespace() {
    for (i, name) in TAB_NAMES.iter().enumerate() {
        assert_eq!(
            *name,
            name.trim(),
            "Tab name at index {} ('{}') should not have leading/trailing whitespace",
            i,
            name
        );
    }
}

// ---------------------------------------------------------------------------
// Tests: tab count matches expected
// ---------------------------------------------------------------------------

#[test]
fn test_tab_count_matches_expected() {
    assert_eq!(
        TAB_NAMES.len(),
        EXPECTED_TAB_COUNT,
        "There should be exactly {} tabs",
        EXPECTED_TAB_COUNT
    );
}

#[test]
fn test_tab_names_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for name in TAB_NAMES {
        assert!(seen.insert(name), "Duplicate tab name found: '{}'", name);
    }
}

// ---------------------------------------------------------------------------
// Tests: TabIndex conversion from usize
// ---------------------------------------------------------------------------

#[test]
fn test_tab_index_from_usize_valid_range() {
    for i in 0..EXPECTED_TAB_COUNT {
        let tab = TabIndex::from_usize(i);
        assert!(
            tab.is_some(),
            "TabIndex::from_usize({}) should return Some",
            i
        );
    }
}

#[test]
fn test_tab_index_from_usize_out_of_range() {
    assert_eq!(TabIndex::from_usize(EXPECTED_TAB_COUNT), None);
    assert_eq!(TabIndex::from_usize(100), None);
    assert_eq!(TabIndex::from_usize(usize::MAX), None);
}

#[test]
fn test_tab_index_roundtrip() {
    // Converting to usize and back should yield the same TabIndex
    let all_tabs = vec![
        TabIndex::Flows,
        TabIndex::Connections,
        TabIndex::Endpoints,
        TabIndex::Policies,
        TabIndex::Metrics,
        TabIndex::Healer,
        TabIndex::AutoPolicy,
        TabIndex::RootCause,
        TabIndex::Simulator,
        TabIndex::Replay,
        TabIndex::Chaos,
        TabIndex::Canary,
        TabIndex::MultiCluster,
    ];

    for tab in &all_tabs {
        let idx = tab.to_usize();
        let roundtripped = TabIndex::from_usize(idx).unwrap();
        assert_eq!(
            *tab, roundtripped,
            "TabIndex::{:?} should roundtrip through usize {}",
            tab, idx
        );
    }
}

#[test]
fn test_tab_index_covers_all_names() {
    // Every TAB_NAMES entry should have a corresponding TabIndex
    for (i, name) in TAB_NAMES.iter().enumerate() {
        let tab = TabIndex::from_usize(i).unwrap();
        assert_eq!(
            tab.display_name(),
            *name,
            "TabIndex at {} should have name '{}', got '{}'",
            i,
            name,
            tab.display_name()
        );
    }
}

// ---------------------------------------------------------------------------
// Tests: TabIndex Display
// ---------------------------------------------------------------------------

#[test]
fn test_tab_index_display() {
    assert_eq!(format!("{}", TabIndex::Flows), "Flows");
    assert_eq!(format!("{}", TabIndex::Connections), "Connections");
    assert_eq!(format!("{}", TabIndex::MultiCluster), "MultiCluster");
}

#[test]
fn test_tab_index_display_matches_name_array() {
    let all_tabs = vec![
        TabIndex::Flows,
        TabIndex::Connections,
        TabIndex::Endpoints,
        TabIndex::Policies,
        TabIndex::Metrics,
        TabIndex::Healer,
        TabIndex::AutoPolicy,
        TabIndex::RootCause,
        TabIndex::Simulator,
        TabIndex::Replay,
        TabIndex::Chaos,
        TabIndex::Canary,
        TabIndex::MultiCluster,
    ];

    for (tab, name) in all_tabs.iter().zip(TAB_NAMES.iter()) {
        assert_eq!(
            format!("{}", tab),
            *name,
            "Display of {:?} should match TAB_NAMES entry",
            tab
        );
    }
}

// ---------------------------------------------------------------------------
// Tests: Tab navigation wrapping logic (as used in the TUI)
// ---------------------------------------------------------------------------

/// Simulate the TUI's right-arrow tab navigation: `(selected + 1) % count`
fn navigate_right(current: usize, count: usize) -> usize {
    (current + 1) % count
}

/// Simulate the TUI's left-arrow tab navigation
fn navigate_left(current: usize, count: usize) -> usize {
    if current == 0 {
        count - 1
    } else {
        current - 1
    }
}

#[test]
fn test_tab_navigation_right_wraps() {
    let last = EXPECTED_TAB_COUNT - 1;
    assert_eq!(
        navigate_right(last, EXPECTED_TAB_COUNT),
        0,
        "Right from last tab should wrap to first"
    );
}

#[test]
fn test_tab_navigation_left_wraps() {
    assert_eq!(
        navigate_left(0, EXPECTED_TAB_COUNT),
        EXPECTED_TAB_COUNT - 1,
        "Left from first tab should wrap to last"
    );
}

#[test]
fn test_tab_navigation_right_increments() {
    for i in 0..(EXPECTED_TAB_COUNT - 1) {
        assert_eq!(
            navigate_right(i, EXPECTED_TAB_COUNT),
            i + 1,
            "Right from tab {} should go to {}",
            i,
            i + 1
        );
    }
}

#[test]
fn test_tab_navigation_left_decrements() {
    for i in 1..EXPECTED_TAB_COUNT {
        assert_eq!(
            navigate_left(i, EXPECTED_TAB_COUNT),
            i - 1,
            "Left from tab {} should go to {}",
            i,
            i - 1
        );
    }
}

#[test]
fn test_full_right_cycle_returns_to_start() {
    let mut pos = 0;
    for _ in 0..EXPECTED_TAB_COUNT {
        pos = navigate_right(pos, EXPECTED_TAB_COUNT);
    }
    assert_eq!(
        pos, 0,
        "Cycling right through all tabs should return to start"
    );
}

#[test]
fn test_full_left_cycle_returns_to_start() {
    let mut pos = 0;
    for _ in 0..EXPECTED_TAB_COUNT {
        pos = navigate_left(pos, EXPECTED_TAB_COUNT);
    }
    assert_eq!(
        pos, 0,
        "Cycling left through all tabs should return to start"
    );
}

// ---------------------------------------------------------------------------
// Tests: specific tab positions match expected module indices
// ---------------------------------------------------------------------------

#[test]
fn test_known_tab_positions() {
    // These match the indices used in src/tui/mod.rs for key handling
    assert_eq!(TAB_NAMES[0], "Flows"); // tab 0: flow list, packet explainer
    assert_eq!(TAB_NAMES[5], "Healer"); // tab 5: self-healer
    assert_eq!(TAB_NAMES[6], "AutoPolicy"); // tab 6: policy learning
    assert_eq!(TAB_NAMES[7], "RootCause"); // tab 7: root-cause analysis
    assert_eq!(TAB_NAMES[8], "Simulator"); // tab 8: what-if simulator
    assert_eq!(TAB_NAMES[9], "Replay"); // tab 9: traffic replay
    assert_eq!(TAB_NAMES[10], "Chaos"); // tab 10: chaos engineering
    assert_eq!(TAB_NAMES[11], "Canary"); // tab 11: canary deployments
    assert_eq!(TAB_NAMES[12], "MultiCluster"); // tab 12: multi-cluster
}
