use std::path::{Path, PathBuf};
use tokio::process::Command;

use crate::error::AppError;

pub struct ProbeResult {
    pub duration_seconds: f64,
    pub width: i32,
    pub height: i32,
}

pub async fn probe_video(input_path: &Path) -> Result<ProbeResult, AppError> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(input_path)
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("ffprobe failed to start: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("ffprobe error: {}", stderr)));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| AppError::Internal(format!("ffprobe json parse error: {}", e)))?;

    let duration_seconds = json["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let video_stream = json["streams"]
        .as_array()
        .and_then(|streams| {
            streams.iter().find(|s| s["codec_type"] == "video")
        });

    let (width, height) = match video_stream {
        Some(stream) => (
            stream["width"].as_i64().unwrap_or(0) as i32,
            stream["height"].as_i64().unwrap_or(0) as i32,
        ),
        None => (0, 0),
    };

    Ok(ProbeResult {
        duration_seconds,
        width,
        height,
    })
}

pub async fn transcode_to_hls(
    input_path: &Path,
    output_dir: &Path,
) -> Result<PathBuf, AppError> {
    tokio::fs::create_dir_all(output_dir)
        .await
        .map_err(|e| AppError::Internal(format!("failed to create output dir: {}", e)))?;

    let segment_pattern = output_dir.join("seg_%03d.ts");
    let manifest_path = output_dir.join("manifest.m3u8");

    let status = Command::new("ffmpeg")
        .args([
            "-i", &input_path.to_string_lossy(),
            "-c:v", "libx264",
            "-preset", "fast",
            "-crf", "23",
            "-c:a", "aac",
            "-b:a", "128k",
            "-hls_time", "4",
            "-hls_playlist_type", "vod",
            "-hls_segment_filename", &segment_pattern.to_string_lossy(),
            &manifest_path.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .await
        .map_err(|e| AppError::Internal(format!("ffmpeg failed to start: {}", e)))?;

    if !status.success() {
        return Err(AppError::Internal("ffmpeg transcoding failed".to_string()));
    }

    Ok(manifest_path)
}
