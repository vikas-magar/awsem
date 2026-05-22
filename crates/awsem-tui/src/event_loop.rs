use crate::app::{App, Input, Tab};
use crate::server::ServerStatus;
use crate::ui;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::time::{Duration, Instant};

fn tab_action(app: &mut App, c: char) -> bool {
    match (app.tab, c) {
        (Tab::Cognito, 'c') => { app.input = Input::CreateUser; true }
        (Tab::Lambda, 'i') => app.lambda_funcs.get(app.cursor).map(|n| app.input = Input::LambdaPayload(n.0.clone())).is_some(),
        (Tab::S3, 'c') => { app.input = Input::CreateBucket; true }
        (Tab::S3, 'u') => {
            let bucket = if app.s3_bucket.is_empty() { app.s3_buckets.get(app.cursor).cloned().unwrap_or_default() } else { app.s3_bucket.clone() };
            if bucket.is_empty() { false } else { app.input = Input::S3UploadKey(bucket); true }
        }
        (Tab::Emr, 's') => app.emr_vcs.get(app.cursor).map(|v| app.input = Input::EmrSubmit(v.0.clone())).is_some(),
        (Tab::Logs, 'f') => { app.input = Input::LogFilter; true }
        (Tab::Secrets, 'c') => { app.input = Input::CreateSecret; true }
        (Tab::Secrets, 'e') => app.secrets.get(app.cursor).map(|n| app.input = Input::EditSecret(n.0.clone())).is_some(),
        _ => false,
    }
}

pub async fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App) -> anyhow::Result<()> {
    use crate::actions;
    loop {
        terminal.draw(|f| ui::render(f, app))?;
        let timeout = if matches!(app.server_status, ServerStatus::Starting) { Duration::from_millis(500) } else { Duration::from_millis(250) };

        if matches!(app.server_status, ServerStatus::Starting) && app.health_checked.elapsed().as_secs() >= 2 {
            app.health_checked = Instant::now();
            if app.aws.check_health().await { app.server_status = ServerStatus::Running; app.start_time = None; app.refresh_all().await; }
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

        if !matches!(app.input, Input::None) {
            match key.code { KeyCode::Esc => { app.input = Input::None; app.input_buf.clear(); } KeyCode::Enter => app.submit_input().await, KeyCode::Backspace => { app.input_buf.pop(); } KeyCode::Char(c) => app.input_buf.push(c), _ => {} }
            continue;
        }

        match key.code { KeyCode::Up => app.cursor = app.cursor.saturating_sub(1), KeyCode::Down => { let m = list_len(app).saturating_sub(1); if app.cursor < m { app.cursor += 1; } } KeyCode::Tab | KeyCode::Right => { app.tab = app.tab.next(); app.cursor = 0; app.refresh_all().await; } KeyCode::BackTab | KeyCode::Left => { app.tab = app.tab.prev(); app.cursor = 0; app.refresh_all().await; } _ => {} }

        match key.code {
            KeyCode::Enter if app.tab == Tab::S3 && app.s3_bucket.is_empty() && app.cursor < app.s3_buckets.len() => {
                app.s3_bucket = app.s3_buckets[app.cursor].clone(); app.cursor = 0; app.refresh_all().await;
            }
            KeyCode::Enter if app.tab == Tab::Emr && app.emr_vc_id.is_empty() && app.cursor < app.emr_vcs.len() => {
                app.emr_vc_id = app.emr_vcs[app.cursor].0.clone(); app.cursor = 0; app.refresh_all().await;
            }
            KeyCode::Esc if app.tab == Tab::S3 && !app.s3_bucket.is_empty() => { app.s3_bucket.clear(); app.s3_objects.clear(); app.cursor = 0; }
            KeyCode::Esc if app.tab == Tab::Emr && !app.emr_vc_id.is_empty() => { app.emr_vc_id.clear(); app.emr_jobs.clear(); app.cursor = 0; }
            _ => {}
        }

        if let KeyCode::Char(c) = key.code {
            if tab_action(app, c) { app.input_buf.clear(); continue; }
            match c {
                'q' | 'Q' => break,
                '?' => app.help_visible = !app.help_visible,
                'r' | 'R' => app.refresh_all().await,
                'S' => match &app.server_status {
                    ServerStatus::Stopped | ServerStatus::Failed(_) => { app.server_status = ServerStatus::Starting; app.start_time = Some(Instant::now()); if app.server.start().await.is_err() { app.server_status = ServerStatus::Failed("start failed".into()); app.start_time = None; } }
                    ServerStatus::Starting | ServerStatus::Running => { app.server.stop().await; app.server_status = ServerStatus::Stopped; app.start_time = None; }
                },
                d if d.is_ascii_digit() => {
                    let tabs = [Tab::Overview, Tab::S3, Tab::Cognito, Tab::Secrets, Tab::Emr, Tab::Lambda, Tab::Logs];
                    let i = d.to_digit(10).unwrap_or(1) as usize - 1;
                    if i < tabs.len() { app.tab = tabs[i]; app.cursor = 0; app.refresh_all().await; }
                }
                'd' if app.tab == Tab::S3 && app.s3_bucket.is_empty() => {
                    if let Some(name) = app.s3_buckets.get(app.cursor).cloned() { app.result = Some(("Delete Bucket".into(), actions::delete_bucket(&app.aws, &name).await)); }
                    app.refresh_all().await;
                }
                'd' if app.tab == Tab::S3 && !app.s3_bucket.is_empty() => {
                    if let Some((key, _)) = app.s3_objects.get(app.cursor) { app.result = Some(("Delete".into(), actions::delete_object(&app.aws, &app.s3_bucket, key).await)); }
                    app.refresh_all().await;
                }
                'd' if app.tab == Tab::Cognito => {
                    if let Some((name, _, _)) = app.cognito_users.get(app.cursor) { app.result = Some(("Delete User".into(), actions::delete_user(&app.aws, name).await)); }
                    app.refresh_all().await;
                }
                'd' if app.tab == Tab::Secrets => {
                    if let Some((name, _, _)) = app.secrets.get(app.cursor) { app.result = Some(("Delete Secret".into(), actions::delete_secret(&app.aws, name).await)); }
                    app.refresh_all().await;
                }
                'd' if app.tab == Tab::Lambda => {
                    if let Some((name, _, _)) = app.lambda_funcs.get(app.cursor) { app.result = Some(("Delete Function".into(), actions::delete_function(&app.aws, name).await)); }
                    app.refresh_all().await;
                }
                'd' if app.tab == Tab::Emr => {
                    if app.emr_vc_id.is_empty() {
                        if let Some((id, _, _)) = app.emr_vcs.get(app.cursor) { app.result = Some(("Delete VC".into(), actions::delete_vc(&app.aws, id).await)); }
                    } else {
                        if let Some((id, _, _)) = app.emr_jobs.get(app.cursor) { app.result = Some(("Cancel Job".into(), actions::cancel_job(&app.aws, &app.emr_vc_id, id).await)); }
                    }
                    app.refresh_all().await;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn list_len(app: &App) -> usize {
    match app.tab {
        Tab::S3 => if app.s3_bucket.is_empty() { app.s3_buckets.len() } else { app.s3_objects.len() },
        Tab::Cognito => app.cognito_users.len(),
        Tab::Secrets => app.secrets.len(),
        Tab::Emr => if app.emr_vc_id.is_empty() { app.emr_vcs.len() } else { app.emr_jobs.len() },
        Tab::Lambda => app.lambda_funcs.len(),
        Tab::Logs => app.logs.len(),
        _ => 0,
    }
}
