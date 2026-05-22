use crate::app::App;
use crate::server::ServerStatus;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

fn card(area: Rect, frame: &mut Frame, title: &str, value: &str, color: Color) {
    let block = Block::default().title(title).borders(Borders::ALL).style(Style::default().fg(color));
    frame.render_widget(Paragraph::new(value).block(block).centered().wrap(Wrap { trim: false }), area);
}

fn preview(stderr: &str, n: usize) -> String { stderr.lines().rev().take(n).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n") }

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    match &app.server_status {
        ServerStatus::Stopped | ServerStatus::Failed(_) | ServerStatus::Starting => {
            let stderr = app.server.stderr_snapshot();
            let mut msg = match &app.server_status {
                ServerStatus::Stopped => " awsem server is not running\n\n Press [S] to start the server".into(),
                ServerStatus::Failed(reason) => { let mut m = format!(" Failed: {reason}"); if !stderr.is_empty() { m.push_str(&format!("\n\n Last logs:\n{}", preview(&stderr, 4))); } m.push_str("\n\n Check log file for details\n\n Press [S] to retry"); m }
                ServerStatus::Starting => { let elapsed = app.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0); let mut m = format!(" Starting... ({elapsed}s)\n\n Press [S] to cancel"); if !stderr.is_empty() { m.push_str(&format!("\n\n {}", preview(&stderr, 3))); } m }
                _ => unreachable!(),
            };
            if !app.log_file.is_empty() { msg.push_str(&format!("\n\n Log: {}", app.log_file)); }
            frame.render_widget(Paragraph::new(msg).centered().style(Style::default().fg(Color::Yellow)), area); return;
        }
        _ => {}
    }
    let chunks = Layout::new(Direction::Vertical, [Constraint::Length(6), Constraint::Length(6), Constraint::Min(0)]).split(area);
    let top = Layout::new(Direction::Horizontal, [Constraint::Ratio(1, 4); 4]).split(chunks[0]);
    let mid = Layout::new(Direction::Horizontal, [Constraint::Ratio(1, 4), Constraint::Ratio(1, 4), Constraint::Ratio(2, 4)]).split(chunks[1]);
    let l = |v: &[String]| v.len().to_string();
    card(top[0], frame, "S3 Buckets", l(&app.s3_buckets).as_str(), Color::Cyan);
    card(top[1], frame, "Cognito Users", app.cognito_users.len().to_string().as_str(), Color::Green);
    card(top[2], frame, "Secrets", app.secrets.len().to_string().as_str(), Color::Magenta);
    card(top[3], frame, "Lambda Funcs", app.lambda_funcs.len().to_string().as_str(), Color::Blue);
    card(mid[0], frame, "EMR VCs", app.emr_vcs.len().to_string().as_str(), Color::Yellow);
    card(mid[1], frame, "Job Runs", app.emr_jobs.len().to_string().as_str(), Color::Red);
    let log_file = if app.log_file.is_empty() { "?".into() } else { app.log_file.clone() };
    card(mid[2], frame, "Server", format!(" Endpoint: {}\n Uptime: {}s\n Log: {}", app.aws.endpoint, app.last_refresh.elapsed().as_secs(), log_file).as_str(), Color::White);
    if let Some(ref e) = app.error { frame.render_widget(Paragraph::new(e.as_str()).style(Style::default().fg(Color::Red)), chunks[2]); }
}
