/// Chaos Engineering View - eBPF-based Fault Injection
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::theme::*;
use crate::modules::chaos::ChaosEngine;

pub struct ChaosView {
    pub selected_experiment_index: usize,
    pub selected_preset_index: usize,
    pub show_presets: bool,
    pub confirmation_mode: bool,
    pub circuit_breaker_confirm: bool,
}

impl ChaosView {
    pub fn new() -> Self {
        Self {
            selected_experiment_index: 0,
            selected_preset_index: 0,
            show_presets: true,
            confirmation_mode: false,
            circuit_breaker_confirm: false,
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.show_presets {
            move_selection_up(&mut self.selected_preset_index);
        } else {
            move_selection_up(&mut self.selected_experiment_index);
        }
    }

    pub fn move_selection_down(&mut self, max: usize) {
        if self.show_presets {
            move_selection_down(&mut self.selected_preset_index, max);
        } else {
            move_selection_down(&mut self.selected_experiment_index, max);
        }
    }

    pub fn toggle_view(&mut self) {
        self.show_presets = !self.show_presets;
    }

    pub fn trigger_confirmation(&mut self) {
        self.confirmation_mode = true;
    }

    pub fn cancel_confirmation(&mut self) {
        self.confirmation_mode = false;
        self.circuit_breaker_confirm = false;
    }

    pub fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, _chaos: Option<&ChaosEngine>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9),  // Header & Status
                Constraint::Min(10),    // Experiments or Presets
                Constraint::Length(10), // Details
            ])
            .split(area);

        // Header
        self.render_header(f, chunks[0]);

        if self.show_presets {
            // Preset experiments
            self.render_presets(f, chunks[1]);
            self.render_preset_details(f, chunks[2]);
        } else {
            // Active experiments
            self.render_active_experiments(f, chunks[1]);
            self.render_experiment_details(f, chunks[2]);
        }
    }

    fn render_header(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let header_text = format!(
            "🌪️  eBPF Chaos Engineering\n\n\
            Status:            Active\n\
            Circuit Breaker:   {} {}\n\
            Active Tests:      2\n\
            Total Runs:        15\n\
            Safety Limits:     50% drop max, 5000ms latency max\n\n\
            View: {} | Press 'v' to toggle",
            if false { "🔴" } else { "✅" },
            if false { "TRIGGERED" } else { "Normal" },
            if self.show_presets {
                "Experiment Presets"
            } else {
                "Active Experiments"
            }
        );

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(WARNING_COLOR))
            .block(bordered_block("Chaos Status"));

        f.render_widget(header, area);
    }

    fn render_presets(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let presets = [
            (
                "Network Partition",
                "Drop 20% of packets",
                "Medium",
                WARNING_COLOR,
            ),
            ("Latency Spike", "Add 500ms delay", "Medium", WARNING_COLOR),
            ("DNS Outage", "Fail 30% DNS lookups", "High", ERROR_COLOR),
            (
                "Connection Reset",
                "Kill 15% connections",
                "Medium",
                WARNING_COLOR,
            ),
            (
                "Bandwidth Limit",
                "Throttle to 10 Mbps",
                "Low",
                SUCCESS_COLOR,
            ),
            (
                "Packet Corruption",
                "Corrupt 5% packets",
                "High",
                ERROR_COLOR,
            ),
            (
                "Total Partition",
                "Drop 100% packets (DANGER)",
                "Critical",
                ERROR_COLOR,
            ),
        ];

        let items: Vec<ListItem> = presets
            .iter()
            .enumerate()
            .map(|(idx, (name, desc, severity, color))| {
                let is_selected = idx == self.selected_preset_index;
                let style = if is_selected {
                    selected_style()
                } else {
                    Style::default().fg(*color)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                let content = format!(
                    "{}{}. {:<25} - {:<30} [{}]",
                    prefix,
                    idx + 1,
                    name,
                    desc,
                    severity
                );
                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let title = if self.confirmation_mode {
            "Chaos Presets [CONFIRM: y/n]"
        } else {
            "Chaos Presets [↑/↓: Select | Enter: Run | v: View Active | b: Circuit Breaker]"
        };

        let list = List::new(items).block(bordered_block(title));

        f.render_widget(list, area);
    }

    fn render_preset_details(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let details = match self.selected_preset_index {
            0 => {
                "🌪️  Network Partition\n\n\
                Type:        Packet Drop\n\
                Rate:        20%\n\
                Target:      All namespaces, egress traffic\n\
                Duration:    5 minutes (auto-cleanup)\n\
                Severity:    Medium\n\n\
                What It Tests:\n\
                Simulates network instability and packet loss.\n\
                Tests application retry logic and resilience."
            }
            1 => {
                "🌪️  Latency Spike\n\n\
                Type:        Latency Injection\n\
                Delay:       500ms (±50ms jitter)\n\
                Target:      Default namespace, both directions\n\
                Duration:    5 minutes (auto-cleanup)\n\
                Severity:    Medium\n\n\
                What It Tests:\n\
                Simulates network congestion and slow responses.\n\
                Tests timeout handling and async patterns."
            }
            2 => {
                "🌪️  DNS Outage\n\n\
                Type:        DNS Failure\n\
                Rate:        30%\n\
                Target:      Port 53 (DNS)\n\
                Duration:    3 minutes (auto-cleanup)\n\
                Severity:    High ⚠️\n\n\
                What It Tests:\n\
                Simulates DNS resolver issues.\n\
                Tests DNS caching and service discovery."
            }
            3 => {
                "🌪️  Connection Reset\n\n\
                Type:        Connection Termination\n\
                Rate:        15%\n\
                Target:      TCP connections\n\
                Duration:    5 minutes (auto-cleanup)\n\
                Severity:    Medium\n\n\
                What It Tests:\n\
                Simulates connection failures.\n\
                Tests reconnection logic and circuit breakers."
            }
            4 => {
                "🌪️  Bandwidth Limit\n\n\
                Type:        Bandwidth Throttling\n\
                Limit:       10 Mbps\n\
                Target:      All traffic\n\
                Duration:    5 minutes (auto-cleanup)\n\
                Severity:    Low\n\n\
                What It Tests:\n\
                Simulates bandwidth constraints.\n\
                Tests performance under limited bandwidth."
            }
            5 => {
                "🌪️  Packet Corruption\n\n\
                Type:        Packet Corruption\n\
                Rate:        5%\n\
                Target:      All traffic\n\
                Duration:    3 minutes (auto-cleanup)\n\
                Severity:    High ⚠️\n\n\
                What It Tests:\n\
                Simulates data corruption in transit.\n\
                Tests checksum validation and error handling."
            }
            6 => {
                "🌪️  Total Partition (DANGER)\n\n\
                Type:        Packet Drop\n\
                Rate:        100% ⚠️\n\
                Target:      Selected namespace only\n\
                Duration:    1 minute (auto-cleanup)\n\
                Severity:    Critical ❌\n\n\
                What It Tests:\n\
                Complete network isolation.\n\
                Tests disaster recovery and failover.\n\n\
                ⚠️  WARNING: This will completely isolate the target!"
            }
            _ => "Select a chaos preset to see details",
        };

        let paragraph = Paragraph::new(details)
            .style(Style::default().fg(TEXT_COLOR))
            .block(bordered_block("Experiment Details"))
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    fn render_active_experiments(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let experiments = [
            (
                "chaos-1234",
                "Network Partition",
                "Active",
                "2m ago",
                SUCCESS_COLOR,
            ),
            (
                "chaos-5678",
                "Latency Spike",
                "Active",
                "5m ago",
                SUCCESS_COLOR,
            ),
        ];

        let items: Vec<ListItem> = experiments
            .iter()
            .enumerate()
            .map(|(idx, (id, name, status, started, color))| {
                let is_selected = idx == self.selected_experiment_index;
                let style = if is_selected {
                    selected_style()
                } else {
                    Style::default().fg(*color)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                let content = format!(
                    "{}{:<15} {:<25} [{:<8}] Started: {}",
                    prefix, id, name, status, started
                );
                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let list = List::new(items).block(bordered_block(
            "Active Experiments [↑/↓: Select | s: Stop | S: Stop All | v: View Presets]",
        ));

        f.render_widget(list, area);
    }

    fn render_experiment_details(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let details = "📊 Active Experiment: Network Partition\n\n\
            ID:              chaos-1234\n\
            Type:            Packet Drop (20%)\n\
            Target:          All namespaces\n\
            Started:         2 minutes ago\n\
            Auto-cleanup:    3 minutes remaining\n\n\
            Metrics:\n\
            Packets Dropped: 1,234\n\
            Connections:     45 affected\n\
            Error Rate:      +3.2%";

        let paragraph = Paragraph::new(details)
            .style(Style::default().fg(INFO_COLOR))
            .block(bordered_block("Experiment Metrics"));

        f.render_widget(paragraph, area);
    }
}
