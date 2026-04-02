# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

- **Build:** `cargo build`
- **Run:** `cargo run` (optionally pass a config path: `cargo run -- path/to/config.toml`)
- **Test all:** `cargo test`
- **Test single:** `cargo test <test_name>`
- **Lint:** `cargo clippy`
- **Format:** `cargo fmt`
- **Check (fast compile check):** `cargo check`

## Environment Variables

- `SENDGRID_API_KEY` — required for email delivery
- `LLM_API_KEY` — required for Claude/OpenAI-compat providers, not needed for Ollama
- `RUST_LOG` — controls log filtering (default shows `rustling=info`)

## Architecture

Rustling is a CLI digest agent (Rust 2024 edition) that collects content from external sources, summarizes it with an LLM, and emails the digest. Designed to run on a cron schedule; each run is idempotent.

**Pipeline flow** (`pipeline.rs` orchestrates):
1. Fetch items from all configured sources concurrently
2. Store in SQLite with URL-based dedup (`INSERT OR IGNORE`)
3. Query items not yet in any digest (within lookback window)
4. Send batch to LLM for summarization, store digest
5. Email digest via SendGrid, mark as sent

**Key modules:**
- `config.rs` — TOML config deserialization + env var secrets (`Secrets` struct)
- `db.rs` — SQLite schema init and all queries (items, digests, digest_items tables)
- `source/mod.rs` — `Source` trait (dyn-compatible via `Pin<Box<dyn Future>>`)
- `source/rss.rs` — RSS/Atom feed implementation using `feed-rs`
- `llm.rs` — Multi-provider LLM client (Claude, Ollama, OpenAI-compatible)
- `email.rs` — SendGrid v3 Mail Send API
- `models.rs` — `Item` and `Digest` data types

**Extensibility:** New data sources implement the `Source` trait in `source/`. The pipeline operates on `Vec<Box<dyn Source>>` and requires no changes when adding sources.

**Config:** `rustling.toml` (see `rustling.example.toml`) for feeds/LLM/email settings. Secrets go in env vars, never in the config file.
