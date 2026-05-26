use std::time::Instant;
use crate::aws_clients::AwsClients;
use crate::form::{FormAction, FormState};
use crate::server::{ServerManager, ServerStatus};
use crate::types::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode { Dashboard, BucketBrowser, DetailPopup, FormPopup, UploadMode }

pub struct App {
    pub aws: AwsClients,
    pub server: ServerManager,
    pub server_status: ServerStatus,
    pub mode: ViewMode,
    pub last_refresh: Instant,
    pub health_checked: Instant,
    pub start_time: Option<Instant>,
    pub server_started: Option<Instant>,
    pub error: Option<String>,
    pub result: Option<(String, String)>,
    pub form: FormState,
    pub help_visible: bool,
    pub s3_buckets: Vec<S3Bucket>,
    pub cognito_users: Vec<CognitoUser>,
    pub secrets: Vec<SecretEntry>,
    pub lambda_funcs: Vec<LambdaFn>,
    pub emr_vcs: Vec<EmrVc>,
    pub logs: Vec<LogEntry>,
    pub logs_filter: String,
    pub log_file: String,
    pub panel_cursors: [usize; 6],
    pub panel_scrolls: [usize; 6],
    pub active_panel: usize,
    pub global_filter: String,
    pub browser_bucket: String,
    pub browser_path: String,
    pub browser_items: Vec<S3Object>,
    pub browser_cursor: usize,
    pub browser_scroll: usize,
    pub browser_focus: usize,
    pub browser_prefixes: Vec<String>,
    pub browser_prefix_cursor: usize,
    pub browser_prefix_scroll: usize,
    pub browser_sort_col: usize,
    pub browser_sort_desc: bool,
    pub popup_title: String,
    pub popup_lines: Vec<(String, String)>,
    pub upload: crate::upload::UploadState,
}

impl App {
    pub fn new(aws: AwsClients, binary: &str, args: &[String]) -> Self {
        Self {
            aws, server: ServerManager::new(binary, args),
            server_status: ServerStatus::Stopped, mode: ViewMode::Dashboard,
            last_refresh: Instant::now(), health_checked: Instant::now(),
            start_time: None, server_started: None, error: None, result: None, help_visible: false,
            form: crate::form::init(&FormAction::CreateBucket),
            s3_buckets: Vec::new(), cognito_users: Vec::new(), secrets: Vec::new(), lambda_funcs: Vec::new(), emr_vcs: Vec::new(),
            logs: Vec::new(), logs_filter: String::new(), log_file: String::new(),
            panel_cursors: [0; 6], panel_scrolls: [0; 6], active_panel: 0, global_filter: String::new(),
            browser_bucket: String::new(), browser_path: String::new(), browser_items: Vec::new(),
            browser_cursor: 0, browser_scroll: 0, browser_focus: 0,
            browser_prefixes: Vec::new(), browser_prefix_cursor: 0, browser_prefix_scroll: 0,
            browser_sort_col: 0, browser_sort_desc: false,
            popup_title: String::new(), popup_lines: Vec::new(),
            upload: crate::upload::UploadState::new("."),
        }
    }

    pub fn open_form(&mut self, action: FormAction) { self.form = crate::form::init(&action); self.mode = ViewMode::FormPopup; }

    pub fn enter_upload_mode(&mut self) { self.upload = crate::upload::UploadState::new("."); self.mode = ViewMode::UploadMode; }

    pub fn panel_len(&self, p: usize) -> usize {
        [self.s3_buckets.len(), self.emr_vcs.len(), self.cognito_users.len(), self.secrets.len(), self.lambda_funcs.len(), self.logs.len()][p]
    }
    pub fn active_panel_len(&self) -> usize { self.panel_len(self.active_panel) }
    pub fn active_cursor(&self) -> usize { self.panel_cursors[self.active_panel] }
    pub fn active_scroll(&self) -> usize { self.panel_scrolls[self.active_panel] }
    pub fn panel_cursor(&self, p: usize) -> usize { self.panel_cursors[p] }
    pub fn panel_scroll(&self, p: usize) -> usize { self.panel_scrolls[p] }
    pub fn set_panel_cursor(&mut self, p: usize, c: usize) { self.panel_cursors[p] = c; }
    pub fn set_panel_scroll(&mut self, p: usize, s: usize) { self.panel_scrolls[p] = s; }

    pub fn start_upload(&mut self, local_path: std::path::PathBuf, s3_key: String) {
        use std::sync::atomic::Ordering;
        let fname = local_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let t = crate::upload::UploadTransfer::new(fname);
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
        let t = crate::upload::UploadTransfer::new(fname);
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
        if !matches!(self.server_status, ServerStatus::Running) { return; }
        self.last_refresh = Instant::now();
        if self.log_file.is_empty() {
            for line in self.server.stderr_snapshot().lines() {
                if let Some(p) = line.strip_prefix("log_file=") { self.log_file = p.to_string(); break; }
            }
        }
        use crate::fetchers;
        let (b, u, s, f, v) = tokio::join!(
            fetchers::s3_buckets(&self.aws), fetchers::cognito_users(&self.aws),
            fetchers::secrets(&self.aws), fetchers::lambda_funcs(&self.aws),
            fetchers::emr_vcs(&self.aws),
        );
        self.s3_buckets = b; self.cognito_users = u; self.secrets = s; self.lambda_funcs = f; self.emr_vcs = v;
        if self.mode == ViewMode::Dashboard && !self.log_file.is_empty() { self.logs = fetchers::read_log_file(&self.log_file, &self.logs_filter).await; }
        let p = self.active_panel;
        let len = self.panel_len(p);
        if self.panel_cursors[p] >= len.saturating_sub(1) { self.panel_cursors[p] = len.saturating_sub(1); }
        if self.panel_scrolls[p] >= len { self.panel_scrolls[p] = len.saturating_sub(1).saturating_sub(1); }
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
            FormAction::S3Upload(..) => { self.mode = ViewMode::Dashboard; return; }
            FormAction::LogFilter => { self.logs_filter = f[0].value.clone(); self.refresh_all().await; return; }
        });
        self.refresh_all().await;
    }
}
