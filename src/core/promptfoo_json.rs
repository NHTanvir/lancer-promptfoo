//! Parser for upstream promptfoo JSON output (`promptfoo eval --output x.json`).
//!
//! Lets `lancer-promptfoo report x.json` ingest results produced by the real
//! promptfoo CLI (or by the subprocess wrapper in `integration::promptfoo`) and
//! convert them into our [`Vulnerability`] schema.

use crate::core::types::{Severity, Vulnerability};
use anyhow::Result;
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct PromptfooOutput {
    pub results: Option<PromptfooResults>,
    #[serde(default)]
    pub stats: Option<serde_json::Value>,
}

/// `results` is sometimes an array, sometimes `{ results: [...] }`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PromptfooResults {
    Array(Vec<PromptfooResult>),
    Wrapped { results: Vec<PromptfooResult> },
}

impl PromptfooResults {
    fn into_vec(self) -> Vec<PromptfooResult> {
        match self {
            PromptfooResults::Array(v) => v,
            PromptfooResults::Wrapped { results } => results,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptfooResult {
    pub prompt: Option<PromptfooPrompt>,
    pub vars: Option<HashMap<String, serde_json::Value>>,
    pub response: Option<PromptfooResponse>,
    pub success: Option<bool>,
    pub score: Option<f64>,
    pub grading_result: Option<PromptfooGradingResult>,
    pub provider: Option<PromptfooProvider>,
    pub test_case: Option<PromptfooTestCase>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct PromptfooPrompt {
    pub raw: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PromptfooResponse {
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PromptfooGradingResult {
    pub pass: Option<bool>,
    pub score: Option<f64>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PromptfooProvider {
    pub id: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptfooTestCase {
    pub metadata: Option<PromptfooMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptfooMetadata {
    pub plugin_id: Option<String>,
    pub strategy: Option<String>,
    pub severity: Option<String>,
    pub harm_category: Option<String>,
}

pub fn parse_promptfoo_output(json_str: &str) -> Result<Vec<Vulnerability>> {
    let output: PromptfooOutput = serde_json::from_str(json_str)?;
    let results = match output.results {
        Some(r) => r.into_vec(),
        None => return Ok(vec![]),
    };
    Ok(results.into_iter().map(result_to_vuln).collect())
}

fn result_to_vuln(r: PromptfooResult) -> Vulnerability {
    let meta = r.test_case.as_ref().and_then(|tc| tc.metadata.as_ref());
    let plugin = meta
        .and_then(|m| m.plugin_id.clone())
        .unwrap_or_else(|| "unknown".into());
    let strategy = meta.and_then(|m| m.strategy.clone());
    let severity = meta
        .and_then(|m| m.severity.clone())
        .map(|s| Severity::from_str_loose(&s))
        .unwrap_or_else(|| infer_severity(&plugin));

    let attack_prompt = r
        .prompt
        .as_ref()
        .and_then(|p| p.raw.clone())
        .unwrap_or_default();

    let model_response = r
        .response
        .as_ref()
        .and_then(|resp| resp.output.as_ref())
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .or_else(|| {
            r.response
                .as_ref()
                .and_then(|resp| resp.error.clone())
                .map(|e| format!("[error] {}", e))
        })
        .unwrap_or_default();

    let pass = r
        .grading_result
        .as_ref()
        .and_then(|g| g.pass)
        .or(r.success)
        .unwrap_or(true);
    let score = r
        .grading_result
        .as_ref()
        .and_then(|g| g.score)
        .or(r.score)
        .unwrap_or(if pass { 1.0 } else { 0.0 });
    let reason = r
        .grading_result
        .as_ref()
        .and_then(|g| g.reason.clone())
        .unwrap_or_default();
    let provider = r
        .provider
        .as_ref()
        .and_then(|p| p.id.clone().or_else(|| p.label.clone()))
        .unwrap_or_else(|| "unknown".into());

    let title = match &strategy {
        Some(s) => format!("[{}] {} via {}", if pass { "Pass" } else { "Finding" }, plugin, s),
        None => format!("[{}] {}", if pass { "Pass" } else { "Finding" }, plugin),
    };

    Vulnerability {
        id: Uuid::new_v4().to_string(),
        severity,
        plugin,
        strategy,
        title,
        description: if reason.is_empty() {
            format!("Imported promptfoo result (pass={})", pass)
        } else {
            reason
        },
        attack_prompt,
        model_response,
        pass,
        score,
        timestamp: Utc::now(),
        provider,
        assertions: Vec::new(),
        metadata: HashMap::new(),
    }
}

fn infer_severity(plugin: &str) -> Severity {
    let p = plugin.to_lowercase();
    if p.contains("sql-injection")
        || p.contains("ssrf")
        || p.contains("shell-injection")
        || p.contains("sandbox")
        || p.contains("secret-")
        || p.contains("pii")
        || p.contains("bola")
        || p.contains("bfla")
        || p.contains("rbac")
        || p.contains("chemical-biological")
        || p.contains("child-exploitation")
    {
        Severity::Critical
    } else if p.contains("prompt-injection")
        || p.contains("prompt-extraction")
        || p.contains("data-exfil")
        || p.contains("debug-access")
        || p.contains("system-prompt-override")
        || p.contains("cybercrime")
    {
        Severity::High
    } else if p.contains("hallucination") || p.contains("excessive-agency") || p.contains("harmful:") || p.contains("bias:") {
        Severity::Medium
    } else if p.contains("off-topic") || p.contains("politics") || p.contains("profanity") || p.contains("insults") {
        Severity::Low
    } else {
        Severity::Informational
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_results() {
        assert!(parse_promptfoo_output(r#"{"results":[]}"#).unwrap().is_empty());
        assert!(parse_promptfoo_output(r#"{"stats":{}}"#).unwrap().is_empty());
    }

    #[test]
    fn wrapped_results() {
        let j = r#"{"results":{"results":[{"success":false,"score":0.9,"prompt":{"raw":"hi"}}]}}"#;
        let v = parse_promptfoo_output(j).unwrap();
        assert_eq!(v.len(), 1);
        assert!(!v[0].pass);
    }
}
