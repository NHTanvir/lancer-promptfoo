//! Cloud share — upload an eval/red-team report and return a public URL.
//! Honors `--no-share` (caller skips this) and `PROMPTFOO_SHARING_ENDPOINT`
//! for a private / self-hosted share server.

use crate::core::types::RedteamReport;
use anyhow::{Context, Result};

/// Upload a report to the sharing endpoint and return the public URL.
pub async fn share_report(report: &RedteamReport, team: Option<&str>) -> Result<String> {
    let endpoint = crate::core::env::sharing_endpoint();
    let url = format!("{}/api/v1/results", endpoint.trim_end_matches('/'));

    let token = crate::auth::load_token().map(|t| t.token);
    let mut body = serde_json::json!({
        "report": report,
        "source": "lancer-promptfoo",
        "version": env!("CARGO_PKG_VERSION"),
    });
    if let Some(t) = team {
        body["team"] = serde_json::Value::String(t.to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    let mut req = client.post(&url).json(&body);
    if let Some(t) = &token {
        req = req.bearer_auth(t);
    }

    let resp = req.send().await.context("share upload failed")?;
    let status = resp.status();
    let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
    if !status.is_success() {
        anyhow::bail!("share endpoint returned HTTP {}: {}", status, json);
    }
    // Endpoint returns { id, url } or similar.
    let share_url = json
        .get("url")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            json.get("id")
                .and_then(|i| i.as_str())
                .map(|id| format!("{}/eval/{}", endpoint.trim_end_matches('/'), id))
        })
        .unwrap_or_else(|| format!("{} (uploaded; no URL returned)", endpoint));
    Ok(share_url)
}
