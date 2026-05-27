use crate::app::{App, ViewMode};
use crate::form::FormResult;
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
            else if app.start_time.is_some_and(|t| t.elapsed().as_secs() > 60) || app.server.check_exit().await.is_some() {
                app.server_status = ServerStatus::Failed;
                app.start_time = None;
            }
        }

        if matches!(app.server_status, ServerStatus::Running) && app.mode == ViewMode::Dashboard && last_auto.elapsed().as_secs() >= 5 { last_auto = Instant::now(); app.refresh_all().await; }

        if !event::poll(timeout)? { continue; }
        let Event::Key(key) = event::read()? else { continue; };
        if key.kind != KeyEventKind::Press { continue; }

        if app.help_visible { app.help_visible = !matches!(key.code, KeyCode::Esc | KeyCode::Char('?')); continue; }
        if app.result.take().is_some() || app.error.take().is_some() { continue; }

        if app.mode == ViewMode::FormPopup {
            match crate::form::handle_key(&mut app.form, key.code) {
                FormResult::Submitted => app.submit_form().await,
                FormResult::Cancelled => app.mode = ViewMode::Dashboard,
                FormResult::Continue => {}
            }
            continue;
        }
        if app.mode == ViewMode::DetailPopup { if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) { app.mode = ViewMode::Dashboard; } continue; }
        if app.mode == ViewMode::ConfirmPopup {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    let id = app.confirm_id.clone(); let _name = app.confirm_item.clone(); let p = app.confirm_panel;
                    app.mode = ViewMode::Dashboard;
                    app.result = Some(match p {
                        0 => ("Delete".into(), crate::actions::delete_bucket(&app.aws, &id).await),
                        1 => ("Delete".into(), crate::actions::delete_vc(&app.aws, &id).await),
                        2 => ("Delete".into(), crate::actions::delete_user(&app.aws, &id).await),
                        3 => ("Delete".into(), crate::actions::delete_secret(&app.aws, &id).await),
                        4 => ("Delete".into(), crate::actions::delete_function(&app.aws, &id).await),
                        _ => unreachable!(),
                    });
                    app.refresh_all().await;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => { app.mode = ViewMode::Dashboard; }
                _ => {}
            }
            continue;
        }
        if app.mode == ViewMode::Dashboard {
            crate::dash_handler::handle_dashboard(app, key.code).await;
            if let KeyCode::Char(c) = key.code {
                if c == 'q' || c == 'Q' { break; }
                crate::dash_handler::handle_dashboard_chars(app, c).await;
            }
            if let KeyCode::Backspace = key.code && !app.global_filter.is_empty() { app.global_filter.pop(); }
        } else if app.mode == ViewMode::BucketBrowser {
            crate::browse_handler::handle_browser(app, key.code).await;
        } else if app.mode == ViewMode::UploadMode {
            crate::browse_handler::handle_upload(app, key.code).await;
        }
    }
    Ok(())
}
