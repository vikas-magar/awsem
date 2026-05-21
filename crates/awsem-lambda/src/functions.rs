use crate::AppState;
use actix_web::{web, HttpResponse};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{json, Value};

#[tracing::instrument(skip(state, body))]
pub async fn create(
    state: web::Data<AppState>,
    body: bytes::Bytes,
) -> HttpResponse {
    let input: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return awsem_core::error::AwsemError::InvalidRequest(e.to_string()).to_response(),
    };
    let name = input.get("FunctionName").and_then(|v| v.as_str()).unwrap_or("");
    if let Err(msg) = crate::name::validate(name) {
        return awsem_core::error::AwsemError::InvalidRequest(format!("Invalid FunctionName: {msg}")).to_response();
    }
    let runtime = input.get("Runtime").and_then(|v| v.as_str()).unwrap_or("provided.al2023");
    let handler = input.get("Handler").and_then(|v| v.as_str()).unwrap_or("");
    let role = input.get("Role").and_then(|v| v.as_str()).unwrap_or("");
    let image = input.get("Image").and_then(|v| v.as_str()).map(|s| s.to_string())
        .or_else(|| state.lambda_runtime_image.clone());
    let arn = format!("arn:aws:lambda:us-east-1:000000000000:function:{name}");
    let timeout = input.get("Timeout").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
    let memory = input.get("MemorySize").and_then(|v| v.as_i64()).unwrap_or(128) as i32;
    let code_zip = input.pointer("/Code/ZipFile").and_then(|v| v.as_str())
        .and_then(|b| BASE64.decode(b).ok());

    let now = chrono::Utc::now().to_rfc3339();
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    if let Err(e) = c.execute(
        "INSERT INTO lambda_functions (name, arn, runtime, handler, image, role, timeout, memory_size, code_zip, created_at, last_modified) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        rusqlite::params![name, arn, runtime, handler, image, role, timeout, memory, code_zip, now],
    ) {
        return awsem_core::error::AwsemError::AlreadyExists(e.to_string()).to_response();
    }
    if let Some(zip) = code_zip {
        crate::extract::save_code_zip(&state.data_dir, name, &zip);
    }
    HttpResponse::Created().json(json!({
        "FunctionName": name, "FunctionArn": arn,
        "Runtime": runtime, "Handler": handler, "Role": role,
        "Timeout": timeout, "MemorySize": memory,
        "LastModified": now,
        "CodeSha256": "0", "Version": "$LATEST",
    }))
}

pub async fn get(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let name = path.into_inner();
    if let Err(msg) = crate::name::validate(&name) {
        return awsem_core::error::AwsemError::InvalidRequest(format!("Invalid FunctionName: {msg}")).to_response();
    }
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
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
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
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

pub async fn delete_fn(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let name = path.into_inner();
    if let Err(msg) = crate::name::validate(&name) {
        return awsem_core::error::AwsemError::InvalidRequest(format!("Invalid FunctionName: {msg}")).to_response();
    }
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
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
