use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use crate::models::{Digest, Item};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("failed to open database: {path}"))?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS items (
                id INTEGER PRIMARY KEY,
                source_name TEXT NOT NULL,
                source_type TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                title TEXT,
                content TEXT,
                published_at TEXT,
                fetched_at TEXT NOT NULL,
                category TEXT
            );

            CREATE TABLE IF NOT EXISTS digests (
                id INTEGER PRIMARY KEY,
                created_at TEXT NOT NULL,
                summary TEXT NOT NULL,
                item_count INTEGER NOT NULL,
                sent INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS digest_items (
                digest_id INTEGER NOT NULL REFERENCES digests(id),
                item_id INTEGER NOT NULL REFERENCES items(id),
                PRIMARY KEY (digest_id, item_id)
            );",
        )
        .with_context(|| "failed to initialize database schema")?;
        Ok(())
    }

    /// Insert an item, ignoring duplicates (by URL).
    /// Returns true if the item was actually inserted.
    pub fn insert_item(&self, item: &Item) -> Result<bool> {
        let rows = self.conn.execute(
            "INSERT OR IGNORE INTO items (source_name, source_type, url, title, content, published_at, fetched_at, category)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                item.source_name,
                item.source_type,
                item.url,
                item.title,
                item.content,
                item.published_at.map(|dt| dt.to_rfc3339()),
                item.fetched_at.to_rfc3339(),
                item.category,
            ],
        )?;
        Ok(rows > 0)
    }

    /// Get items within the lookback window that haven't been included in any digest yet.
    pub fn get_undigested_items(&self, lookback_hours: u64, limit: usize) -> Result<Vec<Item>> {
        let cutoff = Utc::now() - chrono::Duration::hours(lookback_hours as i64);
        let mut stmt = self.conn.prepare(
            "SELECT id, source_name, source_type, url, title, content, published_at, fetched_at, category
             FROM items
             WHERE fetched_at >= ?1
               AND id NOT IN (SELECT item_id FROM digest_items)
             ORDER BY fetched_at DESC
             LIMIT ?2",
        )?;

        let items = stmt
            .query_map(params![cutoff.to_rfc3339(), limit as i64], |row| {
                Ok(Item {
                    id: Some(row.get(0)?),
                    source_name: row.get(1)?,
                    source_type: row.get(2)?,
                    url: row.get(3)?,
                    title: row.get(4)?,
                    content: row.get(5)?,
                    published_at: row
                        .get::<_, Option<String>>(6)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    fetched_at: row
                        .get::<_, String>(7)
                        .ok()
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now),
                    category: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// Store a digest and link it to the given items.
    pub fn insert_digest(&self, summary: &str, item_ids: &[i64]) -> Result<Digest> {
        let now = Utc::now();
        self.conn.execute(
            "INSERT INTO digests (created_at, summary, item_count, sent) VALUES (?1, ?2, ?3, 0)",
            params![now.to_rfc3339(), summary, item_ids.len() as i64],
        )?;
        let digest_id = self.conn.last_insert_rowid();

        for &item_id in item_ids {
            self.conn.execute(
                "INSERT INTO digest_items (digest_id, item_id) VALUES (?1, ?2)",
                params![digest_id, item_id],
            )?;
        }

        Ok(Digest {
            id: Some(digest_id),
            created_at: now,
            summary: summary.to_string(),
            item_count: item_ids.len(),
            sent: false,
        })
    }

    /// Mark a digest as sent.
    pub fn mark_digest_sent(&self, digest_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE digests SET sent = 1 WHERE id = ?1",
            params![digest_id],
        )?;
        Ok(())
    }

    /// Get unsent digests (for retry).
    pub fn get_unsent_digests(&self) -> Result<Vec<Digest>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, summary, item_count, sent FROM digests WHERE sent = 0",
        )?;

        let digests = stmt
            .query_map([], |row| {
                Ok(Digest {
                    id: Some(row.get(0)?),
                    created_at: row
                        .get::<_, String>(1)
                        .ok()
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now),
                    summary: row.get(2)?,
                    item_count: row.get::<_, i64>(3)? as usize,
                    sent: row.get::<_, i64>(4)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(digests)
    }
}
