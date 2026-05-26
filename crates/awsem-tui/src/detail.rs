use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use crate::theme;

pub fn render(frame: &mut Frame, area: Rect, title: &str, lines: &[(String, String)]) {
    let w = 60u16.min(area.width.saturating_sub(4));
    let h = (lines.len() as u16 + 4).min(area.height.saturating_sub(4));
    let x = (area.width - w) / 2;
    let y = (area.height - h) / 2;
    let inner = Rect { x, y, width: w, height: h };
    frame.render_widget(Clear, inner);
    let mut items = vec![Line::from("")];
    for (k, v) in lines {
        items.push(Line::from(vec![
            Span::styled(format!(" {}: ", k), Style::default().fg(theme::LABEL)),
            Span::styled(v, Style::default().fg(theme::FG)),
        ]));
    }
    items.push(Line::from(""));
    items.push(Line::from(Span::styled(" Esc to dismiss", theme::muted())));
    let b = Block::default().borders(Borders::ALL).border_type(BorderType::Plain).border_style(theme::frame())
        .title(format!(" {title} "));
    frame.render_widget(Paragraph::new(items).block(b).wrap(Wrap { trim: false }), inner);
}
