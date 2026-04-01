use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Redirect;

use crate::db::video::find_video_by_token;
use crate::error::AppError;
use crate::state::AppState;
use crate::storage;

pub async fn get_hls_manifest(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Redirect, AppError> {
    let video = find_video_by_token(&state.db, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("video not found".to_string()))?;

    if !video.hls_ready {
        return Err(AppError::NotFound("hls not ready".to_string()));
    }

    let hls_key = video
        .hls_key
        .ok_or_else(|| AppError::NotFound("hls manifest key missing".to_string()))?;

    let presigned_url =
        storage::generate_presigned_url(&state.s3, &state.config.s3_bucket, &hls_key, 3600)
            .await?;

    Ok(Redirect::temporary(&presigned_url))
}

pub async fn stream_video_status(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, AppError> {
    find_video_by_token(&state.db, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("video not found".to_string()))?;

    let db = state.db.clone();
    let video_token = token.clone();

    let stream = async_stream::stream! {
        let mut interval = tokio::time::interval(Duration::from_secs(2));

        loop {
            interval.tick().await;

            let video = find_video_by_token(&db, &video_token).await.ok().flatten();

            match video {
                Some(v) if v.hls_ready => {
                    yield Ok(Event::default()
                        .json_data(serde_json::json!({
                            "status": "done",
                            "hls_ready": true
                        }))
                        .unwrap());
                    break;
                }
                Some(v) => {
                    yield Ok(Event::default()
                        .json_data(serde_json::json!({
                            "status": v.status,
                            "hls_ready": false
                        }))
                        .unwrap());
                }
                None => {
                    yield Ok(Event::default()
                        .json_data(serde_json::json!({
                            "status": "error",
                            "hls_ready": false
                        }))
                        .unwrap());
                    break;
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
