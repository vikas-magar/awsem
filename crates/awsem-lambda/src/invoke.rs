use crate::AppState;
use actix_web::{HttpRequest, HttpResponse, web};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};

#[tracing::instrument(skip(req, body, state))]
pub async fn handle(
    req: HttpRequest,
    body: bytes::Bytes,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let name = path.into_inner();
    if let Err(msg) = crate::name::validate(&name) {
        return awsem_core::error::AwsemError::InvalidRequest(format!(
            "Invalid FunctionName: {msg}"
        ))
        .to_response();
    }
    let invocation_type = req
        .headers()
        .get("X-Amz-Invocation-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("RequestResponse");

    let payload: Value = if body.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(&body).unwrap_or(json!({}))
    };

    let func = {
        let c = awsem_core::lock_db!(state);
        match c.query_row(
            "SELECT name, arn FROM lambda_functions WHERE name = ?1",
            rusqlite::params![name],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ) {
            Ok(f) => f,
            Err(_) => return awsem_core::error::AwsemError::NotFound(name).to_response(),
        }
    };

    if invocation_type == "Event" {
        let _ = state
            .event_bus
            .send(awsem_events::BusEvent::LambdaInvocation {
                function_arn: func.1,
                payload: payload.to_string(),
            });
        return HttpResponse::Accepted().finish();
    }

    let result = crate::execute::run(&func.0, &payload.to_string(), &state).await;

    let response_body = BASE64.encode(result.as_bytes());
    HttpResponse::Ok()
        .insert_header(("X-Amz-Function-Error", "null"))
        .json(json!({
            "StatusCode": 200,
            "ExecutedVersion": "$LATEST",
            "Payload": response_body,
        }))
}
