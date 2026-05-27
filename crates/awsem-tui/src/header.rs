use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::server::ServerStatus;
use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, endpoint: &str, status: &ServerStatus, selected: usize) {
    let status_dot = match status {
        ServerStatus::Running => ("●", theme::SUCCESS),
        ServerStatus::Stopped => ("○", theme::MUTED),
        ServerStatus::Starting => ("◌", theme::WARN),
        ServerStatus::Failed => ("✕", theme::ERROR),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" awsem DASHBOARD ", Style::default().fg(theme::FRAME)),
            Span::styled(format!(" {} ", endpoint), theme::info()),
            Span::styled(format!(" {} ", status_dot.0), Style::default().fg(status_dot.1)),
        ])),
        area,
    );
    let _ = selected;
}
