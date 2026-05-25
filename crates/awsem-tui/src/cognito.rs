use crate::app::App;
use crate::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Min(0)]).split(area);
    frame.render_widget(Paragraph::new(" Cognito  |  [c] create  [d] delete  [r] refresh").style(theme::accent()), vert[0]);
    if app.cognito_users.is_empty() { return frame.render_widget(Paragraph::new("No users — press [c] to create one").style(theme::muted()), vert[1]); }
    let items: Vec<ListItem> = app.cognito_users.iter().enumerate().map(|(i, (name, status, email))| {
        let style = if i == app.cursor { theme::selected() } else { theme::status_style(status) };
        ListItem::new(format!(" {} {:<20} {:<15} {}", if i == app.cursor { "▶" } else { " " }, name, status, email)).style(style)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().title(format!("Users ({})", app.cognito_users.len())).borders(Borders::ALL).border_style(theme::muted())), vert[1]);
}
