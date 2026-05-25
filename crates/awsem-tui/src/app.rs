use crate::aws_clients::AwsClients;
use crate::server::{ServerManager, ServerStatus};
use std::time::Instant;

#[derive(Copy, Clone, PartialEq)]
pub enum Tab { Overview, S3, Cognito, Secrets, Emr, Lambda, Logs }

impl Tab {
    pub const ALL: &[Tab] = &[Self::Overview, Self::S3, Self::Cognito, Self::Secrets, Self::Emr, Self::Lambda, Self::Logs];
    pub fn name(&self) -> &'static str { ["Overview", "S3", "Cognito", "Secrets", "EMR", "Lambda", "Logs"][*self as usize] }
    pub fn key(&self) -> char { ['1', '2', '3', '4', '5', '6', '7'][*self as usize] }
    pub fn prev(&self) -> Self { [Self::Logs, Self::Overview, Self::S3, Self::Cognito, Self::Secrets, Self::Emr, Self::Lambda][*self as usize] }
    pub fn next(&self) -> Self { [Self::S3, Self::Cognito, Self::Secrets, Self::Emr, Self::Lambda, Self::Logs, Self::Overview][*self as usize] }
    pub fn idx(&self) -> usize { *self as usize }
}

pub enum ConfirmAction {
    DeleteBucket(String), DeleteObject(String, String), DeleteCognitoUser(String),
    DeleteSecret(String), DeleteFunction(String), DeleteVc(String), CancelJob(String, String),
}

pub enum Input {
    None, CreateUser, CreatePass(String), LambdaPayload(String), S3UploadKey(String, String),
    EmrSubmit(String), LogFilter, CreateSecret, CreateSecretValue(String), EditSecret(String), CreateBucket,
}

pub struct App {
    pub aws: AwsClients, pub server: ServerManager, pub server_status: ServerStatus, pub tab: Tab,
    pub help_visible: bool, pub error: Option<String>, pub confirming: Option<(String, ConfirmAction)>, pub last_refresh: Instant,
    pub start_time: Option<Instant>, pub health_checked: Instant, pub server_started: Option<Instant>,
    pub input: Input, pub input_buf: String, pub result: Option<(String, String)>, pub cursor: usize,
    pub cursors: [usize; 7],
    pub s3_buckets: Vec<String>, pub s3_objects: Vec<(String, i64, String)>, pub s3_bucket: String,
    pub s3_prefix: String, pub s3_folders: Vec<String>,
    pub s3_col: usize, pub s3_cursors: [usize; 3],
    pub s3_col3_folders: Vec<String>, pub s3_col3_objects: Vec<(String, i64, String)>, pub s3_col3_prefix: String,
    pub cognito_users: Vec<(String, String, String)>,
    pub secrets: Vec<(String, String, String)>,
    pub lambda_funcs: Vec<(String, String, i64)>,
    pub emr_vcs: Vec<(String, String, String)>, pub emr_jobs: Vec<(String, String, String)>, pub emr_vc_id: String,
    pub logs: Vec<(String, String, String, String)>, pub logs_filter: String, pub log_file: String,
}

impl App {
    pub fn new(aws: AwsClients, binary: &str, args: &[String]) -> Self {
        Self {
            aws, server: ServerManager::new(binary, args),
            server_status: ServerStatus::Stopped, tab: Tab::Overview,
            help_visible: false, error: None, confirming: None, last_refresh: Instant::now(),
            start_time: None, health_checked: Instant::now(), server_started: None,
            input: Input::None, input_buf: String::new(), result: None, cursor: 0,
            cursors: [0; 7],
            s3_buckets: Vec::new(), s3_objects: Vec::new(), s3_bucket: String::new(),
            s3_prefix: String::new(), s3_folders: Vec::new(),
            s3_col: 0, s3_cursors: [0; 3],
            s3_col3_folders: Vec::new(), s3_col3_objects: Vec::new(), s3_col3_prefix: String::new(),
            cognito_users: Vec::new(), secrets: Vec::new(),
            lambda_funcs: Vec::new(), emr_vcs: Vec::new(), emr_jobs: Vec::new(), emr_vc_id: String::new(),
            logs: Vec::new(), logs_filter: String::new(), log_file: String::new(),
        }
    }

    pub fn save_cursor(&mut self) { self.cursors[self.tab.idx()] = self.cursor; }
    pub fn load_cursor(&mut self, t: Tab) { self.cursor = self.cursors[t.idx()]; }

    pub async fn refresh_all(&mut self) {
        if !matches!(self.server_status, ServerStatus::Running) { return; }
        self.last_refresh = Instant::now();
        if self.log_file.is_empty() {
            for line in self.server.stderr_snapshot().lines() {
                if let Some(p) = line.strip_prefix("log_file=") { self.log_file = p.to_string(); break; }
            }
        }
        use crate::fetchers;
        match self.tab {
            Tab::Overview => {
                let (b, u, s, f, v, _) = tokio::join!(
                    fetchers::s3_buckets(&self.aws), fetchers::cognito_users(&self.aws),
                    fetchers::secrets(&self.aws), fetchers::lambda_funcs(&self.aws),
                    fetchers::emr_vcs(&self.aws), fetchers::logs(&self.aws),
                );
                self.s3_buckets = b; self.cognito_users = u; self.secrets = s; self.lambda_funcs = f; self.emr_vcs = v;
            }
            Tab::S3 => {
                self.s3_buckets = fetchers::s3_buckets(&self.aws).await;
                if !self.s3_bucket.is_empty() {
                    let (folders, files) = fetchers::s3_folder_objects(&self.aws, &self.s3_bucket, &self.s3_prefix).await;
                    self.s3_folders = folders; self.s3_objects = files;
                }
                if !self.s3_col3_prefix.is_empty() {
                    let (folders, files) = fetchers::s3_folder_objects(&self.aws, &self.s3_bucket, &self.s3_col3_prefix).await;
                    self.s3_col3_folders = folders; self.s3_col3_objects = files;
                }
            }
            Tab::Cognito => self.cognito_users = fetchers::cognito_users(&self.aws).await,
            Tab::Secrets => self.secrets = fetchers::secrets(&self.aws).await,
            Tab::Emr => {
                self.emr_vcs = fetchers::emr_vcs(&self.aws).await;
                self.emr_jobs = if self.emr_vc_id.is_empty() { Vec::new() } else { fetchers::emr_jobs(&self.aws, &self.emr_vc_id).await };
            }
            Tab::Lambda => self.lambda_funcs = fetchers::lambda_funcs(&self.aws).await,
            Tab::Logs => self.logs = fetchers::read_log_file(&self.log_file, &self.logs_filter).await,
        }
        let len = self.list_len();
        if self.cursor > len.saturating_sub(1) { self.cursor = len.saturating_sub(1); }
    }

    pub fn list_len(&self) -> usize {
        match self.tab {
            Tab::Overview => 0,
            Tab::S3 => if self.s3_bucket.is_empty() || self.s3_col == 0 { self.s3_buckets.len() } else if self.s3_col == 1 { self.s3_folders.len() + self.s3_objects.len() } else { self.s3_col3_folders.len() + self.s3_col3_objects.len() },
            Tab::Cognito => self.cognito_users.len(),
            Tab::Secrets => self.secrets.len(),
            Tab::Emr => if self.emr_vc_id.is_empty() { self.emr_vcs.len() } else { self.emr_jobs.len() },
            Tab::Lambda => self.lambda_funcs.len(),
            Tab::Logs => self.logs.len(),
        }
    }
    pub async fn submit_input(&mut self) {
        let val = std::mem::take(&mut self.input_buf);
        let prev = std::mem::replace(&mut self.input, Input::None);
        use crate::actions;
        match prev {
            Input::CreateBucket => self.result = Some(("Create Bucket".into(), actions::create_bucket(&self.aws, &val).await)),
            Input::CreateUser => self.input = Input::CreatePass(val),
            Input::CreatePass(username) => self.result = Some(("Create User".into(), actions::create_user(&self.aws, &username, &val).await)),
            Input::LambdaPayload(func) => {
                let r = actions::invoke_lambda(&self.aws, &func, &val).await;
                self.result = Some(("Invoke".into(), r));
            }
            Input::S3UploadKey(bucket, prefix) => {
                match tokio::fs::read(&val).await {
                    Ok(content) => {
                        let filename = std::path::Path::new(&val).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or(val.clone());
                        let full_key = if prefix.is_empty() { filename } else { format!("{prefix}{filename}") };
                        self.result = Some(("Upload".into(), actions::upload_object(&self.aws, &bucket, &full_key, &content).await));
                    }
                    Err(e) => self.result = Some(("Upload".into(), format!("Error: {e}"))),
                }
            }
            Input::EmrSubmit(vc) => self.result = Some(("Submit Job".into(), actions::submit_job(&self.aws, &vc, &val).await)),
            Input::LogFilter => { self.logs_filter = val; self.refresh_all().await; return; }
            Input::CreateSecret => self.input = Input::CreateSecretValue(val),
            Input::CreateSecretValue(name) => self.result = Some(("Create Secret".into(), actions::create_secret(&self.aws, &name, &val).await)),
            Input::EditSecret(name) => self.result = Some(("Edit Secret".into(), actions::edit_secret(&self.aws, &name, &val).await)),
            _ => {}
        }
        if !matches!(self.input, Input::CreatePass(_) | Input::CreateSecretValue(_)) { self.refresh_all().await; }
    }
}
