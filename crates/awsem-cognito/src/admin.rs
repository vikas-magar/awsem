use crate::store;
use crate::AppState;
use actix_web::HttpResponse;
use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::Argon2;
use rand::rngs::OsRng;
use serde_json::{json, Value};
use awsem_core::error::AwsemError;

pub async fn admin_create_user(input: Value, state: &AppState) -> HttpResponse {
    let username = input.get("Username").and_then(|v| v.as_str()).unwrap_or("");
    let pid = input.get("UserPoolId").and_then(|v| v.as_str()).unwrap_or("us-east-1_default");
    let password = input.pointer("/TemporaryPassword").and_then(|v| v.as_str()).unwrap_or("Pass1234!");
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = match Argon2::default().hash_password(password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(e) => return AwsemError::Internal(e.to_string()).cognito_response(),
    };
    let sub = uuid::Uuid::new_v4().to_string();
    let arn = format!("arn:aws:cognito-idp:us-east-1:000000000000:userpool/{pid}");
    let _ = store::create_user_pool(&state.db, pid, "default", &arn);
    if let Err(e) = store::create_user(&state.db, &sub, pid, username, &password_hash, None, "{}", "CONFIRMED") {
        return AwsemError::InvalidRequest(e).cognito_response();
    }
    let now = chrono::Utc::now().to_rfc3339();
    HttpResponse::Ok().json(json!({"User": {
        "Username": username, "UserStatus": "CONFIRMED", "UserCreateDate": now,
        "UserLastModifiedDate": now, "Enabled": true, "UserPoolId": pid,
    }}))
}

pub async fn admin_get_user(input: Value, state: &AppState) -> HttpResponse {
    let username = input.get("Username").and_then(|v| v.as_str()).unwrap_or("");
    let pid = input.get("UserPoolId").and_then(|v| v.as_str()).unwrap_or("us-east-1_default");
    let user = match store::get_user_by_username(&state.db, pid, username) {
        Ok(u) => u,
        Err(e) => return AwsemError::NotFound(e).cognito_response(),
    };
    let now = chrono::Utc::now().to_rfc3339();
    HttpResponse::Ok().json(json!({
        "Username": username, "UserStatus": user.status, "Enabled": true,
        "UserCreateDate": now, "UserLastModifiedDate": now, "UserPoolId": pid,
        "UserAttributes": [
            {"Name": "sub", "Value": user.id},
            {"Name": "email", "Value": user.email.unwrap_or_default()},
        ],
    }))
}
