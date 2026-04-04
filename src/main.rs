mod config;
mod db;
mod email;
mod llm;
mod models;
mod pipeline;
mod source;

use std::path::PathBuf;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;

use config::{Config, Secrets};

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("rustling.toml"));

    let config = Config::load(&config_path)?;

    let default_directive = if config.verbose {
        "rustling=debug"
    } else {
        "rustling=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(default_directive.parse()?))
        .init();

    info!(config = %config_path.display(), verbose = config.verbose, "loading configuration");

    let secrets = Secrets::from_env()?;
    pipeline::run(&config, &secrets).await
}
