use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, ViewMode};
use crate::browser;
use crate::header;
use crate::panel::{self, PanelConfig};
use crate::sidebar::{self, SidebarItem};
use crate::status;
use crate::table::{Col, left, right};
use crate::theme;

type ColSpec<'a> = (&'a str, f32, fn(&str, usize) -> String);

fn panel_cols(avail: u16, specs: &[ColSpec]) -> Vec<Col> {
    let a = avail.saturating_sub(2) as usize;
    let tr: f32 = specs.iter().map(|(_, r, _)| r).sum();
    let sp = 2 * (specs.len().saturating_sub(1));
    let mut used = 0usize;
    specs.iter().enumerate().map(|(i, (_label, ratio, align))| {
        let w = if i == specs.len() - 1 { a.saturating_sub(used + sp) } else { (a as f32 * ratio / tr).max(6.0) as usize };
        used += w + 2;
        Col { width: w, align: *align }
    }).collect()
}

pub fn render(frame: &mut Frame, app: &App) {
    let outer = Block::default().borders(Borders::ALL).border_type(BorderType::Plain).border_style(theme::frame());
    let inner = outer.inner(frame.area());
    frame.render_widget(outer, frame.area());

    let vert = Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).split(inner);
    header::render(frame, vert[0], &app.aws.endpoint, &app.server_status, 0);

    match app.mode {
        ViewMode::Dashboard => render_dashboard(frame, vert[1], app, app.active_panel, &app.global_filter),
        ViewMode::BucketBrowser => browser::render(frame, vert[1], app),
        ViewMode::UploadMode => {
            let split = Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(vert[1]);
            browser::render(frame, split[0], app);
            crate::upload::render(frame, split[1], &app.upload, &app.browser_bucket, &app.browser_path);
        }
        ViewMode::DetailPopup => crate::detail::render(frame, vert[1], &app.popup_title, &app.popup_lines),
        ViewMode::ConfirmPopup => {
            render_dashboard(frame, vert[1], app, app.active_panel, &app.global_filter);
            render_confirm(frame, frame.area(), &app.confirm_item);
        }
        ViewMode::FormPopup => {
            render_dashboard(frame, vert[1], app, app.active_panel, &app.global_filter);
            crate::form::render(frame, frame.area(), &app.form);
        }
    }

    let age = app.last_refresh.elapsed().as_secs();
    let hints = match app.mode {
        ViewMode::Dashboard => get_dashboard_hints(app.active_panel),
        ViewMode::BucketBrowser => "↑↓ · ↩ drill · ← back · d dl · D dl all · s sort · u upload · r refresh · q quit",
        ViewMode::UploadMode => "↑↓ · space select · Enter dir · u upload · ← parent · Esc close",
        ViewMode::DetailPopup => "Esc/Enter to dismiss",
        ViewMode::ConfirmPopup => "Y/Enter confirm · N/Esc cancel",
        ViewMode::FormPopup => "Tab/↑↓ navigate · Enter submit · Esc cancel",
    };
    status::render(frame, vert[2], hints, age, app.error.as_deref());

    if app.help_visible { crate::help::render(frame, frame.area()); }
    if let Some((title, msg)) = &app.result { render_result(frame, frame.area(), title, msg); }
}

fn render_dashboard(frame: &mut Frame, area: Rect, app: &App, active: usize, filter: &str) {
    let body = Layout::horizontal([Constraint::Length(18), Constraint::Min(0)]).split(area);
    let sb_items = vec![
        SidebarItem { key: '1', label: "S3", count: app.s3_buckets.len() },
        SidebarItem { key: '2', label: "EMR", count: app.emr_vcs.len() },
        SidebarItem { key: '3', label: "Cognito", count: app.cognito_users.len() },
        SidebarItem { key: '4', label: "Secrets", count: app.secrets.len() },
        SidebarItem { key: '5', label: "Lambda", count: app.lambda_funcs.len() },
        SidebarItem { key: '6', label: "Logs", count: app.logs.len() },
    ];
    sidebar::render(frame, body[0], &sb_items, active.min(5));

    let rows = Layout::vertical([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)]).split(body[1]);
    let top = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[0]);
    let mid = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[1]);
    let s3c = panel_cols(top[0].width, &[("Name", 6.0, left), ("Created", 4.0, left)]);
    render_panel(frame, top[0], app, 0, filter, " S3 BUCKETS ", &s3c,
        &app.s3_buckets, |b: &crate::types::S3Bucket| vec![b.name.clone(), b.created.clone()], &[]);
    let emrc = panel_cols(top[1].width, &[("Name", 6.0, left), ("State", 4.0, left)]);
    render_panel(frame, top[1], app, 1, filter, " EMR VIRTUAL CLUSTERS ", &emrc,
        &app.emr_vcs, |v: &crate::types::EmrVc| vec![v.name.clone(), v.state.clone()], &[("Running", app.emr_vcs.iter().filter(|v| v.state.contains("RUNNING")).count())]);
    let cogc = panel_cols(mid[0].width, &[("Username", 4.0, left), ("Status", 3.0, left), ("Email", 3.0, left)]);
    render_panel(frame, mid[0], app, 2, filter, " COGNITO USERS ", &cogc,
        &app.cognito_users, |u: &crate::types::CognitoUser| vec![u.username.clone(), u.status.clone(), u.email.clone()], &[]);
    let secc = panel_cols(mid[1].width, &[("Name", 5.0, left), ("Rot", 2.0, left), ("Status", 3.0, left)]);
    render_panel(frame, mid[1], app, 3, filter, " SECRETS MANAGER ", &secc,
        &app.secrets, |s: &crate::types::SecretEntry| vec![s.name.clone(), s.rotation.clone(), s.status.clone()], &[("Rotation Enabled", app.secrets.iter().filter(|s| s.rotation == "ENABLED").count())]);
    let bottom = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[2]);
    let lamc = panel_cols(bottom[0].width, &[("Name", 5.0, left), ("Runtime", 3.0, left), ("Timeout", 2.0, right)]);
    render_panel(frame, bottom[0], app, 4, filter, " LAMBDA FUNCTIONS ", &lamc,
        &app.lambda_funcs, |f: &crate::types::LambdaFn| vec![f.name.clone(), f.runtime.clone(), format!("{}s", f.timeout)], &[]);
    let logc = panel_cols(bottom[1].width, &[("Level", 2.0, left), ("Timestamp", 3.0, left), ("Message", 5.0, left)]);
    render_panel(frame, bottom[1], app, 5, filter, " LOGS ", &logc,
        &app.logs, |l| vec![l.level.clone(), l.timestamp.clone(), l.message.clone()], &[]);
}

fn get_dashboard_hints(active: usize) -> &'static str { match active {
        0 => "↑↓ · ↩ browse · c create · d delete · u upload · r refresh",
        1 => "↑↓ · s submit · d delete · r refresh", 2 => "↑↓ · c create · d delete · r refresh",
        3 => "↑↓ · c create · e edit · d delete · r refresh", 4 => "↑↓ · i invoke · d delete · r refresh",
        5 => "↑↓ · f filter · r refresh", _ => "↑↓ · r refresh · q quit",
    }
}

#[allow(clippy::too_many_arguments, clippy::redundant_closure)]
fn render_panel<T, F>(frame: &mut Frame, area: Rect, app: &App, idx: usize, global_filter: &str, title: &'static str, cols: &[Col], items: &[T], mapper: F, extra: &[(&str, usize)])
where F: Fn(&T) -> Vec<String> {
    let f = if idx == app.active_panel { global_filter } else { "" };
    let mut summary: Vec<(String, String)> = vec![("Total".into(), items.len().to_string())];
    for (k, v) in extra { summary.push((k.to_string(), v.to_string())); }
    let rows: Vec<Vec<String>> = items.iter().filter(|x| f.is_empty() || mapper(x).join(" ").contains(f)).map(|x| mapper(x)).collect();
    panel::render(frame, area, &PanelConfig {
        title, summary, cols, rows, selected: app.panel_cursor(idx), scroll: app.panel_scroll(idx),
        filter: f.to_string(), focus: app.active_panel == idx,
    });
}

fn render_confirm(frame: &mut Frame, area: Rect, item: &str) {
    let w = 60u16.min(area.width.saturating_sub(4));
    let h = 7u16.min(area.height.saturating_sub(4));
    let inner = Rect { x: (area.width - w) / 2, y: (area.height - h) / 2, width: w, height: h };
    frame.render_widget(Clear, inner);
    let lines = vec![Line::from(""), Line::from(Span::styled(format!(" Delete {}?", item), Style::default().fg(theme::FG))),
        Line::from(""), Line::from(Span::styled(" [Y]es  [N]o", theme::muted()))];
    frame.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme::ACCENT))), inner);
}

fn render_result(frame: &mut Frame, area: Rect, title: &str, msg: &str) {
    let is_err = msg.contains("error") || msg.contains("Error");
    let color = if is_err { theme::ERROR } else { theme::SUCCESS };
    let w = 60u16.min(area.width.saturating_sub(4));
    let h = 8u16.min(area.height.saturating_sub(4));
    let inner = Rect { x: (area.width - w) / 2, y: (area.height - h) / 2, width: w, height: h };
    frame.render_widget(Clear, inner);
    let lines = vec![
        Line::from(""), Line::from(Span::styled(title, Style::default().fg(color))),
        Line::from(""), Line::from(Span::styled(msg, Style::default().fg(theme::FG))),
        Line::from(""), Line::from(Span::styled("Press any key", theme::muted())),
    ];
    frame.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(color))).wrap(Wrap { trim: false }), inner);
}
