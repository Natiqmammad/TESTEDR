mod api;
mod auth;
mod db;
mod error;
mod models;
mod storage;
mod templates;

use std::net::SocketAddr;

use anyhow::Result;
use api::{router, AppState};
use auth::JwtKeys;
use axum::serve;
use sqlx::SqlitePool;
use storage::Storage;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;

#[tokio::main]
async fn main() -> Result<()> {
    let addr: SocketAddr = std::env::var("NS_REGISTRY_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:5665".to_string())
        .parse()
        .expect("invalid addr");
    let database_url =
        std::env::var("NS_REGISTRY_DB").unwrap_or_else(|_| "sqlite://registry.db".to_string());
    let pool = SqlitePool::connect(&database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let storage_root = std::env::var("NS_REGISTRY_STORAGE").unwrap_or_else(|_| "storage".into());
    let storage = Storage::new(storage_root).await?;
    let jwt_secret =
        std::env::var("NS_REGISTRY_SECRET").unwrap_or_else(|_| "insecure-dev-secret".into());
    let state = AppState {
        pool,
        storage,
        jwt: JwtKeys::new(jwt_secret),
    };
    let app = router(state).layer(CompressionLayer::new());
    let listener = TcpListener::bind(addr).await?;
    println!("nightscript-server listening on http://{}", addr);
    serve(listener, app.into_make_service()).await?;
    Ok(())
}
