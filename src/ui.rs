use crate::app::App;
use crate::http_stats::HttpStatus;
use crate::stats::{AppMode, PingStatus, Stats};
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

    match app.mode {
        AppMode::Icmp => render_ping_stats_panel(app, frame, main_chunks[1]),
        AppMode::Http => render_http_stats_panel(app, frame, main_chunks[1]),
    }

    render_help(app, frame, chunks[1]);
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

    // Count selected hosts
    let selected_count = app.hosts.iter().filter(|h| h.selected).count();
    let title = format!(
        "Hosts ({}/{})",
        selected_count,
        app.hosts.len()
    );

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL),
    );

    frame.render_widget(list, area);
}

fn render_ping_stats_panel(app: &App, frame: &mut Frame, area: Rect) {
    let stats_lock = app.stats.read();

    // Build table rows for selected hosts
    let rows: Vec<Row> = app
        .hosts
        .iter()
        .filter(|h| h.selected)
        .filter_map(|host| {
            if let Some(Stats::Ping(stats)) = stats_lock.get(&host.ip) {
                let status_style = match stats.status {
                    PingStatus::Active => Style::default().fg(Color::Green),
                    PingStatus::Timeout => Style::default().fg(Color::Red),
                    PingStatus::Unreachable => Style::default().fg(Color::Yellow),
                    PingStatus::NotStarted => Style::default().fg(Color::Gray),
                };

                Some(Row::new(vec![
                    host.ip.to_string(),
                    format_ping_status(&stats.status),
                    format_duration(stats.last_latency),
                    format_duration(stats.avg_latency),
                    format!("{:.1}%", stats.packet_loss_percent),
                    format!("{}/{}", stats.packets_received, stats.packets_sent),
                ])
                .style(status_style))
            } else {
                None
            }
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

fn render_http_stats_panel(app: &App, frame: &mut Frame, area: Rect) {
    let stats_lock = app.stats.read();

    // Build table rows for selected hosts
    let rows: Vec<Row> = app
        .hosts
        .iter()
        .filter(|h| h.selected)
        .filter_map(|host| {
            if let Some(Stats::Http(stats)) = stats_lock.get(&host.ip) {
                let status_style = match stats.status {
                    HttpStatus::Success => Style::default().fg(Color::Green),
                    HttpStatus::ClientError => Style::default().fg(Color::Yellow),
                    HttpStatus::ServerError | HttpStatus::NetworkError =>
                        Style::default().fg(Color::Red),
                    HttpStatus::NotStarted => Style::default().fg(Color::Gray),
                };

                Some(Row::new(vec![
                    host.ip.to_string(),
                    format_http_status(stats.last_status_code, &stats.status),
                    format_duration(stats.last_response_time),
                    format_duration(stats.avg_response_time),
                    format_size(stats.last_content_size),
                    format_error(&stats.last_error),
                ])
                .style(status_style))
            } else {
                None
            }
        })
        .collect();

    let header = Row::new(vec!["IP", "Status", "Last", "Avg", "Size", "Error"])
        .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let widths = [
        Constraint::Length(20),  // IP
        Constraint::Length(12),  // Status
        Constraint::Length(10),  // Last
        Constraint::Length(10),  // Avg
        Constraint::Length(10),  // Size
        Constraint::Length(30),  // Error
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().title("HTTP Statistics").borders(Borders::ALL));

    frame.render_widget(table, area);
}

fn render_help(app: &App, frame: &mut Frame, area: Rect) {
    let mode_text = match app.mode {
        AppMode::Icmp => "ICMP".to_string(),
        AppMode::Http => format!("HTTP:{}", app.port),
    };

    let mut spans = vec![
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": quit | "),
        Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": nav | "),
        Span::styled("Space", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": toggle | "),
        Span::styled("a", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": all | "),
        Span::styled("d", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": none | "),
        Span::styled("p", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": pause | "),
        Span::styled("s", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": export | "),
    ];

    if app.paused {
        spans.push(Span::styled("⏸ PAUSED", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        spans.push(Span::raw(" | "));
    }

    spans.push(Span::raw("Mode: "));
    spans.push(Span::styled(&mode_text, Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)));

    let help_text = Line::from(spans);
    let paragraph = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

fn format_ping_status(status: &PingStatus) -> String {
    match status {
        PingStatus::NotStarted => "Not Started".to_string(),
        PingStatus::Active => "Active".to_string(),
        PingStatus::Timeout => "Timeout".to_string(),
        PingStatus::Unreachable => "Unreachable".to_string(),
    }
}

fn format_http_status(code: Option<u16>, status: &HttpStatus) -> String {
    match code {
        Some(c) => {
            let text = match c {
                200 => "OK",
                404 => "Not Found",
                500 => "Server Err",
                _ => "",
            };
            if text.is_empty() {
                format!("{}", c)
            } else {
                format!("{} {}", c, text)
            }
        }
        None => format!("{:?}", status),
    }
}

fn format_duration(duration: Option<Duration>) -> String {
    match duration {
        Some(d) => format!("{:.1}ms", d.as_secs_f64() * 1000.0),
        None => "-".to_string(),
    }
}

fn format_size(size: Option<u64>) -> String {
    match size {
        Some(s) if s > 1024 * 1024 => format!("{:.1}MB", s as f64 / (1024.0 * 1024.0)),
        Some(s) if s > 1024 => format!("{:.1}KB", s as f64 / 1024.0),
        Some(s) => format!("{}B", s),
        None => "-".to_string(),
    }
}

fn format_error(error: &Option<String>) -> String {
    match error {
        Some(e) => {
            // Truncate long errors
            if e.len() > 28 {
                format!("{}...", &e[..28])
            } else {
                e.clone()
            }
        }
        None => "-".to_string(),
    }
}
