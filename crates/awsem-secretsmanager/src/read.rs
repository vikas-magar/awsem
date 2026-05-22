use crate::queries;
use crate::AppState;
use actix_web::HttpResponse;
use serde_json::{Value, json};

fn to_date(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s).map(|d| d.timestamp_millis()).unwrap_or(0)
}

pub async fn describe_secret(input: Value, state: &AppState) -> HttpResponse {
    let secret = match input.get("SecretId").and_then(|v| v.as_str())
        .and_then(|id| queries::get_secret_by_name(&state.db, id).ok()
            .or_else(|| queries::get_secret_by_arn(&state.db, id).ok()))
    {
        Some(s) => s,
        None => return awsem_core::error::AwsemError::NotFound("Secret not found".into()).secrets_response(),
    };
    HttpResponse::Ok().json(json!({
        "ARN": secret.arn, "Name": secret.name,
        "Description": secret.description, "KmsKeyId": secret.kms_key_id,
        "CreatedDate": to_date(&secret.created_at),
        "LastChangedDate": to_date(&secret.last_changed),
        "Tags": serde_json::from_str::<Value>(&secret.tags_json).unwrap_or(json!([])),
    }))
}

pub async fn list_secrets(state: &AppState) -> HttpResponse {
    match queries::list_secrets(&state.db) {
        Ok(secrets) => {
            let list: Vec<Value> = secrets.into_iter().map(|s| json!({
                "ARN": s.arn, "Name": s.name,
                "Description": s.description, "KmsKeyId": s.kms_key_id,
                "CreatedDate": to_date(&s.created_at),
                "LastChangedDate": to_date(&s.last_changed),
                "Tags": serde_json::from_str::<Value>(&s.tags_json).unwrap_or(json!([])),
            })).collect();
            HttpResponse::Ok().json(json!({"SecretList": list, "NextToken": null}))
        }
        Err(e) => awsem_core::error::AwsemError::Internal(e).secrets_response(),
    }
}
