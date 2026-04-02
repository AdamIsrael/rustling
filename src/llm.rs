use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::config::{LlmConfig, LlmProvider};
use crate::models::Item;

const DEFAULT_PROMPT: &str = "\
You are a news digest assistant. Summarize the following items into a concise, \
well-organized digest. Group related items by topic. For each item, include the \
title and a 1-2 sentence summary. Highlight the most important or notable items. \
Keep the overall digest readable and scannable.";

pub async fn summarize(
    client: &reqwest::Client,
    config: &LlmConfig,
    api_key: Option<&str>,
    items: &[Item],
) -> Result<String> {
    let prompt = config
        .prompt_template
        .as_deref()
        .unwrap_or(DEFAULT_PROMPT);

    let items_text = format_items_for_prompt(items);

    match config.provider {
        LlmProvider::Claude => call_claude(client, config, api_key, prompt, &items_text).await,
        LlmProvider::Ollama => call_ollama(client, config, prompt, &items_text).await,
        LlmProvider::OpenaiCompat => {
            call_openai_compat(client, config, api_key, prompt, &items_text).await
        }
    }
}

fn format_items_for_prompt(items: &[Item]) -> String {
    let mut buf = String::new();
    for (i, item) in items.iter().enumerate() {
        buf.push_str(&format!("{}. ", i + 1));
        if let Some(title) = &item.title {
            buf.push_str(title);
        }
        buf.push('\n');
        buf.push_str(&format!("   Source: {} | URL: {}\n", item.source_name, item.url));
        if let Some(category) = &item.category {
            buf.push_str(&format!("   Category: {category}\n"));
        }
        if let Some(content) = &item.content {
            // Truncate long content to keep the prompt manageable
            let truncated: String = content.chars().take(500).collect();
            buf.push_str(&format!("   {truncated}\n"));
        }
        buf.push('\n');
    }
    buf
}

async fn call_claude(
    client: &reqwest::Client,
    config: &LlmConfig,
    api_key: Option<&str>,
    system_prompt: &str,
    items_text: &str,
) -> Result<String> {
    let api_key = api_key.context("LLM_API_KEY required for Claude provider")?;

    let body = json!({
        "model": config.model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [
            {"role": "user", "content": format!("Here are the items to summarize:\n\n{items_text}")}
        ]
    });

    let resp: Value = client
        .post(&config.endpoint)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .context("failed to call Claude API")?
        .json()
        .await
        .context("failed to parse Claude API response")?;

    resp["content"][0]["text"]
        .as_str()
        .map(String::from)
        .context("unexpected Claude API response format")
}

async fn call_ollama(
    client: &reqwest::Client,
    config: &LlmConfig,
    system_prompt: &str,
    items_text: &str,
) -> Result<String> {
    let body = json!({
        "model": config.model,
        "stream": false,
        "system": system_prompt,
        "prompt": format!("Here are the items to summarize:\n\n{items_text}")
    });

    let resp: Value = client
        .post(&config.endpoint)
        .json(&body)
        .send()
        .await
        .context("failed to call Ollama API")?
        .json()
        .await
        .context("failed to parse Ollama response")?;

    resp["response"]
        .as_str()
        .map(String::from)
        .context("unexpected Ollama response format")
}

async fn call_openai_compat(
    client: &reqwest::Client,
    config: &LlmConfig,
    api_key: Option<&str>,
    system_prompt: &str,
    items_text: &str,
) -> Result<String> {
    let mut req = client.post(&config.endpoint).json(&json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": format!("Here are the items to summarize:\n\n{items_text}")}
        ]
    }));

    if let Some(key) = api_key {
        req = req.bearer_auth(key);
    }

    let resp: Value = req
        .send()
        .await
        .context("failed to call OpenAI-compatible API")?
        .json()
        .await
        .context("failed to parse OpenAI-compatible response")?;

    resp["choices"][0]["message"]["content"]
        .as_str()
        .map(String::from)
        .context("unexpected OpenAI-compatible response format")
}
