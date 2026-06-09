mod app;
mod config;
mod error;
mod middleware;
mod routes;
use app::AppState;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::Config::from_env()?;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("../../migrations").run(&db).await?;
    let addr = config.bind_addr;
    let state = AppState {
        db,
        config: Arc::new(config),
        graph: graph::client::GraphClient::default(),
    };
    tracing::info!(%addr, "starting galcard api");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app::router(state)).await?;
    Ok(())
}
