use actix_web::{HttpResponse, http::StatusCode};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AwsemError {
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    AlreadyExists(String),
}

impl AwsemError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::AlreadyExists(_) => StatusCode::CONFLICT,
        }
    }

    pub fn to_response(&self) -> HttpResponse {
        let code = self.status_code();
        HttpResponse::build(code).json(json!({
            "message": self.to_string(),
            "__type": "AwsemError",
        }))
    }

    pub fn cognito_response(&self) -> HttpResponse {
        let code = self.status_code();
        let type_name = match self {
            Self::NotFound(_) => "ResourceNotFoundException",
            Self::InvalidRequest(_) => "InvalidParameterException",
            Self::AlreadyExists(_) => "UsernameExistsException",
            Self::Conflict(_) => "ConcurrentModificationException",
            _ => "InternalErrorException",
        };
        HttpResponse::build(code).json(json!({
            "__type": type_name,
            "message": self.to_string(),
        }))
    }

    pub fn secrets_response(&self) -> HttpResponse {
        let code = self.status_code();
        let type_name = match self {
            Self::NotFound(_) => "ResourceNotFoundException",
            Self::InvalidRequest(_) => "InvalidParameterException",
            Self::AlreadyExists(_) => "ResourceExistsException",
            _ => "InternalServiceError",
        };
        HttpResponse::build(code).json(json!({
            "__type": type_name,
            "message": self.to_string(),
        }))
    }
}
