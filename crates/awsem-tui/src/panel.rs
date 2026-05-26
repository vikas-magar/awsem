use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::table::{self, Col};
use crate::theme;

pub struct PanelConfig<'a> {
    pub title: &'static str,
    pub summary: Vec<(String, String)>,
    pub cols: &'a [Col],
    pub rows: Vec<Vec<String>>,
    pub selected: usize,
    pub scroll: usize,
    pub filter: String,
    pub focus: bool,
}

pub fn render(frame: &mut Frame, area: Rect, cfg: &PanelConfig) {
    let bs = if cfg.focus { theme::SELECTED } else { theme::PANEL_BORDER };
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(bs));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut y = inner.y;
    frame.render_widget(Paragraph::new(Line::from(Span::styled(cfg.title, theme::header()))).wrap(Wrap { trim: false }), Rect::new(inner.x, y, inner.width, 1));
    y += 1;

    if !cfg.summary.is_empty() {
        let mut spans = Vec::new();
        for (i, (label, val)) in cfg.summary.iter().enumerate() {
            if i > 0 { spans.push(Span::styled("  ", theme::muted())); }
            spans.push(Span::styled(format!("{label}: "), Style::default().fg(theme::LABEL)));
            spans.push(Span::styled(val.clone(), Style::default().fg(theme::FG)));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false }), Rect::new(inner.x, y, inner.width, 1));
        y += 1;
    }

    let f = if cfg.filter.is_empty() { " filter: / to search".to_string() } else { format!(" filter: {}", cfg.filter) };
    frame.render_widget(Paragraph::new(Line::from(Span::styled(f, theme::muted()))).wrap(Wrap { trim: false }), Rect::new(inner.x, y, inner.width, 1));
    y += 1;

    let table_area = Rect::new(inner.x, y, inner.width, inner.height.saturating_sub(y - inner.y));
    table::render(frame, table_area, cfg.cols, &cfg.rows, cfg.selected, cfg.scroll, "", "");
}
