use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use crate::app::App;
use crate::theme;

fn focus_block(app: &App, idx: usize) -> Style {
    let c = if app.browser_focus == idx { theme::SELECTED } else { theme::PANEL_BORDER };
    Style::default().fg(c)
}

fn render_panel_bg(frame: &mut Frame, area: Rect, title: &str, app: &App, idx: usize) -> Rect {
    let block = Block::default().borders(Borders::ALL).border_style(focus_block(app, idx));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(Line::from(Span::styled(title, theme::header()))).wrap(Wrap { trim: false }), Rect::new(inner.x, inner.y, inner.width, 1));
    inner
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).border_style(theme::frame());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let vert = Layout::vertical([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]).split(inner);
    let mut y = inner.y;

    let path_parts: Vec<&str> = app.browser_path.trim_end_matches('/').split('/').filter(|s| !s.is_empty()).collect();
    let crumb = std::iter::once(app.browser_bucket.as_str()).chain(path_parts.iter().copied()).collect::<Vec<_>>().join(" ▸ ");
    frame.render_widget(Paragraph::new(Line::from(vec![
        Span::styled(" bucket: ", Style::default().fg(theme::LABEL)),
        Span::styled(&app.browser_bucket, Style::default().fg(theme::FG)),
        Span::styled(format!("  objects: {}", app.browser_items.len()), theme::muted()),
    ])), Rect::new(inner.x, y, inner.width, 1));
    y += 1;
    frame.render_widget(Paragraph::new(Line::from(Span::styled(format!(" {crumb}"), theme::accent()))), Rect::new(inner.x, y, inner.width, 1));
    y += 1;

    let horiz = Layout::horizontal([Constraint::Length(16), Constraint::Min(26), Constraint::Length(26)]).split(Rect::new(inner.x, y, inner.width, vert[1].height));

    // Navigator
    let nav = render_panel_bg(frame, horiz[0], " NAVIGATOR ", app, 0);
    let nr = (nav.height.saturating_sub(2)) as usize;
    for i in 0..nr.min(app.browser_prefixes.len()) {
        let idx = app.browser_prefix_scroll + i;
        if idx >= app.browser_prefixes.len() { break; }
        let sel = idx == app.browser_prefix_cursor && app.browser_focus == 0;
        let st = if sel { theme::selected() } else { Style::default().fg(theme::FG) };
        let trimmed = app.browser_prefixes[idx].trim_end_matches('/');
        let name = trimmed.rsplit('/').next().unwrap_or(trimmed);
        let ptr = if sel { "▶ " } else { "  " };
        frame.render_widget(Paragraph::new(Line::from(Span::styled(format!("{ptr}{name}"), st))), Rect::new(nav.x, nav.y + 1 + i as u16, nav.width, 1));
    }

    // Objects
    let objs = render_panel_bg(frame, horiz[1], " OBJECTS ", app, 1);
    let mut y = objs.y + 1;
    let avail = objs.width.saturating_sub(2);
    let size_w = (avail * 2 / 10).clamp(6, 10);
    let mod_w = (avail * 3 / 10).clamp(8, 14);
    let name_w = avail.saturating_sub(size_w + mod_w + 4).max(6);
    let hcols: [(&str, u16); 3] = [("Name", name_w), ("Size", size_w), ("Modified", mod_w)];
    let mut x = objs.x + 1;
    for (hdr, w) in &hcols {
        let sort = if *hdr == "Name" { if app.browser_sort_desc { " ▼" } else { " ▲" } } else { "" };
        frame.render_widget(Paragraph::new(Line::from(Span::styled(format!("{hdr}{sort}"), Style::default().fg(theme::LABEL)))), Rect::new(x, y, *w, 1));
        x += *w + 2;
    }
    y += 1;
    frame.render_widget(Paragraph::new(Line::from(Span::styled("─".repeat(objs.width as usize), theme::muted()))), Rect::new(objs.x, y, objs.width, 1));
    y += 1;

    let mr = (objs.y + objs.height).saturating_sub(y) as usize;
    let rows_shown = mr.min(app.browser_items.len());
    for ri in 0..rows_shown {
        let ii = app.browser_scroll + ri;
        if ii >= app.browser_items.len() { break; }
        let obj = &app.browser_items[ii];
        let sel = ii == app.browser_cursor && app.browser_focus == 1;
        let st = if sel { theme::selected() } else { Style::default().fg(theme::FG) };
        let icon = if obj.is_folder { "▶ " } else { "  " };
        let vals = [format!("{}{}", icon, obj.key), obj.size_str.clone(), obj.modified.clone()];
        let mut x = objs.x + 1;
        for (ci, (_, w)) in hcols.iter().enumerate() {
            let v = if vals[ci].len() > *w as usize { format!("{}…", &vals[ci][..*w as usize - 1]) } else { format!("{:width$}", vals[ci], width = *w as usize) };
            frame.render_widget(Paragraph::new(Line::from(Span::styled(v, st))), Rect::new(x, y + ri as u16, *w, 1));
            x += *w + 2;
        }
    }
    if app.browser_items.len() > mr {
        let pct = (app.browser_scroll + mr).min(app.browser_items.len()) * 100 / app.browser_items.len();
        frame.render_widget(Paragraph::new(Line::from(Span::styled(format!(" {}% ", pct), theme::muted()))).wrap(Wrap { trim: false }), Rect::new(objs.x + objs.width - 7, y + rows_shown as u16, 6, 1));
    }

    // Inspector
    let insp = render_panel_bg(frame, horiz[2], " INSPECTOR ", app, 2);
    if app.browser_focus == 1 && let Some(obj) = app.browser_items.get(app.browser_cursor) {
        let pairs: &[(&str, &str)] = if obj.is_folder { &[("Type", "folder"), ("Prefix", &obj.key)] } else {
            &[("Name", obj.key.rsplit('/').next().unwrap_or(&obj.key)), ("Size", &obj.size_str), ("Modified", &obj.modified), ("Storage", &obj.storage_class)]
        };
        for (i, (k, v)) in (insp.y + 1..).zip(pairs.iter()) {
            frame.render_widget(Paragraph::new(Line::from(vec![
                Span::styled(format!(" {k}: "), Style::default().fg(theme::LABEL)),
                Span::styled(*v, Style::default().fg(theme::FG)),
            ])), Rect::new(insp.x, i, insp.width, 1));
        }
        return;
    }
    frame.render_widget(Paragraph::new(Line::from(Span::styled(" No selection", theme::muted()))), Rect::new(insp.x, insp.y + 1, insp.width, 1));
}
