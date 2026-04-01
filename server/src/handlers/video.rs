use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use crate::db::video::find_video_by_token;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct VideoResponse {
    pub id: String,
    pub token: String,
    pub filename: String,
    pub status: String,
    pub hls_ready: bool,
    pub duration_seconds: Option<f64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub created_at: String,
}

pub async fn get_video_metadata(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<VideoResponse>, AppError> {
    let video = find_video_by_token(&state.db, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("video not found".to_string()))?;

    Ok(Json(VideoResponse {
        id: video.id.to_string(),
        token: video.token,
        filename: video.filename,
        status: video.status,
        hls_ready: video.hls_ready,
        duration_seconds: video.duration_seconds,
        width: video.width,
        height: video.height,
        created_at: video.created_at.to_rfc3339(),
    }))
}

pub async fn get_raw_video_url(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<axum::response::Redirect, AppError> {
    let video = find_video_by_token(&state.db, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("video not found".to_string()))?;

    let presigned_url = crate::storage::generate_presigned_url(
        &state.s3,
        &state.config.s3_bucket,
        &video.storage_key,
        3600,
    )
    .await?;

    Ok(axum::response::Redirect::temporary(&presigned_url))
}
