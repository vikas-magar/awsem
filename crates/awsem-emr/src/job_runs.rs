use crate::AppState;
use actix_web::{web, HttpResponse};
use rusqlite::params;
use serde_json::{json, Value};

pub async fn list(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let vc_id = path.into_inner();
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let mut stmt = match c.prepare(
        "SELECT id, name, arn, virtual_cluster_id, state, created_at FROM emr_job_runs WHERE virtual_cluster_id = ?1 ORDER BY created_at DESC"
    ) {
        Ok(s) => s,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let rows = match stmt.query_map(params![vc_id], |row| {
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "name": row.get::<_, String>(1)?,
            "arn": row.get::<_, String>(2)?,
            "virtualClusterId": row.get::<_, String>(3)?,
            "state": row.get::<_, String>(4)?,
            "createdAt": row.get::<_, String>(5)?,
        }))
    }) {
        Ok(r) => r,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let mut runs = Vec::new();
    for row in rows {
        match row {
            Ok(r) => runs.push(r),
            Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
        }
    }
    HttpResponse::Ok().json(json!({"jobRuns": runs}))
}

pub async fn describe(state: web::Data<AppState>, path: web::Path<(String, String)>) -> HttpResponse {
    let (vc_id, jr_id) = path.into_inner();
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    match c.query_row(
        "SELECT id, name, arn, virtual_cluster_id, state, created_at, job_driver_json FROM emr_job_runs WHERE id = ?1 AND virtual_cluster_id = ?2",
        params![jr_id, vc_id],
        |row| Ok(json!({
            "id": row.get::<_, String>(0)?,
            "name": row.get::<_, String>(1)?,
            "arn": row.get::<_, String>(2)?,
            "virtualClusterId": row.get::<_, String>(3)?,
            "state": row.get::<_, String>(4)?,
            "createdAt": row.get::<_, String>(5)?,
            "jobDriver": serde_json::from_str::<Value>(&row.get::<_, String>(6)?).unwrap_or_default(),
        })),
    ) {
        Ok(run) => HttpResponse::Ok().json(run),
        Err(_) => awsem_core::error::AwsemError::NotFound(jr_id).to_response(),
    }
}

pub async fn cancel(state: web::Data<AppState>, path: web::Path<(String, String)>) -> HttpResponse {
    let (vc_id, jr_id) = path.into_inner();
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    if c.execute("UPDATE emr_job_runs SET state = 'CANCELLED' WHERE id = ?1 AND virtual_cluster_id = ?2", params![jr_id, vc_id]).is_err() {
        return awsem_core::error::AwsemError::NotFound(jr_id).to_response();
    }
    HttpResponse::Ok().json(json!({"id": jr_id, "virtualClusterId": vc_id}))
}
