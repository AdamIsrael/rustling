pub mod rss;

use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use crate::models::Item;

pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn source_type(&self) -> &str;
    fn fetch<'a>(
        &'a self,
        client: &'a reqwest::Client,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Item>>> + Send + 'a>>;
}
