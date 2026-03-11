mod domain;
mod handlers;
mod repo;

use axum::Router;
use crate::app::AppState;

pub fn routes(state: AppState) -> Router {
    handlers::routes(state)
}
