use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Standard success envelope matching the API contract:
/// `{ "data": T, "meta": { "request_id": "...", ... } }`
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    pub meta: ResponseMeta,
}

#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_event_id: Option<String>,
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// Wrapper for returning `ApiResponse` with a specific status code.
pub struct Created<T: Serialize>(pub ApiResponse<T>);

impl<T: Serialize> IntoResponse for Created<T> {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self.0)).into_response()
    }
}

/// Standard list payload: `{ "items": [...], "next_cursor": ... }`
#[derive(Debug, Serialize)]
pub struct ListData<T: Serialize> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}
