use crate::AppState;
use actix_web::HttpResponse;
use serde_json::{json, Value};

pub async fn create_mapping(input: Value, state: &AppState) -> HttpResponse {
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

pub async fn list_mappings(input: Value, state: &AppState) -> HttpResponse {
    let fn_arn = input.get("FunctionName").and_then(|v| v.as_str());
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
    HttpResponse::Ok().json(json!({"EventSourceMappings": mappings, "NextMarker": null}))
}

pub async fn delete_mapping(input: Value, state: &AppState) -> HttpResponse {
    let uuid = input.get("UUID").and_then(|v| v.as_str()).unwrap_or("");
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    if c.execute("DELETE FROM lambda_event_source_mappings WHERE id = ?1", rusqlite::params![uuid]).is_err() {
        return awsem_core::error::AwsemError::NotFound(uuid.into()).to_response();
    }
    HttpResponse::Ok().json(json!({}))
}
