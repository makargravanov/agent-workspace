use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub request_id: String,
    pub error_code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.error_code.as_str() {
            "not_found" => StatusCode::NOT_FOUND,
            "unauthorised" => StatusCode::UNAUTHORIZED,
            "forbidden" => StatusCode::FORBIDDEN,
            "validation_error" => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

impl ApiError {
    pub fn not_found(request_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            error_code: "not_found".to_string(),
            message: message.into(),
            details: None,
        }
    }

    pub fn unauthorised(request_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            error_code: "unauthorised".to_string(),
            message: message.into(),
            details: None,
        }
    }

    pub fn forbidden(request_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            error_code: "forbidden".to_string(),
            message: message.into(),
            details: None,
        }
    }

    pub fn validation_error(request_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            error_code: "validation_error".to_string(),
            message: message.into(),
            details: None,
        }
    }

    pub fn internal(request_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            error_code: "internal_error".to_string(),
            message: message.into(),
            details: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[test]
    fn not_found_returns_404() {
        let err = ApiError::not_found("req-1", "resource not found");
        assert_eq!(err.into_response().status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn unauthorised_returns_401() {
        let err = ApiError::unauthorised("req-1", "not authenticated");
        assert_eq!(err.into_response().status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn forbidden_returns_403() {
        let err = ApiError::forbidden("req-1", "not allowed");
        assert_eq!(err.into_response().status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn validation_error_returns_422() {
        let err = ApiError::validation_error("req-1", "bad input");
        assert_eq!(err.into_response().status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn internal_returns_500() {
        let err = ApiError::internal("req-1", "something went wrong");
        assert_eq!(err.into_response().status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn unknown_error_code_returns_500() {
        let err = ApiError {
            request_id: "req-1".to_string(),
            error_code: "some_unknown_code".to_string(),
            message: "unexpected".to_string(),
            details: None,
        };
        assert_eq!(err.into_response().status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
