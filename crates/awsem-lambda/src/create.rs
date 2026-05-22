use crate::AppState;
use actix_web::{HttpResponse, web};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};

#[tracing::instrument(skip(state, body))]
pub async fn create(state: web::Data<AppState>, body: bytes::Bytes) -> HttpResponse {
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
    let c = awsem_core::lock_db!(state);
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
