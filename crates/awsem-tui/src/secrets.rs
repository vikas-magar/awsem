use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Min(0)]).split(area);
    frame.render_widget(Paragraph::new(" Secrets  |  [c] Create  [d] Delete  [e] Edit value  [r] Refresh").style(Style::default().fg(Color::Magenta)), vert[0]);
    if app.secrets.is_empty() { return frame.render_widget(Paragraph::new("No secrets").style(Style::default().fg(Color::DarkGray)), area); }
    let items: Vec<ListItem> = app.secrets.iter().enumerate().map(|(i, (name, desc, changed))| {
        let style = if i == app.cursor { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default() };
        ListItem::new(format!(" {} {:<30} {:<20} {}", if i == app.cursor { "▶" } else { " " }, name, desc, changed)).style(style)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().title(format!("Secrets ({})", app.secrets.len())).borders(Borders::ALL)), vert[1]);
}
