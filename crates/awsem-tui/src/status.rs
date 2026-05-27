use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, hints: &str, age_secs: u64, err: Option<&str>) {
    let age_str = if age_secs < 5 { String::new() }
        else if age_secs < 60 { format!(" {}s ago", age_secs) }
        else { format!(" {}m ago", age_secs / 60) };
    let warn = err.map(|e| format!(" ⚠ {e}")).unwrap_or_default();

    let text = format!("{hints}{warn}{}", if age_str.is_empty() { String::new() } else { format!("  |  {age_str}") });

    let style = if err.is_some() { Style::default().fg(theme::ERROR) } else if age_secs > 60 { Style::default().fg(theme::WARN) } else { theme::muted() };
    frame.render_widget(Paragraph::new(Line::from(Span::styled(text, style))), area);
}
