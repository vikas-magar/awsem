use crate::app::App;
use crate::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let mode = if app.emr_vc_id.is_empty() { "VCs" } else { &app.emr_vc_id };
    let vc_count = app.emr_vcs.len();
    let job_count = app.emr_jobs.len();
    let vc_h = if job_count == 0 { Constraint::Min(0) } else { Constraint::Length(vc_count.min(5) as u16 + 2) };
    let vert = Layout::new(Direction::Vertical, [Constraint::Length(3), vc_h, Constraint::Min(0)]).split(area);
    frame.render_widget(Paragraph::new(format!(" EMR on EKS  |  {mode}  |  [s] submit  [d] del/cancel  [r] refresh  Enter/Esc")).style(theme::accent()), vert[0]);
    if app.emr_vcs.is_empty() && app.emr_jobs.is_empty() {
        return frame.render_widget(Paragraph::new("No virtual clusters — create one via AWS SDK").style(theme::muted()), vert[1]);
    }
    let vcs: Vec<ListItem> = app.emr_vcs.iter().enumerate().map(|(i, (id, name, state))| {
        let sel = !app.emr_vc_id.is_empty() && app.emr_vc_id == *id || (app.emr_vc_id.is_empty() && i == app.cursor);
        let style = if sel { theme::selected() } else { theme::status_style(state) };
        ListItem::new(format!(" {} {name:<20} {state}", if sel { "▶" } else { " " })).style(style)
    }).collect();
    frame.render_widget(List::new(vcs).block(Block::default().title("Virtual Clusters").borders(Borders::ALL).border_style(theme::muted())), vert[1]);
    if job_count > 0 {
        let jobs: Vec<ListItem> = app.emr_jobs.iter().enumerate().map(|(i, (_id, name, state))| {
            let sel = i == app.cursor && !app.emr_vc_id.is_empty();
            let style = if sel { theme::selected() } else { theme::status_style(state) };
            ListItem::new(format!(" {} {name:<30} {state}", if sel { "▶" } else { " " })).style(style)
        }).collect();
        frame.render_widget(List::new(jobs).block(Block::default().title(format!("Job Runs ({})", app.emr_vc_id)).borders(Borders::ALL).border_style(theme::muted())), vert[2]);
    }
}
