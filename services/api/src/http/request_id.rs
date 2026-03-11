use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderName},
};
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};

use super::error::ApiError;

/// Applies a `SetRequestId` layer that injects an auto-generated UUID into the
/// `x-request-id` header of every incoming request that does not already carry one.
pub fn request_id_layer() -> SetRequestIdLayer<MakeRequestUuid> {
    SetRequestIdLayer::new(
        HeaderName::from_static("x-request-id"),
        MakeRequestUuid,
    )
}

/// Axum extractor that reads the `x-request-id` header set by [`request_id_layer`].
///
/// Returns [`ApiError::internal`] if the header is absent (which should never happen
/// when the middleware is installed, but acts as a fail-fast guard).
pub struct RequestId(pub String);

impl<S> FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| RequestId(s.to_string()))
            .ok_or_else(|| ApiError::internal("missing", "request_id not set"))
    }
}
