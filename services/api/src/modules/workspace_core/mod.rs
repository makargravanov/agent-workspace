pub mod domain;
mod handlers;
pub mod repo;

use crate::state::AppState;
use axum::Router;

pub fn routes() -> Router<AppState> {
    handlers::routes()
}
