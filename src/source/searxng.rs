use std::future::Future;
use std::pin::Pin;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;
use tracing::debug;

use crate::config::TimeRange;
use crate::models::Item;
use crate::source::Source;

pub struct SearxngSearch {
    pub name: String,
    pub instance_url: String,
    pub query: String,
    pub category: Option<String>,
    pub time_range: TimeRange,
}

#[derive(Deserialize)]
struct SearxngResponse {
    results: Vec<SearxngResult>,
}

#[derive(Deserialize)]
struct SearxngResult {
    url: String,
    title: String,
    content: Option<String>,
}

impl Source for SearxngSearch {
    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> &str {
        "searxng"
    }

    fn fetch<'a>(
        &'a self,
        client: &'a reqwest::Client,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Item>>> + Send + 'a>> {
        Box::pin(self.fetch_inner(client))
    }
}

impl SearxngSearch {
    async fn fetch_inner(&self, client: &reqwest::Client) -> Result<Vec<Item>> {
        let url = format!("{}/search", self.instance_url.trim_end_matches('/'));

        let response = client
            .get(&url)
            .query(&[
                ("q", self.query.as_str()),
                ("format", "json"),
                ("time_range", self.time_range.as_str()),
            ])
            .send()
            .await
            .with_context(|| format!("failed to query SearXNG: {}", self.instance_url))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .with_context(|| "failed to read SearXNG response body")?;

        debug!(status = %status, body_len = body.len(), body = %body, "SearXNG raw response");

        let resp: SearxngResponse = serde_json::from_str(&body).with_context(|| {
            format!("failed to parse SearXNG response (status {status}): {body}")
        })?;

        let now = Utc::now();
        let items = resp
            .results
            .into_iter()
            .map(|r| Item {
                id: None,
                source_name: self.name.clone(),
                source_type: "searxng".to_string(),
                url: r.url,
                title: Some(r.title),
                content: r.content,
                published_at: None,
                fetched_at: now,
                category: self.category.clone(),
            })
            .collect();

        Ok(items)
    }
}
