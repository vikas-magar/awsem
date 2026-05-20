use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub pool_id: String,
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
    let access_claims = Claims {
        sub: sub.into(),
        username: username.into(),
        pool_id: pool_id.into(),
        exp: now + 3600,
        iat: now,
    };
    let id_claims = Claims {
        sub: sub.into(),
        username: username.into(),
        pool_id: pool_id.into(),
        exp: now + 86400,
        iat: now,
    };
    let refresh_claims = Claims {
        sub: sub.into(),
        username: username.into(),
        pool_id: pool_id.into(),
        exp: now + 2592000,
        iat: now,
    };
    let key = EncodingKey::from_secret(secret.as_bytes());
    let access = encode(&Header::default(), &access_claims, &key).map_err(|e| e.to_string())?;
    let id = encode(&Header::default(), &id_claims, &key).map_err(|e| e.to_string())?;
    let refresh = encode(&Header::default(), &refresh_claims, &key).map_err(|e| e.to_string())?;
    Ok((access, id, refresh))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, String> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let data = decode::<Claims>(token, &key, &Validation::default()).map_err(|e| e.to_string())?;
    Ok(data.claims)
}
