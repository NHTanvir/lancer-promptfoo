//! Local eval store under `<config_dir>/evals/`. Backs `show`, `list`,
//! `delete`, and `retry` (upstream persists to SQLite; we use one JSON file per
//! run, which is enough for the CLI surface and needs no native dep).

use crate::core::types::RedteamReport;
use anyhow::Result;
use std::path::PathBuf;
use uuid::Uuid;

pub fn evals_dir() -> PathBuf {
    crate::core::env::config_dir().join("evals")
}

#[derive(Debug, Clone)]
pub struct StoredEval {
    pub id: String,
    pub target: String,
    pub timestamp: String,
    pub findings: usize,
    pub path: PathBuf,
}

/// Persist a report and return its generated id.
pub fn save(report: &RedteamReport) -> Result<String> {
    let dir = evals_dir();
    std::fs::create_dir_all(&dir)?;
    let id = format!("eval-{}", &Uuid::new_v4().to_string()[..8]);
    let path = dir.join(format!("{}.json", id));
    let mut envelope = serde_json::to_value(report)?;
    if let Some(obj) = envelope.as_object_mut() {
        obj.insert("evalId".into(), serde_json::Value::String(id.clone()));
    }
    std::fs::write(&path, serde_json::to_string_pretty(&envelope)?)?;
    Ok(id)
}

pub fn list() -> Vec<StoredEval> {
    let dir = evals_dir();
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Ok(report) = serde_json::from_str::<RedteamReport>(&text) {
                    let id = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    out.push(StoredEval {
                        id,
                        target: report.target.clone(),
                        timestamp: report.timestamp.to_rfc3339(),
                        findings: report.summary.total_findings(),
                        path,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    out
}

pub fn load(id: &str) -> Option<RedteamReport> {
    let path = evals_dir().join(format!("{}.json", id.trim_end_matches(".json")));
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn delete(id: &str) -> Result<bool> {
    let path = evals_dir().join(format!("{}.json", id.trim_end_matches(".json")));
    if path.exists() {
        std::fs::remove_file(path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
