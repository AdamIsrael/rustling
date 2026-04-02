use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub verbose: bool,
    #[serde(default = "default_database_path")]
    pub database_path: String,
    #[serde(default = "default_lookback_hours")]
    pub lookback_hours: u64,
    #[serde(default = "default_max_items")]
    pub max_items_per_digest: usize,
    #[serde(default)]
    pub keywords: Keywords,
    #[serde(default)]
    pub feeds: Vec<FeedConfig>,
    #[serde(default)]
    pub searches: Vec<SearchConfig>,
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
pub struct SearchConfig {
    pub name: String,
    pub instance_url: String,
    pub query: String,
    pub category: Option<String>,
    #[serde(default)]
    pub time_range: TimeRange,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    #[default]
    Day,
    Week,
    Month,
    Year,
}

impl TimeRange {
    pub fn as_str(self) -> &'static str {
        match self {
            TimeRange::Day => "day",
            TimeRange::Week => "week",
            TimeRange::Month => "month",
            TimeRange::Year => "year",
        }
    }
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

#[derive(Debug, Clone, Default)]
pub struct Keywords(pub Vec<String>);

impl Keywords {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns true if any keyword is found (case-insensitive) in the given text.
    pub fn matches(&self, text: &str) -> bool {
        if self.0.is_empty() {
            return true;
        }
        let lower = text.to_lowercase();
        self.0.iter().any(|kw| lower.contains(kw))
    }
}

impl<'de> Deserialize<'de> for Keywords {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let keywords = s
            .split(',')
            .map(|k| k.trim().to_lowercase())
            .filter(|k| !k.is_empty())
            .collect();
        Ok(Keywords(keywords))
    }
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
