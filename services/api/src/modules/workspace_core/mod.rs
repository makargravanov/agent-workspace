mod domain;
mod handlers;
mod repo;

use axum::Router;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    handlers::routes()
}
