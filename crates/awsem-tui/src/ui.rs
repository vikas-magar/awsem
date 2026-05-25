use crate::app::{App, Input, Tab};
use crate::server::ServerStatus;
use crate::theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap};

pub fn render(frame: &mut Frame, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)]).split(frame.area());
    render_header(frame, vert[0], app); render_content(frame, vert[1], app); render_footer(frame, vert[2], app);
    if app.help_visible { crate::help::render(frame, frame.area()); }
    if app.result.is_some() { render_result(frame, frame.area(), app); }
    if !matches!(app.input, Input::None) { render_input(frame, frame.area(), app); }
    if app.confirming.is_some() { render_confirm(frame, frame.area(), app); }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = Tab::ALL.iter().map(|t| {
        let sel = std::mem::discriminant(t) == std::mem::discriminant(&app.tab);
        let label = format!(" {} {} ", t.key(), t.name());
        let style = if sel { Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme::MUTED) };
        Line::from(Span::styled(label, style))
    }).collect();
    let status = match &app.server_status {
        ServerStatus::Running => "● Running",
        ServerStatus::Stopped => "○ Stopped",
        ServerStatus::Starting => "◌ Starting",
        ServerStatus::Failed(_) => "✕ Error",
    };
    let status_color = match &app.server_status {
        ServerStatus::Running => theme::SUCCESS,
        ServerStatus::Stopped => theme::MUTED,
        ServerStatus::Starting => theme::WARN,
        ServerStatus::Failed(_) => theme::ERROR,
    };
    let title = Line::from(vec![
        Span::styled(" awsem ", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", app.aws.endpoint), theme::muted()),
        Span::styled(status, Style::default().fg(status_color)),
    ]);
    let block = Block::default().title(title).borders(Borders::ALL).border_style(theme::muted());
    frame.render_widget(Tabs::new(titles).block(block).divider(" ").style(theme::muted()), area);
}

fn render_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.tab {
        Tab::Overview => crate::overview::render(frame, area, app),
        Tab::S3 => crate::s3::render(frame, area, app),
        Tab::Cognito => crate::cognito::render(frame, area, app),
        Tab::Secrets => crate::secrets::render(frame, area, app),
        Tab::Emr => crate::emr::render(frame, area, app),
        Tab::Lambda => crate::lambda::render(frame, area, app),
        Tab::Logs => crate::logs::render(frame, area, app),
    }
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let age = app.last_refresh.elapsed().as_secs();
    let text = if matches!(app.input, Input::None) {
        let mut parts = vec!["↑↓ sel".to_string(), "r ref".to_string(), "? help".to_string(), "S pwr".to_string(), "q quit".to_string()];
        match app.tab {
            Tab::S3 => {
                if app.s3_bucket.is_empty() { parts.push("c create".into()); }
                else { parts.push("u upload".into()); }
                if !app.s3_bucket.is_empty() && !app.s3_prefix.is_empty() { parts.push("Esc up".into()); }
                else { parts.push("Esc back".into()); }
            }
            Tab::Cognito => { parts.push("c create".into()); parts.push("d delete".into()); }
            Tab::Secrets => { parts.push("c create".into()); parts.push("d delete".into()); parts.push("e edit".into()); }
            Tab::Emr => { parts.push("s submit".into()); parts.push("d del/cancel".into()); }
            Tab::Lambda => { parts.push("i invoke".into()); parts.push("d delete".into()); }
            Tab::Logs => { parts.push("f filter".into()); }
            _ => {}
        }
        if app.log_file.is_empty() { parts.push(format!("{}s", age)); }
        else { parts.push(format!("{} {}s", &app.log_file[..app.log_file.len().min(20)], age)); }
        parts.join("  |  ")
    } else { "Esc cancel  Enter submit  |  typing...".into() };
    let style = if age > 30 && matches!(app.input, Input::None) { Style::default().fg(theme::ERROR) } else { theme::muted() };
    frame.render_widget(Paragraph::new(text).style(style), area);
}

fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    let prompt = match &app.input {
        Input::CreateUser => "New username:",
        Input::CreatePass(_) => "Password:",
        Input::LambdaPayload(_) => "Payload (JSON):",
        Input::S3UploadKey(..) => "Local file path:",
        Input::EmrSubmit(_) => "Entry point (s3://path):",
        Input::LogFilter => "Filter:",
        Input::CreateSecret => "Secret name:",
        Input::CreateSecretValue(_) => "Secret value:",
        Input::EditSecret(_) => "New value:",
        Input::CreateBucket => "Bucket name:",
        _ => return,
    };
    let cursor = if app.input_buf.len() < 60 { format!("{}{}", app.input_buf, "█") } else { format!("...{}█", &app.input_buf[app.input_buf.len()-57..]) };
    let w = std::cmp::min(70, area.width);
    let inner = Rect { x: (area.width - w) / 2, y: area.height / 2 - 2, width: w, height: 3 };
    frame.render_widget(Clear, inner);
    frame.render_widget(Paragraph::new(cursor).block(Block::default().title(format!(" {prompt} ")).borders(Borders::ALL).border_style(theme::accent())).wrap(Wrap { trim: false }), inner);
}

fn render_result(frame: &mut Frame, area: Rect, app: &App) {
    let (title, msg) = app.result.as_ref().unwrap();
    let is_err = msg.starts_with("ServiceError") || msg.starts_with("Unknown") || msg.contains("error") || msg.contains("Error");
    let color = if is_err { theme::ERROR } else { theme::SUCCESS };
    let lines = vec![
        Line::from(""),
        Line::from(format!(" {} ", title)).style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from(format!(" {}", msg)).style(Style::default().fg(theme::TEXT)),
        Line::from(""),
        Line::from(" Press any key to dismiss").style(theme::muted()),
    ];
    let w = std::cmp::min(70, area.width); let h = std::cmp::min(12, area.height);
    let inner = Rect { x: (area.width - w) / 2, y: (area.height - h) / 2, width: w, height: h };
    frame.render_widget(Clear, inner);
    frame.render_widget(Paragraph::new(lines).block(Block::default().title(" Result ").borders(Borders::ALL).border_style(Style::default().fg(color))).alignment(Alignment::Center), inner);
}

fn render_confirm(frame: &mut Frame, area: Rect, app: &App) {
    let (msg, _) = app.confirming.as_ref().unwrap();
    let lines = vec![
        Line::from(""),
        Line::from(format!(" {} ", msg)).style(Style::default().fg(theme::WARN).add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from(" (y)es  /  (n)o ").style(theme::muted()),
    ];
    let w = std::cmp::min(60, area.width);
    let h = std::cmp::min(7, area.height);
    let inner = Rect { x: (area.width - w) / 2, y: (area.height - h) / 2, width: w, height: h };
    frame.render_widget(Clear, inner);
    frame.render_widget(Paragraph::new(lines).block(Block::default().title(" Confirm ").borders(Borders::ALL).border_style(theme::warn())).alignment(Alignment::Center), inner);
}
