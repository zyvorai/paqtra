#![allow(dead_code)]
/// Canary Deployment View - Sidecarless Progressive Traffic Shifting
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::modules::canary::CanaryEngine;
use super::theme::*;

pub struct CanaryView {
    pub selected_canary_index: usize,
    pub show_details: bool,
    pub confirmation_mode: ConfirmationType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmationType {
    None,
    Promote,
    Rollback,
    Pause,
}

impl CanaryView {
    pub fn new() -> Self {
        Self {
            selected_canary_index: 0,
            show_details: false,
            confirmation_mode: ConfirmationType::None,
        }
    }

    pub fn move_selection_up(&mut self) {
        move_selection_up(&mut self.selected_canary_index);
    }

    pub fn move_selection_down(&mut self, max: usize) {
        move_selection_down(&mut self.selected_canary_index, max);
    }

    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    pub fn trigger_confirmation(&mut self, confirmation_type: ConfirmationType) {
        self.confirmation_mode = confirmation_type;
    }

    pub fn cancel_confirmation(&mut self) {
        self.confirmation_mode = ConfirmationType::None;
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        _canary: Option<&CanaryEngine>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Header
                Constraint::Min(10),    // Canary List
                Constraint::Length(10), // Traffic Split Visualization
            ])
            .split(area);

        // Header
        self.render_header(f, chunks[0]);

        // Canary List
        self.render_canary_list(f, chunks[1]);

        // Traffic Split
        self.render_traffic_split(f, chunks[2]);
    }

    fn render_header(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let header_text = format!(
            "🚢 Sidecarless Canary Deployments\n\n\
            Status:            Active\n\
            Active Canaries:   2\n\
            Total Deployed:    15\n\
            Success Rate:      93%\n\
            Rollbacks:         1\n\n\
            Progressive traffic shifting without sidecar overhead"
        );

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(INFO_COLOR))
            .block(bordered_block("Canary Status"));

        f.render_widget(header, area);
    }

    fn render_canary_list(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let canaries = vec![
            ("frontend-v2.1", "default", "Running", "60%", SUCCESS_COLOR),
            ("api-v3.0", "production", "Running", "20%", INFO_COLOR),
        ];

        let items: Vec<ListItem> = canaries
            .iter()
            .enumerate()
            .map(|(idx, (name, ns, state, traffic, color))| {
                let is_selected = idx == self.selected_canary_index;
                let style = if is_selected {
                    selected_style()
                } else {
                    Style::default().fg(*color)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                let content = format!(
                    "{}{:<20} ns:{:<12} [{:<8}] Traffic: {}",
                    prefix, name, ns, state, traffic
                );
                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let title = if self.confirmation_mode != ConfirmationType::None {
            match self.confirmation_mode {
                ConfirmationType::Promote => "Canaries [CONFIRM PROMOTE: y/n]",
                ConfirmationType::Rollback => "Canaries [CONFIRM ROLLBACK: y/n]",
                ConfirmationType::Pause => "Canaries [CONFIRM PAUSE: y/n]",
                _ => "Canaries",
            }
        } else {
            "Active Canaries [↑/↓: Select | p: Promote | r: Rollback | +: Progress | d: Details]"
        };

        let list = List::new(items).block(bordered_block(title));

        f.render_widget(list, area);
    }

    fn render_traffic_split(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Traffic split visualization
        let split_text = "📊 Traffic Distribution (frontend-v2.1)\n\n\
            Stable (v2.0):  ████████░░ 40%\n\
            Canary (v2.1):  ██████████████ 60%\n\n\
            Metrics:\n\
            Canary Success:  99.2%  (↑ from 98.5%)\n\
            Canary Latency:  45ms   (vs 47ms stable)\n\
            Error Rate:      0.8%   (threshold: 1%)";

        let split = Paragraph::new(split_text)
            .style(Style::default().fg(TEXT_COLOR))
            .block(bordered_block("Traffic Split"));

        f.render_widget(split, chunks[0]);

        // Health gauge
        let health_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Length(4)])
            .split(chunks[1]);

        // Canary health
        let canary_health = 0.992; // 99.2%
        let gauge1 = Gauge::default()
            .block(bordered_block("Canary Health"))
            .gauge_style(Style::default().fg(SUCCESS_COLOR))
            .percent((canary_health * 100.0) as u16)
            .label(format!("{:.1}%", canary_health * 100.0));

        f.render_widget(gauge1, health_chunks[0]);

        // Promotion progress
        let progress = 0.6; // 60% traffic
        let gauge2 = Gauge::default()
            .block(bordered_block("Progress"))
            .gauge_style(Style::default().fg(INFO_COLOR))
            .percent((progress * 100.0) as u16)
            .label(format!("{}% → 100%", (progress * 100.0) as u16));

        f.render_widget(gauge2, health_chunks[1]);
    }
}
