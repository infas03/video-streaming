use crate::config::AppConfig;
use aws_sdk_s3::config::{Credentials, Region};

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
