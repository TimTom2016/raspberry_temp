use std::time::Duration;

use axum::{Router, http::StatusCode};
use axum_embed::ServeEmbed;
use rust_embed::Embed;
use sea_orm::{Database, DatabaseConnection};
use tokio::net::TcpListener;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
mod dto;
mod entity;
mod handler;
mod measure;
#[derive(Clone)]
pub struct AppState {
    db: DatabaseConnection,
}

#[derive(Embed, Clone)]
#[folder = "dist/"]
struct Assets;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sqlx=warn".parse().unwrap()),
        )
        .init();
    dotenvy::dotenv()?;
    let db_url = std::env::var("DATABASE_URL").unwrap_or("sqlite://db.sqlite?mode=rwc".to_string());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or("0.0.0.0:3000".to_string());
    let db = Database::connect(db_url).await?;
    tracing::info!("Connected to database");
    // synchronizes database schema with entity definitions
    db.get_schema_registry("raspberry_temp::entity::*")
        .sync(&db)
        .await?;
    tracing::info!("Synced database schema");
    let state = AppState { db };

    // Create shared shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    measure::spawn_measurement_task(state.clone(), shutdown_rx, 1).await;
    tracing::info!("Started background measurement task");

    // Create a regular axum app.
    let app = Router::new()
        .merge(handler::router())
        .fallback_service(ServeEmbed::<Assets>::new())
        .layer((
            TraceLayer::new_for_http(),
            // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
            // requests don't hang forever.
            TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(10)),
        ))
        .with_state(state);

    // Create a `TcpListener` using tokio.
    tracing::info!("Listening on {}", bind_addr);
    let listener = TcpListener::bind(bind_addr).await.unwrap();

    // Run the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    let _ = shutdown_tx.send(true);

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
