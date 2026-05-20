use crate::AppState;
use actix_web::HttpResponse;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{json, Value};

pub async fn invoke_function(input: Value, state: &AppState) -> HttpResponse {
    let name = input.get("FunctionName").and_then(|v| v.as_str()).unwrap_or("");
    let payload = input.get("Payload").and_then(|v| v.as_str()).unwrap_or("{}");
    let func = {
        let c = match state.db.lock() {
            Ok(c) => c,
            Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).to_response(),
        };
        match c.query_row(
            "SELECT name, arn, runtime, handler FROM lambda_functions WHERE name = ?1",
            rusqlite::params![name],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)),
        ) {
            Ok(f) => f,
            Err(_) => return awsem_core::error::AwsemError::NotFound(name.into()).to_response(),
        }
    };
    let _decoded_payload = match BASE64.decode(payload) {
        Ok(d) => String::from_utf8_lossy(&d).to_string(),
        Err(_) => payload.to_string(),
    };
    tracing::info!("Invoking Lambda {name} (runtime: {})", func.2);
    let result = crate::execute::run(&func.0, &func.2, &func.3).await;
    let response_body = BASE64.encode(result.as_bytes());
    HttpResponse::Ok()
        .insert_header(("X-Amz-Function-Error", "null"))
        .json(json!({
            "StatusCode": 200,
            "ExecutedVersion": "$LATEST",
            "Payload": response_body,
        }))
}
