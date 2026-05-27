use crate::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub struct SidebarItem {
    pub key: char,
    pub label: &'static str,
    pub count: usize,
}

pub fn render(frame: &mut Frame, area: Rect, items: &[SidebarItem], active: usize) {
    let block = Block::default().borders(Borders::RIGHT).border_style(theme::panel_border());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![Line::from(Span::styled(" AWS SERVICES ", theme::header()))];
    lines.push(Line::from(Span::styled("─".repeat(inner.width as usize), theme::dim())));

    for (i, item) in items.iter().enumerate() {
        let label = format!(" [{}] {} {}", item.key, item.label, item.count);
        let style = if i == active { theme::sidebar_active() } else { theme::sidebar() };
        lines.push(Line::from(Span::styled(label, style)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" SHORTCUTS ", theme::header())));
    lines.push(Line::from(Span::styled("─".repeat(inner.width as usize), theme::dim())));
    lines.push(Line::from(Span::styled(" [?] Help", theme::sidebar())));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(" [S] Start/Stop", theme::sidebar())));
    lines.push(Line::from(Span::styled(" [r] Refresh", theme::sidebar())));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
