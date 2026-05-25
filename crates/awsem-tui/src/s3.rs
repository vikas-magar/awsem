use crate::app::App;
use crate::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

fn fmt_modified(d: &str) -> &str { if d.len() >= 16 { &d[5..16] } else { d } }

fn render_col(frame: &mut Frame, area: Rect, app: &App, col: usize, title: &str) {
    let is_active = col == app.s3_col;
    let border = if is_active { theme::accent() } else { theme::muted() };
    let (folders, objects): (Vec<String>, &[(String, i64, String)]) = match col {
        0 => (app.s3_buckets.clone(), &[]),
        1 => (app.s3_folders.clone(), &app.s3_objects),
        _ => (app.s3_col3_folders.clone(), &app.s3_col3_objects),
    };
    let mut items: Vec<ListItem> = Vec::new();
    for (i, name) in folders.iter().enumerate() {
        let sel = is_active && i == app.cursor;
        let s = if sel { Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD) } else { theme::accent() };
        items.push(ListItem::new(format!(" +  {}/", name.trim_end_matches('/'))).style(s));
    }
    for (i, (key, size, modified)) in objects.iter().enumerate() {
        let idx = folders.len() + i;
        let sel = is_active && idx == app.cursor;
        let s = if sel { theme::selected() } else { Style::default() };
        let display = key.rsplit('/').next().unwrap_or(key);
        items.push(ListItem::new(format!(" {}  {:<35} {:>8}  {}", if sel { "▶" } else { " " }, display, theme::fmt_size(*size), fmt_modified(modified))).style(s));
    }
    frame.render_widget(Clear, area);
    frame.render_widget(List::new(items).block(Block::default().title(title).borders(Borders::ALL).border_style(border)), area);
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(1), Constraint::Min(0)]).split(area);
    if app.s3_bucket.is_empty() {
        frame.render_widget(Paragraph::new(" Buckets  |  [c] create  [d] delete  [r] refresh").style(theme::accent()), vert[0]);
        return render_col(frame, vert[1], app, 0, "Buckets");
    }
    let path = if app.s3_prefix.is_empty() { app.s3_bucket.clone() } else { format!("{}/{}", app.s3_bucket, app.s3_prefix) };
    frame.render_widget(Paragraph::new(format!(" {}  |  ←→ cols  u up  c cr  d del  r ref", path)).style(theme::accent()), vert[0]);
    let has_col3 = !app.s3_col3_prefix.is_empty();
    let col_specs = if has_col3 { vec![Constraint::Ratio(1, 4), Constraint::Ratio(1, 4), Constraint::Ratio(1, 2)] } else { vec![Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)] };
    let cols = Layout::new(Direction::Horizontal, &col_specs).split(vert[1]);
    render_col(frame, cols[0], app, 0, "Buckets");
    let t1 = if app.s3_prefix.is_empty() { &app.s3_bucket } else { app.s3_prefix.trim_end_matches('/') };
    render_col(frame, cols[1], app, 1, t1);
    if has_col3 { render_col(frame, cols[2], app, 2, app.s3_col3_prefix.trim_end_matches('/')); }
}
