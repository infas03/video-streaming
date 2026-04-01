mod config;
mod db;
mod error;
mod handlers;
mod models;
mod storage;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let app = Router::new().layer(CorsLayer::permissive()).layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    tracing::info!("API server listening on 0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}
