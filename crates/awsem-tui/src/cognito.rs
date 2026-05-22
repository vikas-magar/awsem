use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Min(0)]).split(area);
    frame.render_widget(Paragraph::new(" Cognito  |  [c] Create user  [d] Delete user  [r] Refresh").style(Style::default().fg(Color::Green)), vert[0]);
    if app.cognito_users.is_empty() { return frame.render_widget(Paragraph::new("No users").style(Style::default().fg(Color::DarkGray)), area); }
    let items: Vec<ListItem> = app.cognito_users.iter().enumerate().map(|(i, (name, status, email))| {
        let style = if i == app.cursor { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(if status == "CONFIRMED" || status == "Confirmed" { Color::Green } else { Color::Yellow }) };
        ListItem::new(format!(" {} {:<20} {:<15} {}", if i == app.cursor { "▶" } else { " " }, name, status, email)).style(style)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().title(format!("Users ({})", app.cognito_users.len())).borders(Borders::ALL)), vert[1]);
}
