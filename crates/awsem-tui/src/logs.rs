use crate::app::App;
use crate::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Min(0)]).split(area);
    let filter = if app.logs_filter.is_empty() { "none".into() } else { app.logs_filter.clone() };
    frame.render_widget(Paragraph::new(format!(" Logs  |  filter: {filter}  |  [f] set  [r] refresh")).style(theme::accent()), vert[0]);
    if app.logs.is_empty() { return frame.render_widget(Paragraph::new("No log entries — check the server is running").style(theme::muted()), vert[1]); }
    let items: Vec<ListItem> = app.logs.iter().enumerate().map(|(i, (ts, level, _target, msg))| {
        let lc = match level.as_str() { "INFO" => theme::SUCCESS, "WARN" => theme::WARN, "ERROR" => theme::ERROR, "DEBUG" => theme::ACCENT, "TRACE" => theme::MUTED, _ => theme::TEXT };
        let style = if i == app.cursor { Style::default().fg(lc).add_modifier(Modifier::BOLD) } else { Style::default().fg(lc) };
        ListItem::new(format!(" {} {ts} {level:<5} {msg}", if i == app.cursor { "▶" } else { " " })).style(style)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().title(format!("Entries ({})", app.logs.len())).borders(Borders::ALL).border_style(theme::muted())), vert[1]);
}
