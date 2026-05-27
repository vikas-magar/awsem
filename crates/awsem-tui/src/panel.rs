use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::table::{self, Col};
use crate::theme;
use crate::app::App;

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
            if i > 0 { spans.push(Span::styled("  ", theme::dim())); }
            spans.push(Span::styled(format!("{label}: "), Style::default().fg(theme::LABEL)));
            spans.push(Span::styled(val.clone(), Style::default().fg(theme::FG)));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false }), Rect::new(inner.x, y, inner.width, 1));
        y += 1;
    }

    let f = if cfg.filter.is_empty() { " filter: / to search".to_string() } else { format!(" filter: {}", cfg.filter) };
    frame.render_widget(Paragraph::new(Line::from(Span::styled(f, theme::info()))).wrap(Wrap { trim: false }), Rect::new(inner.x, y, inner.width, 1));
    y += 1;

    let table_area = Rect::new(inner.x, y, inner.width, inner.height.saturating_sub(y - inner.y));
    table::render(frame, table_area, cfg.cols, &cfg.rows, cfg.selected, cfg.scroll, "", "");
}

type ColSpec<'a> = (&'a str, f32, fn(&str, usize) -> String);

pub fn panel_cols(avail: u16, specs: &[ColSpec]) -> Vec<Col> {
    let a = avail.saturating_sub(2) as usize;
    let tr: f32 = specs.iter().map(|(_, r, _)| r).sum();
    let sp = 2 * (specs.len().saturating_sub(1));
    let mut used = 0usize;
    specs.iter().enumerate().map(|(i, (_label, ratio, align))| {
        let w = if i == specs.len() - 1 { a.saturating_sub(used + sp) } else { (a as f32 * ratio / tr).max(6.0) as usize };
        used += w + 2;
        Col { width: w, align: *align }
    }).collect()
}

pub fn get_dashboard_hints(active: usize) -> &'static str { match active {
    0 => "↑↓ · ↩ browse · c create · d delete · u upload · r refresh",
    1 => "↑↓ · s submit · d delete · r refresh", 2 => "↑↓ · c create · d delete · r refresh",
    3 => "↑↓ · c create · e edit · d delete · r refresh", 4 => "↑↓ · i invoke · d delete · r refresh",
    5 => "↑↓ · f filter · r refresh", _ => "↑↓ · r refresh · q quit",
    }
}

#[allow(clippy::too_many_arguments, clippy::redundant_closure)]
pub fn render_panel<T, F>(frame: &mut Frame, area: Rect, app: &App, idx: usize, global_filter: &str, title: &'static str, cols: &[Col], items: &[T], mapper: F, extra: &[(&str, usize)])
where F: Fn(&T) -> Vec<String> {
    let f = if idx == app.active_panel { global_filter } else { "" };
    let mut summary: Vec<(String, String)> = vec![("Total".into(), items.len().to_string())];
    for (k, v) in extra { summary.push((k.to_string(), v.to_string())); }
    let rows: Vec<Vec<String>> = items.iter().filter(|x| f.is_empty() || mapper(x).join(" ").contains(f)).map(|x| mapper(x)).collect();
    render(frame, area, &PanelConfig {
        title, summary, cols, rows, selected: app.panel_cursor(idx), scroll: app.panel_scroll(idx),
        filter: f.to_string(), focus: app.active_panel == idx,
    });
}
