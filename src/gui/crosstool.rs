//! Cross-tool local HTTP server for Lancer ecosystem integration.
//!
//! Each Lancer-clone GUI exposes a small server on `127.0.0.1:<port>` (written
//! to `%TEMP%/lancer-promptfoo.port`) so the main Lancer GUI can discover it and
//! "Send to <tool>" payloads flow between the 30 tools.
//!
//! Endpoints:
//!   GET  /v1/state               — current jobs / progress
//!   POST /v1/import              — accept a target/file/record from another tool
//!   GET  /v1/export?format=json  — pull the current result set (json|csv|sarif|markdown)

use crate::core::report::{self, OutputFormat};
use crate::core::types::RedteamReport;
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Shared state between the GUI and the cross-tool server.
#[derive(Clone, Default)]
pub struct SharedState {
    pub inner: Arc<Mutex<SharedInner>>,
}

#[derive(Default)]
pub struct SharedInner {
    pub status: String,
    pub running: bool,
    pub report: Option<RedteamReport>,
    /// Targets imported from other Lancer tools, awaiting a run.
    pub imported_targets: Vec<String>,
}

/// Start the cross-tool server on an OS-assigned port and write the port file.
/// Returns the bound address.
pub async fn start(state: SharedState) -> std::io::Result<SocketAddr> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // Advertise the port for Lancer discovery.
    let port_file = std::env::temp_dir().join("lancer-promptfoo.port");
    let _ = std::fs::write(&port_file, addr.port().to_string());

    let app = Router::new()
        .route("/v1/state", get(get_state))
        .route("/v1/import", post(post_import))
        .route("/v1/export", get(get_export))
        .with_state(state);

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(addr)
}

async fn get_state(State(s): State<SharedState>) -> Json<serde_json::Value> {
    let inner = s.inner.lock().unwrap();
    Json(serde_json::json!({
        "tool": "lancer-promptfoo",
        "running": inner.running,
        "status": inner.status,
        "hasReport": inner.report.is_some(),
        "summary": inner.report.as_ref().map(|r| &r.summary),
        "importedTargets": inner.imported_targets,
    }))
}

async fn post_import(
    State(s): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let target = body
        .get("target")
        .or_else(|| body.get("url"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    if !target.is_empty() {
        s.inner.lock().unwrap().imported_targets.push(target.clone());
    }
    Json(serde_json::json!({ "accepted": !target.is_empty(), "target": target }))
}

async fn get_export(
    State(s): State<SharedState>,
    Query(params): Query<HashMap<String, String>>,
) -> (axum::http::StatusCode, String) {
    let fmt = params
        .get("format")
        .and_then(|f| OutputFormat::parse(f))
        .unwrap_or(OutputFormat::Json);
    let inner = s.inner.lock().unwrap();
    match &inner.report {
        Some(report) => match report::render(report, fmt) {
            Ok(body) => (axum::http::StatusCode::OK, body),
            Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        },
        None => (axum::http::StatusCode::NOT_FOUND, "no report yet".into()),
    }
}
