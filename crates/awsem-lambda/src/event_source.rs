use crate::AppState;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(rename = "FunctionName")]
    pub function_name: Option<String>,
}

pub async fn create(
    state: web::Data<AppState>,
    body: bytes::Bytes,
) -> HttpResponse {
    let input: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return awsem_core::error::AwsemError::InvalidRequest(e.to_string()).to_response(),
    };
    let fn_arn = input.get("FunctionName").and_then(|v| v.as_str()).unwrap_or("");
    let event_arn = input.get("EventSourceArn").and_then(|v| v.as_str()).unwrap_or("");
    let batch = input.get("BatchSize").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
    let id = uuid::Uuid::new_v4().to_string();
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    if let Err(e) = c.execute(
        "INSERT INTO lambda_event_source_mappings (id, function_arn, event_source_arn, enabled, batch_size) VALUES (?1, ?2, ?3, 1, ?4)",
        rusqlite::params![id, fn_arn, event_arn, batch],
    ) {
        return awsem_core::error::AwsemError::AlreadyExists(e.to_string()).to_response();
    }
    HttpResponse::Ok().json(json!({
        "UUID": id, "FunctionArn": fn_arn, "EventSourceArn": event_arn,
        "BatchSize": batch, "State": "Enabled",
    }))
}

pub async fn list(
    state: web::Data<AppState>,
    query: web::Query<ListQuery>,
) -> HttpResponse {
    let fn_arn = query.function_name.as_deref();
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let mut stmt = match c.prepare(
        "SELECT id, function_arn, event_source_arn, batch_size FROM lambda_event_source_mappings WHERE (?1 IS NULL OR function_arn = ?1)"
    ) {
        Ok(s) => s,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let rows = match stmt.query_map(rusqlite::params![fn_arn], |row| {
        Ok(json!({
            "UUID": row.get::<_, String>(0)?,
            "FunctionArn": row.get::<_, String>(1)?,
            "EventSourceArn": row.get::<_, String>(2)?,
            "BatchSize": row.get::<_, i32>(3)?,
            "State": "Enabled",
        }))
    }) {
        Ok(r) => r,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let mut mappings = Vec::new();
    for row in rows {
        match row {
            Ok(m) => mappings.push(m),
            Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
        }
    }
    HttpResponse::Ok().json(json!({"EventSourceMappings": mappings, "NextMarker": serde_json::Value::Null}))
}

pub async fn delete(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let uuid = path.into_inner();
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    if c.execute("DELETE FROM lambda_event_source_mappings WHERE id = ?1", rusqlite::params![uuid]).is_err() {
        return awsem_core::error::AwsemError::NotFound(uuid).to_response();
    }
    HttpResponse::Ok().json(json!({}))
}
