use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Min(0)]).split(area);
    frame.render_widget(Paragraph::new(" Lambda  |  [i] Invoke  [d] Delete  [r] Refresh").style(Style::default().fg(Color::Blue)), vert[0]);
    if app.lambda_funcs.is_empty() { return frame.render_widget(Paragraph::new("No functions").style(Style::default().fg(Color::DarkGray)), area); }
    let items: Vec<ListItem> = app.lambda_funcs.iter().enumerate().map(|(i, (name, runtime, timeout))| {
        let style = if i == app.cursor { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default() };
        ListItem::new(format!(" {} {:<25} {:<15} {}s", if i == app.cursor { "▶" } else { " " }, name, runtime, timeout)).style(style)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().title(format!("Functions ({})", app.lambda_funcs.len())).borders(Borders::ALL)), vert[1]);
}
