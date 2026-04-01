use sqlx::PgPool;
use uuid::Uuid;

use crate::models::video::{TranscodeJob, Video};

pub async fn insert_video(
    pool: &PgPool,
    token: &str,
    filename: &str,
    size_bytes: i64,
    mime_type: &str,
    storage_key: &str,
) -> Result<Video, sqlx::Error> {
    sqlx::query_as::<_, Video>(
        "INSERT INTO videos (token, filename, size_bytes, mime_type, storage_key, status)
         VALUES ($1, $2, $3, $4, $5, 'ready')
         RETURNING *",
    )
    .bind(token)
    .bind(filename)
    .bind(size_bytes)
    .bind(mime_type)
    .bind(storage_key)
    .fetch_one(pool)
    .await
}

pub async fn find_video_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Video>, sqlx::Error> {
    sqlx::query_as::<_, Video>("SELECT * FROM videos WHERE token = $1")
        .bind(token)
        .fetch_optional(pool)
        .await
}

pub async fn update_video_status(
    pool: &PgPool,
    video_id: Uuid,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE videos SET status = $1, updated_at = now() WHERE id = $2")
        .bind(status)
        .bind(video_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_video_hls_ready(
    pool: &PgPool,
    video_id: Uuid,
    hls_key: &str,
    duration_seconds: Option<f64>,
    width: Option<i32>,
    height: Option<i32>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE videos
         SET hls_ready = true, hls_key = $1, status = 'done',
             duration_seconds = $2, width = $3, height = $4, updated_at = now()
         WHERE id = $5",
    )
    .bind(hls_key)
    .bind(duration_seconds)
    .bind(width)
    .bind(height)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_transcode_job(
    pool: &PgPool,
    video_id: Uuid,
) -> Result<TranscodeJob, sqlx::Error> {
    sqlx::query_as::<_, TranscodeJob>(
        "INSERT INTO transcode_jobs (video_id) VALUES ($1) RETURNING *",
    )
    .bind(video_id)
    .fetch_one(pool)
    .await
}

pub async fn update_transcode_job_status(
    pool: &PgPool,
    job_id: Uuid,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    let timestamp_field = match status {
        "running" => "started_at",
        "done" | "error" => "completed_at",
        _ => "started_at",
    };

    let query = format!(
        "UPDATE transcode_jobs
         SET status = $1, error_message = $2, attempts = attempts + 1, {} = now()
         WHERE id = $3",
        timestamp_field
    );

    sqlx::query(&query)
        .bind(status)
        .bind(error_message)
        .bind(job_id)
        .execute(pool)
        .await?;
    Ok(())
}
