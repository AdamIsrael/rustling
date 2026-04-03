use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::TokioChildProcess;
use rmcp::transport::StreamableHttpClientTransport;
use serde_json::Value;
use tokio::process::Command;
use tracing::debug;

use crate::config::{McpMappingConfig, McpSourceConfig, McpTransportConfig, MappingStrategy};
use crate::models::Item;
use crate::source::Source;

const MCP_TIMEOUT: Duration = Duration::from_secs(60);

pub struct McpSource {
    pub config: McpSourceConfig,
}

impl Source for McpSource {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn source_type(&self) -> &str {
        "mcp"
    }

    fn fetch<'a>(
        &'a self,
        _client: &'a reqwest::Client,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Item>>> + Send + 'a>> {
        Box::pin(self.fetch_inner())
    }
}

impl McpSource {
    async fn fetch_inner(&self) -> Result<Vec<Item>> {
        tokio::time::timeout(MCP_TIMEOUT, self.do_fetch())
            .await
            .with_context(|| format!("MCP source '{}' timed out after {MCP_TIMEOUT:?}", self.config.name))?
    }

    async fn do_fetch(&self) -> Result<Vec<Item>> {
        // Build tool arguments
        let arguments = self
            .config
            .tool_args
            .as_ref()
            .and_then(|v| v.as_object().cloned());

        let params = CallToolRequestParams::new(self.config.tool_name.clone())
            .with_arguments(arguments.unwrap_or_default());

        // Connect and call tool based on transport type
        let result = match &self.config.transport {
            McpTransportConfig::Stdio { command, args, env } => {
                let mut cmd = Command::new(command);
                cmd.args(args);
                for (k, v) in env {
                    cmd.env(k, v);
                }
                let transport = TokioChildProcess::new(cmd)
                    .with_context(|| format!("failed to spawn MCP server: {command}"))?;
                let client = ().serve(transport)
                    .await
                    .with_context(|| format!("failed to initialize MCP session with: {command}"))?;
                let result = client.call_tool(params).await
                    .with_context(|| format!("failed to call tool '{}'", self.config.tool_name))?;
                client.cancel().await.ok();
                result
            }
            McpTransportConfig::Sse { url } => {
                let transport = StreamableHttpClientTransport::from_uri(url.as_str());
                let client = ().serve(transport)
                    .await
                    .with_context(|| format!("failed to connect to MCP server at: {url}"))?;
                let result = client.call_tool(params).await
                    .with_context(|| format!("failed to call tool '{}'", self.config.tool_name))?;
                client.cancel().await.ok();
                result
            }
        };

        // Check for tool errors
        if result.is_error == Some(true) {
            let error_text: String = result
                .content
                .iter()
                .filter_map(|c| c.raw.as_text().map(|t| t.text.clone()))
                .collect::<Vec<_>>()
                .join("\n");
            bail!("MCP tool '{}' returned error: {error_text}", self.config.tool_name);
        }

        // Extract text content from response
        let text_blocks: Vec<String> = result
            .content
            .iter()
            .filter_map(|c| c.raw.as_text().map(|t| t.text.clone()))
            .collect();

        debug!(
            source = self.config.name.as_str(),
            tool = self.config.tool_name.as_str(),
            text_blocks = text_blocks.len(),
            "MCP tool response"
        );

        for (i, block) in text_blocks.iter().enumerate() {
            debug!(source = self.config.name.as_str(), block_index = i, text = %block, "MCP response block");
        }

        // Map response to items
        map_response(&self.config.name, &self.config.category, &self.config.mapping, &text_blocks)
    }
}

fn map_response(
    source_name: &str,
    category: &Option<String>,
    mapping: &McpMappingConfig,
    text_blocks: &[String],
) -> Result<Vec<Item>> {
    let now = Utc::now();

    match mapping.strategy {
        MappingStrategy::JsonArray => {
            // Concatenate all text blocks and parse as JSON
            let combined: String = text_blocks.join("");
            let parsed: Value = serde_json::from_str(&combined)
                .with_context(|| format!("MCP response is not valid JSON: {combined}"))?;

            let arr = parsed
                .as_array()
                .with_context(|| "MCP response JSON is not an array")?;

            let items = arr
                .iter()
                .filter_map(|obj| {
                    let url = extract_field(obj, mapping.url_field.as_deref())?;
                    Some(Item {
                        id: None,
                        source_name: source_name.to_string(),
                        source_type: "mcp".to_string(),
                        url,
                        title: extract_field(obj, mapping.title_field.as_deref()),
                        content: extract_field(obj, mapping.content_field.as_deref()),
                        published_at: None,
                        fetched_at: now,
                        category: category.clone(),
                    })
                })
                .collect();
            Ok(items)
        }
        MappingStrategy::SingleJson => {
            let combined: String = text_blocks.join("");
            let obj: Value = serde_json::from_str(&combined)
                .with_context(|| format!("MCP response is not valid JSON: {combined}"))?;

            let url = extract_field(&obj, mapping.url_field.as_deref())
                .with_context(|| "MCP single_json response missing url field")?;

            Ok(vec![Item {
                id: None,
                source_name: source_name.to_string(),
                source_type: "mcp".to_string(),
                url,
                title: extract_field(&obj, mapping.title_field.as_deref()),
                content: extract_field(&obj, mapping.content_field.as_deref()),
                published_at: None,
                fetched_at: now,
                category: category.clone(),
            }])
        }
        MappingStrategy::TextBlock => {
            let items = text_blocks
                .iter()
                .enumerate()
                .map(|(i, text)| Item {
                    id: None,
                    source_name: source_name.to_string(),
                    source_type: "mcp".to_string(),
                    url: format!("mcp://{source_name}/item/{i}"),
                    title: None,
                    content: Some(text.clone()),
                    published_at: None,
                    fetched_at: now,
                    category: category.clone(),
                })
                .collect();
            Ok(items)
        }
    }
}

fn extract_field(value: &Value, field: Option<&str>) -> Option<String> {
    let field = field?;
    let v = value.get(field)?;
    match v {
        Value::String(s) => Some(s.clone()),
        _ => Some(v.to_string()),
    }
}
