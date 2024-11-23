use clap::Parser;
use tracing::Level;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// The port to listen on
    #[arg(short, long, env = "PORT", default_value_t = 8000)]
    pub(crate) port: u16,

    /// Log filter level
    #[arg(short, long,env="RUST_LOG", default_value_t = Level::INFO)]
    pub(crate) log_level: Level,
}
