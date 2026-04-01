use tracing_subscriber::EnvFilter;
use video_streaming::config::AppConfig;
use video_streaming::db;
use video_streaming::storage;
use video_streaming::worker;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env();

    let db_pool = db::create_pool(&config.database_url).await;
    let s3_client = storage::create_s3_client(&config);
    let redis_client =
        redis::Client::open(config.redis_url.as_str()).expect("invalid redis url");

    tracing::info!("transcode worker starting");
    worker::run_worker_loop(config, db_pool, s3_client, redis_client).await;
}
