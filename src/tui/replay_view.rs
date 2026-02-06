/// Replay View - Traffic Recording & Playback
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::ebpf::MapReader;
use crate::modules::replay::ReplayEngine;
use super::theme::*;

pub struct ReplayView;

impl ReplayView {
    pub fn new() -> Self {
        Self
    }

    pub fn render<M: MapReader>(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        replay: Option<&ReplayEngine<M>>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Header
                Constraint::Min(10),    // Recordings List
                Constraint::Length(8),  // Comparison Results
            ])
            .split(area);

        // Header
        self.render_header(f, chunks[0], replay);

        // Recordings
        self.render_recordings(f, chunks[1]);

        // Latest Comparison
        self.render_comparison(f, chunks[2]);
    }

    fn render_header<M: MapReader>(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        replay: Option<&ReplayEngine<M>>,
    ) {
        let stats = if let Some(engine) = replay {
            let s = engine.stats();
            format!(
                "📹 Traffic Replay System\n\n\
                Status:            {}\n\
                Total Recordings:  {}\n\
                Total Flows:       {}\n\n\
                Press 'r' to refresh recordings list",
                if s.recording_in_progress { "🔴 Recording" } else { "⏹️  Idle" },
                s.total_recordings,
                s.total_flows_recorded,
            )
        } else {
            "Traffic Replay not initialized".to_string()
        };

        let content = Paragraph::new(stats)
            .style(Style::default().fg(ORANGE))
            .block(Block::default().borders(Borders::ALL).title("Replay Status").border_style(Style::default().fg(BORDER_COLOR)));

        f.render_widget(content, area);
    }

    fn render_recordings(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let recordings = vec![
            ("rec-prod-baseline", "1000", "5.2 MB", "2h ago", RECORDING_ACTIVE_COLOR),
            ("rec-policy-test", "450", "2.1 MB", "30m ago", RECORDING_RECENT_COLOR),
            ("rec-migration", "2500", "12 MB", "1d ago", RECORDING_OLD_COLOR),
            ("rec-incident-123", "180", "890 KB", "3d ago", RECORDING_WARNING_COLOR),
        ];

        let items: Vec<ListItem> = recordings
            .iter()
            .map(|(name, flows, size, age, color)| {
                let content = format!(
                    "{:<25} Flows: {:>5}  Size: {:>8}  Age: {}",
                    name, flows, size, age
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
                .title("Available Recordings (4)")
                .border_style(Style::default().fg(BORDER_COLOR)),
        );

        f.render_widget(list, area);
    }

    fn render_comparison(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Comparison Stats
        let stats_text =
            "📊 Last Replay Comparison:\n\n\
            Total Flows:       1000\n\
            Identical:         950 (95%)\n\
            Verdict Changed:   50\n\
            New Drops:         30\n\
            Fixed Drops:       20";

        let stats = Paragraph::new(stats_text)
            .style(Style::default().fg(TEXT_COLOR))
            .block(Block::default().borders(Borders::ALL).title("Comparison").border_style(Style::default().fg(BORDER_COLOR)));

        f.render_widget(stats, chunks[0]);

        // Similarity Gauge
        let similarity = 0.95; // 95%
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Similarity")
                    .border_style(Style::default().fg(BORDER_COLOR)),
            )
            .gauge_style(Style::default().fg(PROGRESS_NORMAL_COLOR))
            .percent((similarity * 100.0) as u16)
            .label(format!("{}%", (similarity * 100.0) as u16));

        f.render_widget(gauge, chunks[1]);
    }
}
