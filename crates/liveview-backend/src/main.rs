use clap::Parser;
use std::net::SocketAddr;
use tokio::net::TcpListener;
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

    let app = axum::Router::new().route("/", axum::routing::get(|| async { "Hello, world!" }));
    axum::serve(listener, app).await?;

    Ok(())
}
