pub mod stream;
pub mod upload;
pub mod video;

use axum::Router;
use crate::state::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
}
