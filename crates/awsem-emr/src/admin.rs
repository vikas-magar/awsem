use crate::AppState;
use actix_web::{HttpResponse, web};
use rusqlite::params;
use serde_json::{Value, json};

pub async fn job_complete(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: bytes::Bytes,
) -> HttpResponse {
    let id = path.into_inner();
    let input: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return awsem_core::error::AwsemError::InvalidRequest(e.to_string()).emr_response(),
    };
    let logs = input.get("logs").and_then(|v| v.as_str()).unwrap_or("");
    let exit_code = input.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
    let c = awsem_core::lock_db!(state);
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = c.execute(
        "UPDATE emr_job_runs SET state = 'COMPLETED', logs = ?1, exit_code = ?2, finished_at = ?3 WHERE id = ?4",
        params![logs, exit_code, now, id],
    ) {
        return awsem_core::error::AwsemError::Internal(e.to_string()).emr_response();
    }
    HttpResponse::Ok().json(json!({"status": "ok"}))
}

pub async fn job_fail(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: bytes::Bytes,
) -> HttpResponse {
    let id = path.into_inner();
    let input: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return awsem_core::error::AwsemError::InvalidRequest(e.to_string()).emr_response(),
    };
    let logs = input.get("logs").and_then(|v| v.as_str()).unwrap_or("");
    let exit_code = input.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(1);
    let c = awsem_core::lock_db!(state);
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = c.execute(
        "UPDATE emr_job_runs SET state = 'FAILED', logs = ?1, exit_code = ?2, finished_at = ?3 WHERE id = ?4",
        params![logs, exit_code, now, id],
    ) {
        return awsem_core::error::AwsemError::Internal(e.to_string()).emr_response();
    }
    HttpResponse::Ok().json(json!({"status": "ok"}))
}
