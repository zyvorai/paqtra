/// Simulator View - What-If Scenario Testing
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::ebpf::MapReader;
use crate::modules::simulator::Simulator;

pub struct SimulatorView {
    pub should_simulate: bool,
}

impl SimulatorView {
    pub fn new() -> Self {
        Self {
            should_simulate: false,
        }
    }

    pub fn trigger_simulation(&mut self) {
        self.should_simulate = true;
    }

    pub fn render<M: MapReader>(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        simulator: Option<&Simulator<M>>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Header & Stats
                Constraint::Min(8),     // Simulation Results
                Constraint::Length(8),  // Risk Assessment
            ])
            .split(area);

        // Header
        self.render_header(f, chunks[0]);

        // Simulation Results
        self.render_results(f, chunks[1], simulator);

        // Risk Assessment
        self.render_risk(f, chunks[2]);
    }

    fn render_header(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let content = Paragraph::new(
            "🔮 What-If Simulator\n\n\
            Test policy changes safely before applying to production.\n\
            Predict impact, assess risk, identify affected services.\n\n\
            Available Scenarios:\n\
            • Add/Remove/Modify Policy  • Block/Allow Traffic\n\
            • Block External IP          • Default Deny Mode\n\n\
            Press 's' to run a demo simulation",
        )
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title("Simulator Overview"));

        f.render_widget(content, area);
    }

    fn render_results<M: MapReader>(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        _simulator: Option<&Simulator<M>>,
    ) {
        let scenarios = vec![
            ("✓ Block External IP 1.2.3.4", "3", "Medium", Color::Yellow),
            ("✓ Add DNS Policy", "1", "Low", Color::Green),
            ("✓ Default Deny in prod", "9", "Critical", Color::Red),
            ("✓ Modify ingress rules", "5", "Medium", Color::Yellow),
            ("✓ Allow port 8080", "2", "Low", Color::Green),
        ];

        let items: Vec<ListItem> = scenarios
            .iter()
            .map(|(name, risk_score, risk_level, color)| {
                let content = format!(
                    "{:<35} Risk: {}/10  Level: {}",
                    name, risk_score, risk_level
                );
                ListItem::new(Line::from(Span::styled(
                    content,
                    Style::default().fg(*color),
                )))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Recent Simulations"),
        );

        f.render_widget(list, area);
    }

    fn render_risk(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let risk_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Impact Analysis
        let impact_text =
            "📊 Impact:\n\
            Flows Affected:    120\n\
            Services:          5\n\
            Namespaces:        2\n\
            Critical Svcs:     0";

        let impact = Paragraph::new(impact_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Impact"));

        f.render_widget(impact, risk_chunks[0]);

        // Risk Meter
        let risk_level = 0.3; // 30%
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Risk Level"),
            )
            .gauge_style(Style::default().fg(Color::Yellow))
            .percent((risk_level * 100.0) as u16)
            .label(format!("{}% - MEDIUM", (risk_level * 100.0) as u16));

        f.render_widget(gauge, risk_chunks[1]);
    }
}
