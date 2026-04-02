use std::future::Future;
use std::pin::Pin;

use anyhow::{Context, Result};
use chrono::Utc;

use crate::models::Item;
use crate::source::Source;

pub struct RssFeed {
    pub name: String,
    pub url: String,
    pub category: Option<String>,
}

impl Source for RssFeed {
    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> &str {
        "rss"
    }

    fn fetch<'a>(
        &'a self,
        client: &'a reqwest::Client,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Item>>> + Send + 'a>> {
        Box::pin(self.fetch_inner(client))
    }
}

impl RssFeed {
    async fn fetch_inner(&self, client: &reqwest::Client) -> Result<Vec<Item>> {
        let bytes = client
            .get(&self.url)
            .send()
            .await
            .with_context(|| format!("failed to fetch feed: {}", self.url))?
            .bytes()
            .await
            .with_context(|| format!("failed to read response body: {}", self.url))?;

        let feed = feed_rs::parser::parse(&bytes[..])
            .with_context(|| format!("failed to parse feed: {}", self.url))?;

        let now = Utc::now();
        let items = feed
            .entries
            .into_iter()
            .filter_map(|entry| {
                // Skip entries without a usable link
                let url = entry
                    .links
                    .first()
                    .map(|l| l.href.clone())
                    .or_else(|| entry.id.starts_with("http").then(|| entry.id.clone()))?;

                let title = entry.title.map(|t| t.content);
                let content = entry
                    .summary
                    .map(|s| s.content)
                    .or_else(|| entry.content.and_then(|c| c.body));

                let published_at = entry.published.or(entry.updated);

                Some(Item {
                    id: None,
                    source_name: self.name.clone(),
                    source_type: "rss".to_string(),
                    url,
                    title,
                    content,
                    published_at,
                    fetched_at: now,
                    category: self.category.clone(),
                })
            })
            .collect();

        Ok(items)
    }
}
