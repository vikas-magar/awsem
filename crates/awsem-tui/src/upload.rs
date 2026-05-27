pub use crate::upload_state::*;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};
use ratatui::Frame;
use crate::theme;

const SPINNER: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

pub fn render(frame: &mut Frame, area: Rect, state: &UploadState, bucket: &str, prefix: &str) {
    let block = Block::default().borders(Borders::ALL).border_style(theme::frame()).title(format!(" UPLOAD → s3://{bucket}/{prefix} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let mut y = inner.y;

    let p = state.path.to_string_lossy();
    frame.render_widget(Paragraph::new(Line::from(vec![Span::styled(" ", Style::default().fg(theme::FG)), Span::styled(&*p, Style::default().fg(theme::INFO))])).wrap(Wrap { trim: false }), Rect::new(inner.x, y, inner.width, 1));
    y += 1;
    frame.render_widget(Paragraph::new(Line::from(Span::styled("─".repeat(inner.width as usize), theme::dim()))), Rect::new(inner.x, y, inner.width, 1));
    y += 1;

    let half = inner.width as usize / 2;
    let list_w = half.saturating_sub(2);
    let xfer_w = inner.width as usize - list_w - 4;
    let list_area = Rect::new(inner.x + 1, y, list_w as u16, (inner.y + inner.height).saturating_sub(y + 2) as u16);
    let xfer_area = Rect::new(inner.x + list_w as u16 + 3, y, xfer_w as u16, list_area.height);

    let max_rows = (list_area.y + list_area.height).saturating_sub(list_area.y) as usize;
    let count = max_rows.min(state.entries.len().saturating_sub(state.scroll));
    for (ly, i) in (list_area.y..).zip((0..count).map(|i| state.scroll + i)) {
        let e = &state.entries[i];
        let sel = i == state.cursor;
        let picked = state.selected.contains(&i);
        let ptr = if sel { "▸" } else { " " };
        let icon = if picked { "▶" } else if e.is_dir { "📁" } else { " " };
        let st = if sel { theme::selected() } else { Style::default().fg(theme::FG) };
        frame.render_widget(Paragraph::new(Line::from(Span::styled(format!("{}{} {}  {}", ptr, icon, e.name, e.size_str), st))), Rect::new(list_area.x, ly, list_area.width, 1));
    }

    let max_ty = xfer_area.y + xfer_area.height;
    for (ty, t) in (xfer_area.y..).zip(state.transfers.iter()) {
        if ty >= max_ty { break; }
        let pct = t.progress_pct();
        let label = if t.is_done() {
            if t.error.lock().unwrap().is_some() { "✕ failed".into() } else { "✓".into() }
        } else {
            let chars: Vec<char> = SPINNER.chars().collect();
            let si = (state.spinner / 3) as usize % chars.len();
            format!("{} {:>3}%", chars[si], pct)
        };
        let st = if t.is_done() { if t.error.lock().unwrap().is_some() { theme::error() } else { theme::info() } } else { theme::warn() };
        frame.render_widget(Paragraph::new(Line::from(vec![Span::styled(&t.name, Style::default().fg(theme::FG)), Span::styled(format!(" {}", label), st)])), Rect::new(xfer_area.x, ty, xfer_area.width, 1));
        if !t.is_done() && pct > 0 {
            let bar_w = xfer_area.width.saturating_sub(t.name.len() as u16 + 8).min(20);
            frame.render_widget(Gauge::default().percent(pct as u16).style(theme::warn()).fg(theme::ACCENT).label(""), Rect::new(xfer_area.x + t.name.len() as u16 + 6, ty, bar_w, 1));
        }
    }

    let hint_y = inner.y + inner.height - 1;
    frame.render_widget(Paragraph::new(Line::from(Span::styled(" ↑↓ nav · space sel(dir=all) · → dir · ← parent · u↩ upload · Esc close ", theme::dim()))), Rect::new(inner.x, hint_y, inner.width, 1));
}
