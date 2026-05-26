use crate::theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(" awsem TUI — Keys ", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(" ↑/↓  select    1-7  tab     Tab/→  next     S  toggle power"),
        Line::from(" r  refresh    ?  help    q  quit"),
        Line::from(""),
        Line::from(Span::styled(" Tab    c   d   e   i   u   s   f   Enter/Esc", theme::accent())),
        Line::from(" S3     •   •           •   •           •   folder nav"),
        Line::from(" Cognito   •   •"),
        Line::from(" Secrets   •   •   •"),
        Line::from(" EMR            •               •       enter VC/job"),
        Line::from(" Lambda        •       •"),
        Line::from(" Logs                  •"),
        Line::from(""),
        Line::from(" c=create  d=delete/cancel  e=edit  i=invoke  u=upload"),
        Line::from(" s=submit  f=filter"),
        Line::from(" Enter=open/folder in  Esc=back/up  d→y/N to confirm delete"),
    ];
    let w = std::cmp::min(60, area.width);
    let h = std::cmp::min(23, area.height);
    let inner = Rect { x: (area.width - w) / 2, y: (area.height - h) / 2, width: w, height: h };
    frame.render_widget(Clear, inner);
    frame.render_widget(Paragraph::new(lines).block(Block::default().title(" Help ").borders(Borders::ALL).border_style(theme::accent())).alignment(Alignment::Center), inner);
}
