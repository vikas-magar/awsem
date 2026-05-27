use crate::theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

pub fn render(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(" awsem TUI — Key Bindings ", Style::default().fg(theme::FRAME).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(" NAVIGATION ", theme::header())),
        Line::from(" ↑/↓          Select item in active panel"),
        Line::from(" ←/→          Switch between dashboard panels"),
        Line::from(" Tab           Next panel in browser"),
        Line::from(" 1-6           Jump to service panel"),
        Line::from(" Enter         Detail / drill into selected item"),
        Line::from(" Esc           Back / close"),
        Line::from(" Backspace     Parent directory (browser)"),
        Line::from(""),
        Line::from(Span::styled(" DASHBOARD ACTIONS ", theme::header())),
        Line::from(" c             Create (bucket/user/secret)"),
        Line::from(" d             Delete selected (with confirm)"),
        Line::from(" e             Edit secret value"),
        Line::from(" i             Invoke Lambda function"),
        Line::from(" s             Submit EMR job"),
        Line::from(" u             Upload to S3 bucket"),
        Line::from(" f             Filter logs"),
        Line::from(" r / R         Refresh all data"),
        Line::from(" S             Start/Stop awsem server"),
        Line::from(""),
        Line::from(Span::styled(" S3 BROWSER ", theme::header())),
        Line::from(" d             Download selected object"),
        Line::from(" D             Download all in current prefix"),
        Line::from(" u             Upload mode"),
        Line::from(" r / R         Refresh current view"),
        Line::from(""),
        Line::from(Span::styled(" UPLOAD MODE ", theme::header())),
        Line::from(" Space         Toggle selection"),
        Line::from(" Enter / u     Upload selected files"),
        Line::from(" Backspace     Parent directory"),
        Line::from(" Esc           Close upload panel"),
        Line::from(""),
        Line::from(Span::styled(" SEARCH & FILTER ", theme::header())),
        Line::from(" Type          Filter current panel items"),
        Line::from(" Backspace     Clear last filter character"),
        Line::from(""),
        Line::from(Span::styled(" OTHER ", theme::header())),
        Line::from(" ?             Toggle this help screen"),
        Line::from(" q / Q         Quit"),
    ];
    let w = area.width.saturating_sub(8).min(56);
    let h = lines.len() as u16 + 2;
    let inner = Rect { x: (area.width - w) / 2, y: (area.height - h) / 2, width: w, height: h };
    frame.render_widget(Clear, inner);
    frame.render_widget(Paragraph::new(lines).block(Block::default().borders(Borders::ALL).border_type(BorderType::Plain).border_style(theme::frame())).alignment(Alignment::Left), inner);
}
