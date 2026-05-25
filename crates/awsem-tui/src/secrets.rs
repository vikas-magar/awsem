use crate::app::App;
use crate::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Min(0)]).split(area);
    frame.render_widget(Paragraph::new(" Secrets  |  [c] create  [d] delete  [e] edit  [r] refresh").style(theme::accent()), vert[0]);
    if app.secrets.is_empty() { return frame.render_widget(Paragraph::new("No secrets — press [c] to create one").style(theme::muted()), vert[1]); }
    let items: Vec<ListItem> = app.secrets.iter().enumerate().map(|(i, (name, desc, changed))| {
        let style = if i == app.cursor { theme::selected() } else { Style::default() };
        ListItem::new(format!(" {} {:<30} {:<20} {}", if i == app.cursor { "▶" } else { " " }, name, desc, changed)).style(style)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().title(format!("Secrets ({})", app.secrets.len())).borders(Borders::ALL).border_style(theme::muted())), vert[1]);
}
