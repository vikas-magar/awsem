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
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::AlreadyExists(_) => StatusCode::CONFLICT,
        }
    }

    fn aws_json(&self, type_name: &str) -> serde_json::Value {
        json!({"__type": type_name, "Message": self.to_string()})
    }

    pub fn to_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(self.aws_json("AwsemError"))
    }

    pub fn cognito_response(&self) -> HttpResponse {
        let t = match self {
            Self::NotFound(_) => "ResourceNotFoundException",
            Self::InvalidRequest(_) => "InvalidParameterException",
            Self::AlreadyExists(_) => "UsernameExistsException",
            Self::Conflict(_) => "ConcurrentModificationException",
            _ => "InternalErrorException",
        };
        HttpResponse::build(self.status_code()).json(self.aws_json(t))
    }

    pub fn secrets_response(&self) -> HttpResponse {
        let t = match self {
            Self::NotFound(_) => "ResourceNotFoundException",
            Self::InvalidRequest(_) => "InvalidParameterException",
            Self::AlreadyExists(_) => "ResourceExistsException",
            _ => "InternalServiceError",
        };
        HttpResponse::build(self.status_code()).json(self.aws_json(t))
    }

    pub fn emr_response(&self) -> HttpResponse {
        let t = match self {
            Self::NotImplemented(_) => "UnknownOperationException",
            Self::NotFound(_) => "ResourceNotFoundException",
            Self::InvalidRequest(_) => "ValidationException",
            Self::AlreadyExists(_) => "ValidationException",
            Self::Conflict(_) => "ValidationException",
            _ => "InternalServerError",
        };
        HttpResponse::build(self.status_code()).json(self.aws_json(t))
    }
}
