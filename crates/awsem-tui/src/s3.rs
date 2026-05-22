use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Min(0)]).split(area);
    let mode = if app.s3_bucket.is_empty() { "buckets" } else { &app.s3_bucket };
    let hints = if app.s3_bucket.is_empty() { "[c] Create  [d] Delete Bucket" } else { "[u] Upload  [d] Delete" };
    frame.render_widget(Paragraph::new(format!(" S3 Browser  |  {mode}  |  {hints}  [r] Refresh  Enter↑↓Esc")).style(Style::default().fg(Color::Cyan)), vert[0]);
    if app.s3_bucket.is_empty() {
        if app.s3_buckets.is_empty() { return frame.render_widget(Paragraph::new("No buckets").style(Style::default().fg(Color::DarkGray)), area); }
        let items: Vec<ListItem> = app.s3_buckets.iter().enumerate().map(|(i, name)| {
            let style = if i == app.cursor { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default() };
            ListItem::new(format!(" {}  {}", if i == app.cursor { "▶" } else { " " }, name)).style(style)
        }).collect();
        frame.render_widget(List::new(items).block(Block::default().title("Buckets ▼").borders(Borders::ALL)), vert[1]);
    } else {
        if app.s3_objects.is_empty() { return frame.render_widget(Paragraph::new("No objects").style(Style::default().fg(Color::DarkGray)), area); }
        let items: Vec<ListItem> = app.s3_objects.iter().enumerate().map(|(i, (key, size))| {
            let style = if i == app.cursor { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default() };
            ListItem::new(format!(" {}  {:<50} {:>8}B", if i == app.cursor { "▶" } else { " " }, key, size)).style(style)
        }).collect();
        frame.render_widget(List::new(items).block(Block::default().title(format!("Objects in {}", app.s3_bucket)).borders(Borders::ALL)), vert[1]);
    }
}
