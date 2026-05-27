use crate::app::{App, ViewMode};
use crossterm::event::KeyCode;

pub async fn handle_browser(app: &mut App, c: KeyCode) {
    match c {
        KeyCode::Left if app.browser_focus > 0 => { app.browser_focus -= 1; }
        KeyCode::Right | KeyCode::Tab if app.browser_focus < 1 => { app.browser_focus += 1; }
        KeyCode::Up if app.browser_cursor > 0 => { app.browser_cursor -= 1; }
        KeyCode::Down if app.browser_cursor + 1 < app.browser_items.len() => { app.browser_cursor += 1; }
        KeyCode::Char('d') => {
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
        KeyCode::Char('s') => { app.browser_sort_desc = !app.browser_sort_desc; app.browser_items.sort_by(|a, b| if app.browser_sort_desc { b.key.cmp(&a.key) } else { a.key.cmp(&b.key) }); }
        KeyCode::Char('r') | KeyCode::Char('R') => { let x = crate::fetchers::s3_objects(&app.aws, &app.browser_bucket, &app.browser_path).await; app.browser_items = x; }
        KeyCode::Enter => {
            if let Some(obj) = app.browser_items.get(app.browser_cursor).filter(|x| x.is_folder) {
                let np = if app.browser_path.is_empty() { format!("{}/", obj.key) } else { format!("{}{}/", app.browser_path, obj.key) };
                let x = crate::fetchers::s3_objects(&app.aws, &app.browser_bucket, &np).await;
                app.browser_path = np; app.browser_items = x; app.browser_cursor = 0; app.browser_scroll = 0;
            }
        }
        KeyCode::Backspace => {
            if app.browser_path.is_empty() { app.mode = ViewMode::Dashboard; } else {
                let mut parts: Vec<&str> = app.browser_path.trim_end_matches('/').split('/').collect();
                parts.pop();
                let np = if parts.is_empty() { String::new() } else { format!("{}/", parts.join("/")) };
                let x = crate::fetchers::s3_objects(&app.aws, &app.browser_bucket, &np).await;
                app.browser_path = np; app.browser_items = x; app.browser_cursor = 0; app.browser_scroll = 0;
            }
        }
        KeyCode::Esc => app.mode = ViewMode::Dashboard,
        _ => {}
    }
    let m = 25usize; let (c, s) = (app.browser_cursor, &mut app.browser_scroll); if c >= *s + m { *s = c.saturating_sub(m / 2); } else if c < *s { *s = c; }
}

pub async fn handle_upload(app: &mut App, c: KeyCode) {
    match c {
        KeyCode::Up if app.upload.cursor > 0 => { app.upload.cursor -= 1; }
        KeyCode::Down if app.upload.cursor + 1 < app.upload.entries.len() => { app.upload.cursor += 1; }
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
    app.upload.spinner = app.upload.spinner.wrapping_add(1);
    if !app.upload.transfers.is_empty() && app.upload.transfers.iter().all(|t| t.is_done()) {
        let x = crate::fetchers::s3_objects(&app.aws, &app.browser_bucket, &app.browser_path).await;
        app.browser_items = x;
        app.upload.transfers.clear();
    }
}
