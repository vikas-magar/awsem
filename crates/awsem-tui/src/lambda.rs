use crate::app::App;
use crate::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

fn fmt_runtime(r: &str) -> &str {
    r.trim_start_matches("Runtime").trim_start_matches('(').trim_end_matches(')')
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Min(0)]).split(area);
    frame.render_widget(Paragraph::new(" Lambda  |  [i] invoke  [d] delete  [r] refresh").style(theme::accent()), vert[0]);
    if app.lambda_funcs.is_empty() { return frame.render_widget(Paragraph::new("No functions").style(theme::muted()), vert[1]); }
    let items: Vec<ListItem> = app.lambda_funcs.iter().enumerate().map(|(i, (name, runtime, timeout))| {
        let style = if i == app.cursor { theme::selected() } else { Style::default() };
        ListItem::new(format!(" {} {:<25} {:<15} {}s", if i == app.cursor { "▶" } else { " " }, name, fmt_runtime(runtime), timeout)).style(style)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().title(format!("Functions ({})", app.lambda_funcs.len())).borders(Borders::ALL).border_style(theme::muted())), vert[1]);
}
