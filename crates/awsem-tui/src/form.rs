use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use crate::theme;

#[derive(Clone)]
pub struct FormField {
    pub label: &'static str,
    pub value: String,
    pub required: bool,
    pub sensitive: bool,
}

pub struct FormState {
    pub title: &'static str,
    pub fields: Vec<FormField>,
    pub focus: usize,
    pub action: FormAction,
    pub error: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum FormAction {
    CreateBucket, CreateUser, CreateSecret, EditSecret(String),
    InvokeLambda(String), EmrSubmit(String), LogFilter,
}

pub enum FormResult { Submitted, Cancelled, Continue }

pub fn init(action: &FormAction) -> FormState {
    let (title, fields) = match action {
        FormAction::CreateBucket => ("Create Bucket", vec![
            FormField { label: "Bucket Name", value: String::new(), required: true, sensitive: false },
        ]),
        FormAction::CreateUser => ("Create User", vec![
            FormField { label: "Username", value: String::new(), required: true, sensitive: false },
            FormField { label: "Email", value: String::new(), required: false, sensitive: false },
            FormField { label: "Temporary Password", value: String::new(), required: false, sensitive: true },
        ]),
        FormAction::CreateSecret => ("Create Secret", vec![
            FormField { label: "Secret Name", value: String::new(), required: true, sensitive: false },
            FormField { label: "Secret Value", value: String::new(), required: true, sensitive: true },
            FormField { label: "Description", value: String::new(), required: false, sensitive: false },
        ]),
        FormAction::EditSecret(_) => ("Edit Secret", vec![
            FormField { label: "New Value", value: String::new(), required: true, sensitive: true },
        ]),
        FormAction::InvokeLambda(_) => ("Invoke Lambda", vec![
            FormField { label: "Payload (JSON)", value: String::new(), required: true, sensitive: false },
        ]),
        FormAction::EmrSubmit(_) => ("Submit EMR Job", vec![
            FormField { label: "Entry Point (S3 URI)", value: String::new(), required: true, sensitive: false },
            FormField { label: "Execution Role ARN", value: "arn:aws:iam::123456789012:role/emr-role".into(), required: false, sensitive: false },
            FormField { label: "Release Label", value: "emr-7.5.0-latest".into(), required: false, sensitive: false },
            FormField { label: "Spark Submit Params", value: "--conf spark.executor.instances=1".into(), required: false, sensitive: false },
        ]),
        FormAction::LogFilter => ("Log Filter", vec![
            FormField { label: "Filter Pattern", value: String::new(), required: false, sensitive: false },
        ]),
    };
    FormState { title, fields, focus: 0, action: action.clone(), error: None }
}

pub fn render(frame: &mut Frame, area: Rect, form: &FormState) {
    let n = form.fields.len();
    let h = (n as u16 * 2 + 6).min(area.height.saturating_sub(4));
    let w = 60u16.min(area.width.saturating_sub(4));
    let x = (area.width - w) / 2;
    let y = (area.height - h) / 2;
    let inner = Rect { x, y, width: w, height: h };
    frame.render_widget(Clear, inner);

    let mut lines = vec![Line::from("")];
    for (i, field) in form.fields.iter().enumerate() {
        let prefix = if i == form.focus { " ▸ " } else { "   " };
        let req = if field.required { " *" } else { "" };
        let display = if field.sensitive && !field.value.is_empty() {
            "•".repeat(field.value.len())
        } else {
            let cursor = if i == form.focus { "█" } else { "" };
            format!("{}{}", field.value, cursor)
        };
        let label_style = if i == form.focus { Style::default().fg(theme::ACCENT) } else { Style::default().fg(theme::LABEL) };
        lines.push(Line::from(vec![
            Span::styled(format!("{}{}:{} ", prefix, field.label, req), label_style),
            Span::styled(display, Style::default().fg(theme::FG)),
        ]));
        lines.push(Line::from(Span::styled("".repeat(w as usize), theme::dim())));
    }
    if let Some(ref err) = form.error {
        lines.push(Line::from(Span::styled(format!(" ⚠ {}", err), Style::default().fg(theme::ERROR))));
    } else {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(" Tab/↑↓ navigate · Enter submit · Esc cancel", theme::muted())));

    let b = Block::default().borders(Borders::ALL).border_type(BorderType::Plain).border_style(theme::frame())
        .title(format!(" {} ", form.title));
    frame.render_widget(Paragraph::new(lines).block(b).wrap(Wrap { trim: false }), inner);
}

pub fn handle_key(form: &mut FormState, key: ratatui::crossterm::event::KeyCode) -> FormResult {
    match key {
        ratatui::crossterm::event::KeyCode::Esc => FormResult::Cancelled,
        ratatui::crossterm::event::KeyCode::Enter => {
            for (i, f) in form.fields.iter().enumerate() {
                if f.required && f.value.is_empty() {
                    form.focus = i;
                    form.error = Some(format!("{} is required", f.label));
                    return FormResult::Continue;
                }
            }
            form.error = None;
            FormResult::Submitted
        }
        ratatui::crossterm::event::KeyCode::Tab | ratatui::crossterm::event::KeyCode::Down => {
            form.focus = (form.focus + 1) % form.fields.len();
            form.error = None;
            FormResult::Continue
        }
        ratatui::crossterm::event::KeyCode::BackTab | ratatui::crossterm::event::KeyCode::Up => {
            form.focus = if form.focus == 0 { form.fields.len() - 1 } else { form.focus - 1 };
            form.error = None;
            FormResult::Continue
        }
        ratatui::crossterm::event::KeyCode::Backspace => {
            form.fields[form.focus].value.pop();
            FormResult::Continue
        }
        ratatui::crossterm::event::KeyCode::Char(c) => {
            form.fields[form.focus].value.push(c);
            FormResult::Continue
        }
        _ => FormResult::Continue,
    }
}
