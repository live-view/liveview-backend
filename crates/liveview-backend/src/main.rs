use clap::Parser;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

mod args;

use args::Args;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(args.log_level.as_str()))
        .init();

    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], args.port))).await?;
    tracing::info!("Listening on {}", listener.local_addr()?);

    // Add Cross-Origin Resource Sharing (CORS) middleware to the application
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(true);

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "Hello, world!" }))
        .layer(cors_layer);
    axum::serve(listener, app).await?;

    Ok(())
}
