use crate::app::{App, Input, Tab};

pub fn switch_tab(app: &mut App, t: Tab) {
    app.save_cursor(); app.tab = t; app.load_cursor(t);
}

pub fn tab_action(app: &mut App, c: char) -> bool {
    match (app.tab, c) {
        (Tab::Cognito, 'c') => { app.input = Input::CreateUser; true }
        (Tab::Lambda, 'i') => app.lambda_funcs.get(app.cursor).map(|n| app.input = Input::LambdaPayload(n.0.clone())).is_some(),
        (Tab::S3, 'c') => { app.input = Input::CreateBucket; true }
        (Tab::S3, 'u') => {
            let bucket = if app.s3_bucket.is_empty() { app.s3_buckets.get(app.cursor).cloned().unwrap_or_default() } else { app.s3_bucket.clone() };
            let prefix = if bucket.is_empty() { String::new() } else if app.s3_col == 2 { app.s3_col3_prefix.clone() } else { app.s3_prefix.clone() };
            if bucket.is_empty() { false } else { app.input = Input::S3UploadKey(bucket, prefix); true }
        }
        (Tab::Emr, 's') => app.emr_vcs.get(app.cursor).map(|v| app.input = Input::EmrSubmit(v.0.clone())).is_some(),
        (Tab::Logs, 'f') => { app.input = Input::LogFilter; true }
        (Tab::Secrets, 'c') => { app.input = Input::CreateSecret; true }
        (Tab::Secrets, 'e') => app.secrets.get(app.cursor).map(|n| app.input = Input::EditSecret(n.0.clone())).is_some(),
        _ => false,
    }
}

pub async fn s3_enter(app: &mut App) -> bool {
    match app.s3_col {
        0 if app.cursor < app.s3_buckets.len() => {
            app.s3_cursors[0] = app.cursor; app.s3_bucket = app.s3_buckets[app.cursor].clone();
            app.s3_prefix.clear(); app.s3_col3_prefix.clear(); app.s3_col = 1;
            app.cursor = app.s3_cursors[1]; app.refresh_all().await; true
        }
        1 if app.cursor < app.s3_folders.len() => {
            app.s3_cursors[1] = app.cursor; app.s3_col3_prefix = app.s3_folders[app.cursor].clone();
            app.s3_col = 2; app.cursor = app.s3_cursors[2]; app.refresh_all().await; true
        }
        2 if app.cursor < app.s3_col3_folders.len() => {
            app.s3_cursors[2] = app.cursor; app.s3_col3_prefix = app.s3_col3_folders[app.cursor].clone();
            app.cursor = 0; app.refresh_all().await; true
        }
        _ => false,
    }
}

pub fn s3_esc(app: &mut App) -> bool {
    match app.s3_col {
        2 => {
            app.s3_cursors[2] = 0; app.s3_col = 1; app.cursor = app.s3_cursors[1];
            app.s3_col3_prefix.clear(); app.s3_col3_folders.clear(); app.s3_col3_objects.clear(); true
        }
        1 => {
            app.s3_cursors[1] = 0; app.s3_col = 0; app.cursor = app.s3_cursors[0];
            app.s3_bucket.clear(); app.s3_prefix.clear(); app.s3_folders.clear(); app.s3_objects.clear();
            app.s3_col3_prefix.clear(); app.s3_col3_folders.clear(); app.s3_col3_objects.clear(); true
        }
        0 => { app.s3_cursors = [0; 3]; app.cursor = 0; true }
        _ => false,
    }
}
