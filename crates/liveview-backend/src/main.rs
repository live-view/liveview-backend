use std::{net::SocketAddr, sync::Arc};

use alloy::providers::ProviderBuilder;
use clap::Parser;
use eyre::Context;
use socketioxide::SocketIo;
use tokio::{fs, net::TcpListener};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod args;
mod data;
mod handlers;
mod interfaces;
mod routes;
mod state;
mod utils;

use args::Args;
use data::Data;
use state::{AppState, ChainState};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();

    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let data = serde_json::from_str::<Data>(
        &fs::read_to_string(args.data_path)
            .await
            .context("Failed to read data file")?,
    )
    .context("Failed to parse data file")?;

    // Create a new state for the application
    let app_state = Arc::new(AppState {
        mainnet: ChainState {
            multicall_address: data.mainnet.multicall_address,
            provider: Arc::new(
                ProviderBuilder::new()
                    .on_builtin(data.mainnet.rpc_url.as_str())
                    .await?,
            ),
        },
        base: ChainState {
            multicall_address: data.base.multicall_address,
            provider: Arc::new(
                ProviderBuilder::new()
                    .on_builtin(data.base.rpc_url.as_str())
                    .await?,
            ),
        },
        arbitrum: ChainState {
            multicall_address: data.arbitrum.multicall_address,
            provider: Arc::new(
                ProviderBuilder::new()
                    .on_builtin(data.arbitrum.rpc_url.as_str())
                    .await?,
            ),
        },
        optimism: ChainState {
            multicall_address: data.optimism.multicall_address,
            provider: Arc::new(
                ProviderBuilder::new()
                    .on_builtin(data.optimism.rpc_url.as_str())
                    .await?,
            ),
        },
        polygon: ChainState {
            multicall_address: data.polygon.multicall_address,
            provider: Arc::new(
                ProviderBuilder::new()
                    .on_builtin(data.polygon.rpc_url.as_str())
                    .await?,
            ),
        },
        bsc: ChainState {
            multicall_address: data.bsc.multicall_address,
            provider: Arc::new(
                ProviderBuilder::new()
                    .on_builtin(data.bsc.rpc_url.as_str())
                    .await?,
            ),
        },
    });

    // Create a new Socket.IO layer
    let (socket_layer, socket_io) = SocketIo::builder()
        .with_state(Arc::clone(&app_state))
        .build_layer();
    socket_io.ns("/api/ws", handlers::ws);

    // Add Cross-Origin Resource Sharing (CORS) middleware to the application
    let cors_layer = CorsLayer::permissive();

    // Trace requests to the application
    let trace_layer = TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default());

    let app = axum::Router::new()
        .route("/api/search", axum::routing::get(routes::search::search))
        // .layer(socket_layer)
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(socket_layer),
        )
        .with_state(Arc::clone(&app_state))
        .layer(cors_layer)
        .layer(trace_layer);

    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], args.port))).await?;
    tracing::info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
