use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Video {
    pub id: Uuid,
    pub token: String,
    pub filename: String,
    pub size_bytes: i64,
    pub mime_type: String,
    pub storage_key: String,
    pub status: String,
    pub hls_ready: bool,
    pub hls_key: Option<String>,
    pub duration_seconds: Option<f64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TranscodeJob {
    pub id: Uuid,
    pub video_id: Uuid,
    pub status: String,
    pub error_message: Option<String>,
    pub attempts: i32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}
