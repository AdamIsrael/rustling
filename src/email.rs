use anyhow::{Context, Result, bail};
use serde_json::json;

use crate::config::EmailConfig;
use crate::models::Digest;

pub async fn send_digest(
    client: &reqwest::Client,
    config: &EmailConfig,
    api_key: &str,
    digest: &Digest,
) -> Result<()> {
    let subject = match &config.subject_prefix {
        Some(prefix) => format!("{prefix} - {}", digest.created_at.format("%Y-%m-%d")),
        None => format!("Rustling Digest - {}", digest.created_at.format("%Y-%m-%d")),
    };

    let personalizations: Vec<_> = config
        .to
        .iter()
        .map(|addr| json!({"to": [{"email": addr}]}))
        .collect();

    let body = json!({
        "personalizations": personalizations,
        "from": {"email": config.from},
        "subject": subject,
        "content": [
            {
                "type": "text/html",
                "value": &digest.summary,
            }
        ]
    });

    let resp = client
        .post("https://api.sendgrid.com/v3/mail/send")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("failed to send email via SendGrid")?;

    let status = resp.status();
    if !status.is_success() {
        let error_body = resp.text().await.unwrap_or_default();
        bail!("SendGrid returned {status}: {error_body}");
    }

    Ok(())
}
