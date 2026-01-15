mod app;
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
use std::io;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup panic hook to restore terminal
    setup_panic_hook();

    // Parse XML file
    let xml_path = "output.xml";
    let ips = parser::parse_nmap_xml(xml_path)
        .context("Failed to parse nmap XML. Make sure output.xml exists.")?;

    if ips.is_empty() {
        eprintln!("No hosts found in {}. Please run nmap first.", xml_path);
        return Ok(());
    }

    // Create app
    let mut app = App::new(ips);

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
