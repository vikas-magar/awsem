use std::time::Instant;
use crate::aws_clients::AwsClients;
use crate::form::{FormAction, FormState};
use crate::server::{ServerManager, ServerStatus};
use crate::types::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode { Dashboard, BucketBrowser, DetailPopup, FormPopup, UploadMode, ConfirmPopup }

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
    pub browser_sort_desc: bool,
    pub popup_title: String,
    pub popup_lines: Vec<(String, String)>,
    pub confirm_item: String,
    pub confirm_id: String,
    pub confirm_panel: usize,
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
            browser_sort_desc: false,
            popup_title: String::new(), popup_lines: Vec::new(),
            confirm_item: String::new(), confirm_id: String::new(), confirm_panel: 0,
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
}
