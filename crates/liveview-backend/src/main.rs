use std::{net::SocketAddr, sync::Arc};

use axum::{routing, Router};
use clap::Parser;
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::EnvFilter;

mod args;
mod state;

use args::Args;
use state::AppState;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();

    let env_filter = EnvFilter::builder()
        .with_default_directive(args.log_level.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], args.port))).await?;

    // Create a new state for the application
    let app_state = Arc::new(AppState {});

    // Add Cross-Origin Resource Sharing (CORS) middleware to the application
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Trace requests to the application
    let trace_layer = TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default());

    let app = Router::new()
        .route("/", routing::get(|| async { "Hello, world!" }))
        .with_state(Arc::clone(&app_state))
        .layer(cors_layer)
        .layer(trace_layer);

    tracing::info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
