use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub pool_id: String,
    pub token_use: String,
    pub iss: String,
    pub auth_time: usize,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_tokens(
    sub: &str,
    username: &str,
    pool_id: &str,
    secret: &str,
) -> Result<(String, String, String), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let iss = format!("https://cognito-idp.us-east-1.amazonaws.com/{pool_id}");
    let base = |token_use: &str, exp_offset: usize| Claims {
        sub: sub.into(),
        username: username.into(),
        pool_id: pool_id.into(),
        token_use: token_use.into(),
        iss: iss.clone(),
        auth_time: now,
        exp: now + exp_offset,
        iat: now,
    };
    let key = EncodingKey::from_secret(secret.as_bytes());
    let access =
        encode(&Header::default(), &base("access", 3600), &key).map_err(|e| e.to_string())?;
    let id = encode(&Header::default(), &base("id", 86400), &key).map_err(|e| e.to_string())?;
    let refresh =
        encode(&Header::default(), &base("refresh", 2592000), &key).map_err(|e| e.to_string())?;
    Ok((access, id, refresh))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, String> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let data = decode::<Claims>(token, &key, &Validation::default()).map_err(|e| e.to_string())?;
    Ok(data.claims)
}
