use std::{net::SocketAddr, sync::Arc};

use alloy::providers::ProviderBuilder;
use clap::Parser;
use socketioxide::SocketIo;
use tokio::{fs, net::TcpListener};
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::EnvFilter;

mod args;
mod data;
mod handlers;
mod state;

use args::Args;
use data::Data;
use state::AppState;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();

    let env_filter = EnvFilter::builder()
        .with_default_directive(args.log_level.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let data = serde_json::from_str::<Data>(&fs::read_to_string(args.data_path).await?)?;

    // Create a new state for the application
    let app_state = Arc::new(AppState {
        mainnet: Arc::new(ProviderBuilder::new().on_builtin(&data.mainnet).await?),
        base: Arc::new(ProviderBuilder::new().on_builtin(&data.base).await?),
        arbitrum: Arc::new(ProviderBuilder::new().on_builtin(&data.arbitrum).await?),
        optimism: Arc::new(ProviderBuilder::new().on_builtin(&data.optimism).await?),
        polygon: Arc::new(ProviderBuilder::new().on_builtin(&data.polygon).await?),
        bsc: Arc::new(ProviderBuilder::new().on_builtin(&data.bsc).await?),
    });

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
