use crate::app::App;
use crate::stats::PingStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
    Frame,
};
use std::time::Duration;

pub fn render(app: &App, frame: &mut Frame) {
    let size = frame.area();

    // Create main layout: hosts on left, stats on right
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(size);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    render_host_list(app, frame, main_chunks[0]);
    render_stats_panel(app, frame, main_chunks[1]);
    render_help(frame, chunks[1]);
}

fn render_host_list(app: &App, frame: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = app
        .hosts
        .iter()
        .enumerate()
        .map(|(i, host)| {
            let checkbox = if host.selected { "[x]" } else { "[ ]" };
            let emoji = if host.selected { " 🔌" } else { "   " };
            let content = format!("{} {}{}", checkbox, host.ip, emoji);

            let style = if i == app.selected_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Hosts (Space: toggle, ↑↓: navigate)")
            .borders(Borders::ALL),
    );

    frame.render_widget(list, area);
}

fn render_stats_panel(app: &App, frame: &mut Frame, area: Rect) {
    let stats_lock = app.ping_stats.read();

    // Build table rows for selected hosts
    let rows: Vec<Row> = app
        .hosts
        .iter()
        .filter(|h| h.selected)
        .filter_map(|host| {
            stats_lock.get(&host.ip).map(|stats| {
                let status_style = match stats.status {
                    PingStatus::Active => Style::default().fg(Color::Green),
                    PingStatus::Timeout => Style::default().fg(Color::Red),
                    PingStatus::Unreachable => Style::default().fg(Color::Yellow),
                    PingStatus::NotStarted => Style::default().fg(Color::Gray),
                };

                Row::new(vec![
                    host.ip.to_string(),
                    format_status(&stats.status),
                    format_latency(stats.last_latency),
                    format_latency(stats.avg_latency),
                    format!("{:.1}%", stats.packet_loss_percent),
                    format!("{}/{}", stats.packets_received, stats.packets_sent),
                ])
                .style(status_style)
            })
        })
        .collect();

    let header = Row::new(vec!["IP", "Status", "Last", "Avg", "Loss", "Packets"])
        .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let widths = [
        Constraint::Length(20),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().title("Ping Statistics").borders(Borders::ALL));

    frame.render_widget(table, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = Line::from(vec![
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": quit | "),
        Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": navigate | "),
        Span::styled("Space", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": toggle | "),
        Span::styled("🔌", Style::default().fg(Color::Cyan)),
        Span::raw(": pinging"),
    ]);

    let paragraph = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

fn format_status(status: &PingStatus) -> String {
    match status {
        PingStatus::NotStarted => "Not Started".to_string(),
        PingStatus::Active => "Active".to_string(),
        PingStatus::Timeout => "Timeout".to_string(),
        PingStatus::Unreachable => "Unreachable".to_string(),
    }
}

fn format_latency(latency: Option<Duration>) -> String {
    match latency {
        Some(d) => format!("{:.1}ms", d.as_secs_f64() * 1000.0),
        None => "-".to_string(),
    }
}
