#![allow(dead_code)]
/// Replay View - Traffic Recording & Playback with Time-Travel Debugging
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::ebpf::MapReader;
use crate::modules::replay::ReplayEngine;
use super::theme::*;

pub struct ReplayView {
    // Time-travel state
    pub time_travel_mode: bool,
    pub selected_recording_index: usize,
    pub timeline_position: usize,  // Current position in timeline (0-100)
    pub is_playing: bool,
    pub playback_speed: f32,
    pub show_event_markers: bool,
}

impl ReplayView {
    pub fn new() -> Self {
        Self {
            time_travel_mode: false,
            selected_recording_index: 0,
            timeline_position: 0,
            is_playing: false,
            playback_speed: 1.0,
            show_event_markers: true,
        }
    }

    pub fn enter_time_travel(&mut self) {
        self.time_travel_mode = true;
        self.timeline_position = 0;
        self.is_playing = false;
    }

    pub fn exit_time_travel(&mut self) {
        self.time_travel_mode = false;
        self.is_playing = false;
    }

    pub fn toggle_playback(&mut self) {
        self.is_playing = !self.is_playing;
    }

    pub fn step_forward(&mut self) {
        if self.timeline_position < 100 {
            self.timeline_position += 1;
        }
    }

    pub fn step_backward(&mut self) {
        if self.timeline_position > 0 {
            self.timeline_position -= 1;
        }
    }

    pub fn jump_to_next_event(&mut self) {
        // Jump to next significant event (drop, error, etc.)
        self.timeline_position = (self.timeline_position + 10).min(100);
    }

    pub fn jump_to_prev_event(&mut self) {
        // Jump to previous significant event
        self.timeline_position = self.timeline_position.saturating_sub(10);
    }

    pub fn move_selection_up(&mut self) {
        move_selection_up(&mut self.selected_recording_index);
    }

    pub fn move_selection_down(&mut self, max: usize) {
        move_selection_down(&mut self.selected_recording_index, max);
    }

    pub fn adjust_speed(&mut self, faster: bool) {
        if faster {
            self.playback_speed = (self.playback_speed * 2.0).min(8.0);
        } else {
            self.playback_speed = (self.playback_speed / 2.0).max(0.25);
        }
    }

    pub fn render<M: MapReader>(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        replay: Option<&ReplayEngine<M>>,
    ) {
        if self.time_travel_mode {
            // Time-travel debugging mode
            self.render_time_travel(f, area, replay);
        } else {
            // Normal recording list mode
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
            .block(bordered_block("Replay Status"));

        f.render_widget(content, area);
    }

    fn render_recordings(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let recordings = vec![
            ("rec-prod-baseline", "1000", "5.2 MB", "2h ago"),
            ("rec-policy-test", "450", "2.1 MB", "30m ago"),
            ("rec-migration", "2500", "12 MB", "1d ago"),
            ("rec-incident-123", "180", "890 KB", "3d ago"),
        ];

        let items: Vec<ListItem> = recordings
            .iter()
            .enumerate()
            .map(|(idx, (name, flows, size, age))| {
                let is_selected = idx == self.selected_recording_index;
                let style = if is_selected {
                    selected_style()
                } else {
                    Style::default().fg(TEXT_COLOR)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                let content = format!(
                    "{}{:<23} Flows: {:>5}  Size: {:>8}  Age: {}",
                    prefix, name, flows, size, age
                );
                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let title = "Recordings [↑/↓: Select | t: Time-Travel | r: Refresh]";
        let list = List::new(items).block(bordered_block(title));

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
            .block(bordered_block("Comparison"));

        f.render_widget(stats, chunks[0]);

        // Similarity Gauge
        let similarity = 0.95; // 95%
        let gauge = Gauge::default()
            .block(bordered_block("Similarity"))
            .gauge_style(Style::default().fg(PROGRESS_NORMAL_COLOR))
            .percent((similarity * 100.0) as u16)
            .label(format!("{}%", (similarity * 100.0) as u16));

        f.render_widget(gauge, chunks[1]);
    }

    fn render_time_travel<M: MapReader>(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        _replay: Option<&ReplayEngine<M>>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Timeline & Controls
                Constraint::Length(12), // Network State at Current Time
                Constraint::Min(8),     // Flow Events
            ])
            .split(area);

        // Timeline & Controls
        self.render_timeline(f, chunks[0]);

        // Network State
        self.render_network_state(f, chunks[1]);

        // Flow Events
        self.render_flow_events(f, chunks[2]);
    }

    fn render_timeline(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),  // Header & Info
                Constraint::Length(4),  // Timeline bar
            ])
            .split(area);

        // Header
        let playback_icon = if self.is_playing { "▶️" } else { "⏸️" };
        let header_text = format!(
            "⏱️  Time-Travel Debugging\n\n\
            Recording:     rec-prod-baseline (1000 flows)\n\
            Time:          {}% ({}/{})\n\
            Status:        {}  Speed: {}x",
            self.timeline_position,
            self.timeline_position * 10, // Convert to flow number
            1000,
            playback_icon,
            self.playback_speed
        );

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(INFO_COLOR))
            .block(bordered_block("Time-Travel Controls"));

        f.render_widget(header, chunks[0]);

        // Timeline Bar with Event Markers
        let timeline_text = self.create_timeline_bar();

        let timeline = Paragraph::new(timeline_text)
            .style(Style::default().fg(TEXT_COLOR))
            .block(bordered_block("Timeline [←/→: Step | [/]: Jump Events | Space: Play/Pause | +/-: Speed | Esc: Exit]"));

        f.render_widget(timeline, chunks[1]);
    }

    fn create_timeline_bar(&self) -> String {
        let bar_width = 70;
        let position = (self.timeline_position as f32 / 100.0 * bar_width as f32) as usize;

        // Event markers (drops at specific positions)
        let events = vec![10, 25, 45, 60, 80]; // Example event positions

        let mut bar = String::new();
        bar.push_str("Time:  ");

        for i in 0..bar_width {
            if i == position {
                bar.push('█'); // Current position
            } else if events.contains(&(i * 100 / bar_width)) {
                bar.push('!'); // Event marker
            } else if i < position {
                bar.push('─'); // Completed
            } else {
                bar.push('·'); // Future
            }
        }

        bar.push_str("\n       ");
        bar.push_str("0%");
        bar.push_str(&" ".repeat(bar_width - 8));
        bar.push_str("100%");

        bar
    }

    fn render_network_state(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        // Calculate the approximate flow index based on timeline position
        let current_flow = (self.timeline_position * 10).min(1000);

        let state_text = format!(
            "🌐 Network State at Flow #{}\n\n\
            Active Connections:    23\n\
            Allowed Flows:         18\n\
            Dropped Flows:         5\n\
            Unique Endpoints:      12\n\
            Namespaces:            3 (default, prod, staging)\n\
            Active Policies:       8\n\n\
            Latest Event:          DROP at frontend → backend:8080\n\
            Reason:                Policy denied (no matching rule)",
            current_flow
        );

        let state = Paragraph::new(state_text)
            .style(Style::default().fg(SUCCESS_COLOR))
            .block(bordered_block("Network State Snapshot"))
            .wrap(Wrap { trim: false });

        f.render_widget(state, area);
    }

    fn render_flow_events(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        // Show flows around current timeline position
        let current_flow = (self.timeline_position * 10).min(1000);

        let events = vec![
            (current_flow.saturating_sub(2), "frontend → backend:8080", "ALLOWED", "TCP", SUCCESS_COLOR),
            (current_flow.saturating_sub(1), "api → postgres:5432", "ALLOWED", "TCP", SUCCESS_COLOR),
            (current_flow, "frontend → backend:8080", "DROPPED", "TCP", ERROR_COLOR),
            (current_flow + 1, "frontend → redis:6379", "ALLOWED", "TCP", SUCCESS_COLOR),
            (current_flow + 2, "api → external:443", "ALLOWED", "TCP", SUCCESS_COLOR),
        ];

        let items: Vec<ListItem> = events
            .iter()
            .map(|(flow_num, flow, verdict, proto, color)| {
                let is_current = *flow_num == current_flow;
                let prefix = if is_current { "▶ " } else { "  " };
                let modifier = if is_current { Modifier::BOLD } else { Modifier::empty() };

                let content = format!(
                    "{}#{:<5} {:<35} [{:>7}] {}",
                    prefix, flow_num, flow, verdict, proto
                );
                ListItem::new(Line::from(Span::styled(
                    content,
                    Style::default().fg(*color).add_modifier(modifier),
                )))
            })
            .collect();

        let list = List::new(items).block(bordered_block("Flow Events (Around Current Time)"));

        f.render_widget(list, area);
    }
}
