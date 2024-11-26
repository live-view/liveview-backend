use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// The port to listen on
    #[arg(short, long, env = "PORT", default_value_t = 8000)]
    pub(crate) port: u16,

    /// Path to the data directory
    #[arg(short, long, env = "DATA_PATH", visible_alias = "data")]
    pub(crate) data_path: PathBuf,
}
