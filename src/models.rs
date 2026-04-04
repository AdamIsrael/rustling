use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Item {
    pub id: Option<i64>,
    pub source_name: String,
    pub source_type: String,
    pub url: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub fetched_at: DateTime<Utc>,
    pub category: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Digest {
    pub id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub summary: String,
    pub item_count: usize,
    #[allow(dead_code)]
    pub sent: bool,
}
