use axum::extract::{Multipart, State};
use axum::Json;
use serde::Serialize;

use crate::db::video::{insert_transcode_job, insert_video};
use crate::error::AppError;
use crate::state::AppState;
use crate::storage;

const ALLOWED_MIME_TYPES: &[&str] = &[
    "video/mp4",
    "video/webm",
    "video/quicktime",
    "video/x-msvideo",
    "video/x-matroska",
];

#[derive(Serialize)]
pub struct UploadResponse {
    pub video_id: String,
    pub token: String,
    pub share_url: String,
}

pub async fn handle_upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    let mut field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
        .ok_or_else(|| AppError::BadRequest("no file field provided".to_string()))?;

    let filename = field
        .file_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "upload.mp4".to_string());

    let content_type = field
        .content_type()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "video/mp4".to_string());

    if !ALLOWED_MIME_TYPES.contains(&content_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "unsupported file type: {}",
            content_type
        )));
    }

    let token = nanoid::nanoid!(12, &nanoid::alphabet::SAFE);
    let storage_key = format!("raw/{}.{}", token, extract_extension(&filename));
    let bucket = &state.config.s3_bucket;

    let upload_id =
        storage::start_multipart_upload(&state.s3, bucket, &storage_key, &content_type).await?;

    let mut parts = Vec::new();
    let mut part_number: i32 = 1;
    let mut total_bytes: i64 = 0;
    let mut buffer = Vec::with_capacity(5 * 1024 * 1024);
    let max_bytes = state.config.max_upload_bytes as i64;
    let mut mime_verified = false;

    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if !mime_verified && total_bytes == 0 {
            if let Some(kind) = infer::get(&chunk) {
                let detected = kind.mime_type();
                if !ALLOWED_MIME_TYPES.contains(&detected) {
                    storage::abort_multipart_upload(&state.s3, bucket, &storage_key, &upload_id)
                        .await
                        .ok();
                    return Err(AppError::BadRequest(format!(
                        "detected file type not allowed: {}",
                        detected
                    )));
                }
            }
            mime_verified = true;
        }

        total_bytes += chunk.len() as i64;

        if total_bytes > max_bytes {
            storage::abort_multipart_upload(&state.s3, bucket, &storage_key, &upload_id)
                .await
                .ok();
            return Err(AppError::BadRequest("file exceeds 1 GB limit".to_string()));
        }

        buffer.extend_from_slice(&chunk);

        if buffer.len() >= 5 * 1024 * 1024 {
            let part_data = std::mem::replace(&mut buffer, Vec::with_capacity(5 * 1024 * 1024));
            let part = storage::upload_part(
                &state.s3,
                bucket,
                &storage_key,
                &upload_id,
                part_number,
                part_data,
            )
            .await?;
            parts.push(part);
            part_number += 1;
        }
    }

    if !buffer.is_empty() {
        let part = storage::upload_part(
            &state.s3,
            bucket,
            &storage_key,
            &upload_id,
            part_number,
            buffer,
        )
        .await?;
        parts.push(part);
    }

    if total_bytes == 0 {
        storage::abort_multipart_upload(&state.s3, bucket, &storage_key, &upload_id)
            .await
            .ok();
        return Err(AppError::BadRequest("empty file".to_string()));
    }

    storage::complete_multipart_upload(&state.s3, bucket, &storage_key, &upload_id, parts).await?;

    let video = insert_video(
        &state.db,
        &token,
        &filename,
        total_bytes,
        &content_type,
        &storage_key,
    )
    .await?;

    insert_transcode_job(&state.db, video.id).await?;

    let mut conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    redis::cmd("LPUSH")
        .arg("transcode_jobs")
        .arg(video.id.to_string())
        .query_async::<()>(&mut conn)
        .await?;

    Ok(Json(UploadResponse {
        video_id: video.id.to_string(),
        token: token.clone(),
        share_url: format!("/v/{}", token),
    }))
}

fn extract_extension(filename: &str) -> &str {
    filename
        .rsplit('.')
        .next()
        .unwrap_or("mp4")
}
