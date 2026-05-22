use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), Constraint::Length(8), Constraint::Min(0)]).split(area);
    let mode = if app.emr_vc_id.is_empty() { "VCs" } else { &app.emr_vc_id };
    frame.render_widget(Paragraph::new(format!(" EMR on EKS  |  {mode}  |  [s] Submit  [d] Del/Cancel  [r] Ref  Enter/Esc")).style(Style::default().fg(Color::Yellow)), vert[0]);
    let vcs: Vec<ListItem> = app.emr_vcs.iter().enumerate().map(|(i, (id, name, state))| {
        let sel = app.emr_vc_id == *id || (app.emr_vc_id.is_empty() && i == app.cursor);
        let style = if sel { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(match state.as_str() { "RUNNING" | "Running" => Color::Green, "TERMINATED" | "Terminated" => Color::Red, _ => Color::Yellow }) };
        ListItem::new(format!(" {} {name:<20} {state}", if sel { "▶" } else { " " })).style(style)
    }).collect();
    frame.render_widget(List::new(vcs).block(Block::default().title("Virtual Clusters").borders(Borders::ALL)), vert[1]);
    let jobs: Vec<ListItem> = app.emr_jobs.iter().enumerate().map(|(i, (_id, name, state))| {
        let sel = i == app.cursor && !app.emr_vc_id.is_empty();
        let style = if sel { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(match state.as_str() { "COMPLETED" | "Completed" => Color::Green, "FAILED" | "Failed" => Color::Red, _ => Color::Yellow }) };
        ListItem::new(format!(" {} {name:<30} {state}", if sel { "▶" } else { " " })).style(style)
    }).collect();
    frame.render_widget(List::new(jobs).block(Block::default().title(format!("Job Runs ({})", app.emr_vc_id)).borders(Borders::ALL)), vert[2]);
}
