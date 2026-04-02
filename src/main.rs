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
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("rustling=info".parse()?))
        .init();

    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("rustling.toml"));

    info!(config = %config_path.display(), "loading configuration");

    let config = Config::load(&config_path)?;
    let secrets = Secrets::from_env()?;

    pipeline::run(&config, &secrets).await
}
