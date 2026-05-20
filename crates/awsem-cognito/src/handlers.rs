use crate::jwt;
use crate::store;
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::rngs::OsRng;
use serde_json::{json, Value};

fn default_pool_id() -> &'static str {
    "us-east-1_default"
}

#[tracing::instrument(skip(req, body, state))]
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
                .cognito_response()
        }
    };
    match target {
        "AWSCognitoIdentityProviderService.SignUp" => handle_sign_up(input, &state).await,
        "AWSCognitoIdentityProviderService.InitiateAuth" => {
            handle_initiate_auth(input, &state).await
        }
        "AWSCognitoIdentityProviderService.RespondToAuthChallenge" => {
            HttpResponse::Ok().json(json!({"ChallengeName": "NEW_PASSWORD_REQUIRED", "Session": "mock-session"}))
        }
        "AWSCognitoIdentityProviderService.GlobalSignOut" => HttpResponse::Ok().json(json!({})),
        "AWSCognitoIdentityProviderService.GetUser" => handle_get_user(input, &state).await,
        _ => awsem_core::error::AwsemError::NotImplemented(target.into()).cognito_response(),
    }
}

async fn handle_sign_up(input: Value, state: &AppState) -> HttpResponse {
    let username = input.get("Username").and_then(|v| v.as_str()).unwrap_or("");
    let password = input.get("Password").and_then(|v| v.as_str()).unwrap_or("");
    let pool_id = default_pool_id();
    let _ = store::create_user_pool(
        &state.db,
        pool_id,
        "default",
        &format!("arn:aws:cognito-idp:us-east-1:000000000000:userpool/{pool_id}"),
    );
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = match Argon2::default()
        .hash_password(password.as_bytes(), &salt)
    {
        Ok(h) => h.to_string(),
        Err(e) => {
            return awsem_core::error::AwsemError::Internal(e.to_string()).cognito_response()
        }
    };
    let sub = uuid::Uuid::new_v4().to_string();
    if let Err(e) = store::create_user(&state.db, &sub, pool_id, username, &password_hash, None, "{}") {
        return awsem_core::error::AwsemError::InvalidRequest(e).cognito_response();
    }
    HttpResponse::Ok().json(json!({
        "UserConfirmed": false,
        "UserSub": sub,
        "CodeDeliveryDetails": null,
    }))
}

async fn handle_initiate_auth(input: Value, state: &AppState) -> HttpResponse {
    let auth_params = input
        .get("AuthParameters")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let username = auth_params.get("USERNAME").and_then(|v| v.as_str()).unwrap_or("");
    let password = auth_params.get("PASSWORD").and_then(|v| v.as_str()).unwrap_or("");
    let pool_id = default_pool_id();
    let user = match store::get_user_by_username(&state.db, pool_id, username) {
        Ok(u) => u,
        Err(e) => return awsem_core::error::AwsemError::NotFound(e).cognito_response(),
    };
    let parsed = match PasswordHash::new(&user.password_hash) {
        Ok(p) => p,
        Err(e) => return awsem_core::error::AwsemError::Internal(e.to_string()).cognito_response(),
    };
    if Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_err()
    {
        return awsem_core::error::AwsemError::InvalidRequest(
            "Incorrect username or password".into(),
        )
        .cognito_response();
    }
    let (access, id, refresh) = match jwt::create_tokens(&user.id, username, pool_id, &state.jwt_secret) {
        Ok(t) => t,
        Err(e) => return awsem_core::error::AwsemError::Internal(e).cognito_response(),
    };
    HttpResponse::Ok().json(json!({
        "AuthenticationResult": {
            "AccessToken": access,
            "ExpiresIn": 3600,
            "IdToken": id,
            "RefreshToken": refresh,
            "TokenType": "Bearer",
        }
    }))
}

async fn handle_get_user(input: Value, state: &AppState) -> HttpResponse {
    let access_token = input.get("AccessToken").and_then(|v| v.as_str()).unwrap_or("");
    let claims = match jwt::verify_token(access_token, &state.jwt_secret) {
        Ok(c) => c,
        Err(e) => return awsem_core::error::AwsemError::InvalidRequest(e).cognito_response(),
    };
    HttpResponse::Ok().json(json!({
        "Username": claims.username,
        "UserAttributes": [{"Name": "sub", "Value": claims.sub}],
    }))
}
