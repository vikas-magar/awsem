use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, hints: &str, age_secs: u64) {
    let age_str = if age_secs < 5 { String::new() }
        else if age_secs < 60 { format!(" {}s ago", age_secs) }
        else { format!(" {}m ago", age_secs / 60) };

    let text = if age_str.is_empty() { hints.to_string() }
        else { format!("{}  |  {}", hints, age_str) };

    let style = if age_secs > 60 { Style::default().fg(theme::ERROR) } else { theme::muted() };
    frame.render_widget(Paragraph::new(Line::from(Span::styled(text, style))), area);
}
