use std::{net::SocketAddr, sync::Arc};

use clap::Parser;
use socketioxide::SocketIo;
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::EnvFilter;

mod args;
mod handlers;
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

    // Create a new state for the application
    let app_state = Arc::new(Mutex::new(AppState { count: 0 }));

    // Create a new Socket.IO layer
    let (socket_layer, socket_io) = SocketIo::builder()
        .with_state(Arc::clone(&app_state))
        .build_layer();
    socket_io.ns("/ws", handlers::ws);

    // Add Cross-Origin Resource Sharing (CORS) middleware to the application
    let cors_layer = CorsLayer::permissive();

    // Trace requests to the application
    let trace_layer = TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default());

    let app = axum::Router::new()
        .layer(socket_layer)
        .with_state(Arc::clone(&app_state))
        .layer(cors_layer)
        .layer(trace_layer);

    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], args.port))).await?;
    tracing::info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
