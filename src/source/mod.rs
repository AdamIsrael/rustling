#[cfg(feature = "mcp")]
pub mod mcp;
pub mod rss;
pub mod searxng;

use std::future::Future;
use std::pin::Pin;

use crate::models::Item;
use anyhow::Result;

pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    #[allow(dead_code)]
    fn source_type(&self) -> &str;
    fn fetch<'a>(
        &'a self,
        client: &'a reqwest::Client,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Item>>> + Send + 'a>>;
}
