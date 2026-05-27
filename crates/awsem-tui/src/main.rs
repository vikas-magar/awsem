mod app;
mod aws_clients;
mod server;
mod types;
mod theme;
mod table;
mod header;
mod sidebar;
mod panel;
mod browser;
mod status;
mod fetchers;
mod actions;
mod help;
mod form;
mod detail;
mod upload_state;
mod upload;
mod dash_handler;
mod browse_handler;
mod app_actions;
mod event_loop;
mod ui;

use clap::Parser;
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use server::ServerStatus;
use std::io::stdout;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "http://localhost:4566")]
    endpoint: String,
    #[arg(long, default_value = "target/debug/awsem")]
    binary: String,
    #[arg(long)]
    args: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let aws = aws_clients::AwsClients::new(&args.endpoint).await;
    let mut app = app::App::new(aws, &args.binary, &args.args);
    app.server_status = if app.aws.check_health().await { ServerStatus::Running } else { ServerStatus::Stopped };
    if matches!(app.server_status, ServerStatus::Running) { app.refresh_all().await; }

    let res = event_loop::run(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::event::DisableMouseCapture)?;
    terminal.show_cursor()?;
    app.server.stop().await;
    if let Err(e) = res { eprintln!("Error: {e}"); }
    Ok(())
}
