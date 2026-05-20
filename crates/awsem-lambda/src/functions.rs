use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::{json, Value};

pub async fn handle(
    req: HttpRequest,
    body: bytes::Bytes,
    state: web::Data<AppState>,
) -> HttpResponse {
    let target = req
        .headers()
        .get("X-Amz-Target")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let input: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return awsem_core::error::AwsemError::InvalidRequest(e.to_string())
                .to_response()
        }
    };
    match target {
        "AWSLambda.CreateFunction" => create_function(input, &state).await,
        "AWSLambda.GetFunction" => get_function(input, &state).await,
        "AWSLambda.ListFunctions" => list_functions(&state).await,
        "AWSLambda.DeleteFunction" => delete_function(input, &state).await,
        "AWSLambda.Invoke" => crate::invoke::invoke_function(input, &state).await,
        "AWSLambda.CreateEventSourceMapping" => crate::event_source::create_mapping(input, &state).await,
        "AWSLambda.ListEventSourceMappings" => crate::event_source::list_mappings(input, &state).await,
        "AWSLambda.DeleteEventSourceMapping" => crate::event_source::delete_mapping(input, &state).await,
        _ => awsem_core::error::AwsemError::NotImplemented(target.into()).to_response(),
    }
}

async fn create_function(input: Value, state: &AppState) -> HttpResponse {
    let name = input.get("FunctionName").and_then(|v| v.as_str()).unwrap_or("");
    let runtime = input.get("Runtime").and_then(|v| v.as_str()).unwrap_or("provided.al2023");
    let handler = input.get("Handler").and_then(|v| v.as_str()).unwrap_or("");
    let role = input.get("Role").and_then(|v| v.as_str()).unwrap_or("");
    let arn = format!("arn:aws:lambda:us-east-1:000000000000:function:{name}");
    let timeout = input.get("Timeout").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
    let memory = input.get("MemorySize").and_then(|v| v.as_i64()).unwrap_or(128) as i32;
    let now = chrono::Utc::now().to_rfc3339();
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    if let Err(e) = c.execute(
        "INSERT INTO lambda_functions (name, arn, runtime, handler, role, timeout, memory_size, created_at, last_modified) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        rusqlite::params![name, arn, runtime, handler, role, timeout, memory, now],
    ) {
        return awsem_core::error::AwsemError::AlreadyExists(e.to_string()).to_response();
    }
    HttpResponse::Ok().json(json!({
        "FunctionName": name, "FunctionArn": arn,
        "Runtime": runtime, "Handler": handler, "Role": role,
        "Timeout": timeout, "MemorySize": memory,
        "LastModified": now,
    }))
}

async fn get_function(input: Value, state: &AppState) -> HttpResponse {
    let name = input.get("FunctionName").and_then(|v| v.as_str()).unwrap_or("");
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
        Err(_) => return awsem_core::error::AwsemError::NotFound(name.into()).to_response(),
    };
    HttpResponse::Ok().json(json!({"Configuration": func}))
}

async fn list_functions(state: &AppState) -> HttpResponse {
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
    HttpResponse::Ok().json(json!({"Functions": funcs, "NextMarker": null}))
}

async fn delete_function(input: Value, state: &AppState) -> HttpResponse {
    let name = input.get("FunctionName").and_then(|v| v.as_str()).unwrap_or("");
    let c = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
    };
    if c.execute("DELETE FROM lambda_functions WHERE name = ?1", rusqlite::params![name]).is_err() {
        return awsem_core::error::AwsemError::NotFound(name.into()).to_response();
    }
    HttpResponse::Ok().json(json!({}))
}
