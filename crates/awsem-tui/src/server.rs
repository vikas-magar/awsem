use std::process::ExitStatus;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};

pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Failed,
}

pub struct ServerManager {
    pub child: Option<Child>,
    binary: String,
    args: Vec<String>,
    stderr_buf: Arc<Mutex<String>>,
}

impl ServerManager {
    pub fn new(binary: &str, args: &[String]) -> Self {
        Self {
            child: None,
            binary: binary.to_string(),
            args: args.to_vec(),
            stderr_buf: Arc::new(Mutex::new(String::new())),
        }
    }

    pub async fn start(&mut self) -> Result<(), String> {
        let mut child = Command::new(&self.binary)
            .args(&self.args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start awsem: {e}"))?;

        if let Some(mut stderr) = child.stderr.take() {
            let buf = self.stderr_buf.clone();
            tokio::spawn(async move {
                let mut tmp = [0u8; 4096];
                while let Ok(n) = stderr.read(&mut tmp).await {
                    if n == 0 { break; }
                    if let Ok(mut guard) = buf.lock() {
                        guard.push_str(&String::from_utf8_lossy(&tmp[..n]));
                    }
                }
            });
        }

        self.child = Some(child);
        Ok(())
    }

    pub fn stderr_snapshot(&self) -> String {
        self.stderr_buf
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Returns (exit_status, stderr) if process has exited.
    pub async fn check_exit(&mut self) -> Option<(ExitStatus, String)> {
        let child = self.child.as_mut()?;
        let status = child.try_wait().ok()??;
        let stderr = self.stderr_snapshot();
        Some((status, stderr))
    }

    pub async fn stop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill().await;
            let _ = child.wait().await;
            self.child = None;
        }
    }
}

impl Drop for ServerManager {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.start_kill();
        }
    }
}
