use std::time::Duration;

use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};

use crate::config::AppConfig;
use crate::error::AppError;

pub fn create_s3_client(config: &AppConfig) -> aws_sdk_s3::Client {
    let credentials = Credentials::new(
        &config.s3_access_key,
        &config.s3_secret_key,
        None,
        None,
        "env",
    );

    let s3_config = aws_sdk_s3::Config::builder()
        .endpoint_url(&config.s3_endpoint)
        .region(Region::new(config.s3_region.clone()))
        .credentials_provider(credentials)
        .force_path_style(true)
        .build();

    aws_sdk_s3::Client::from_conf(s3_config)
}

pub async fn start_multipart_upload(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    content_type: &str,
) -> Result<String, AppError> {
    let response = client
        .create_multipart_upload()
        .bucket(bucket)
        .key(key)
        .content_type(content_type)
        .send()
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

    response
        .upload_id()
        .map(|id| id.to_string())
        .ok_or_else(|| AppError::Storage("missing upload id".to_string()))
}

pub async fn upload_part(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
    part_number: i32,
    body: Vec<u8>,
) -> Result<CompletedPart, AppError> {
    let response = client
        .upload_part()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .part_number(part_number)
        .body(ByteStream::from(body))
        .send()
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

    let etag = response
        .e_tag()
        .ok_or_else(|| AppError::Storage("missing etag".to_string()))?;

    Ok(CompletedPart::builder()
        .e_tag(etag)
        .part_number(part_number)
        .build())
}

pub async fn complete_multipart_upload(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
    parts: Vec<CompletedPart>,
) -> Result<(), AppError> {
    let completed = CompletedMultipartUpload::builder()
        .set_parts(Some(parts))
        .build();

    client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .multipart_upload(completed)
        .send()
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

    Ok(())
}

pub async fn abort_multipart_upload(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
) -> Result<(), AppError> {
    client
        .abort_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .send()
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

    Ok(())
}

pub async fn upload_object(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    body: Vec<u8>,
    content_type: &str,
) -> Result<(), AppError> {
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(body))
        .content_type(content_type)
        .send()
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

    Ok(())
}

pub async fn generate_presigned_url(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    expiry_seconds: u64,
) -> Result<String, AppError> {
    let presign_config = PresigningConfig::expires_in(Duration::from_secs(expiry_seconds))
        .map_err(|e| AppError::Storage(e.to_string()))?;

    let request = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .presigned(presign_config)
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

    Ok(request.uri().to_string())
}

pub async fn download_object(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
) -> Result<Vec<u8>, AppError> {
    let response = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

    let bytes = response
        .body
        .collect()
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?
        .into_bytes()
        .to_vec();

    Ok(bytes)
}
