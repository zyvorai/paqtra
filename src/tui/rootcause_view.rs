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
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Header
                Constraint::Min(10),    // Drop Analysis
                Constraint::Length(8),  // Fixes
            ])
            .split(area);

        // Header
        self.render_header(f, chunks[0], rootcause);

        // Drop Analysis
        self.render_drops(f, chunks[1]);

        // Recommended Fixes
        self.render_fixes(f, chunks[2]);
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

    fn render_fixes(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Recommended Fixes
        let fixes_text =
            "🔧 Recommended Fixes:\n\n\
            1. Add allow-8080 policy\n\
            2. Enable DNS egress\n\
            3. Adjust MTU to 1450\n\
            4. Add DB access policy";

        let fixes = Paragraph::new(fixes_text)
            .style(Style::default().fg(SUCCESS_COLOR))
            .block(Block::default().borders(Borders::ALL).title("Fixes").border_style(Style::default().fg(BORDER_COLOR)));

        f.render_widget(fixes, chunks[0]);

        // Fix Details
        let details_text =
            "📝 Policy YAML:\n\n\
            apiVersion: cilium.io/v2\n\
            kind: CiliumNetworkPolicy\n\
            metadata:\n\
              name: allow-backend";

        let details = Paragraph::new(details_text)
            .style(Style::default().fg(INFO_COLOR))
            .block(Block::default().borders(Borders::ALL).title("Details").border_style(Style::default().fg(BORDER_COLOR)));

        f.render_widget(details, chunks[1]);
    }
}
