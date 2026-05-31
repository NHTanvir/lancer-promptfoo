//! HTTP provider shim — receives promptfoo attack POSTs, proxies them to the
//! real target, and captures full request/response pairs for Lancer's HTTP
//! history (Burp-style replay). Point a promptfoo `http` provider at this shim
//! and every attack becomes visible/replayable in Lancer.

use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::{error, info};
use uuid::Uuid;

/// Response body the shim returns to promptfoo.
#[derive(Debug, Serialize)]
pub struct ShimResponse {
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
}

/// A captured HTTP exchange stored by the shim (one JSONL line on flush).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpExchange {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub attack_prompt: String,
    pub request_url: String,
    pub request_method: String,
    pub request_headers: HashMap<String, String>,
    pub request_body: String,
    pub response_status: u16,
    pub response_headers: HashMap<String, String>,
    pub response_body: String,
    pub latency_ms: u64,
}

#[derive(Clone)]
pub struct ShimState {
    pub target_url: String,
    pub client: Client,
    pub history: Arc<Mutex<Vec<HttpExchange>>>,
    pub extra_headers: HashMap<String, String>,
}

pub async fn run_shim_server(
    bind_addr: SocketAddr,
    target_url: impl Into<String>,
    extra_headers: HashMap<String, String>,
) -> Result<Arc<Mutex<Vec<HttpExchange>>>> {
    let state = ShimState {
        target_url: target_url.into(),
        client: Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?,
        history: Arc::new(Mutex::new(Vec::new())),
        extra_headers,
    };
    let history = Arc::clone(&state.history);

    let app = Router::new()
        .route("/", post(handle_attack))
        .route("/chat", post(handle_attack))
        .route("/api/chat", post(handle_attack))
        .with_state(state);

    info!("Shim server listening on {}", bind_addr);
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("Failed to bind to {}", bind_addr))?;
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("Shim server error: {}", e);
        }
    });
    Ok(history)
}

async fn handle_attack(State(state): State<ShimState>, body: axum::body::Bytes) -> impl IntoResponse {
    let body_str = String::from_utf8_lossy(&body).to_string();
    let attack_prompt = extract_prompt(&body_str);
    let start = Instant::now();
    let result = forward_request(&state, &body_str).await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok((status_code, resp_headers, resp_body)) => {
            let exchange = HttpExchange {
                id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                attack_prompt: attack_prompt.clone(),
                request_url: state.target_url.clone(),
                request_method: "POST".into(),
                request_headers: state.extra_headers.clone(),
                request_body: body_str,
                response_status: status_code,
                response_headers: resp_headers,
                response_body: resp_body.clone(),
                latency_ms,
            };
            let preview: String = attack_prompt.chars().take(60).collect();
            info!("Shim: {} -> {} ({} ms)", preview, status_code, latency_ms);
            state.history.lock().unwrap().push(exchange);
            let output = extract_output(&resp_body);
            (
                StatusCode::OK,
                Json(ShimResponse { output, error: None, cached: Some(false) }),
            )
                .into_response()
        }
        Err(e) => {
            error!("Shim forward error: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(ShimResponse { output: String::new(), error: Some(e.to_string()), cached: None }),
            )
                .into_response()
        }
    }
}

async fn forward_request(
    state: &ShimState,
    body: &str,
) -> Result<(u16, HashMap<String, String>, String)> {
    let mut req = state
        .client
        .post(&state.target_url)
        .header("Content-Type", "application/json")
        .body(body.to_string());
    for (k, v) in &state.extra_headers {
        req = req.header(k, v);
    }
    let resp = req.send().await.context("Forward request failed")?;
    let status = resp.status().as_u16();
    let mut headers = HashMap::new();
    for (name, val) in resp.headers() {
        if let Ok(v) = val.to_str() {
            headers.insert(name.to_string(), v.to_string());
        }
    }
    let body = resp.text().await.context("Reading response body failed")?;
    Ok((status, headers, body))
}

fn extract_prompt(body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        for key in ["prompt", "message", "input", "query", "text"] {
            if let Some(serde_json::Value::String(s)) = v.get(key) {
                return s.clone();
            }
        }
        if let Some(msgs) = v.get("messages").and_then(|m| m.as_array()) {
            if let Some(first) = msgs.first() {
                if let Some(serde_json::Value::String(s)) = first.get("content") {
                    return s.clone();
                }
            }
        }
    }
    body.chars().take(200).collect()
}

fn extract_output(resp_body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(resp_body) {
        for key in ["output", "reply", "response", "answer", "text", "content", "message"] {
            if let Some(serde_json::Value::String(s)) = v.get(key) {
                return s.clone();
            }
        }
        if let Some(content) = v.pointer("/choices/0/message/content").and_then(|c| c.as_str()) {
            return content.to_string();
        }
        if let Some(text) = v.pointer("/content/0/text").and_then(|c| c.as_str()) {
            return text.to_string();
        }
    }
    resp_body.to_string()
}

/// Write captured exchanges to a JSONL file for replay.
pub async fn write_history(history: &[HttpExchange], path: &std::path::Path) -> Result<()> {
    let mut lines = Vec::new();
    for ex in history {
        lines.push(serde_json::to_string(ex)?);
    }
    tokio::fs::write(path, lines.join("\n"))
        .await
        .context("Failed to write HTTP history")?;
    Ok(())
}
