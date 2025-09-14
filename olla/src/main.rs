mod client;
mod config;
mod coordinator;
mod device;
mod node;
mod tunnels;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
enum AppMode {
    Client,
    Node,
}

#[derive(Parser, Debug)]
struct AppArguments {
    /// Path to the config toml file
    config: PathBuf,
    /// App mode: client, mode
    mode: AppMode,
}

impl From<&str> for AppMode {
    fn from(mode: &str) -> Self {
        match mode {
            "client" => AppMode::Client,
            "node" => AppMode::Node,
            _ => panic!("unknown app mode: {}", mode),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = AppArguments::parse();
    match args.mode {
        AppMode::Client => client::run(args.config).await,
        AppMode::Node => node::run(args.config).await,
    }
}
