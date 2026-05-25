use crate::app::{App, ConfirmAction, Input, Tab};
use crate::nav;
use crate::server::ServerStatus;
use crate::ui;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::time::{Duration, Instant};

pub async fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;
        let timeout = if matches!(app.server_status, ServerStatus::Starting) { Duration::from_millis(200) } else { Duration::from_millis(100) };

        if matches!(app.server_status, ServerStatus::Starting) && app.health_checked.elapsed().as_secs() >= 2 {
            app.health_checked = Instant::now();
            if app.aws.check_health().await { app.server_status = ServerStatus::Running; app.server_started = Some(Instant::now()); app.start_time = None; app.refresh_all().await; }
            else if app.start_time.is_some_and(|t| t.elapsed().as_secs() > 60) {
                let s = app.server.stderr_snapshot();
                app.server_status = ServerStatus::Failed(if s.is_empty() { "startup timeout (60s)".into() } else { format!("startup timeout — last:\n{}", s.lines().rev().take(5).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")) });
                app.start_time = None;
            } else if let Some((exit, stderr)) = app.server.check_exit().await { app.server_status = ServerStatus::Failed(format!("exited({exit}): {stderr}")); app.start_time = None; }
        }

        if !event::poll(timeout)? { continue; }
        let Event::Key(key) = event::read()? else { continue; };
        if key.kind != KeyEventKind::Press { continue; }
        if app.result.is_some() { app.result = None; continue; }

        if app.confirming.is_some() {
            match key.code {
                KeyCode::Char('y' | 'Y') => {
                    let (_, action) = app.confirming.take().unwrap();
                    use crate::actions;
                    let r = match action {
                        ConfirmAction::DeleteBucket(n) => ("Delete Bucket".into(), actions::delete_bucket(&app.aws, &n).await),
                        ConfirmAction::DeleteObject(b, k) => ("Delete".into(), actions::delete_object(&app.aws, &b, &k).await),
                        ConfirmAction::DeleteCognitoUser(n) => ("Delete User".into(), actions::delete_user(&app.aws, &n).await),
                        ConfirmAction::DeleteSecret(n) => ("Delete Secret".into(), actions::delete_secret(&app.aws, &n).await),
                        ConfirmAction::DeleteFunction(n) => ("Delete Function".into(), actions::delete_function(&app.aws, &n).await),
                        ConfirmAction::DeleteVc(i) => ("Delete VC".into(), actions::delete_vc(&app.aws, &i).await),
                        ConfirmAction::CancelJob(v, j) => ("Cancel Job".into(), actions::cancel_job(&app.aws, &v, &j).await),
                    };
                    app.result = Some(r); app.refresh_all().await;
                }
                KeyCode::Char('n' | 'N') | KeyCode::Esc => { app.confirming = None; }
                _ => {}
            }
            continue;
        }

        if !matches!(app.input, Input::None) {
            match key.code { KeyCode::Esc => { app.input = Input::None; app.input_buf.clear(); } KeyCode::Enter => app.submit_input().await, KeyCode::Backspace => { app.input_buf.pop(); } KeyCode::Char(c) => app.input_buf.push(c), _ => {} }
            continue;
        }

        match key.code {
            KeyCode::Up => app.cursor = app.cursor.saturating_sub(1),
            KeyCode::Down => { let m = app.list_len().saturating_sub(1); if app.cursor < m { app.cursor += 1; } }
            KeyCode::Tab => { nav::switch_tab(app, app.tab.next()); app.refresh_all().await; }
            KeyCode::BackTab => { nav::switch_tab(app, app.tab.prev()); app.refresh_all().await; }
            KeyCode::Right if app.tab == Tab::S3 && app.s3_col < 2 => {
                let can = if app.s3_col == 0 { !app.s3_bucket.is_empty() } else { app.cursor < app.s3_folders.len() };
                if can {
                    app.s3_cursors[app.s3_col] = app.cursor;
                    app.s3_col += 1;
                    app.cursor = app.s3_cursors[app.s3_col];
                    if app.s3_col == 2 && app.s3_col3_prefix.is_empty() {
                        let f = app.s3_folders.get(app.s3_cursors[1]).cloned().unwrap_or_default();
                        app.s3_col3_prefix = f; app.refresh_all().await;
                    }
                }
            }
            KeyCode::Right => { nav::switch_tab(app, app.tab.next()); app.refresh_all().await; }
            KeyCode::Left if app.tab == Tab::S3 && app.s3_col > 0 => {
                app.s3_cursors[app.s3_col] = app.cursor;
                app.s3_col -= 1;
                app.cursor = app.s3_cursors[app.s3_col];
                if app.s3_col < 2 { app.s3_col3_prefix.clear(); app.s3_col3_folders.clear(); app.s3_col3_objects.clear(); }
            }
            KeyCode::Left => { nav::switch_tab(app, app.tab.prev()); app.refresh_all().await; }
            _ => {}
        }

        match key.code {
            KeyCode::Enter if app.tab == Tab::S3 => { nav::s3_enter(app).await; }
            KeyCode::Enter if app.tab == Tab::Emr && app.emr_vc_id.is_empty() && app.cursor < app.emr_vcs.len() => {
                app.emr_vc_id = app.emr_vcs[app.cursor].0.clone(); app.cursor = 0; app.refresh_all().await;
            }
            KeyCode::Esc if app.tab == Tab::S3 => { nav::s3_esc(app); }
            KeyCode::Esc if app.tab == Tab::Emr && !app.emr_vc_id.is_empty() => { app.emr_vc_id.clear(); app.emr_jobs.clear(); app.cursor = 0; }
            _ => {}
        }

        if let KeyCode::Char(c) = key.code {
            if nav::tab_action(app, c) { app.input_buf.clear(); continue; }
            match c {
                'q' | 'Q' => break,
                '?' => app.help_visible = !app.help_visible,
                'r' | 'R' => app.refresh_all().await,
                'S' => match &app.server_status {
                    ServerStatus::Stopped | ServerStatus::Failed(_) => { app.server_status = ServerStatus::Starting; app.start_time = Some(Instant::now()); if app.server.start().await.is_err() { app.server_status = ServerStatus::Failed("start failed".into()); app.start_time = None; } }
                    ServerStatus::Starting | ServerStatus::Running => { app.server.stop().await; app.server_status = ServerStatus::Stopped; app.start_time = None; app.server_started = None; }
                },
                d if d.is_ascii_digit() => {
                    let tabs = [Tab::Overview, Tab::S3, Tab::Cognito, Tab::Secrets, Tab::Emr, Tab::Lambda, Tab::Logs];
                    let i = d.to_digit(10).unwrap_or(1) as usize - 1;
                    if i < tabs.len() { nav::switch_tab(app, tabs[i]); app.refresh_all().await; }
                }
                'd' if app.tab == Tab::S3 && app.s3_bucket.is_empty() => {
                    if let Some(name) = app.s3_buckets.get(app.cursor).cloned() { app.confirming = Some((format!("Delete bucket '{name}'?"), ConfirmAction::DeleteBucket(name))); }
                }
                'd' if app.tab == Tab::S3 && !app.s3_bucket.is_empty() && app.s3_col == 1 && app.cursor >= app.s3_folders.len() => {
                    if let Some((key, _, _)) = app.s3_objects.get(app.cursor - app.s3_folders.len()) { app.confirming = Some((format!("Delete '{key}'?"), ConfirmAction::DeleteObject(app.s3_bucket.clone(), key.clone()))); }
                }
                'd' if app.tab == Tab::S3 && !app.s3_bucket.is_empty() && app.s3_col == 2 && app.cursor >= app.s3_col3_folders.len() => {
                    if let Some((key, _, _)) = app.s3_col3_objects.get(app.cursor - app.s3_col3_folders.len()) { app.confirming = Some((format!("Delete '{key}'?"), ConfirmAction::DeleteObject(app.s3_bucket.clone(), key.clone()))); }
                }
                'd' if app.tab == Tab::Cognito => {
                    if let Some((name, _, _)) = app.cognito_users.get(app.cursor) { app.confirming = Some((format!("Delete user '{name}'?"), ConfirmAction::DeleteCognitoUser(name.clone()))); }
                }
                'd' if app.tab == Tab::Secrets => {
                    if let Some((name, _, _)) = app.secrets.get(app.cursor) { app.confirming = Some((format!("Delete secret '{name}'?"), ConfirmAction::DeleteSecret(name.clone()))); }
                }
                'd' if app.tab == Tab::Lambda => {
                    if let Some((name, _, _)) = app.lambda_funcs.get(app.cursor) { app.confirming = Some((format!("Delete function '{name}'?"), ConfirmAction::DeleteFunction(name.clone()))); }
                }
                'd' if app.tab == Tab::Emr => {
                    if app.emr_vc_id.is_empty() {
                        if let Some((id, _, _)) = app.emr_vcs.get(app.cursor) { app.confirming = Some((format!("Delete VC '{id}'?"), ConfirmAction::DeleteVc(id.clone()))); }
                    } else {
                        if let Some((id, _, _)) = app.emr_jobs.get(app.cursor) { app.confirming = Some((format!("Cancel job '{id}'?"), ConfirmAction::CancelJob(app.emr_vc_id.clone(), id.clone()))); }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

