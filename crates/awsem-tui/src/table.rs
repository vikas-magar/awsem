use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::{Frame, layout::Rect, style::Style};
use crate::theme;

pub struct Col {
    pub width: usize,
    pub align: fn(&str, usize) -> String,
}

fn pad(s: &str, w: usize, align: fn(&str, usize) -> String) -> String {
    if s.len() > w { format!("{}…", &s[..w.saturating_sub(1)]) }
    else { align(s, w) }
}

pub fn left(s: &str, w: usize) -> String { format!("{:<w$}", s) }
pub fn right(s: &str, w: usize) -> String { format!("{:>w$}", s) }

#[allow(clippy::too_many_arguments)]
pub fn render(frame: &mut Frame, area: Rect, cols: &[Col], rows: &[Vec<String>], selected: usize, scroll: usize, _title: &str, _filter: &str) {
    let h = area.height as usize;
    let max_rows = h.saturating_sub(1);
    let rows_shown = max_rows.min(rows.len());
    if max_rows == 0 { return; }

    let col_spacing = 2;
    let col_w: Vec<usize> = cols.iter().map(|c| c.width).collect();
    let x_offsets: Vec<u16> = {
        let mut x = area.x;
        let mut off = Vec::new();
        for &w in &col_w { off.push(x); x += w as u16 + col_spacing as u16; }
        off
    };

    for row_idx in 0..rows_shown {
        let item_idx = scroll + row_idx;
        let row_y = area.y + row_idx as u16;
        if item_idx >= rows.len() { break; }
        let is_sel = item_idx == selected;
        for (ci, col) in cols.iter().enumerate() {
            let val = rows[item_idx].get(ci).map(|s| s.as_str()).unwrap_or("");
            let display = if is_sel && ci == 0 { format!("▸{}", val) } else { val.to_string() };
            let padded = pad(&display, col_w[ci], col.align);
            let style = if is_sel { theme::selected() } else { Style::default().fg(theme::FG) };
            frame.render_widget(Paragraph::new(Line::from(Span::styled(padded, style))).wrap(Wrap { trim: false }), Rect::new(x_offsets[ci], row_y, col_w[ci] as u16, 1));
        }
    }

    if rows.len() > max_rows {
        let pct = (scroll + max_rows).min(rows.len()) * 100 / rows.len();
        let indicator = format!(" {}% ", pct);
        frame.render_widget(Paragraph::new(Line::from(Span::styled(&indicator, theme::muted()))).wrap(Wrap { trim: false }), Rect::new(area.x + area.width - indicator.len() as u16 - 1, area.y + rows_shown as u16, indicator.len() as u16 + 1, 1));
    }
}
