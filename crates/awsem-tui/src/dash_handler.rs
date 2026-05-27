use crate::app::{App, ViewMode};
use crate::form::FormAction;
use crate::server::ServerStatus;
use crossterm::event::KeyCode;

pub async fn handle_dashboard(app: &mut App, c: KeyCode) {
    match c {
        KeyCode::Up if app.active_cursor() > 0 => { app.set_panel_cursor(app.active_panel, app.active_cursor() - 1); }
        KeyCode::Down if app.active_cursor() + 1 < app.active_panel_len() => { app.set_panel_cursor(app.active_panel, app.active_cursor() + 1); }
        KeyCode::Left if app.active_panel > 0 => { app.set_panel_scroll(app.active_panel, app.active_scroll()); app.active_panel -= 1; }
        KeyCode::Right if app.active_panel < 6 => { app.set_panel_scroll(app.active_panel, app.active_scroll()); app.active_panel += 1; }
        KeyCode::Enter if app.active_panel == 0 => {
            if let Some(b) = app.s3_buckets.get(app.active_cursor()) {
                        let items = crate::fetchers::s3_objects(&app.aws, &b.name, "").await;
                        app.browser_bucket = b.name.clone(); app.browser_path = String::new(); app.browser_items = items.clone();
                        app.browser_cursor = 0; app.browser_scroll = 0; app.browser_focus = 0; app.browser_sort_col = 0; app.browser_sort_desc = false;
                        app.mode = ViewMode::BucketBrowser;
            }
        }
        KeyCode::Enter if app.active_panel == 1 => {
            if let Some(x) = app.emr_vcs.get(app.active_cursor()) {
                let jobs = crate::fetchers::emr_jobs(&app.aws, &x.id).await;
                app.popup_title = format!("EMR Jobs — {}", x.name);
                app.popup_lines = jobs.iter().map(|j| {
                    let ec = j.exit_code.map(|c| c.to_string()).unwrap_or_else(|| "-".into());
                    (j.id.clone(), format!("{} | {} | exit={}", j.name, j.state, ec))
                }).collect();
                if app.popup_lines.is_empty() { app.popup_lines.push(("(no jobs)".into(), String::new())); }
                app.mode = ViewMode::DetailPopup;
            }
        }
        KeyCode::Enter if app.active_panel == 2 => {
            if let Some(x) = app.cognito_users.get(app.active_cursor()) {
                app.popup_title = "Cognito User Detail".into();
                app.popup_lines = vec![
                    ("Username".into(), x.username.clone()), ("Status".into(), x.status.clone()),
                    ("Email".into(), x.email.clone()), ("Created".into(), x.created.clone()),
                ];
                app.mode = ViewMode::DetailPopup;
            }
        }
        KeyCode::Enter if app.active_panel == 3 => {
            if let Some(x) = app.secrets.get(app.active_cursor()) {
                app.popup_title = "Secret Detail".into();
                app.popup_lines = vec![
                    ("Name".into(), x.name.clone()), ("ARN".into(), x.arn.clone()),
                    ("Description".into(), x.description.clone()),
                    ("Last Changed".into(), x.last_changed.clone()),
                    ("Rotation".into(), x.rotation.clone()), ("Status".into(), x.status.clone()),
                ];
                app.mode = ViewMode::DetailPopup;
            }
        }
        KeyCode::Enter if app.active_panel == 4 => {
            if let Some(x) = app.lambda_funcs.get(app.active_cursor()) {
                app.popup_title = "Lambda Function Detail".into();
                app.popup_lines = vec![
                    ("Name".into(), x.name.clone()), ("Runtime".into(), x.runtime.clone()),
                    ("Timeout".into(), format!("{}s", x.timeout)),
                    ("Handler".into(), x.handler.clone()),
                    ("Last Modified".into(), x.last_modified.clone()),
                    ("Memory".into(), format!("{}MB", x.memory)),
                ];
                app.mode = ViewMode::DetailPopup;
            }
        }
        _ => {}
    }
}

pub async fn handle_dashboard_chars(app: &mut App, c: char) {
    match c {
        '1'..='6' => { let i = (c as u8 - b'1') as usize; if i <= 5 { app.active_panel = i; } }
        'S' => if matches!(app.server_status, ServerStatus::Stopped | ServerStatus::Failed(_)) {
            app.server_status = ServerStatus::Starting;
            app.start_time = Some(std::time::Instant::now());
            if app.server.start().await.is_err() { app.server_status = ServerStatus::Failed("start failed".into()); app.start_time = None; }
        } else { app.server.stop().await; app.server_status = ServerStatus::Stopped; app.start_time = None; app.server_started = None; }
        'r' | 'R' => app.refresh_all().await,
        'c' if app.active_panel == 0 => app.open_form(FormAction::CreateBucket),
        'c' if app.active_panel == 2 => app.open_form(FormAction::CreateUser),
        'c' if app.active_panel == 3 => app.open_form(FormAction::CreateSecret),
        'd' => {
            let idx = app.active_cursor();
            let entry = match app.active_panel {
                0 => app.s3_buckets.get(idx).map(|x| (x.name.clone(), x.name.clone())),
                1 => app.emr_vcs.get(idx).map(|x| (x.id.clone(), x.name.clone())),
                2 => app.cognito_users.get(idx).map(|x| (x.username.clone(), x.username.clone())),
                3 => app.secrets.get(idx).map(|x| (x.name.clone(), x.name.clone())),
                4 => app.lambda_funcs.get(idx).map(|x| (x.name.clone(), x.name.clone())),
                _ => None,
            };
            if let Some((id, name)) = entry { app.confirm_id = id; app.confirm_item = name; app.confirm_panel = app.active_panel; app.mode = ViewMode::ConfirmPopup; }
        }
        'e' if app.active_panel == 3 => { if let Some(s) = app.secrets.get(app.active_cursor()) { app.open_form(FormAction::EditSecret(s.name.clone())); } }
        'i' if app.active_panel == 4 => { if let Some(f) = app.lambda_funcs.get(app.active_cursor()) { app.open_form(FormAction::InvokeLambda(f.name.clone())); } }
        's' if app.active_panel == 1 => { if let Some(vc) = app.emr_vcs.get(app.active_cursor()) { app.open_form(FormAction::EmrSubmit(vc.id.clone())); } }
        'u' if app.active_panel == 0 => {
            if let Some(b) = app.s3_buckets.get(app.active_cursor()) {
                app.browser_bucket = b.name.clone(); app.browser_path = String::new(); app.browser_items = Vec::new();
                app.mode = ViewMode::BucketBrowser; app.enter_upload_mode();
            }
        }
        'f' if app.active_panel == 5 => app.open_form(FormAction::LogFilter),
        _ if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '*' => app.global_filter.push(c),
        _ => {}
    }
}
