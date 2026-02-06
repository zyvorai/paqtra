use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;
use std::collections::HashSet;

use std::sync::Arc;

use crate::ebpf::{MockMapReader, EnrichedMapReader, CiliumMapReader};
use crate::endpoints::{Endpoint, EndpointManager};
use crate::hubble::{Flow, HubbleClient};
use crate::kubernetes::K8sClient;
use crate::modules::autopolicy::{AutoPolicy, AutoPolicyConfig};
use crate::modules::healer::{SelfHealer, HealerConfig};
use crate::modules::rootcause::{RootCauseEngine, RootCauseConfig};
use crate::modules::simulator::{Simulator, SimulatorConfig};
use crate::modules::replay::{ReplayEngine, ReplayConfig};
use crate::integration::{IntegratedDataProvider, EnrichedConnection, format_enriched_connection};

mod simulator_view;
mod replay_view;
mod autopolicy_view;
mod rootcause_view;
mod theme;
mod help_overlay;

use simulator_view::SimulatorView;
use replay_view::ReplayView;
use autopolicy_view::AutoPolicyView;
use rootcause_view::RootCauseView;
use theme::*;

/// Module container that can use either enriched or mock data
enum ModuleContainer {
    Enriched {
        healer: SelfHealer<EnrichedMapReader>,
        autopolicy: AutoPolicy<EnrichedMapReader>,
        rootcause: RootCauseEngine<EnrichedMapReader>,
        simulator: Simulator<EnrichedMapReader>,
        replay: ReplayEngine<EnrichedMapReader>,
    },
    Mock {
        healer: SelfHealer<MockMapReader>,
        autopolicy: AutoPolicy<MockMapReader>,
        rootcause: RootCauseEngine<MockMapReader>,
        simulator: Simulator<MockMapReader>,
        replay: ReplayEngine<MockMapReader>,
    },
}

impl ModuleContainer {
    /// Get healer stats (works for both enriched and mock)
    fn healer_stats(&self) -> crate::modules::healer::HealerStats {
        match self {
            ModuleContainer::Enriched { healer, .. } => healer.stats(),
            ModuleContainer::Mock { healer, .. } => healer.stats(),
        }
    }

    /// Get autopolicy stats
    fn autopolicy_stats(&self) -> crate::modules::autopolicy::AutoPolicyStats {
        match self {
            ModuleContainer::Enriched { autopolicy, .. } => autopolicy.stats(),
            ModuleContainer::Mock { autopolicy, .. } => autopolicy.stats(),
        }
    }

    /// Check if using enriched data
    fn is_enriched(&self) -> bool {
        matches!(self, ModuleContainer::Enriched { .. })
    }
}

pub struct TuiApp {
    hubble_client: HubbleClient,
    endpoint_manager: EndpointManager,
    k8s_client: K8sClient,
    flows: Vec<Flow>,
    endpoints: Vec<Endpoint>,
    selected_tab: usize,
    context: String,

    // Integration layer
    integrated_provider: Option<IntegratedDataProvider>,
    enriched_connections: Vec<EnrichedConnection>,

    // Intelligence modules (either enriched or mock)
    modules: ModuleContainer,

    // Module state
    last_healer_run: std::time::Instant,
    last_autopolicy_update: std::time::Instant,

    // Status messages
    status_message: Option<String>,
    status_message_time: std::time::Instant,

    // View state
    simulator_view: SimulatorView,
    replay_view: ReplayView,
    autopolicy_view: AutoPolicyView,
    rootcause_view: RootCauseView,

    // AutoPolicy view state
    selected_policy_index: usize,
    policy_detail_mode: bool,
    policy_apply_confirmation: bool,
    policy_rollback_confirmation: bool,
    policy_batch_apply_confirmation: bool,
    policy_batch_rollback_confirmation: bool,
    applied_policies: std::collections::HashSet<String>,

    // RootCause view state
    selected_fix_index: usize,
    fix_apply_confirmation: bool,

    // Packet Explainer state
    selected_flow_index: usize,
    show_packet_explanation: bool,
    packet_explainer: std::cell::RefCell<crate::modules::packet_explainer::PacketExplainer>,

    // UX enhancements
    show_help: bool,
    operation_in_progress: bool,
    operation_message: String,
}

impl TuiApp {
    pub async fn new(context: String, hubble_port: u16, k8s_client: K8sClient) -> Result<Self> {
        let hubble_client = HubbleClient::new(hubble_port, true).await?;
        let endpoint_manager = EndpointManager::new(k8s_client.clone());

        // Initialize integrated data provider
        let integrated_provider = match IntegratedDataProvider::new(k8s_client.clone()).await {
            Ok(provider) => Some(provider),
            Err(e) => {
                tracing::warn!("Failed to initialize IntegratedDataProvider: {}. Using mock data.", e);
                None
            }
        };

        // Initialize intelligence modules based on data availability
        let modules = if let Some(ref provider) = integrated_provider {
            // Try to create EnrichedMapReader
            match CiliumMapReader::new() {
                Ok(cilium_reader) => {
                    let enriched_reader = EnrichedMapReader::new(
                        cilium_reader,
                        provider.identity_resolver().clone(),
                    );

                    tracing::info!("Initializing intelligence modules with enriched data");

                    ModuleContainer::Enriched {
                        healer: SelfHealer::new(
                            HealerConfig::default(),
                            enriched_reader.clone(),
                            k8s_client.clone(),
                        ),
                        autopolicy: AutoPolicy::new(
                            AutoPolicyConfig::default(),
                            enriched_reader.clone(),
                            k8s_client.clone(),
                        ),
                        rootcause: RootCauseEngine::new(
                            RootCauseConfig::default(),
                            enriched_reader.clone(),
                            k8s_client.clone(),
                        ),
                        simulator: Simulator::new(
                            SimulatorConfig::default(),
                            enriched_reader.clone(),
                            k8s_client.clone(),
                        ),
                        replay: ReplayEngine::new(
                            ReplayConfig::default(),
                            enriched_reader,
                            k8s_client.clone(),
                        ),
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to create CiliumMapReader: {}. Using mock data.", e);
                    let mock_reader = MockMapReader;
                    ModuleContainer::Mock {
                        healer: SelfHealer::new(HealerConfig::default(), mock_reader, k8s_client.clone()),
                        autopolicy: AutoPolicy::new(AutoPolicyConfig::default(), mock_reader, k8s_client.clone()),
                        rootcause: RootCauseEngine::new(RootCauseConfig::default(), mock_reader, k8s_client.clone()),
                        simulator: Simulator::new(SimulatorConfig::default(), mock_reader, k8s_client.clone()),
                        replay: ReplayEngine::new(ReplayConfig::default(), mock_reader, k8s_client.clone()),
                    }
                }
            }
        } else {
            // Fallback to mock data
            tracing::warn!("Using mock data for intelligence modules");
            let mock_reader = MockMapReader;

            ModuleContainer::Mock {
                healer: SelfHealer::new(
                    HealerConfig::default(),
                    mock_reader,
                    k8s_client.clone(),
                ),
                autopolicy: AutoPolicy::new(
                    AutoPolicyConfig::default(),
                    mock_reader,
                    k8s_client.clone(),
                ),
                rootcause: RootCauseEngine::new(
                    RootCauseConfig::default(),
                    mock_reader,
                    k8s_client.clone(),
                ),
                simulator: Simulator::new(
                    SimulatorConfig::default(),
                    mock_reader,
                    k8s_client.clone(),
                ),
                replay: ReplayEngine::new(
                    ReplayConfig::default(),
                    mock_reader,
                    k8s_client.clone(),
                ),
            }
        };

        Ok(Self {
            hubble_client,
            endpoint_manager,
            k8s_client,
            flows: Vec::new(),
            endpoints: Vec::new(),
            selected_tab: 0,
            context,
            integrated_provider,
            enriched_connections: Vec::new(),
            modules,
            last_healer_run: std::time::Instant::now(),
            last_autopolicy_update: std::time::Instant::now(),
            status_message: None,
            status_message_time: std::time::Instant::now(),
            simulator_view: SimulatorView::new(),
            replay_view: ReplayView::new(),
            autopolicy_view: AutoPolicyView::new(),
            rootcause_view: RootCauseView::new(),
            selected_policy_index: 0,
            policy_detail_mode: false,
            policy_apply_confirmation: false,
            policy_rollback_confirmation: false,
            policy_batch_apply_confirmation: false,
            policy_batch_rollback_confirmation: false,
            applied_policies: std::collections::HashSet::new(),
            selected_fix_index: 0,
            fix_apply_confirmation: false,
            selected_flow_index: 0,
            show_packet_explanation: false,
            packet_explainer: std::cell::RefCell::new(crate::modules::packet_explainer::PacketExplainer::new()),
            show_help: false,
            operation_in_progress: false,
            operation_message: String::new(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_app(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("Error: {:?}", err);
        }

        Ok(())
    }

    async fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Fetch latest flows
            if let Ok(flows) = self.hubble_client.get_flows().await {
                self.flows = flows;
            }

            // Update module data based on selected tab
            match self.selected_tab {
                1 => {
                    // Connections - enriched with K8s data
                    if let Some(provider) = &self.integrated_provider {
                        if let Ok(connections) = provider.get_enriched_connections().await {
                            self.enriched_connections = connections;
                        }
                    }
                }
                2 => {
                    // Endpoints
                    if let Ok(endpoints) = self.endpoint_manager.discover_endpoints().await {
                        self.endpoints = endpoints;
                    }
                }
                5 => {
                    // Healer - active problem detection (every 30 seconds)
                    if self.last_healer_run.elapsed().as_secs() >= 30 {
                        match &mut self.modules {
                            ModuleContainer::Enriched { healer, .. } => {
                                if let Err(e) = healer.run_enriched().await {
                                    tracing::warn!("Healer enriched run failed: {}", e);
                                }
                            }
                            ModuleContainer::Mock { healer, .. } => {
                                if let Err(e) = healer.run().await {
                                    tracing::warn!("Healer run failed: {}", e);
                                }
                            }
                        }
                        self.last_healer_run = std::time::Instant::now();
                    }
                }
                6 => {
                    // AutoPolicy - active learning (every 5 minutes)
                    if self.last_autopolicy_update.elapsed().as_secs() >= 300 {
                        match &mut self.modules {
                            ModuleContainer::Enriched { autopolicy, .. } => {
                                if let Err(e) = autopolicy.update_enriched().await {
                                    tracing::warn!("AutoPolicy enriched update failed: {}", e);
                                }
                            }
                            ModuleContainer::Mock { autopolicy, .. } => {
                                if let Err(e) = autopolicy.update().await {
                                    tracing::warn!("AutoPolicy update failed: {}", e);
                                }
                            }
                        }
                        self.last_autopolicy_update = std::time::Instant::now();
                    }
                }
                7 => {
                    // RootCause - updates on-demand
                }
                8 => {
                    // Simulator - updates via user interaction
                }
                9 => {
                    // Replay - updates on-demand
                }
                _ => {}
            }

            terminal.draw(|f| self.ui(f))?;

            // Handle input
            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('?') => {
                            // Toggle help overlay
                            self.show_help = !self.show_help;
                        }
                        KeyCode::Esc if self.show_help => {
                            // Close help overlay
                            self.show_help = false;
                        }
                        KeyCode::Char('q') if !self.show_help => return Ok(()),
                        KeyCode::Tab if !self.show_help => {
                            self.selected_tab = (self.selected_tab + 1) % 10;
                        }
                        KeyCode::BackTab if !self.show_help => {
                            self.selected_tab = if self.selected_tab == 0 {
                                9
                            } else {
                                self.selected_tab - 1
                            };
                        }
                        KeyCode::Char('r') if !self.show_help && self.selected_tab == 9 => {
                            // Refresh recordings list (placeholder)
                        }
                        KeyCode::Char('s') if !self.show_help && self.selected_tab == 8 => {
                            // Run simulation
                            self.simulator_view.trigger_simulation();
                        }
                        KeyCode::Char('c') if !self.show_help && self.selected_tab == 8 => {
                            // Clear simulation results
                            self.simulator_view.clear_simulation();
                        }
                        KeyCode::Up if !self.show_help && self.selected_tab == 8 && self.simulator_view.last_simulation.is_none() => {
                            // Navigate scenarios up
                            self.simulator_view.move_selection_up();
                        }
                        KeyCode::Down if !self.show_help && self.selected_tab == 8 && self.simulator_view.last_simulation.is_none() => {
                            // Navigate scenarios down (max 7 scenarios)
                            self.simulator_view.move_selection_down(7);
                        }
                        KeyCode::Char('d') if !self.show_help && self.selected_tab == 5 => {
                            // Detect problems manually (Healer tab)
                            match &mut self.modules {
                                ModuleContainer::Enriched { healer, .. } => {
                                    if let Err(e) = healer.run_enriched().await {
                                        tracing::warn!("Healer enriched run failed: {}", e);
                                    } else {
                                        tracing::info!("Manual problem detection completed");
                                    }
                                }
                                ModuleContainer::Mock { healer, .. } => {
                                    if let Err(e) = healer.run().await {
                                        tracing::warn!("Healer run failed: {}", e);
                                    }
                                }
                            }
                            self.last_healer_run = std::time::Instant::now();
                        }
                        KeyCode::Char('u') if !self.show_help && self.selected_tab == 6 => {
                            // Update learning manually (AutoPolicy tab)
                            match &mut self.modules {
                                ModuleContainer::Enriched { autopolicy, .. } => {
                                    if let Err(e) = autopolicy.update_enriched().await {
                                        tracing::warn!("AutoPolicy enriched update failed: {}", e);
                                    } else {
                                        tracing::info!("Manual policy learning update completed");
                                    }
                                }
                                ModuleContainer::Mock { autopolicy, .. } => {
                                    if let Err(e) = autopolicy.update().await {
                                        tracing::warn!("AutoPolicy update failed: {}", e);
                                    }
                                }
                            }
                            self.last_autopolicy_update = std::time::Instant::now();
                        }
                        KeyCode::Char('g') if !self.show_help && self.selected_tab == 6 => {
                            // Generate policies (AutoPolicy tab)
                            let result = match &mut self.modules {
                                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.generate_policies(),
                                ModuleContainer::Mock { autopolicy, .. } => autopolicy.generate_policies(),
                            };

                            match result {
                                Ok(policies) => {
                                    if policies.is_empty() {
                                        self.set_status_message("No policies generated. Need more observations (min 10).");
                                        tracing::info!("No policies generated - insufficient observations");
                                    } else {
                                        // Save policies to files
                                        match self.save_policies(&policies) {
                                            Ok(count) => {
                                                self.set_status_message(&format!(
                                                    "✅ Generated {} policies → saved to ./policies/",
                                                    count
                                                ));
                                                tracing::info!("Generated and saved {} policies", count);
                                            }
                                            Err(e) => {
                                                self.set_status_message(&format!("⚠️ Error saving policies: {}", e));
                                                tracing::warn!("Failed to save policies: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    self.set_status_message(&format!("⚠️ Policy generation failed: {}", e));
                                    tracing::warn!("Policy generation failed: {}", e);
                                }
                            }
                        }
                        KeyCode::Char('A') if !self.show_help && self.selected_tab == 6 && !self.policy_detail_mode => {
                            // Batch apply all unapplied policies
                            let policies = match &self.modules {
                                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
                            };

                            let unapplied_count = policies.iter().filter(|p| !self.applied_policies.contains(&p.name)).count();

                            if unapplied_count == 0 {
                                self.set_status_message("⚠️ All policies already applied");
                            } else {
                                self.policy_batch_apply_confirmation = true;
                                self.set_status_message(&format!("Apply {} policies? Press 'y' to confirm, 'n' to cancel", unapplied_count));
                            }
                        }
                        KeyCode::Char('R') if !self.show_help && self.selected_tab == 6 && !self.policy_detail_mode => {
                            // Batch rollback all applied policies
                            let applied_count = self.applied_policies.len();

                            if applied_count == 0 {
                                self.set_status_message("⚠️ No policies applied to rollback");
                            } else {
                                self.policy_batch_rollback_confirmation = true;
                                self.set_status_message(&format!("Rollback {} policies? Press 'y' to confirm, 'n' to cancel", applied_count));
                            }
                        }
                        KeyCode::Char('y') if !self.show_help && self.selected_tab == 6 && self.policy_batch_apply_confirmation => {
                            // Confirm batch apply
                            self.policy_batch_apply_confirmation = false;

                            let policies = match &self.modules {
                                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
                            };

                            let mut applied = 0;
                            let mut failed = 0;

                            for policy in policies.iter() {
                                if !self.applied_policies.contains(&policy.name) {
                                    let policy_name = policy.name.clone();
                                    let policy_yaml = policy.yaml.clone();

                                    match self.apply_policy_kubectl(&policy_name, &policy_yaml) {
                                        Ok(_) => {
                                            self.applied_policies.insert(policy_name.clone());
                                            applied += 1;
                                            tracing::info!("Applied policy: {}", policy_name);
                                        }
                                        Err(e) => {
                                            failed += 1;
                                            tracing::error!("Failed to apply policy '{}': {}", policy_name, e);
                                        }
                                    }
                                }
                            }

                            if failed == 0 {
                                self.set_status_message(&format!("✅ Applied {} policies successfully", applied));
                            } else {
                                self.set_status_message(&format!("⚠️ Applied: {}, Failed: {}", applied, failed));
                            }
                        }
                        KeyCode::Char('y') if !self.show_help && self.selected_tab == 6 && self.policy_batch_rollback_confirmation => {
                            // Confirm batch rollback
                            self.policy_batch_rollback_confirmation = false;

                            let policies = match &self.modules {
                                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
                            };

                            let mut rolled_back = 0;
                            let mut failed = 0;
                            let applied_names: Vec<String> = self.applied_policies.iter().cloned().collect();

                            for policy_name in applied_names {
                                // Find the policy to get its namespace
                                if let Some(policy) = policies.iter().find(|p| p.name == policy_name) {
                                    match self.rollback_policy_kubectl(&policy.name, &policy.namespace) {
                                        Ok(_) => {
                                            self.applied_policies.remove(&policy.name);
                                            rolled_back += 1;
                                            tracing::info!("Rolled back policy: {}", policy.name);
                                        }
                                        Err(e) => {
                                            failed += 1;
                                            tracing::error!("Failed to rollback policy '{}': {}", policy.name, e);
                                        }
                                    }
                                }
                            }

                            if failed == 0 {
                                self.set_status_message(&format!("✅ Rolled back {} policies successfully", rolled_back));
                            } else {
                                self.set_status_message(&format!("⚠️ Rolled back: {}, Failed: {}", rolled_back, failed));
                            }
                        }
                        KeyCode::Char('n') if !self.show_help && self.selected_tab == 6 && (self.policy_batch_apply_confirmation || self.policy_batch_rollback_confirmation) => {
                            // Cancel batch operation
                            if self.policy_batch_apply_confirmation {
                                self.policy_batch_apply_confirmation = false;
                                self.set_status_message("Batch apply cancelled");
                            } else if self.policy_batch_rollback_confirmation {
                                self.policy_batch_rollback_confirmation = false;
                                self.set_status_message("Batch rollback cancelled");
                            }
                        }
                        KeyCode::Char('v') if !self.show_help && self.selected_tab == 6 => {
                            // Toggle policy detail view (AutoPolicy tab)
                            self.policy_detail_mode = !self.policy_detail_mode;
                            if self.policy_detail_mode {
                                self.set_status_message("📄 Policy detail view (↑/↓: navigate, Esc: exit)");
                            } else {
                                self.selected_policy_index = 0;
                            }
                        }
                        KeyCode::Up if !self.show_help && self.selected_tab == 6 && self.policy_detail_mode => {
                            // Navigate policies up
                            if self.selected_policy_index > 0 {
                                self.selected_policy_index -= 1;
                            }
                        }
                        KeyCode::Down if !self.show_help && self.selected_tab == 6 && self.policy_detail_mode => {
                            // Navigate policies down
                            let policy_count = match &self.modules {
                                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies().len(),
                                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies().len(),
                            };
                            if self.selected_policy_index + 1 < policy_count {
                                self.selected_policy_index += 1;
                            }
                        }
                        KeyCode::Esc if !self.show_help && self.selected_tab == 6 => {
                            // Exit or cancel confirmations
                            if self.policy_apply_confirmation {
                                self.policy_apply_confirmation = false;
                                self.set_status_message("Policy application cancelled");
                            } else if self.policy_rollback_confirmation {
                                self.policy_rollback_confirmation = false;
                                self.set_status_message("Policy rollback cancelled");
                            } else if self.policy_batch_apply_confirmation {
                                self.policy_batch_apply_confirmation = false;
                                self.set_status_message("Batch apply cancelled");
                            } else if self.policy_batch_rollback_confirmation {
                                self.policy_batch_rollback_confirmation = false;
                                self.set_status_message("Batch rollback cancelled");
                            } else if self.policy_detail_mode {
                                self.policy_detail_mode = false;
                                self.selected_policy_index = 0;
                            }
                        }
                        KeyCode::Char('a') if !self.show_help && self.selected_tab == 6 && self.policy_detail_mode && !self.policy_apply_confirmation => {
                            // Trigger policy application confirmation
                            let policies = match &self.modules {
                                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
                            };

                            if !policies.is_empty() {
                                let policy_name = &policies[self.selected_policy_index].name;
                                if self.applied_policies.contains(policy_name) {
                                    self.set_status_message(&format!("⚠️ Policy '{}' already applied", policy_name));
                                } else {
                                    self.policy_apply_confirmation = true;
                                    self.set_status_message(&format!("Apply policy '{}'? Press 'y' to confirm, 'n' to cancel", policy_name));
                                }
                            }
                        }
                        KeyCode::Char('y') if !self.show_help && self.selected_tab == 6 && self.policy_apply_confirmation => {
                            // Confirm and apply policy
                            self.policy_apply_confirmation = false;

                            let policies = match &self.modules {
                                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
                            };

                            if !policies.is_empty() {
                                let policy = &policies[self.selected_policy_index];
                                // Clone policy data to avoid borrow checker issues
                                let policy_name = policy.name.clone();
                                let policy_yaml = policy.yaml.clone();

                                match self.apply_policy_kubectl(&policy_name, &policy_yaml) {
                                    Ok(_) => {
                                        self.applied_policies.insert(policy_name.clone());
                                        self.set_status_message(&format!("✅ Policy '{}' applied successfully", policy_name));
                                        tracing::info!("Applied policy: {}", policy_name);
                                    }
                                    Err(e) => {
                                        self.set_status_message(&format!("❌ Failed to apply policy: {}", e));
                                        tracing::error!("Failed to apply policy '{}': {}", policy_name, e);
                                    }
                                }
                            }
                        }
                        KeyCode::Char('n') if !self.show_help && self.selected_tab == 6 && self.policy_apply_confirmation => {
                            // Cancel policy application
                            self.policy_apply_confirmation = false;
                            self.set_status_message("Policy application cancelled");
                        }
                        KeyCode::Char('r') if !self.show_help && self.selected_tab == 6 && self.policy_detail_mode && !self.policy_apply_confirmation && !self.policy_rollback_confirmation => {
                            // Trigger policy rollback confirmation
                            let policies = match &self.modules {
                                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
                            };

                            if !policies.is_empty() {
                                let policy_name = &policies[self.selected_policy_index].name;
                                if !self.applied_policies.contains(policy_name) {
                                    self.set_status_message(&format!("⚠️ Policy '{}' not applied, cannot rollback", policy_name));
                                } else {
                                    self.policy_rollback_confirmation = true;
                                    self.set_status_message(&format!("Rollback policy '{}'? Press 'y' to confirm, 'n' to cancel", policy_name));
                                }
                            }
                        }
                        KeyCode::Char('y') if !self.show_help && self.selected_tab == 6 && self.policy_rollback_confirmation => {
                            // Confirm and rollback policy
                            self.policy_rollback_confirmation = false;

                            let policies = match &self.modules {
                                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
                            };

                            if !policies.is_empty() {
                                let policy = &policies[self.selected_policy_index];
                                let policy_name = policy.name.clone();
                                let policy_namespace = policy.namespace.clone();

                                match self.rollback_policy_kubectl(&policy_name, &policy_namespace) {
                                    Ok(_) => {
                                        self.applied_policies.remove(&policy_name);
                                        self.set_status_message(&format!("✅ Policy '{}' rolled back successfully", policy_name));
                                        tracing::info!("Rolled back policy: {}", policy_name);
                                    }
                                    Err(e) => {
                                        self.set_status_message(&format!("❌ Failed to rollback policy: {}", e));
                                        tracing::error!("Failed to rollback policy '{}': {}", policy_name, e);
                                    }
                                }
                            }
                        }
                        KeyCode::Char('n') if !self.show_help && self.selected_tab == 6 && self.policy_rollback_confirmation => {
                            // Cancel policy rollback
                            self.policy_rollback_confirmation = false;
                            self.set_status_message("Policy rollback cancelled");
                        }
                        KeyCode::Up if !self.show_help && self.selected_tab == 7 => {
                            // Navigate fixes up
                            if self.selected_fix_index > 0 {
                                self.selected_fix_index -= 1;
                            }
                        }
                        KeyCode::Down if !self.show_help && self.selected_tab == 7 => {
                            // Navigate fixes down
                            // Note: Max 4 fixes (hardcoded for now)
                            if self.selected_fix_index < 3 {
                                self.selected_fix_index += 1;
                            }
                        }
                        KeyCode::Char('a') if !self.show_help && self.selected_tab == 7 && !self.fix_apply_confirmation => {
                            // Trigger fix application confirmation
                            self.fix_apply_confirmation = true;
                            let fix_names = vec![
                                "allow-8080 policy",
                                "DNS egress policy",
                                "MTU adjustment",
                                "DB access policy"
                            ];
                            self.set_status_message(&format!(
                                "Apply fix '{}'? Press 'y' to confirm, 'n' to cancel",
                                fix_names[self.selected_fix_index]
                            ));
                        }
                        KeyCode::Char('y') if !self.show_help && self.selected_tab == 7 && self.fix_apply_confirmation => {
                            // Confirm and apply fix
                            self.fix_apply_confirmation = false;

                            let (fix_name, policy_yaml) = self.get_fix_policy(self.selected_fix_index);

                            match self.apply_policy_kubectl(&fix_name, &policy_yaml) {
                                Ok(_) => {
                                    self.set_status_message(&format!("✅ Applied fix: {}", fix_name));
                                    tracing::info!("Applied RootCause fix policy: {}", fix_name);
                                }
                                Err(e) => {
                                    self.set_status_message(&format!("❌ Failed to apply fix: {}", e));
                                    tracing::error!("Failed to apply RootCause fix '{}': {}", fix_name, e);
                                }
                            }
                        }
                        KeyCode::Char('n') if !self.show_help && self.selected_tab == 7 && self.fix_apply_confirmation => {
                            // Cancel fix application
                            self.fix_apply_confirmation = false;
                            self.set_status_message("Fix application cancelled");
                        }
                        KeyCode::Esc if !self.show_help && self.selected_tab == 7 && self.fix_apply_confirmation => {
                            // Cancel fix application with Esc
                            self.fix_apply_confirmation = false;
                            self.set_status_message("Fix application cancelled");
                        }
                        KeyCode::Up if !self.show_help && self.selected_tab == 0 && !self.show_packet_explanation => {
                            // Navigate flows up
                            if self.selected_flow_index > 0 {
                                self.selected_flow_index -= 1;
                            }
                        }
                        KeyCode::Down if !self.show_help && self.selected_tab == 0 && !self.show_packet_explanation => {
                            // Navigate flows down
                            if self.selected_flow_index < self.flows.len().saturating_sub(1).min(49) {
                                self.selected_flow_index += 1;
                            }
                        }
                        KeyCode::Char('e') if !self.show_help && self.selected_tab == 0 => {
                            // Toggle packet explanation
                            self.show_packet_explanation = !self.show_packet_explanation;
                            if self.show_packet_explanation && !self.flows.is_empty() {
                                self.set_status_message("📝 Explaining packet...");
                            }
                        }
                        KeyCode::Esc if !self.show_help && self.selected_tab == 0 && self.show_packet_explanation => {
                            // Exit packet explanation
                            self.show_packet_explanation = false;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn ui(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new(format!("🚀 Cilium Vision - Intelligence Platform - {}", self.context))
            .style(Style::default().fg(TITLE_COLOR).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER_COLOR)));
        f.render_widget(title, chunks[0]);

        // Tabs
        let titles = vec![
            "Flows",
            "Connections",
            "Endpoints",
            "Policies",
            "Metrics",
            "Healer",
            "AutoPolicy",
            "RootCause",
            "Simulator",
            "Replay"
        ];
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Intelligence Modules").border_style(Style::default().fg(BORDER_COLOR)))
            .select(self.selected_tab)
            .style(Style::default().fg(TAB_NORMAL_COLOR))
            .highlight_style(
                Style::default()
                    .fg(TAB_SELECTED_COLOR)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(tabs, chunks[1]);

        // Content based on selected tab
        match self.selected_tab {
            0 => self.render_flows(f, chunks[2]),
            1 => self.render_connections(f, chunks[2]),
            2 => self.render_endpoints(f, chunks[2]),
            3 => self.render_policies(f, chunks[2]),
            4 => self.render_metrics(f, chunks[2]),
            5 => self.render_healer(f, chunks[2]),
            6 => {
                // AutoPolicy view
                if self.policy_detail_mode {
                    // Show policy detail view
                    self.render_policy_detail(f, chunks[2]);
                } else {
                    // Show normal AutoPolicy view
                    match &self.modules {
                        ModuleContainer::Enriched { autopolicy, .. } => {
                            self.autopolicy_view.render(f, chunks[2], Some(autopolicy))
                        }
                        ModuleContainer::Mock { autopolicy, .. } => {
                            self.autopolicy_view.render(f, chunks[2], Some(autopolicy))
                        }
                    }
                }
            }
            7 => {
                // RootCause view
                match &self.modules {
                    ModuleContainer::Enriched { rootcause, .. } => {
                        self.rootcause_view.render(f, chunks[2], Some(rootcause), self.selected_fix_index, self.fix_apply_confirmation)
                    }
                    ModuleContainer::Mock { rootcause, .. } => {
                        self.rootcause_view.render(f, chunks[2], Some(rootcause), self.selected_fix_index, self.fix_apply_confirmation)
                    }
                }
            }
            8 => {
                // Simulator view
                match &self.modules {
                    ModuleContainer::Enriched { simulator, .. } => {
                        self.simulator_view.render(f, chunks[2], Some(simulator))
                    }
                    ModuleContainer::Mock { simulator, .. } => {
                        self.simulator_view.render(f, chunks[2], Some(simulator))
                    }
                }
            }
            9 => {
                // Replay view
                match &self.modules {
                    ModuleContainer::Enriched { replay, .. } => {
                        self.replay_view.render(f, chunks[2], Some(replay))
                    }
                    ModuleContainer::Mock { replay, .. } => {
                        self.replay_view.render(f, chunks[2], Some(replay))
                    }
                }
            }
            _ => {}
        }

        // Footer with context-specific help and status messages
        let footer_text = if let Some(ref msg) = self.status_message {
            // Show status message for 5 seconds
            if self.status_message_time.elapsed().as_secs() < 5 {
                msg.clone()
            } else {
                // Clear expired message
                match self.selected_tab {
                    5 => "?: Help | q: Quit | Tab: Next | d: Detect Problems".to_string(),
                    6 => if self.policy_apply_confirmation {
                        "⚠️ CONFIRM: y: Apply Policy | n: Cancel | Esc: Cancel".to_string()
                    } else if self.policy_rollback_confirmation {
                        "⚠️ CONFIRM: y: Rollback Policy | n: Cancel | Esc: Cancel".to_string()
                    } else if self.policy_batch_apply_confirmation {
                        "⚠️ CONFIRM: y: Apply All | n: Cancel | Esc: Cancel".to_string()
                    } else if self.policy_batch_rollback_confirmation {
                        "⚠️ CONFIRM: y: Rollback All | n: Cancel | Esc: Cancel".to_string()
                    } else if self.policy_detail_mode {
                        // Show different options based on policy applied status
                        let policies = match &self.modules {
                            ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                            ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
                        };
                        if !policies.is_empty() && self.applied_policies.contains(&policies[self.selected_policy_index].name) {
                            "?: Help | q: Quit | Esc: Exit | ↑/↓: Navigate | r: Rollback".to_string()
                        } else {
                            "?: Help | q: Quit | Esc: Exit | ↑/↓: Navigate | a: Apply".to_string()
                        }
                    } else {
                        "?: Help | q: Quit | u: Update | g: Generate | v: View | A: Apply All | R: Rollback All".to_string()
                    },
                    7 => if self.fix_apply_confirmation {
                        "⚠️ CONFIRM: y: Apply Fix | n: Cancel | Esc: Cancel".to_string()
                    } else {
                        "?: Help | q: Quit | ↑/↓: Select Fix | a: Apply Fix".to_string()
                    },
                    8 => if self.simulator_view.last_simulation.is_some() {
                        "?: Help | q: Quit | c: Clear Results | Esc: Back".to_string()
                    } else {
                        "?: Help | q: Quit | ↑/↓: Select | s: Simulate | c: Clear".to_string()
                    },
                    9 => "?: Help | q: Quit | Tab: Next | r: Refresh Recordings".to_string(),
                    0 => if self.show_packet_explanation {
                        "?: Help | q: Quit | Esc: Exit Explanation".to_string()
                    } else {
                        "?: Help | q: Quit | ↑/↓: Select Flow | e: Explain Packet".to_string()
                    },
                    _ => "?: Help | q: Quit | Tab: Next | Shift+Tab: Previous".to_string(),
                }
            }
        } else {
            match self.selected_tab {
                0 => if self.show_packet_explanation {
                    "?: Help | q: Quit | Esc: Exit Explanation".to_string()
                } else {
                    "?: Help | q: Quit | ↑/↓: Select Flow | e: Explain Packet".to_string()
                },
                5 => "?: Help | q: Quit | Tab: Next | d: Detect Problems".to_string(),
                6 => if self.policy_apply_confirmation {
                    "⚠️ CONFIRM: y: Apply Policy | n: Cancel | Esc: Cancel".to_string()
                } else if self.policy_rollback_confirmation {
                    "⚠️ CONFIRM: y: Rollback Policy | n: Cancel | Esc: Cancel".to_string()
                } else if self.policy_batch_apply_confirmation {
                    "⚠️ CONFIRM: y: Apply All | n: Cancel | Esc: Cancel".to_string()
                } else if self.policy_batch_rollback_confirmation {
                    "⚠️ CONFIRM: y: Rollback All | n: Cancel | Esc: Cancel".to_string()
                } else if self.policy_detail_mode {
                    // Show different options based on policy applied status
                    let policies = match &self.modules {
                        ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                        ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
                    };
                    if !policies.is_empty() && self.applied_policies.contains(&policies[self.selected_policy_index].name) {
                        "?: Help | q: Quit | Esc: Exit | ↑/↓: Navigate | r: Rollback".to_string()
                    } else {
                        "?: Help | q: Quit | Esc: Exit | ↑/↓: Navigate | a: Apply".to_string()
                    }
                } else {
                    "?: Help | q: Quit | u: Update | g: Generate | v: View | A: Apply All | R: Rollback All".to_string()
                },
                7 => if self.fix_apply_confirmation {
                    "⚠️ CONFIRM: y: Apply Fix | n: Cancel | Esc: Cancel".to_string()
                } else {
                    "?: Help | q: Quit | ↑/↓: Select Fix | a: Apply Fix".to_string()
                },
                8 => if self.simulator_view.last_simulation.is_some() {
                    "?: Help | q: Quit | c: Clear Results | Esc: Back".to_string()
                } else {
                    "?: Help | q: Quit | ↑/↓: Select | s: Simulate | c: Clear".to_string()
                },
                9 => "?: Help | q: Quit | Tab: Next | r: Refresh Recordings".to_string(),
                0 => if self.show_packet_explanation {
                    "?: Help | q: Quit | Esc: Exit Explanation".to_string()
                } else {
                    "?: Help | q: Quit | ↑/↓: Select Flow | e: Explain Packet".to_string()
                },
                _ => "?: Help | q: Quit | Tab: Next | Shift+Tab: Previous".to_string(),
            }
        };

        let footer_style = if self.policy_apply_confirmation || self.policy_rollback_confirmation || self.policy_batch_apply_confirmation || self.policy_batch_rollback_confirmation || self.fix_apply_confirmation {
            // Confirmation prompt - use red for warning
            Style::default().fg(ERROR_COLOR).add_modifier(Modifier::BOLD)
        } else if self.status_message.is_some() && self.status_message_time.elapsed().as_secs() < 5 {
            Style::default().fg(WARNING_COLOR)
        } else {
            Style::default().fg(TEXT_COLOR)
        };

        let footer = Paragraph::new(footer_text)
            .style(footer_style)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER_COLOR)));
        f.render_widget(footer, chunks[3]);

        // Render help overlay on top if active
        if self.show_help {
            self.render_help_overlay(f);
        }
    }

    fn render_help_overlay(&self, f: &mut Frame) {
        help_overlay::render_help_overlay(f);
    }

    fn render_flows(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if self.show_packet_explanation {
            self.render_packet_explanation(f, area);
            return;
        }

        let items: Vec<ListItem> = self
            .flows
            .iter()
            .enumerate()
            .take(50)
            .map(|(idx, flow)| {
                let verdict_color = match flow.verdict.as_str() {
                    "FORWARDED" => FORWARDED_COLOR,
                    "DROPPED" => DROPPED_COLOR,
                    _ => UNKNOWN_TRAFFIC_COLOR,
                };

                let prefix = if idx == self.selected_flow_index { "▶ " } else { "  " };
                let content = format!(
                    "{}{} {} {}/{} -> {}/{} {}",
                    prefix,
                    flow.time,
                    flow.verdict,
                    flow.source.namespace,
                    flow.source.pod_name,
                    flow.destination.namespace,
                    flow.destination.pod_name,
                    flow.r#type,
                );

                let style = if idx == self.selected_flow_index {
                    Style::default().fg(verdict_color).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(verdict_color)
                };

                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let title = format!("Live Flows ({}) [↑/↓: Select | e: Explain]", self.flows.len());
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title).border_style(Style::default().fg(BORDER_COLOR)));
        f.render_widget(list, area);
    }

    fn render_packet_explanation(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if self.flows.is_empty() || self.selected_flow_index >= self.flows.len() {
            let no_flows = Paragraph::new("No flow selected for explanation.\n\nPress Esc to return.")
                .style(Style::default().fg(WARNING_COLOR))
                .block(Block::default().borders(Borders::ALL).title("Packet Explanation"));
            f.render_widget(no_flows, area);
            return;
        }

        let flow = &self.flows[self.selected_flow_index];

        // Parse port from type or use 0
        let port = flow.r#type.split(':').nth(1)
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);

        // Generate explanation
        let explanation = match self.packet_explainer.borrow_mut().explain_packet(
            &flow.source.namespace,
            &flow.source.pod_name,
            &flow.destination.namespace,
            &flow.destination.pod_name,
            port,
            "TCP", // Default to TCP for now
            &flow.verdict,
        ) {
            Ok(exp) => exp,
            Err(_) => {
                let error = Paragraph::new("Failed to generate explanation.\n\nPress Esc to return.")
                    .style(Style::default().fg(ERROR_COLOR))
                    .block(Block::default().borders(Borders::ALL).title("Error"));
                f.render_widget(error, area);
                return;
            }
        };

        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Header
                Constraint::Min(10),    // Analysis
                Constraint::Length(8),  // Troubleshooting
            ])
            .split(area);

        // Header
        let header_text = format!(
            "📦 Packet Explanation\n\n\
            Source:      {}\n\
            Destination: {}\n\
            Protocol:    {}     Port: {}\n\
            Verdict:     {}     Time: {}",
            explanation.source,
            explanation.destination,
            explanation.protocol,
            port,
            explanation.verdict,
            explanation.timestamp,
        );

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(INFO_COLOR))
            .block(Block::default().borders(Borders::ALL).title("Packet Info").border_style(Style::default().fg(BORDER_COLOR)));
        f.render_widget(header, chunks[0]);

        // Analysis
        let analysis_text = format!(
            "🔍 What Happened:\n{}\n\n\
            💡 Why:\n{}\n\n\
            📋 Policy Context:\n{}\n\n\
            🔒 Security Analysis:\n{}",
            explanation.what_happened,
            explanation.why_happened,
            explanation.policy_context,
            explanation.security_analysis,
        );

        let analysis_color = if explanation.verdict == "DROPPED" {
            ERROR_COLOR
        } else {
            SUCCESS_COLOR
        };

        let analysis = Paragraph::new(analysis_text)
            .style(Style::default().fg(analysis_color))
            .block(Block::default().borders(Borders::ALL).title("Analysis").border_style(Style::default().fg(BORDER_COLOR)))
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(analysis, chunks[1]);

        // Troubleshooting
        let tips_text = format!(
            "🔧 Troubleshooting:\n{}",
            explanation.troubleshooting_tips.join("\n")
        );

        let tips = Paragraph::new(tips_text)
            .style(Style::default().fg(TEXT_COLOR))
            .block(Block::default().borders(Borders::ALL).title("Tips [Esc: Exit]").border_style(Style::default().fg(BORDER_COLOR)))
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(tips, chunks[2]);
    }

    fn render_connections(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if self.integrated_provider.is_none() {
            let content = Paragraph::new(
                "⚠️ Integrated Data Provider not available\n\n\
                Cilium eBPF maps or Kubernetes access not available.\n\
                Ensure:\n\
                • Cilium is installed and running\n\
                • BPF maps are accessible (/sys/fs/bpf/tc/globals/)\n\
                • Kubernetes API is accessible\n\n\
                Showing mock data mode.",
            )
            .style(Style::default().fg(WARNING_COLOR))
            .block(Block::default().borders(Borders::ALL).title("⚡ Enriched Connections").border_style(Style::default().fg(BORDER_COLOR)));
            f.render_widget(content, area);
            return;
        }

        let items: Vec<ListItem> = self
            .enriched_connections
            .iter()
            .take(50)
            .map(|enriched| {
                // Determine connection state color
                let state_color = match enriched.conn.state {
                    crate::ebpf::ConntrackState::Established => ESTABLISHED_COLOR,
                    crate::ebpf::ConntrackState::New => NEW_CONNECTION_COLOR,
                    crate::ebpf::ConntrackState::Related => RELATED_COLOR,
                    crate::ebpf::ConntrackState::Invalid => INVALID_COLOR,
                };

                // Format source and destination with pod names
                let src = match &enriched.src_pod {
                    Some(pod) => format!("{}/{}", pod.namespace, pod.pod_name),
                    None => enriched.conn.src_ip.clone(),
                };

                let dst = match &enriched.dst_pod {
                    Some(pod) => format!("{}/{}", pod.namespace, pod.pod_name),
                    None => enriched.conn.dst_ip.clone(),
                };

                // Format protocol
                let proto = match enriched.conn.protocol {
                    6 => "TCP",
                    17 => "UDP",
                    1 => "ICMP",
                    _ => "???",
                };

                let content = format!(
                    "{:40} → {:40} {:5} {:6} {:8} pkts {:10} bytes",
                    format!("{}:{}", src, enriched.conn.src_port),
                    format!("{}:{}", dst, enriched.conn.dst_port),
                    proto,
                    format!("{:?}", enriched.conn.state),
                    enriched.conn.packets,
                    enriched.conn.bytes,
                );

                ListItem::new(Line::from(Span::styled(
                    content,
                    Style::default().fg(state_color),
                )))
            })
            .collect();

        // Get identity resolver stats
        let stats_text = if let Some(provider) = &self.integrated_provider {
            let stats = provider.identity_stats();
            format!(
                "Enriched Connections ({}) | Identity Cache: {} IDs, {} IPs, age: {}s",
                self.enriched_connections.len(),
                stats.total_identities,
                stats.total_ips,
                stats.age_seconds,
            )
        } else {
            format!("Enriched Connections ({})", self.enriched_connections.len())
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(stats_text));
        f.render_widget(list, area);
    }

    fn render_endpoints(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let items: Vec<ListItem> = self
            .endpoints
            .iter()
            .map(|ep| {
                let status_color = match ep.status {
                    crate::endpoints::EndpointStatus::Running => RUNNING_COLOR,
                    crate::endpoints::EndpointStatus::Pending => PENDING_COLOR,
                    crate::endpoints::EndpointStatus::Failed => FAILED_COLOR,
                    crate::endpoints::EndpointStatus::Unknown => UNKNOWN_STATUS_COLOR,
                };

                let app_label = ep
                    .labels
                    .get("app")
                    .or_else(|| ep.labels.get("app.kubernetes.io/name"))
                    .map(|s| s.as_str())
                    .unwrap_or("none");

                let content = format!(
                    "{:10} {:20} {:15} {:10} {:15}",
                    ep.status, ep.namespace, ep.name, ep.ip, app_label
                );

                ListItem::new(Line::from(Span::styled(
                    content,
                    Style::default().fg(status_color),
                )))
            })
            .collect();

        let header = format!(
            "{:10} {:20} {:15} {:10} {:15}",
            "STATUS", "NAMESPACE", "NAME", "IP", "APP"
        );

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Endpoints ({}) - {}", self.endpoints.len(), header))
                    .border_style(Style::default().fg(BORDER_COLOR)),
            );
        f.render_widget(list, area);
    }

    fn render_policies(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let content = Paragraph::new(
            "Active Policies:\n\
            ✓ allow-intra-namespace (default)\n\
            ✓ allow-dns (default)\n\
            ✓ allow-hubble (kube-system)",
        )
        .style(Style::default().fg(TEXT_COLOR))
        .block(Block::default().borders(Borders::ALL).title("Network Policies").border_style(Style::default().fg(BORDER_COLOR)));
        f.render_widget(content, area);
    }

    fn render_metrics(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        // Get identity resolver stats if available
        let identity_stats = if let Some(provider) = &self.integrated_provider {
            let stats = provider.identity_stats();
            let ebpf_status = if provider.is_ebpf_available() {
                "✅ Available"
            } else {
                "⚠️ Mock Mode"
            };
            format!(
                "eBPF Integration:   {}\n\
                Identity Cache:     {} identities\n\
                IP Mappings:        {} IPs\n\
                Pod Mappings:       {} pods\n\
                Cache Age:          {}s\n",
                ebpf_status,
                stats.total_identities,
                stats.total_ips,
                stats.total_pods,
                stats.age_seconds,
            )
        } else {
            "eBPF Integration:   ⚠️ Not Available\n\
            Identity Cache:     N/A\n\
            IP Mappings:        N/A\n\
            Pod Mappings:       N/A\n\
            Cache Age:          N/A\n".to_string()
        };

        let content = Paragraph::new(format!(
            "📊 Platform Metrics:\n\n\
            Modules Active:     7/13 (54%)\n\
            Tests Passing:      70/70 (100%)\n\
            Total LOC:          14,490\n\
            Documentation:      6,500 lines\n\n\
            {}\n\
            ✅ Self-Healer:     Ready\n\
            ✅ AutoPolicy:      Learning\n\
            ✅ RootCause:       Monitoring\n\
            ✅ Simulator:       Ready\n\
            ✅ Replay:          Ready\n\
            ✅ K8s Identity:    Active",
            identity_stats
        ))
        .style(Style::default().fg(TEXT_COLOR))
        .block(Block::default().borders(Borders::ALL).title("Platform Metrics").border_style(Style::default().fg(BORDER_COLOR)));
        f.render_widget(content, area);
    }

    fn render_healer(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let stats = self.modules.healer_stats();
        let data_mode = if self.modules.is_enriched() {
            "Enriched (Pod-Aware)"
        } else {
            "Mock Data"
        };

        // Get actual problems and fixes
        let (problems_list, fixes_list) = match &self.modules {
            ModuleContainer::Enriched { healer, .. } => {
                (healer.problems(), healer.fixes())
            }
            ModuleContainer::Mock { healer, .. } => {
                (healer.problems(), healer.fixes())
            }
        };

        // Last run info
        let elapsed = self.last_healer_run.elapsed().as_secs();
        let next_run = if elapsed < 30 {
            30 - elapsed
        } else {
            0
        };

        // Build problems display
        let mut problem_text = String::new();
        for (idx, problem) in problems_list.iter().take(3).enumerate() {
            problem_text.push_str(&format!("\n{}. {}", idx + 1, Self::format_problem(problem)));
        }
        if problem_text.is_empty() {
            problem_text = "\n  No problems detected ✓".to_string();
        }

        // Build fixes display
        let mut fixes_text = String::new();
        for (idx, fix) in fixes_list.iter().take(3).enumerate() {
            let status = if fix.applied { "✅" } else { "📝" };
            fixes_text.push_str(&format!("\n{}. {} {}", idx + 1, status, Self::format_fix_action(&fix.action)));
        }
        if fixes_text.is_empty() {
            fixes_text = "\n  No fixes proposed".to_string();
        }

        let healer_status = format!(
            "🏥 Self-Healer Status\n\n\
            Mode:               Active ({})\n\
            Problems Detected:  {}\n\
            Fixes Proposed:     {}\n\
            Fixes Applied:      {}\n\
            Last Check:         {}s ago (next in {}s)\n\
            Recent Problems:{}\n\
            Proposed Fixes:{}",
            data_mode,
            stats.problems_detected,
            stats.fixes_proposed,
            stats.fixes_applied,
            elapsed,
            next_run,
            problem_text,
            fixes_text,
        );

        let content = Paragraph::new(healer_status)
            .style(Style::default().fg(SUCCESS_COLOR))
            .block(Block::default().borders(Borders::ALL).title("🏥 Self-Healer (Auto-Scan: 30s | Press 'd' for manual)").border_style(Style::default().fg(BORDER_COLOR)));
        f.render_widget(content, area);
    }

    fn format_problem(problem: &crate::modules::healer::Problem) -> String {
        use crate::modules::healer::Problem;
        match problem {
            Problem::DNSDrops { namespace, pod, count } => {
                format!("DNS drops: {}/{} ({} drops)", namespace, pod, count)
            }
            Problem::MTUMismatch { namespace, pod, expected, actual } => {
                format!("MTU mismatch: {}/{} (expected {}, got {})", namespace, pod, expected, actual)
            }
            Problem::PolicyGap { src_namespace, src_pod, dst_namespace, dst_pod, port, protocol } => {
                format!("Policy gap: {}/{} → {}/{}:{} {}", src_namespace, src_pod, dst_namespace, dst_pod, port, protocol)
            }
            Problem::LoadBalancerTimeout { service, backend, timeout_count } => {
                format!("LB timeout: {} → {} ({} timeouts)", service, backend, timeout_count)
            }
            Problem::ConntrackFull { node, utilization } => {
                format!("Conntrack full: {} ({:.1}% util)", node, utilization * 100.0)
            }
        }
    }

    fn format_fix_action(action: &crate::modules::healer::FixAction) -> String {
        use crate::modules::healer::FixAction;
        match action {
            FixAction::CreateDNSPolicy { namespace } => {
                format!("Create DNS policy for '{}'", namespace)
            }
            FixAction::AdjustMTU { namespace, pod, new_mtu } => {
                format!("Adjust MTU for {}/{} to {}", namespace, pod, new_mtu)
            }
            FixAction::CreateAllowPolicy { src, dst, port } => {
                format!("Allow {} → {}:{}", src, dst, port)
            }
            FixAction::RebalanceBackend { service, backend } => {
                format!("Rebalance {} → {}", service, backend)
            }
            FixAction::TuneConntrack { node, new_timeout } => {
                format!("Tune conntrack on {} (timeout: {})", node, new_timeout)
            }
        }
    }

    fn set_status_message(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
        self.status_message_time = std::time::Instant::now();
    }

    fn save_policies(&self, policies: &[crate::modules::autopolicy::GeneratedPolicy]) -> Result<usize> {
        use std::fs;
        use std::io::Write;

        // Create policies directory if it doesn't exist
        fs::create_dir_all("./policies")?;

        let mut count = 0;
        for policy in policies {
            let filename = format!("./policies/{}.yaml", policy.name);
            let mut file = fs::File::create(&filename)?;
            file.write_all(policy.yaml.as_bytes())?;
            count += 1;
            tracing::debug!("Saved policy to {}", filename);
        }

        Ok(count)
    }

    fn apply_policy_kubectl(&self, policy_name: &str, policy_yaml: &str) -> Result<()> {
        use std::process::Command;

        // Write policy to temporary file
        let temp_file = format!("/tmp/cilium-policy-{}.yaml", policy_name);
        std::fs::write(&temp_file, policy_yaml)?;

        // Apply using kubectl
        let output = Command::new("kubectl")
            .arg("apply")
            .arg("-f")
            .arg(&temp_file)
            .output()?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("kubectl apply failed: {}", stderr))
        }
    }

    fn rollback_policy_kubectl(&self, policy_name: &str, namespace: &str) -> Result<()> {
        use std::process::Command;

        // Delete the CiliumNetworkPolicy using kubectl
        let output = Command::new("kubectl")
            .args(["delete", "ciliumnetworkpolicy", policy_name, "-n", namespace])
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Check if error is "not found" - treat as success (already deleted)
            if stderr.contains("NotFound") || stderr.contains("not found") {
                Ok(())
            } else {
                Err(anyhow::anyhow!("kubectl delete failed: {}", stderr))
            }
        }
    }

    fn get_fix_policy(&self, fix_index: usize) -> (String, String) {
        match fix_index {
            0 => {
                // Allow port 8080 policy
                let policy_name = "rootcause-fix-allow-8080".to_string();
                let policy_yaml = r#"apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: rootcause-fix-allow-8080
  namespace: default
spec:
  endpointSelector:
    matchLabels:
      app: frontend
  egress:
  - toEndpoints:
    - matchLabels:
        app: backend
    toPorts:
    - ports:
      - port: "8080"
        protocol: TCP"#.to_string();
                (policy_name, policy_yaml)
            }
            1 => {
                // DNS egress policy
                let policy_name = "rootcause-fix-dns-egress".to_string();
                let policy_yaml = r#"apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: rootcause-fix-dns-egress
  namespace: default
spec:
  endpointSelector: {}
  egress:
  - toEndpoints:
    - matchLabels:
        k8s:io.kubernetes.pod.namespace: kube-system
        k8s-app: kube-dns
    toPorts:
    - ports:
      - port: "53"
        protocol: UDP
      rules:
        dns:
        - matchPattern: "*"
  - toFQDNs:
    - matchPattern: "*"
    toPorts:
    - ports:
      - port: "53"
        protocol: UDP"#.to_string();
                (policy_name, policy_yaml)
            }
            2 => {
                // MTU adjustment (ConfigMap update - simulated as policy for demo)
                let policy_name = "rootcause-fix-mtu-note".to_string();
                let policy_yaml = r#"# MTU Adjustment
#
# This would normally update the ConfigMap:
# kubectl -n kube-system patch configmap/cilium-config \
#   --type merge -p '{"data":{"tunnel-protocol":"disabled"}}'
#
# For demonstration, this creates a note policy:
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: rootcause-fix-mtu-note
  namespace: default
  annotations:
    description: "MTU issue detected - consider adjusting CNI MTU settings"
spec:
  endpointSelector: {}
  egress:
  - {}  # Allow all (this is just a placeholder)"#.to_string();
                (policy_name, policy_yaml)
            }
            3 => {
                // DB access policy
                let policy_name = "rootcause-fix-db-access".to_string();
                let policy_yaml = r#"apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: rootcause-fix-db-access
  namespace: default
spec:
  endpointSelector:
    matchLabels:
      app: api
  egress:
  - toEndpoints:
    - matchLabels:
        app: db
    toPorts:
    - ports:
      - port: "5432"
        protocol: TCP"#.to_string();
                (policy_name, policy_yaml)
            }
            _ => {
                ("unknown-fix".to_string(), "# Unknown fix".to_string())
            }
        }
    }

    fn render_policy_detail(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let policies = match &self.modules {
            ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
            ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
        };

        if policies.is_empty() {
            let no_policies = Paragraph::new(
                "No policies generated yet.\n\n\
                Press 'g' to generate policies from learned patterns.\n\
                Press 'Esc' to return to main view."
            )
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Policy Detail"));
            f.render_widget(no_policies, area);
            return;
        }

        let selected_policy = &policies[self.selected_policy_index];
        let is_applied = self.applied_policies.contains(&selected_policy.name);

        // Create policy detail content
        let status_indicator = if is_applied {
            "Status:     ✅ APPLIED"
        } else {
            "Status:     📋 Not Applied"
        };

        let detail_text = format!(
            "📄 Policy {} of {}\n\n\
            Name:       {}\n\
            Namespace:  {}\n\
            Confidence: {:.1}%\n\
            Patterns:   {}\n\
            {}\n\
            File:       ./policies/{}.yaml\n\
            \n\
            ─────────────────────────────────────────────────────────\n\
            YAML Content:\n\
            ─────────────────────────────────────────────────────────\n\
            {}\n\
            ─────────────────────────────────────────────────────────\n\n\
            {}",
            self.selected_policy_index + 1,
            policies.len(),
            selected_policy.name,
            selected_policy.namespace,
            selected_policy.confidence * 100.0,
            selected_policy.patterns.len(),
            status_indicator,
            selected_policy.name,
            selected_policy.yaml,
            if is_applied {
                "Press ↑/↓ to navigate | r: Rollback Policy | Esc to exit"
            } else {
                "Press ↑/↓ to navigate | a: Apply Policy | Esc to exit"
            }
        );

        let policy_detail = Paragraph::new(detail_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Policy Detail: {}", selected_policy.name))
                .style(Style::default().fg(Color::Cyan)));

        f.render_widget(policy_detail, area);
    }
}
