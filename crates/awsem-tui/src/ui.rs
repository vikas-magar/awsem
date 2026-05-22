use crate::app::{App, Input, Tab};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};

pub fn render(frame: &mut Frame, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)]).split(frame.area());
    render_header(frame, vert[0], app); render_content(frame, vert[1], app); render_footer(frame, vert[2], app);
    if app.help_visible { render_help(frame, frame.area()); }
    if app.result.is_some() { render_result(frame, frame.area(), app); }
    if !matches!(app.input, Input::None) { render_input(frame, frame.area(), app); }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = Tab::ALL.iter().map(|t| {
        let sel = std::mem::discriminant(t) == std::mem::discriminant(&app.tab);
        Line::from(Span::styled(format!(" {} {} ", t.key(), t.name()), if sel { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) }))
    }).collect();
    let status = match &app.server_status { crate::server::ServerStatus::Running => "● Running", crate::server::ServerStatus::Stopped => "○ Stopped", crate::server::ServerStatus::Starting => "◌ Starting", crate::server::ServerStatus::Failed(_) => "✕ Error" };
    let title = format!(" awsem TUI · {}  {}", app.aws.endpoint, status);
    frame.render_widget(Tabs::new(titles).block(Block::default().title(title).borders(Borders::ALL)), area);
}

fn render_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.tab { Tab::Overview => crate::overview::render(frame, area, app), Tab::S3 => crate::s3::render(frame, area, app), Tab::Cognito => crate::cognito::render(frame, area, app), Tab::Secrets => crate::secrets::render(frame, area, app), Tab::Emr => crate::emr::render(frame, area, app), Tab::Lambda => crate::lambda::render(frame, area, app), Tab::Logs => crate::logs::render(frame, area, app), }
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let age = app.last_refresh.elapsed().as_secs();
    let text = if matches!(app.input, Input::None) {
        let s3 = if app.tab == Tab::S3 { if app.s3_bucket.is_empty() { "  Enter→open  " } else { "  Esc←back  d del  " } } else { "" };
        format!(" ↑↓ sel  r Ref  ? Help  S Pwr  q Quit  Tab⇄  |  {}{}{}", app.log_file, if age > 0 { format!("  {}s", age) } else { String::new() }, s3)
    } else { " Esc Cancel  Enter Submit  |  typing...".to_string() };
    frame.render_widget(Paragraph::new(text).style(Style::default().fg(if age > 30 { Color::Red } else { Color::DarkGray })), area);
}

fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    let prompt = match &app.input { Input::CreateUser => "New username:", Input::CreatePass(_) => "Password:", Input::LambdaPayload(_) => "Payload (JSON):", Input::S3UploadKey(_) => "Object key:", Input::EmrSubmit(_) => "Spark entry (s3://path):", Input::LogFilter => "Filter:", Input::CreateSecret => "Secret name:", Input::CreateSecretValue(_) => "Secret value:", Input::EditSecret(_) => "New secret value:", Input::CreateBucket => "Bucket name:", _ => return };
    let cursor = if app.input_buf.len() < 60 { format!("{}{}", app.input_buf, "█") } else { format!("...{}█", &app.input_buf[app.input_buf.len()-57..]) };
    let w = std::cmp::min(70, area.width);
    frame.render_widget(Paragraph::new(cursor).block(Block::default().title(format!(" {prompt} ")).borders(Borders::ALL).style(Style::default().bg(Color::Black))).wrap(Wrap { trim: false }), Rect { x: (area.width - w) / 2, y: area.height / 2 - 2, width: w, height: 3 });
}

fn render_result(frame: &mut Frame, area: Rect, app: &App) {
    let (title, msg) = app.result.as_ref().unwrap();
    let lines = vec![Line::from(""), Line::from(format!(" {title} ")).bold(), Line::from(""), Line::from(format!(" {msg}")), Line::from(""), Line::from(" Press any key to dismiss")];
    let w = std::cmp::min(70, area.width); let h = std::cmp::min(12, area.height);
    frame.render_widget(Paragraph::new(lines).block(Block::default().title(" Result ").borders(Borders::ALL).style(Style::default().bg(Color::Black))).alignment(Alignment::Center), Rect { x: (area.width - w) / 2, y: (area.height - h) / 2, width: w, height: h });
}

fn render_help(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""), Line::from(" awsem TUI — Keys").bold(), Line::from(""),
        Line::from(" ↑/↓  select   1-7  tab   Tab  next   S  pwr"), Line::from(" r  refresh   ?  help   q  quit"), Line::from(""),
        Line::from(" Tab   c   d   e   i   u   s   f   Enter/Esc"), Line::from(" S3    •   •       •   •           •      •"),
        Line::from(" Cognito  •   •"), Line::from(" Secrets  •   •   •"), Line::from(" EMR           •               •      •"),
        Line::from(" Lambda       •       •"), Line::from(" Logs                 •"),
        Line::from(""), Line::from(" c=create  d=delete/cancel  e=edit  i=invoke"), Line::from(" u=upload  s=submit  f=filter"), Line::from(" Enter=open/S3+EMR  Esc=cancel/back"),
    ];
    let w = std::cmp::min(58, area.width); let h = std::cmp::min(22, area.height);
    frame.render_widget(Paragraph::new(lines).block(Block::default().title(" Help ").borders(Borders::ALL).style(Style::default().bg(Color::Black))).alignment(Alignment::Center), Rect { x: (area.width - w) / 2, y: (area.height - h) / 2, width: w, height: h });
}
