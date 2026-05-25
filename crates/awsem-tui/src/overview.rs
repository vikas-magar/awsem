use crate::app::App;
use crate::server::ServerStatus;
use crate::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

fn card(area: Rect, frame: &mut Frame, title: &str, value: &str, color: Style) {
    let block = Block::default().title(title).borders(Borders::ALL).border_style(color);
    frame.render_widget(Paragraph::new(value).block(block).centered().wrap(Wrap { trim: false }), area);
}

fn preview(stderr: &str, n: usize) -> String {
    stderr.lines().rev().take(n).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
}

fn uptime(app: &App) -> String {
    const DASH: &str = "—";
    let start = match app.server_started { Some(s) => s, None => return DASH.into() };
    let secs = start.elapsed().as_secs();
    if secs < 60 { format!("{secs}s") }
    else if secs < 3600 { format!("{}m {}s", secs / 60, secs % 60) }
    else { format!("{}h {}m", secs / 3600, (secs % 3600) / 60) }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    match &app.server_status {
        ServerStatus::Stopped | ServerStatus::Failed(_) | ServerStatus::Starting => {
            let stderr = app.server.stderr_snapshot();
            let mut msg = match &app.server_status {
                ServerStatus::Stopped => "awsem server is not running\n\nPress [S] to start".into(),
                ServerStatus::Failed(reason) => { let mut m = format!("Failed: {reason}"); if !stderr.is_empty() { m.push_str(&format!("\n\nLast logs:\n{}", preview(&stderr, 4))); } m.push_str("\n\nPress [S] to retry"); m }
                ServerStatus::Starting => { let elapsed = app.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0); let mut m = format!("Starting... ({elapsed}s)\n\nPress [S] to cancel"); if !stderr.is_empty() { m.push_str(&format!("\n\n{}", preview(&stderr, 3))); } m }
                _ => unreachable!(),
            };
            if !app.log_file.is_empty() { msg.push_str(&format!("\n\nLog: {}", app.log_file)); }
            frame.render_widget(Paragraph::new(msg).centered().style(Style::default().fg(theme::WARN)), area); return;
        }
        _ => {}
    }
    let chunks = Layout::new(Direction::Vertical, [Constraint::Length(6), Constraint::Length(6), Constraint::Min(0)]).split(area);
    let top = Layout::new(Direction::Horizontal, [Constraint::Ratio(1, 4); 4]).split(chunks[0]);
    let mid = Layout::new(Direction::Horizontal, [Constraint::Ratio(1, 4), Constraint::Ratio(1, 4), Constraint::Ratio(2, 4)]).split(chunks[1]);
    card(top[0], frame, "S3 Buckets", &app.s3_buckets.len().to_string(), theme::accent());
    card(top[1], frame, "Cognito Users", &app.cognito_users.len().to_string(), theme::success());
    card(top[2], frame, "Secrets", &app.secrets.len().to_string(), Style::default().fg(ratatui::style::Color::Magenta));
    card(top[3], frame, "Lambda Funcs", &app.lambda_funcs.len().to_string(), ratatui::style::Color::Blue.into());
    card(mid[0], frame, "EMR VCs", &app.emr_vcs.len().to_string(), theme::warn());
    card(mid[1], frame, "Job Runs", &app.emr_jobs.len().to_string(), ratatui::style::Color::Red.into());
    card(mid[2], frame, "Server", &format!("Endpoint: {}\nUptime: {}\nLog: {}", app.aws.endpoint, uptime(app), if app.log_file.is_empty() { "?" } else { &app.log_file }), theme::muted());
    if let Some(ref e) = app.error {
        frame.render_widget(Paragraph::new(e.as_str()).style(Style::default().fg(theme::ERROR)), chunks[2]);
    }
}
