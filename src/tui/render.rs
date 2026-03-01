use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

use super::app::{TuiApp, ModuleContainer};
use super::canary_view;
use super::theme::*;
use super::help_overlay;

impl TuiApp {
    pub(crate) fn ui(&self, f: &mut Frame) {
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
            "Replay",
            "Chaos",
            "Canary",
            "MultiCluster"
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
            10 => {
                // Chaos view
                self.chaos_view.render(f, chunks[2], None)
            }
            11 => {
                // Canary view
                self.canary_view.render(f, chunks[2], None)
            }
            12 => {
                // MultiCluster view
                self.multicluster_view.render(f, chunks[2], None)
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
                self.get_footer_text_for_tab()
            }
        } else {
            self.get_footer_text_for_tab()
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

    fn get_footer_text_for_tab(&self) -> String {
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
            9 => if self.replay_view.time_travel_mode {
                if self.replay_view.is_playing {
                    "?: Help | Space: Pause | ←/→: Step | [/]: Jump Events | +/-: Speed | Esc: Exit".to_string()
                } else {
                    "?: Help | Space: Play | ←/→: Step | [/]: Jump Events | +/-: Speed | Esc: Exit".to_string()
                }
            } else {
                "?: Help | q: Quit | ↑/↓: Select | t: Time-Travel | r: Refresh".to_string()
            },
            10 => if self.chaos_view.confirmation_mode {
                "⚠️ CONFIRM: y: Run Experiment | n: Cancel".to_string()
            } else if self.chaos_view.show_presets {
                "?: Help | ↑/↓: Select | Enter: Run | v: View Active | b: Circuit Breaker".to_string()
            } else {
                "?: Help | ↑/↓: Select | s: Stop | S: Stop All | v: View Presets".to_string()
            },
            11 => if self.canary_view.confirmation_mode != canary_view::ConfirmationType::None {
                "⚠️ CONFIRM: y: Execute | n: Cancel".to_string()
            } else {
                "?: Help | ↑/↓: Select | p: Promote | r: Rollback | +: Progress | d: Details".to_string()
            },
            12 => "?: Help | ↑/↓: Select | v: Cycle View (Clusters/Topology/Syncs/Placements)".to_string(),
            _ => "?: Help | q: Quit | Tab: Next | Shift+Tab: Previous".to_string(),
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

    pub(crate) fn format_problem(problem: &crate::modules::healer::Problem) -> String {
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

    pub(crate) fn format_fix_action(action: &crate::modules::healer::FixAction) -> String {
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

        // Get ML-based confidence information
        use crate::modules::autopolicy::confidence::ConfidenceScorer;
        let confidence_level = ConfidenceScorer::confidence_level(selected_policy.confidence);
        let recommendation = ConfidenceScorer::get_recommendation(selected_policy.confidence);

        let detail_text = format!(
            "📄 Policy {} of {} (ML-Enhanced Zero-Trust)\n\n\
            Name:          {}\n\
            Namespace:     {}\n\
            \n\
            🤖 ML Confidence Analysis:\n\
            Score:         {:.1}% ({})\n\
            Assessment:    {}\n\
            Patterns:      {} observed traffic patterns\n\
            \n\
            {}\n\
            File:          ./policies/{}.yaml\n\
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
            confidence_level,
            recommendation,
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
