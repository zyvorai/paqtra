/// Multi-Cluster Autopilot View - Global Orchestration
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::modules::multicluster::MultiClusterAutopilot;
use super::theme::*;

pub struct MultiClusterView {
    pub selected_cluster_index: usize,
    pub view_mode: ViewMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Clusters,     // List of clusters
    Topology,     // Cluster topology map
    Syncs,        // Active policy syncs
    Placements,   // Workload placements
}

impl MultiClusterView {
    pub fn new() -> Self {
        Self {
            selected_cluster_index: 0,
            view_mode: ViewMode::Clusters,
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_cluster_index > 0 {
            self.selected_cluster_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self, max: usize) {
        if self.selected_cluster_index < max - 1 {
            self.selected_cluster_index += 1;
        }
    }

    pub fn cycle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Clusters => ViewMode::Topology,
            ViewMode::Topology => ViewMode::Syncs,
            ViewMode::Syncs => ViewMode::Placements,
            ViewMode::Placements => ViewMode::Clusters,
        };
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: ratatui::layout::Rect,
        _autopilot: Option<&MultiClusterAutopilot>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9),  // Header
                Constraint::Min(10),    // Main View
                Constraint::Length(8),  // Details
            ])
            .split(area);

        // Header
        self.render_header(f, chunks[0]);

        // Main view based on mode
        match self.view_mode {
            ViewMode::Clusters => {
                self.render_clusters(f, chunks[1]);
                self.render_cluster_details(f, chunks[2]);
            }
            ViewMode::Topology => {
                self.render_topology(f, chunks[1]);
                self.render_topology_stats(f, chunks[2]);
            }
            ViewMode::Syncs => {
                self.render_syncs(f, chunks[1]);
                self.render_sync_details(f, chunks[2]);
            }
            ViewMode::Placements => {
                self.render_placements(f, chunks[1]);
                self.render_placement_details(f, chunks[2]);
            }
        }
    }

    fn render_header(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let view_name = match self.view_mode {
            ViewMode::Clusters => "Cluster List",
            ViewMode::Topology => "Topology Map",
            ViewMode::Syncs => "Policy Syncs",
            ViewMode::Placements => "Workload Placements",
        };

        let header_text = format!(
            "🌐 Multi-Cluster Autopilot\n\n\
            View:              {}\n\
            Total Clusters:    4\n\
            Active:            3\n\
            Degraded:          1\n\
            Active Syncs:      2\n\
            Regions:           3 (us-east-1, eu-west-1, ap-south-1)\n\n\
            Press 'v' to cycle views",
            view_name
        );

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(INFO_COLOR))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Autopilot Status")
                    .border_style(Style::default().fg(BORDER_COLOR))
            );

        f.render_widget(header, area);
    }

    fn render_clusters(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let clusters = vec![
            ("prod-us-east-1", "AWS", "us-east-1", "Active", SUCCESS_COLOR),
            ("prod-eu-west-1", "AWS", "eu-west-1", "Active", SUCCESS_COLOR),
            ("prod-ap-south-1", "GCP", "ap-south-1", "Degraded", WARNING_COLOR),
            ("staging-us-west-2", "Azure", "us-west-2", "Active", SUCCESS_COLOR),
        ];

        let items: Vec<ListItem> = clusters
            .iter()
            .enumerate()
            .map(|(idx, (name, provider, region, state, color))| {
                let is_selected = idx == self.selected_cluster_index;
                let style = if is_selected {
                    Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(*color)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                let content = format!(
                    "{}{:<20} {:<6} {:<15} [{}]",
                    prefix, name, provider, region, state
                );
                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Managed Clusters [↑/↓: Select | v: Cycle View]")
                .border_style(Style::default().fg(BORDER_COLOR)),
        );

        f.render_widget(list, area);
    }

    fn render_cluster_details(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let details = "📊 Cluster: prod-us-east-1\n\n\
            Provider:        AWS\n\
            Region:          us-east-1\n\
            Nodes:           12/12 healthy\n\
            Pods:            456\n\
            CPU Usage:       45%\n\
            Memory Usage:    62%\n\
            Network:         ✅ Connected to 3 clusters";

        let paragraph = Paragraph::new(details)
            .style(Style::default().fg(TEXT_COLOR))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Cluster Details")
                    .border_style(Style::default().fg(BORDER_COLOR))
            );

        f.render_widget(paragraph, area);
    }

    fn render_topology(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let topology = "🗺️  Global Cluster Topology\n\n\
            ┌─────────────────────────────────────────────┐\n\
            │  us-east-1 ◄───────────► eu-west-1         │\n\
            │     │  (12ms)              │                │\n\
            │     │                      │                │\n\
            │     │                      │                │\n\
            │     ▼ (45ms)               ▼ (78ms)         │\n\
            │  ap-south-1 ◄──────► us-west-2             │\n\
            │              (95ms)                         │\n\
            └─────────────────────────────────────────────┘\n\n\
            Legend:\n\
            ◄───► Connected    (latency in ms)\n\
            ◄ ─ ► Degraded";

        let paragraph = Paragraph::new(topology)
            .style(Style::default().fg(INFO_COLOR))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Network Topology")
                    .border_style(Style::default().fg(BORDER_COLOR))
            )
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    fn render_topology_stats(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let stats = "📈 Topology Statistics\n\n\
            Connected Pairs:    6\n\
            Avg Latency:        55ms\n\
            Total Bandwidth:    10 Gbps\n\
            Regions:            3\n\
            Cloud Providers:    2 (AWS, GCP, Azure)";

        let paragraph = Paragraph::new(stats)
            .style(Style::default().fg(TEXT_COLOR))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Stats")
                    .border_style(Style::default().fg(BORDER_COLOR))
            );

        f.render_widget(paragraph, area);
    }

    fn render_syncs(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let syncs = vec![
            ("NetworkPolicy sync", "us-east-1 → eu-west-1", "In Progress", INFO_COLOR),
            ("CiliumPolicy sync", "us-east-1 → all", "Completed", SUCCESS_COLOR),
        ];

        let items: Vec<ListItem> = syncs
            .iter()
            .map(|(name, clusters, status, color)| {
                let content = format!("{:<25} {:<30} [{}]", name, clusters, status);
                ListItem::new(Line::from(Span::styled(
                    content,
                    Style::default().fg(*color),
                )))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Active Policy Syncs")
                .border_style(Style::default().fg(BORDER_COLOR)),
        );

        f.render_widget(list, area);
    }

    fn render_sync_details(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let details = "🔄 Sync: NetworkPolicy sync\n\n\
            Source:          us-east-1\n\
            Targets:         eu-west-1\n\
            Policy Type:     NetworkPolicy\n\
            Status:          In Progress (60%)\n\
            Started:         2 minutes ago\n\
            Policies Synced: 12/20";

        let paragraph = Paragraph::new(details)
            .style(Style::default().fg(TEXT_COLOR))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Sync Details")
                    .border_style(Style::default().fg(BORDER_COLOR))
            );

        f.render_widget(paragraph, area);
    }

    fn render_placements(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let placements = vec![
            ("payment-service", "us-east-1", "Lower latency", "95%", SUCCESS_COLOR),
            ("analytics-job", "ap-south-1", "Cost optimization", "82%", INFO_COLOR),
            ("cache-redis", "eu-west-1", "Region affinity", "88%", SUCCESS_COLOR),
        ];

        let items: Vec<ListItem> = placements
            .iter()
            .map(|(workload, cluster, reason, conf, color)| {
                let content = format!(
                    "{:<20} → {:<18} | {} ({})",
                    workload, cluster, reason, conf
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
                .title("Placement Recommendations")
                .border_style(Style::default().fg(BORDER_COLOR)),
        );

        f.render_widget(list, area);
    }

    fn render_placement_details(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let details = "📍 Placement: payment-service\n\n\
            Recommended:     us-east-1\n\
            Reason:          Lower latency to customers\n\
            Confidence:      95%\n\
            \n\
            Analysis:\n\
            • 45% of users in us-east region\n\
            • Latency improvement: 120ms → 15ms\n\
            • Resource availability: High";

        let paragraph = Paragraph::new(details)
            .style(Style::default().fg(TEXT_COLOR))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Placement Details")
                    .border_style(Style::default().fg(BORDER_COLOR))
            );

        f.render_widget(paragraph, area);
    }
}
