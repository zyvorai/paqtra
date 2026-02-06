/// RootCause View - Drop Analysis & Troubleshooting
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::ebpf::MapReader;
use crate::modules::rootcause::RootCauseEngine;
use super::theme::*;

pub struct RootCauseView;

impl RootCauseView {
    pub fn new() -> Self {
        Self
    }

    pub fn render<M: MapReader>(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        rootcause: Option<&RootCauseEngine<M>>,
        selected_fix_index: usize,
        fix_apply_confirmation: bool,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Header
                Constraint::Min(10),    // Drop Analysis
                Constraint::Length(12),  // Fixes
            ])
            .split(area);

        // Header
        self.render_header(f, chunks[0], rootcause);

        // Drop Analysis
        self.render_drops(f, chunks[1]);

        // Recommended Fixes
        self.render_fixes(f, chunks[2], selected_fix_index, fix_apply_confirmation);
    }

    fn render_header<M: MapReader>(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        _rootcause: Option<&RootCauseEngine<M>>,
    ) {
        let stats =
            "🔍 Root-Cause Analysis Engine\n\n\
            Status:            Active\n\
            Drops Analyzed:    142\n\
            Issues Found:      8\n\
            Patterns:          12\n\
            Anomalies:         2\n\
            Last Analysis:     Just now";

        let content = Paragraph::new(stats)
            .style(Style::default().fg(ERROR_COLOR))
            .block(Block::default().borders(Borders::ALL).title("Analysis Status").border_style(Style::default().fg(BORDER_COLOR)));

        f.render_widget(content, area);
    }

    fn render_drops(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let drops = vec![
            (
                "Policy Denied",
                "frontend → backend:8080",
                "Critical",
                "12 drops",
            ),
            (
                "DNS Blocked",
                "app → 8.8.8.8:53",
                "Critical",
                "45 drops",
            ),
            (
                "MTU Exceeded",
                "service-a → service-b",
                "Medium",
                "8 drops",
            ),
            (
                "Port Not Allowed",
                "api → db:5432",
                "Medium",
                "23 drops",
            ),
            (
                "Invalid Packet",
                "10.0.1.5 → 10.0.2.10",
                "Low",
                "3 drops",
            ),
        ];

        let items: Vec<ListItem> = drops
            .iter()
            .map(|(reason, flow, severity, count)| {
                let color = severity_color(severity);
                let content = format!(
                    "{:<18} {:<30} [{:<8}] {}",
                    reason, flow, severity, count
                );
                ListItem::new(Line::from(Span::styled(
                    content,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Recent Packet Drops (Top 5)")
                .border_style(Style::default().fg(BORDER_COLOR)),
        );

        f.render_widget(list, area);
    }

    fn render_fixes(&self, f: &mut Frame, area: ratatui::layout::Rect, selected_fix_index: usize, fix_apply_confirmation: bool) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // Recommended Fixes (selectable list)
        let fix_names = vec![
            "Add allow-8080 policy",
            "Enable DNS egress",
            "Adjust MTU to 1450",
            "Add DB access policy",
        ];

        let items: Vec<ListItem> = fix_names
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                let style = if idx == selected_fix_index {
                    Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(SUCCESS_COLOR)
                };

                let prefix = if idx == selected_fix_index { "▶ " } else { "  " };
                let content = format!("{}{}. {}", prefix, idx + 1, name);

                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let fixes_title = if fix_apply_confirmation {
            "Fixes [CONFIRM: y/n]"
        } else {
            "Fixes [↑/↓: Select | a: Apply]"
        };

        let fixes = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(fixes_title)
                .border_style(Style::default().fg(BORDER_COLOR)),
        );

        f.render_widget(fixes, chunks[0]);

        // Fix Details - Show YAML preview for selected fix
        let details_text = match selected_fix_index {
            0 => {
                "📝 Policy: rootcause-fix-allow-8080\n\
                Namespace: default\n\n\
                Allows frontend → backend:8080 TCP\n\n\
                This policy permits traffic from frontend\n\
                pods to backend pods on port 8080,\n\
                resolving the detected policy deny."
            }
            1 => {
                "📝 Policy: rootcause-fix-dns-egress\n\
                Namespace: default\n\n\
                Enables DNS resolution for all pods\n\n\
                This policy allows egress to kube-dns\n\
                on port 53 UDP, resolving DNS blocks."
            }
            2 => {
                "📝 Note: MTU Adjustment\n\
                Namespace: default\n\n\
                MTU exceeds detected on path\n\n\
                Consider adjusting CNI MTU settings\n\
                in cilium-config ConfigMap or\n\
                disabling tunneling protocol."
            }
            3 => {
                "📝 Policy: rootcause-fix-db-access\n\
                Namespace: default\n\n\
                Allows api → db:5432 TCP\n\n\
                This policy permits API pods to\n\
                access database pods on port 5432."
            }
            _ => "Select a fix to see details",
        };

        let details = Paragraph::new(details_text)
            .style(Style::default().fg(INFO_COLOR))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Policy Details")
                    .border_style(Style::default().fg(BORDER_COLOR)),
            );

        f.render_widget(details, chunks[1]);
    }
}
