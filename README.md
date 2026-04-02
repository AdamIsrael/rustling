# Rustling

A command-line digest agent that collects content from RSS feeds and web searches, summarizes it with an LLM, and delivers a digest via email.

Designed to run on a schedule (e.g. cron). Each run is idempotent — duplicate items are ignored and unsent digests are retried automatically.

## How it works

1. **Collect** — Fetches items from configured RSS/Atom feeds and SearXNG searches
2. **Store** — Saves items to a local SQLite database, deduplicating by URL
3. **Summarize** — Sends new items to an LLM (Claude, Ollama, or any OpenAI-compatible endpoint) to generate a grouped digest
4. **Deliver** — Emails the digest via SendGrid

## Setup

### Build

```sh
cargo build --release
```

### Configure

Copy the example config and edit it:

```sh
cp rustling.example.toml rustling.toml
```

#### Feeds

Add RSS or Atom feed URLs:

```toml
[[feeds]]
name = "Hacker News"
url = "https://hnrss.org/frontpage"
category = "tech"

[[feeds]]
name = "Rust Blog"
url = "https://blog.rust-lang.org/feed.xml"
category = "rust"
```

#### SearXNG searches

Query a [SearXNG](https://docs.searxng.org/) instance to collect web search results:

```toml
[[searches]]
name = "Rust news"
instance_url = "https://searxng.example.com"
query = "rust programming language"
category = "rust"
time_range = "day"   # day (default), week, month, or year
```

Multiple searches can be configured. Each `[[searches]]` entry queries the given SearXNG instance and collects the results as digest items. The `time_range` parameter filters results to the specified recency.

#### Keyword filtering

Optionally filter collected items (from all sources) by keywords. Items whose title or content don't contain at least one keyword are dropped before storage:

```toml
keywords = "rust, kubernetes, llm, security"
```

If omitted, all items are kept.

#### LLM provider

Choose one of three providers:

**Claude (Anthropic API):**

```toml
[llm]
provider = "claude"
endpoint = "https://api.anthropic.com/v1/messages"
model = "claude-sonnet-4-20250514"
```

**Ollama (local):**

```toml
[llm]
provider = "ollama"
endpoint = "http://localhost:11434/api/generate"
model = "llama3"
```

**OpenAI-compatible:**

```toml
[llm]
provider = "openai_compat"
endpoint = "https://api.openai.com/v1/chat/completions"
model = "gpt-4"
```

You can optionally override the summarization prompt:

```toml
[llm]
prompt_template = "Your custom system prompt here..."
```

#### Email

```toml
[email]
from = "digest@yourdomain.com"
to = ["alice@example.com", "bob@example.com"]
subject_prefix = "Rustling Digest"
```

#### General settings

```toml
database_path = "rustling.db"   # SQLite database location
lookback_hours = 24             # How far back to include items
max_items_per_digest = 50       # Cap items sent to the LLM
verbose = false                 # Enable debug-level logging
```

### Environment variables

Set these before running:

```sh
export SENDGRID_API_KEY="SG.your-key-here"
export LLM_API_KEY="sk-your-key-here"  # required for Claude and OpenAI-compat; not needed for Ollama
```

## Usage

```sh
# Run with default config (./rustling.toml)
cargo run --release

# Run with a specific config file
cargo run --release -- /path/to/rustling.toml
```

### Running on a schedule

Add a crontab entry to run daily at 8am:

```
0 8 * * * cd /path/to/rustling && SENDGRID_API_KEY=... LLM_API_KEY=... ./target/release/rustling
```

### Logging

Rustling uses `tracing` for structured logging. Control verbosity with `RUST_LOG`:

```sh
RUST_LOG=rustling=debug cargo run    # verbose output
RUST_LOG=rustling=warn cargo run     # only warnings and errors
```
