use std::convert::Infallible;

use axum::{
    extract::{FromRequestParts, Query},
    http::request::Parts,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

/// Raw deserialization target for query params — kept private to avoid a
/// recursive `FromRequestParts` call on `PaginationParams` itself.
#[derive(Debug, Deserialize)]
struct RawPaginationParams {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_per_page")]
    per_page: u32,
}

/// Validated pagination input extracted from `?page=&per_page=` query params.
///
/// `per_page` is silently clamped to 100 even if the caller supplies a larger value.
#[derive(Debug)]
pub struct PaginationParams {
    pub page: u32,
    pub per_page: u32,
}

impl<S> FromRequestParts<S> for PaginationParams
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(raw) = Query::<RawPaginationParams>::from_request_parts(parts, state)
            .await
            .unwrap_or_else(|_| Query(RawPaginationParams { page: 1, per_page: 20 }));

        Ok(PaginationParams {
            page: raw.page.max(1),
            per_page: raw.per_page.min(100),
        })
    }
}

/// A single page of results, serialisable as the standard JSON envelope used
/// across all list endpoints.
#[derive(Debug, Serialize)]
pub struct Page<T: Serialize> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

impl<T: Serialize> Page<T> {
    pub fn new(items: Vec<T>, total: u64, page: u32, per_page: u32) -> Self {
        let total_pages = if per_page == 0 {
            0
        } else {
            ((total as f64) / (per_page as f64)).ceil() as u32
        };
        Self { items, total, page, per_page, total_pages }
    }
}

impl<T: Serialize> IntoResponse for Page<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use serde_json::Value;

    #[test]
    fn page_serializes_with_expected_fields() {
        let page = Page::new(vec![1u32, 2, 3], 10, 1, 3);
        let json = serde_json::to_value(&page).unwrap();

        assert_eq!(json["items"], Value::Array(vec![1.into(), 2.into(), 3.into()]));
        assert_eq!(json["total"], 10);
        assert_eq!(json["page"], 1);
        assert_eq!(json["per_page"], 3);
        assert_eq!(json["total_pages"], 4);
    }

    #[test]
    fn page_total_pages_rounds_up() {
        let page = Page::new(vec![0u32; 5], 11, 1, 5);
        assert_eq!(page.total_pages, 3);
    }

    #[tokio::test]
    async fn per_page_over_100_is_clamped_to_100() {
        let req = Request::builder()
            .uri("/?per_page=999&page=2")
            .body(Body::empty())
            .unwrap();
        let (mut parts, _) = req.into_parts();

        let params = PaginationParams::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(params.per_page, 100);
        assert_eq!(params.page, 2);
    }

    #[tokio::test]
    async fn missing_params_fall_back_to_defaults() {
        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
        let (mut parts, _) = req.into_parts();

        let params = PaginationParams::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 20);
    }
}
