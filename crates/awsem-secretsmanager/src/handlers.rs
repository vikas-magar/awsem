use crate::queries;
use crate::store;
use crate::AppState;
use actix_web::{HttpRequest, HttpResponse, web};
use serde_json::{Value, json};

fn make_arn(name: &str) -> String {
    format!("arn:aws:secretsmanager:us-east-1:000000000000:secret:{name}")
}

fn find_secret(input: &Value, state: &AppState) -> Result<store::Secret, HttpResponse> {
    let secret_id = input.get("SecretId").and_then(|v| v.as_str()).unwrap_or("");
    queries::get_secret_by_name(&state.db, secret_id)
        .or_else(|_| queries::get_secret_by_arn(&state.db, secret_id))
        .map_err(|e| awsem_core::error::AwsemError::NotFound(e).secrets_response())
}

#[tracing::instrument(skip(req, body, state))]
pub async fn handle(req: HttpRequest, body: bytes::Bytes, state: web::Data<AppState>) -> HttpResponse {
    let target = req.headers().get("X-Amz-Target").and_then(|v| v.to_str().ok()).unwrap_or("");
    let input: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return awsem_core::error::AwsemError::InvalidRequest(e.to_string()).secrets_response(),
    };
    match target {
        "secretsmanager.CreateSecret" => create_secret(input, &state).await,
        "secretsmanager.GetSecretValue" => get_secret_value(input, &state).await,
        "secretsmanager.PutSecretValue" => put_secret_value(input, &state).await,
        "secretsmanager.UpdateSecret" => update_secret(input, &state).await,
        "secretsmanager.DeleteSecret" => delete_secret(input, &state).await,
        "secretsmanager.DescribeSecret" => crate::read::describe_secret(input, &state).await,
        "secretsmanager.ListSecrets" => crate::read::list_secrets(&state).await,
        "secretsmanager.RestoreSecret" => restore_secret(input, &state).await,
        _ => awsem_core::error::AwsemError::NotImplemented(target.into()).secrets_response(),
    }
}

async fn create_secret(input: Value, state: &AppState) -> HttpResponse {
    let name = input.get("Name").and_then(|v| v.as_str()).unwrap_or("");
    if name.is_empty() { return awsem_core::error::AwsemError::InvalidRequest("Name is required".into()).secrets_response(); }
    let secret_string = input.get("SecretString").and_then(|v| v.as_str()).unwrap_or("");
    let description = input.get("Description").and_then(|v| v.as_str());
    let kms_key_id = input.get("KmsKeyId").and_then(|v| v.as_str());
    let id = uuid::Uuid::new_v4().to_string();
    let arn = make_arn(name);
    match store::create_secret(&state.db, &id, name, &arn, secret_string, description, kms_key_id) {
        Ok(version_id) => HttpResponse::Ok().json(json!({"ARN": arn, "Name": name, "VersionId": version_id})),
        Err(e) => awsem_core::error::AwsemError::AlreadyExists(e).secrets_response(),
    }
}

async fn get_secret_value(input: Value, state: &AppState) -> HttpResponse {
    let secret = match find_secret(&input, state) { Ok(s) => s, Err(e) => return e };
    if secret.deleted_at.is_some() { return awsem_core::error::AwsemError::NotFound("Secret is deleted".into()).secrets_response(); }
    let version = match queries::get_latest_version(&state.db, &secret.id) {
        Ok(v) => v, Err(e) => return awsem_core::error::AwsemError::Internal(e).secrets_response(),
    };
    HttpResponse::Ok().json(json!({"ARN": secret.arn, "Name": secret.name, "SecretString": version.value, "VersionId": version.version_id}))
}

async fn put_secret_value(input: Value, state: &AppState) -> HttpResponse {
    let secret_string = input.get("SecretString").and_then(|v| v.as_str()).unwrap_or("");
    let secret = match find_secret(&input, state) { Ok(s) => s, Err(e) => return e };
    match store::put_secret_value(&state.db, &secret.id, secret_string) {
        Ok(version_id) => HttpResponse::Ok().json(json!({"ARN": secret.arn, "Name": secret.name, "VersionId": version_id})),
        Err(e) => awsem_core::error::AwsemError::Internal(e).secrets_response(),
    }
}

async fn update_secret(input: Value, state: &AppState) -> HttpResponse {
    let description = input.get("Description").and_then(|v| v.as_str());
    let kms_key_id = input.get("KmsKeyId").and_then(|v| v.as_str());
    let secret = match find_secret(&input, state) { Ok(s) => s, Err(e) => return e };
    if let Err(e) = store::update_secret(&state.db, &secret.id, description, kms_key_id) {
        return awsem_core::error::AwsemError::Internal(e).secrets_response();
    }
    HttpResponse::Ok().json(json!({"ARN": secret.arn, "Name": secret.name}))
}

async fn delete_secret(input: Value, state: &AppState) -> HttpResponse {
    let secret = match find_secret(&input, state) { Ok(s) => s, Err(e) => return e };
    if let Err(e) = store::soft_delete(&state.db, &secret.id) {
        return awsem_core::error::AwsemError::Internal(e).secrets_response();
    }
    HttpResponse::Ok().json(json!({"ARN": secret.arn, "Name": secret.name, "DeletionDate": chrono::Utc::now().timestamp_millis()}))
}

async fn restore_secret(input: Value, state: &AppState) -> HttpResponse {
    let secret = match find_secret(&input, state) { Ok(s) => s, Err(e) => return e };
    if let Err(e) = store::restore(&state.db, &secret.id) {
        return awsem_core::error::AwsemError::Internal(e).secrets_response();
    }
    HttpResponse::Ok().json(json!({"ARN": secret.arn, "Name": secret.name}))
}
