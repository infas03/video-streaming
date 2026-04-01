mod config;
mod db;
mod error;
mod handlers;
mod models;
mod state;
mod storage;

use axum::Router;
use config::AppConfig;
use state::AppState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env();

    let db = db::create_pool(&config.database_url).await;
    let s3 = storage::create_s3_client(&config);
    let redis =
        redis::Client::open(config.redis_url.as_str()).expect("invalid redis url");

    let state = AppState {
        db,
        s3,
        redis,
        config: config.clone(),
    };

    let app = Router::new()
        .nest("/api", handlers::api_routes())
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{}", config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("API server listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
