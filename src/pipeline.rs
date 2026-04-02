use anyhow::{Context, Result};
use tracing::{info, warn, error};

use crate::config::{Config, Secrets};
use crate::db::Database;
use crate::email;
use crate::llm;
use crate::models::Item;
use crate::source::Source;
use crate::source::rss::RssFeed;

pub async fn run(config: &Config, secrets: &Secrets) -> Result<()> {
    let db = Database::open(&config.database_path)?;
    let client = reqwest::Client::new();

    // Build sources from config
    let sources: Vec<Box<dyn Source>> = config
        .feeds
        .iter()
        .map(|f| {
            Box::new(RssFeed {
                name: f.name.clone(),
                url: f.url.clone(),
                category: f.category.clone(),
            }) as Box<dyn Source>
        })
        .collect();

    // 1. Collect items from all sources
    let mut total_fetched = 0usize;
    let mut total_new = 0usize;

    for source in &sources {
        match source.fetch(&client).await {
            Ok(items) => {
                let count = items.len();
                total_fetched += count;
                let new = store_items(&db, &items)?;
                total_new += new;
                info!(
                    source = source.name(),
                    fetched = count,
                    new = new,
                    "collected items"
                );
            }
            Err(e) => {
                warn!(source = source.name(), error = %e, "failed to fetch source, skipping");
            }
        }
    }

    info!(total_fetched, total_new, "collection complete");

    // 2. Get undigested items
    let items = db.get_undigested_items(config.lookback_hours, config.max_items_per_digest)?;
    if items.is_empty() {
        info!("no new items to digest");
        return Ok(());
    }
    info!(count = items.len(), "items to summarize");

    // 3. Summarize via LLM
    let summary = llm::summarize(
        &client,
        &config.llm,
        secrets.llm_api_key.as_deref(),
        &items,
    )
    .await
    .context("LLM summarization failed")?;

    let item_ids: Vec<i64> = items.iter().filter_map(|i| i.id).collect();
    let digest = db.insert_digest(&summary, &item_ids)?;
    let digest_id = digest.id.expect("digest should have an id after insert");
    info!(digest_id, item_count = digest.item_count, "digest created");

    // 4. Send email
    match email::send_digest(&client, &config.email, &secrets.sendgrid_api_key, &digest).await {
        Ok(()) => {
            db.mark_digest_sent(digest_id)?;
            info!(digest_id, "digest email sent");
        }
        Err(e) => {
            error!(digest_id, error = %e, "failed to send digest email (will retry next run)");
        }
    }

    Ok(())
}

fn store_items(db: &Database, items: &[Item]) -> Result<usize> {
    let mut new_count = 0;
    for item in items {
        if db.insert_item(item)? {
            new_count += 1;
        }
    }
    Ok(new_count)
}
