use crate::AppState;
use actix_web::{HttpResponse, web};
use serde_json::json;

pub async fn get(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let name = path.into_inner();
    if let Err(msg) = crate::name::validate(&name) {
        return awsem_core::error::AwsemError::InvalidRequest(format!("Invalid FunctionName: {msg}")).to_response();
    }
    let c = awsem_core::lock_db!(state);
    let func = match c.query_row(
        "SELECT name, arn, runtime, handler, role, timeout, memory_size, last_modified FROM lambda_functions WHERE name = ?1",
        rusqlite::params![name],
        |row| Ok(json!({
            "FunctionName": row.get::<_, String>(0)?,
            "FunctionArn": row.get::<_, String>(1)?,
            "Runtime": row.get::<_, String>(2)?,
            "Handler": row.get::<_, String>(3)?,
            "Role": row.get::<_, String>(4)?,
            "Timeout": row.get::<_, i32>(5)?,
            "MemorySize": row.get::<_, i32>(6)?,
            "LastModified": row.get::<_, String>(7)?,
        })),
    ) {
        Ok(f) => f,
        Err(_) => return awsem_core::error::AwsemError::NotFound(name).to_response(),
    };
    HttpResponse::Ok().json(json!({"Configuration": func}))
}

pub async fn list(state: web::Data<AppState>) -> HttpResponse {
    let c = awsem_core::lock_db!(state);
    let mut stmt = match c.prepare(
        "SELECT name, arn, runtime, handler, role, timeout, memory_size, last_modified FROM lambda_functions ORDER BY last_modified DESC"
    ) {
        Ok(s) => s,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let rows = match stmt.query_map([], |row| {
        Ok(json!({
            "FunctionName": row.get::<_, String>(0)?,
            "FunctionArn": row.get::<_, String>(1)?,
            "Runtime": row.get::<_, String>(2)?,
            "Handler": row.get::<_, String>(3)?,
            "LastModified": row.get::<_, String>(7)?,
        }))
    }) {
        Ok(r) => r,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    let mut funcs = Vec::new();
    for row in rows {
        match row {
            Ok(f) => funcs.push(f),
            Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
        }
    }
    HttpResponse::Ok().json(json!({"Functions": funcs, "NextMarker": serde_json::Value::Null}))
}

pub async fn delete_fn(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let name = path.into_inner();
    if let Err(msg) = crate::name::validate(&name) {
        return awsem_core::error::AwsemError::InvalidRequest(format!("Invalid FunctionName: {msg}")).to_response();
    }
    let c = awsem_core::lock_db!(state);
    if c.execute("DELETE FROM lambda_functions WHERE name = ?1", rusqlite::params![name]).is_err() {
        return awsem_core::error::AwsemError::NotFound(name).to_response();
    }
    if let Some(dir) = state.data_dir.as_ref() {
        let path = format!("{dir}/lambdas/{name}");
        if let Err(e) = std::fs::remove_dir_all(&path) {
            tracing::warn!("Failed to remove {path}: {e}");
        }
    }
    HttpResponse::Ok().json(json!({}))
}
