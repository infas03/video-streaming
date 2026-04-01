pub mod transcode;

use std::path::PathBuf;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::db;
use crate::db::video::{
    find_video_by_token, set_video_hls_ready, update_transcode_job_status, update_video_status,
};
use crate::error::AppError;
use crate::storage;

pub async fn run_worker_loop(
    config: AppConfig,
    db_pool: sqlx::PgPool,
    s3_client: aws_sdk_s3::Client,
    redis_client: redis::Client,
) {
    tracing::info!("worker loop started, waiting for jobs");

    loop {
        let mut conn = match redis_client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("redis connection error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        let result: Result<(String, String), _> =
            redis::cmd("BRPOP")
                .arg("transcode_jobs")
                .arg(0)
                .query_async(&mut conn)
                .await;

        let video_id_str = match result {
            Ok((_, id)) => id,
            Err(e) => {
                tracing::error!("redis brpop error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        let video_id = match Uuid::parse_str(&video_id_str) {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("invalid video id from queue: {}", e);
                continue;
            }
        };

        tracing::info!(video_id = %video_id, "picked up transcode job");

        let db_pool = db_pool.clone();
        let s3_client = s3_client.clone();
        let config = config.clone();

        tokio::spawn(async move {
            if let Err(e) = process_transcode_job(&config, &db_pool, &s3_client, video_id).await {
                tracing::error!(video_id = %video_id, "transcode job failed: {}", e);
                update_video_status(&db_pool, video_id, "error").await.ok();
            }
        });
    }
}

async fn process_transcode_job(
    config: &AppConfig,
    db_pool: &sqlx::PgPool,
    s3_client: &aws_sdk_s3::Client,
    video_id: Uuid,
) -> Result<(), AppError> {
    update_video_status(db_pool, video_id, "transcoding").await?;

    let video = sqlx::query_as::<_, crate::models::video::Video>(
        "SELECT * FROM videos WHERE id = $1",
    )
    .bind(video_id)
    .fetch_one(db_pool)
    .await?;

    let job = sqlx::query_as::<_, crate::models::video::TranscodeJob>(
        "SELECT * FROM transcode_jobs WHERE video_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(video_id)
    .fetch_one(db_pool)
    .await?;

    update_transcode_job_status(db_pool, job.id, "running", None).await?;

    let tmp_dir = PathBuf::from(format!("/tmp/video-streaming/{}", video_id));
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let extension = video
        .storage_key
        .rsplit('.')
        .next()
        .unwrap_or("mp4");
    let raw_file_path = tmp_dir.join(format!("input.{}", extension));

    tracing::info!(video_id = %video_id, "downloading raw file from storage");
    let raw_bytes = storage::download_object(s3_client, &config.s3_bucket, &video.storage_key).await?;
    tokio::fs::write(&raw_file_path, &raw_bytes)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    tracing::info!(video_id = %video_id, "probing video metadata");
    let probe = transcode::probe_video(&raw_file_path).await?;

    let hls_output_dir = tmp_dir.join("hls");

    tracing::info!(video_id = %video_id, "starting hls transcoding");
    transcode::transcode_to_hls(&raw_file_path, &hls_output_dir).await?;

    tracing::info!(video_id = %video_id, "uploading hls segments to storage");
    let hls_prefix = format!("hls/{}", video.token);
    upload_hls_segments(s3_client, &config.s3_bucket, &hls_output_dir, &hls_prefix).await?;

    let hls_key = format!("{}/manifest.m3u8", hls_prefix);
    set_video_hls_ready(
        db_pool,
        video_id,
        &hls_key,
        Some(probe.duration_seconds),
        Some(probe.width),
        Some(probe.height),
    )
    .await?;

    update_transcode_job_status(db_pool, job.id, "done", None).await?;

    tracing::info!(video_id = %video_id, "cleaning up temp files");
    tokio::fs::remove_dir_all(&tmp_dir).await.ok();

    tracing::info!(video_id = %video_id, "transcode job completed");
    Ok(())
}

async fn upload_hls_segments(
    s3_client: &aws_sdk_s3::Client,
    bucket: &str,
    hls_dir: &PathBuf,
    hls_prefix: &str,
) -> Result<(), AppError> {
    let mut entries = tokio::fs::read_dir(hls_dir)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let content_type = if filename.ends_with(".m3u8") {
            "application/vnd.apple.mpegurl"
        } else if filename.ends_with(".ts") {
            "video/mp2t"
        } else {
            "application/octet-stream"
        };

        let key = format!("{}/{}", hls_prefix, filename);
        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        storage::upload_object(s3_client, bucket, &key, bytes, content_type).await?;
    }

    Ok(())
}
