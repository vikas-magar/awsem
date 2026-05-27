use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::theme;

pub fn render_confirm(frame: &mut Frame, area: Rect, item: &str) {
    let w = 60u16.min(area.width.saturating_sub(4));
    let h = 7u16.min(area.height.saturating_sub(4));
    let inner = Rect { x: (area.width - w) / 2, y: (area.height - h) / 2, width: w, height: h };
    frame.render_widget(Clear, inner);
    let lines = vec![Line::from(""), Line::from(Span::styled(format!(" Delete {}?", item), Style::default().fg(theme::FG))),
        Line::from(""), Line::from(Span::styled(" [Y]es  [N]o", theme::muted()))];
    frame.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme::ACCENT))), inner);
}

pub fn render_result(frame: &mut Frame, area: Rect, title: &str, msg: &str) {
    let is_err = msg.contains("error") || msg.contains("Error");
    let color = if is_err { theme::ERROR } else { theme::SUCCESS };
    let w = 60u16.min(area.width.saturating_sub(4));
    let h = 8u16.min(area.height.saturating_sub(4));
    let inner = Rect { x: (area.width - w) / 2, y: (area.height - h) / 2, width: w, height: h };
    frame.render_widget(Clear, inner);
    let lines = vec![
        Line::from(""), Line::from(Span::styled(title, Style::default().fg(color))),
        Line::from(""), Line::from(Span::styled(msg, Style::default().fg(theme::FG))),
        Line::from(""), Line::from(Span::styled("Press any key", theme::muted())),
    ];
    frame.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(color))).wrap(Wrap { trim: false }), inner);
}
