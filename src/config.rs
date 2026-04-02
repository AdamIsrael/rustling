use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_database_path")]
    pub database_path: String,
    #[serde(default = "default_lookback_hours")]
    pub lookback_hours: u64,
    #[serde(default = "default_max_items")]
    pub max_items_per_digest: usize,
    pub feeds: Vec<FeedConfig>,
    pub llm: LlmConfig,
    pub email: EmailConfig,
}

#[derive(Debug, Deserialize)]
pub struct FeedConfig {
    pub name: String,
    pub url: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub endpoint: String,
    pub model: String,
    pub prompt_template: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    Claude,
    Ollama,
    OpenaiCompat,
}

#[derive(Debug, Deserialize)]
pub struct EmailConfig {
    pub from: String,
    pub to: Vec<String>,
    pub subject_prefix: Option<String>,
}

pub struct Secrets {
    pub sendgrid_api_key: String,
    pub llm_api_key: Option<String>,
}

fn default_database_path() -> String {
    "rustling.db".to_string()
}

fn default_lookback_hours() -> u64 {
    24
}

fn default_max_items() -> usize {
    50
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Config =
            toml::from_str(&content).with_context(|| "failed to parse config file")?;
        Ok(config)
    }
}

impl Secrets {
    pub fn from_env() -> Result<Self> {
        let sendgrid_api_key = std::env::var("SENDGRID_API_KEY")
            .with_context(|| "SENDGRID_API_KEY environment variable not set")?;
        let llm_api_key = std::env::var("LLM_API_KEY").ok();
        Ok(Self {
            sendgrid_api_key,
            llm_api_key,
        })
    }
}
