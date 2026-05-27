use std::time::Instant;
use crate::app::{App, ViewMode};
use crate::form::FormAction;

impl App {
    pub fn start_upload(&mut self, local_path: std::path::PathBuf, s3_key: String) {
        use std::sync::atomic::Ordering;
        let fname = local_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let t = crate::upload_state::UploadTransfer::new(fname);
        let prog = t.progress.clone(); let done = t.done.clone(); let err = t.error.clone();
        self.upload.transfers.push(t);
        let aws = self.aws.clone(); let bucket = self.browser_bucket.clone();
        tokio::spawn(async move {
            match tokio::fs::read(&local_path).await {
                Ok(c) => {
                    prog.store(50, Ordering::Relaxed);
                    let r = aws.s3.put_object().bucket(&bucket).key(&s3_key).body(aws_sdk_s3::primitives::ByteStream::from(c)).send().await;
                    prog.store(100, Ordering::Relaxed);
                    done.store(true, Ordering::Relaxed);
                    if let Err(e) = r { *err.lock().unwrap() = Some(format!("{e}")); }
                }
                Err(e) => { done.store(true, Ordering::Relaxed); *err.lock().unwrap() = Some(format!("{e}")); }
            }
        });
    }

    pub fn start_download(&mut self, s3_key: String, local_dest: std::path::PathBuf) {
        use std::sync::atomic::Ordering;
        let fname = std::path::Path::new(&s3_key).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let t = crate::upload_state::UploadTransfer::new(fname);
        let prog = t.progress.clone(); let done = t.done.clone(); let err = t.error.clone();
        self.upload.transfers.push(t);
        let aws = self.aws.clone(); let bucket = self.browser_bucket.clone();
        tokio::spawn(async move {
            match aws.s3.get_object().bucket(&bucket).key(&s3_key).send().await {
                Ok(resp) => {
                    prog.store(30, Ordering::Relaxed);
                    match resp.body.collect().await {
                        Ok(data) => {
                            prog.store(60, Ordering::Relaxed);
                            if let Some(parent) = local_dest.parent() { let _ = tokio::fs::create_dir_all(parent).await; }
                            match tokio::fs::write(&local_dest, data.into_bytes()).await {
                                Ok(_) => prog.store(100, Ordering::Relaxed),
                                Err(e) => *err.lock().unwrap() = Some(format!("{e}")),
                            }
                        }
                        Err(e) => *err.lock().unwrap() = Some(format!("{e}")),
                    }
                    done.store(true, Ordering::Relaxed);
                }
                Err(e) => { done.store(true, Ordering::Relaxed); *err.lock().unwrap() = Some(format!("{e}")); }
            }
        });
    }

    pub async fn refresh_all(&mut self) {
        if !matches!(self.server_status, crate::server::ServerStatus::Running) { return; }
        self.last_refresh = Instant::now();
        self.error = None;
        if self.log_file.is_empty() {
            for line in self.server.stderr_snapshot().lines() {
                if let Some(p) = line.strip_prefix("log_file=") { self.log_file = p.to_string(); break; }
            }
            if self.log_file.is_empty() {
                for dir in &["./logs", &format!("{}/.config/awsem/logs", std::env::var("HOME").unwrap_or_default())] {
                    if let Ok(e) = std::fs::read_dir(dir) {
                        let mut files: Vec<_> = e.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "log")).collect();
                        files.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));
                        if let Some(latest) = files.last() { self.log_file = latest.path().to_string_lossy().to_string(); break; }
                    }
                }
            }
        }
        use crate::fetchers;
        let (b, u, s, f, v) = tokio::join!(
            fetchers::s3_buckets(&self.aws), fetchers::cognito_users(&self.aws),
            fetchers::secrets(&self.aws), fetchers::lambda_funcs(&self.aws),
            fetchers::emr_vcs(&self.aws),
        );
        let s3_ok = !b.is_empty();
        self.s3_buckets = b; self.cognito_users = u; self.secrets = s; self.lambda_funcs = f; self.emr_vcs = v;
        if !s3_ok { self.error = Some("S3 proxy unavailable — check RustFS/K8s".into()); }
        if self.mode == ViewMode::Dashboard && !self.log_file.is_empty() { self.logs = fetchers::read_log_file(&self.log_file, &self.logs_filter).await; }
        let p = self.active_panel;
        let len = self.panel_len(p);
        if len > 0 {
            if self.panel_cursors[p] >= len { self.panel_cursors[p] = len - 1; }
            if self.panel_scrolls[p] >= len { self.panel_scrolls[p] = len - 1; }
        } else {
            self.panel_cursors[p] = 0;
            self.panel_scrolls[p] = 0;
        }
    }

    pub async fn submit_form(&mut self) {
        self.mode = ViewMode::Dashboard;
        use crate::actions;
        let a = self.form.action.clone();
        let f = &self.form.fields;
        self.result = Some(match &a {
            FormAction::CreateBucket => ("Create Bucket".into(), actions::create_bucket(&self.aws, &f[0].value).await),
            FormAction::CreateUser => ("Create User".into(), actions::create_user(&self.aws, &f[0].value, &f[1].value, &f[2].value).await),
            FormAction::CreateSecret => ("Create Secret".into(), actions::create_secret(&self.aws, &f[0].value, &f[1].value, &f[2].value).await),
            FormAction::EditSecret(name) => ("Edit Secret".into(), actions::edit_secret(&self.aws, name, &f[0].value).await),
            FormAction::InvokeLambda(func) => ("Invoke".into(), actions::invoke_lambda(&self.aws, func, &f[0].value).await),
            FormAction::EmrSubmit(vc) => ("Submit Job".into(), actions::submit_job(&self.aws, vc, &f[0].value, &f[1].value, &f[2].value, &f[3].value).await),
            FormAction::LogFilter => { self.logs_filter = f[0].value.clone(); self.refresh_all().await; return; }
        });
        self.refresh_all().await;
    }
}
