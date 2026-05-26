use crate::app::{App, ViewMode};
use crate::form::{FormAction, FormResult};
use crate::server::ServerStatus;
use crate::ui;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::time::{Duration, Instant};

pub async fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App) -> anyhow::Result<()> {
    let mut last_auto = Instant::now();
    loop {
        terminal.draw(|f| ui::render(f, app))?;
        let timeout = if matches!(app.server_status, ServerStatus::Starting) { Duration::from_millis(200) } else { Duration::from_millis(100) };

        if matches!(app.server_status, ServerStatus::Starting) && app.health_checked.elapsed().as_secs() >= 2 {
            app.health_checked = Instant::now();
            if app.aws.check_health().await { app.server_status = ServerStatus::Running; app.server_started = Some(Instant::now()); app.start_time = None; app.refresh_all().await; }
            else if app.start_time.is_some_and(|t| t.elapsed().as_secs() > 60) {
                let s = app.server.stderr_snapshot();
                let lines: Vec<&str> = s.lines().rev().take(5).collect();
                app.server_status = ServerStatus::Failed(if lines.is_empty() { "startup timeout (60s)".into() } else { format!("timeout — last:\n{}", lines.into_iter().rev().collect::<Vec<_>>().join("\n")) });
                app.start_time = None;
            } else if let Some((exit, stderr)) = app.server.check_exit().await { app.server_status = ServerStatus::Failed(format!("exited({exit}): {stderr}")); app.start_time = None; }
        }

        if matches!(app.server_status, ServerStatus::Running) && app.mode == ViewMode::Dashboard && last_auto.elapsed().as_secs() >= 5 { last_auto = Instant::now(); app.refresh_all().await; }

        if !event::poll(timeout)? { continue; }
        let Event::Key(key) = event::read()? else { continue; };
        if key.kind != KeyEventKind::Press { continue; }

        if app.help_visible { app.help_visible = !matches!(key.code, KeyCode::Esc | KeyCode::Char('?')); continue; }
        if app.result.take().is_some() || app.error.take().is_some() { continue; }

        if app.mode == ViewMode::FormPopup {
            match crate::form::handle_key(&mut app.form, key.code) {
                FormResult::Submitted(_) => app.submit_form().await,
                FormResult::Cancelled => app.mode = ViewMode::Dashboard,
                FormResult::Continue => {}
            }
            continue;
        }
        if app.mode == ViewMode::DetailPopup { if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) { app.mode = ViewMode::Dashboard; } continue; }
        if app.mode == ViewMode::Dashboard {
            match key.code {
                KeyCode::Up if app.active_cursor() > 0 => { app.set_panel_cursor(app.active_panel, app.active_cursor() - 1); }
                KeyCode::Down if app.active_cursor() < app.active_panel_len().saturating_sub(1) => { app.set_panel_cursor(app.active_panel, app.active_cursor() + 1); }
                KeyCode::Left if app.active_panel > 0 => { app.set_panel_scroll(app.active_panel, app.active_scroll()); app.active_panel -= 1; }
                KeyCode::Right if app.active_panel < 5 => { app.set_panel_scroll(app.active_panel, app.active_scroll()); app.active_panel += 1; }
                KeyCode::Enter if app.active_panel == 0 => {
                    if let Some(b) = app.s3_buckets.get(app.active_cursor()) {
                        let items = crate::fetchers::s3_objects(&app.aws, &b.name, "").await;
                        app.browser_bucket = b.name.clone(); app.browser_path = String::new(); app.browser_items = items.clone();
                        app.browser_cursor = 0; app.browser_scroll = 0; app.browser_focus = 1;
                        app.browser_prefixes = items.iter().filter(|x| x.is_folder).map(|x| x.key.clone()).collect();
                        app.browser_prefix_cursor = 0; app.browser_prefix_scroll = 0; app.browser_sort_col = 0; app.browser_sort_desc = false;
                        app.mode = ViewMode::BucketBrowser;
                    }
                }
                KeyCode::Backspace if !app.global_filter.is_empty() => { app.global_filter.pop(); }
                _ => {}
            }
            if let KeyCode::Char(c) = key.code {
                match c {
                    '1'..='6' => { let i = (c as u8 - b'1') as usize; if i <= 5 { app.active_panel = i; } }
                    'S' => if matches!(app.server_status, ServerStatus::Stopped | ServerStatus::Failed(_)) { app.server_status = ServerStatus::Starting; app.start_time = Some(Instant::now()); if app.server.start().await.is_err() { app.server_status = ServerStatus::Failed("start failed".into()); app.start_time = None; } } else { app.server.stop().await; app.server_status = ServerStatus::Stopped; app.start_time = None; app.server_started = None; }
                    'r' | 'R' => app.refresh_all().await,
                    '?' => app.help_visible = !app.help_visible,
                    'q' | 'Q' => break,
                    'c' if app.active_panel == 0 => app.open_form(FormAction::CreateBucket),
                    'c' if app.active_panel == 2 => app.open_form(FormAction::CreateUser),
                    'c' if app.active_panel == 3 => app.open_form(FormAction::CreateSecret),
                    'd' => {
                        let idx = app.active_cursor();
                        match app.active_panel {
                            0 => { if let Some(x) = app.s3_buckets.get(idx) { app.result = Some(("Delete".into(), crate::actions::delete_bucket(&app.aws, &x.name).await)); } }
                            1 => { if let Some(x) = app.emr_vcs.get(idx) { app.result = Some(("Delete".into(), crate::actions::delete_vc(&app.aws, &x.id).await)); } }
                            2 => { if let Some(x) = app.cognito_users.get(idx) { app.result = Some(("Delete".into(), crate::actions::delete_user(&app.aws, &x.username).await)); } }
                            3 => { if let Some(x) = app.secrets.get(idx) { app.result = Some(("Delete".into(), crate::actions::delete_secret(&app.aws, &x.name).await)); } }
                            4 => { if let Some(x) = app.lambda_funcs.get(idx) { app.result = Some(("Delete".into(), crate::actions::delete_function(&app.aws, &x.name).await)); } }
                            _ => {}
                        }
                        app.refresh_all().await;
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
        } else if app.mode == ViewMode::BucketBrowser {
            match key.code {
                KeyCode::Left if app.browser_focus > 0 => { app.browser_focus -= 1; }
                KeyCode::Right | KeyCode::Tab if app.browser_focus < 2 => { app.browser_focus += 1; }
                KeyCode::Up => match app.browser_focus { 0 if app.browser_prefix_cursor > 0 => app.browser_prefix_cursor -= 1, 1 if app.browser_cursor > 0 => app.browser_cursor -= 1, _ => {} }
                KeyCode::Down => match app.browser_focus { 0 => { let l = app.browser_prefixes.len().saturating_sub(1); if app.browser_prefix_cursor < l { app.browser_prefix_cursor += 1; } } 1 => { let l = app.browser_items.len().saturating_sub(1); if app.browser_cursor < l { app.browser_cursor += 1; } } _ => {} }
                KeyCode::Char('d') if app.browser_focus == 1 => {
                    if let Some(obj) = app.browser_items.get(app.browser_cursor).filter(|o| !o.is_folder) {
                        let fname = std::path::Path::new(&obj.key).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        app.start_download(obj.key.clone(), app.upload.path.join(fname));
                    }
                }
                KeyCode::Char('D') => {
                    let prefix = if app.browser_path.is_empty() { String::new() } else { app.browser_path.clone() };
                    let objects = crate::fetchers::s3_all_keys(&app.aws, &app.browser_bucket, &prefix).await;
                    for key in objects { app.start_download(key.clone(), app.upload.path.join(&key)); }
                }
                KeyCode::Char('u') => app.enter_upload_mode(),
                KeyCode::Char('r') | KeyCode::Char('R') => { let x = crate::fetchers::s3_objects(&app.aws, &app.browser_bucket, &app.browser_path).await; app.browser_items = x; app.browser_prefixes = app.browser_items.iter().filter(|o| o.is_folder).map(|o| o.key.clone()).collect(); }
                KeyCode::Enter => {
                    let name = if app.browser_focus == 1 { app.browser_items.get(app.browser_cursor).filter(|x| x.is_folder).map(|x| x.key.clone()) } else { app.browser_prefixes.get(app.browser_prefix_cursor).cloned() };
                    if let Some(n) = name {
                        let np = if app.browser_path.is_empty() { format!("{n}/") } else { format!("{}{n}/", app.browser_path) };
                        let x = crate::fetchers::s3_objects(&app.aws, &app.browser_bucket, &np).await;
                        app.browser_path = np; app.browser_items = x; app.browser_cursor = 0; app.browser_scroll = 0;
                        app.browser_prefixes = app.browser_items.iter().filter(|o| o.is_folder).map(|o| o.key.clone()).collect();
                        app.browser_prefix_cursor = 0; app.browser_prefix_scroll = 0;
                    }
                }
                KeyCode::Backspace => {
                    if app.browser_path.is_empty() { app.mode = ViewMode::Dashboard; } else {
                        let mut parts: Vec<&str> = app.browser_path.trim_end_matches('/').split('/').collect();
                        parts.pop();
                        let np = if parts.is_empty() { String::new() } else { format!("{}/", parts.join("/")) };
                        let x = crate::fetchers::s3_objects(&app.aws, &app.browser_bucket, &np).await;
                        app.browser_path = np; app.browser_items = x; app.browser_cursor = 0; app.browser_scroll = 0;
                        app.browser_prefixes = app.browser_items.iter().filter(|o| o.is_folder).map(|o| o.key.clone()).collect();
                        app.browser_prefix_cursor = 0; app.browser_prefix_scroll = 0;
                    }
                }
                KeyCode::Esc => app.mode = ViewMode::Dashboard,
                _ => {}
            }
            let m = 25usize; let (c, s) = (app.browser_cursor, &mut app.browser_scroll); if c >= *s + m { *s = c.saturating_sub(m / 2); } else if c < *s { *s = c; }
            let (c, s) = (app.browser_prefix_cursor, &mut app.browser_prefix_scroll); if c >= *s + m { *s = c.saturating_sub(m / 2); } else if c < *s { *s = c; }
        } else if app.mode == ViewMode::UploadMode {
            match key.code {
                KeyCode::Up if app.upload.cursor > 0 => { app.upload.cursor -= 1; }
                KeyCode::Down => { let l = app.upload.entries.len().saturating_sub(1); if app.upload.cursor < l { app.upload.cursor += 1; } }
                KeyCode::Enter if app.upload.entries.get(app.upload.cursor).is_some_and(|e| e.is_dir) => app.upload.enter_dir(),
                KeyCode::Char(' ') => app.upload.toggle_select(),
                KeyCode::Backspace | KeyCode::Left => app.upload.go_up(),
                KeyCode::Enter | KeyCode::Char('u') => { for (name, path) in app.upload.upload_queue() { let key = if app.browser_path.is_empty() { name.clone() } else { format!("{}{}", app.browser_path, name) }; app.start_upload(path, key); } app.upload.selected.clear(); }
                KeyCode::Esc => app.mode = ViewMode::BucketBrowser,
                _ => {}
            }
            let m = 20usize;
            if app.upload.cursor >= app.upload.scroll + m { app.upload.scroll = app.upload.cursor.saturating_sub(m / 2); }
            if app.upload.cursor < app.upload.scroll { app.upload.scroll = app.upload.cursor; }
            if !app.upload.transfers.is_empty() && app.upload.transfers.iter().all(|t| t.is_done()) {
                let x = crate::fetchers::s3_objects(&app.aws, &app.browser_bucket, &app.browser_path).await;
                app.browser_items = x;
                app.browser_prefixes = app.browser_items.iter().filter(|o| o.is_folder).map(|o| o.key.clone()).collect();
                app.upload.transfers.clear();
            }
        }
    }
    Ok(())
}
