mod app;
mod http_checker;
mod http_stats;
mod parser;
mod pinger;
mod stats;
mod ui;

use anyhow::{Context, Result};
use app::App;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use stats::AppMode;
use std::io;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup panic hook to restore terminal
    setup_panic_hook();

    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut mode = AppMode::Icmp;
    let mut port = 80;
    let mut xml_path = "output.xml".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--http" => mode = AppMode::Http,
            "--port" => {
                i += 1;
                if i < args.len() {
                    port = args[i].parse().unwrap_or_else(|_| {
                        eprintln!("Invalid port number: {}", args[i]);
                        eprintln!("Using default port 80");
                        80
                    });
                } else {
                    eprintln!("--port requires a value");
                    return Ok(());
                }
            }
            path if !path.starts_with("--") => xml_path = path.to_string(),
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                eprintln!("Usage: tui-ether-pinger [--http] [--port PORT] [xml-file]");
                eprintln!("  --http          Use HTTP checking mode (default: ICMP ping)");
                eprintln!("  --port PORT     Port to check (default: 80, HTTP mode only)");
                return Ok(());
            }
        }
        i += 1;
    }

    // Parse XML file
    let ips = parser::parse_nmap_xml(&xml_path)
        .context(format!("Failed to parse nmap XML: {}", xml_path))?;

    if ips.is_empty() {
        eprintln!("No hosts found in {}. Please run nmap first.", xml_path);
        return Ok(());
    }

    // Create app with selected mode and port
    let mut app = App::new(ips, mode, port);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Shutdown app (stop all ping tasks)
    app.shutdown().await;

    result
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Render UI
        terminal.draw(|f| ui::render(app, f))?;

        // Handle events with timeout for periodic refresh
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key)?;

                if app.should_quit {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);

        // Call original hook
        original_hook(panic_info);
    }));
}
