pub mod stream;
pub mod upload;
pub mod video;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use crate::state::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/upload", post(upload::handle_upload))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024 + 1024))
        .route("/videos/{token}", get(video::get_video_metadata))
        .route("/videos/{token}/raw", get(video::get_raw_video_url))
        .route("/videos/{token}/manifest.m3u8", get(stream::get_hls_manifest))
        .route("/videos/{token}/status", get(stream::stream_video_status))
}
