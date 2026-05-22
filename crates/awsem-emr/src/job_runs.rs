use crate::AppState;
use actix_web::{HttpResponse, web};
use chrono::NaiveDateTime;
use rusqlite::params;
use serde_json::{Value, json};

fn to_rfc3339(sqlite_ts: &str) -> String {
    NaiveDateTime::parse_from_str(sqlite_ts, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.and_utc().to_rfc3339())
        .unwrap_or_else(|_| sqlite_ts.to_string())
}

pub async fn list(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let vc_id = path.into_inner();
    let c = awsem_core::lock_db!(state);
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
            "createdAt": to_rfc3339(&row.get::<_, String>(5)?),
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

pub async fn describe(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (vc_id, jr_id) = path.into_inner();
    let c = awsem_core::lock_db!(state);
    match c.query_row(
        "SELECT id, name, arn, virtual_cluster_id, state, created_at, job_driver_json, COALESCE(logs,''), COALESCE(exit_code,0) FROM emr_job_runs WHERE id = ?1 AND virtual_cluster_id = ?2",
        params![jr_id, vc_id],
        |row| Ok(json!({
            "id": row.get::<_, String>(0)?,
            "name": row.get::<_, String>(1)?,
            "arn": row.get::<_, String>(2)?,
            "virtualClusterId": row.get::<_, String>(3)?,
            "state": row.get::<_, String>(4)?,
            "createdAt": to_rfc3339(&row.get::<_, String>(5)?),
            "jobDriver": serde_json::from_str::<Value>(&row.get::<_, String>(6)?).unwrap_or_default(),
            "logs": row.get::<_, String>(7)?,
            "exitCode": row.get::<_, i64>(8)?,
        })),
    ) {
        Ok(run) => HttpResponse::Ok().json(json!({"jobRun": run})),
        Err(_) => awsem_core::error::AwsemError::NotFound(jr_id).to_response(),
    }
}

pub async fn cancel(state: web::Data<AppState>, path: web::Path<(String, String)>) -> HttpResponse {
    let (vc_id, jr_id) = path.into_inner();
    let c = awsem_core::lock_db!(state);
    if c.execute(
        "UPDATE emr_job_runs SET state = 'CANCELLED' WHERE id = ?1 AND virtual_cluster_id = ?2",
        params![jr_id, vc_id],
    )
    .is_err()
    {
        return awsem_core::error::AwsemError::NotFound(jr_id).to_response();
    }
    HttpResponse::Ok().json(json!({"id": jr_id, "virtualClusterId": vc_id}))
}
