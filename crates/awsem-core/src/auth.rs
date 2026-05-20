use actix_web::HttpRequest;

#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub access_key: String,
    pub region: String,
    pub service: String,
    pub account_id: String,
}

impl Default for AuthInfo {
    fn default() -> Self {
        Self {
            access_key: "000000000000".into(),
            region: "us-east-1".into(),
            service: "s3".into(),
            account_id: "000000000000".into(),
        }
    }
}

pub fn extract_auth_info(req: &HttpRequest) -> AuthInfo {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Some(cred) = auth_header.split("Credential=").nth(1) {
        let parts: Vec<&str> = cred.split('/').collect();
        if parts.len() >= 4 {
            return AuthInfo {
                access_key: parts[0].to_string(),
                region: parts[2].to_string(),
                service: parts[3].to_string(),
                account_id: parts[0].to_string(),
            };
        }
    }
    AuthInfo::default()
}
