use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub s3: aws_sdk_s3::Client,
    pub redis: redis::Client,
    pub config: AppConfig,
}
