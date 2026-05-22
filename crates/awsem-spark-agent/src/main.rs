use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Command;

fn main() {
    let job_run_id = std::env::var("JOB_RUN_ID").unwrap_or_default();
    let awsem_endpoint = std::env::var("AWSEM_ENDPOINT")
        .unwrap_or_else(|_| "http://host.docker.internal:4566".into());
    let args: Vec<String> = std::env::args().skip(1).collect();

    let result = Command::new("/opt/spark/bin/spark-submit")
        .args(&args)
        .output();

    let (exit_code, stdout, stderr) = match result {
        Ok(out) => {
            let code = out.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            (code, stdout, stderr)
        }
        Err(e) => {
            let msg = format!("Failed to execute spark-submit: {e}");
            report(&awsem_endpoint, &job_run_id, &msg, -1);
            eprintln!("{msg}");
            std::process::exit(1);
        }
    };

    let all_logs = if stderr.is_empty() {
        stdout.clone()
    } else {
        format!("{stdout}\n{stderr}")
    };

    report(&awsem_endpoint, &job_run_id, &all_logs, exit_code);

    if !stdout.is_empty() {
        println!("{stdout}");
    }
    if !stderr.is_empty() {
        eprintln!("{stderr}");
    }

    std::process::exit(exit_code);
}

fn report(endpoint: &str, job_run_id: &str, logs: &str, exit_code: i32) {
    if endpoint.is_empty() || job_run_id.is_empty() {
        eprintln!("awsem-spark-agent: missing JOB_RUN_ID or AWSEM_ENDPOINT, skipping report");
        return;
    }

    let path = if exit_code == 0 {
        format!("/admin/api/emr/jobs/{job_run_id}/complete")
    } else {
        format!("/admin/api/emr/jobs/{job_run_id}/fail")
    };

    let escaped = logs
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");

    let body = format!("{{\"logs\":\"{escaped}\",\"exit_code\":{exit_code}}}");

    if let Err(e) = http_post(endpoint, &path, &body) {
        eprintln!("awsem-spark-agent: failed to report status: {e}");
    }
}

fn http_post(base_url: &str, path: &str, body: &str) -> Result<(), String> {
    let rest = base_url.strip_prefix("http://").unwrap_or(base_url);
    let (hostport, base_path) = match rest.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (rest, String::new()),
    };
    let (host, port_str) = hostport.split_once(':').unwrap_or((hostport, "80"));
    let port: u16 = port_str.parse().unwrap_or(80);
    let full_path = format!("{base_path}{path}");

    let mut conn = TcpStream::connect((host, port))
        .map_err(|e| format!("connect({host}:{port}): {e}"))?;

    conn.set_write_timeout(Some(std::time::Duration::from_secs(5))).ok();
    conn.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();

    let request = format!(
        "POST {full_path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n\
         {body}",
        body.len()
    );

    conn.write_all(request.as_bytes())
        .map_err(|e| format!("write: {e}"))?;

    let mut buf = [0u8; 1024];
    let _ = conn.read(&mut buf);
    Ok(())
}
