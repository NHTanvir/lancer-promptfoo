//! SARIF 2.1.0 renderer — for GitHub Code Scanning ingestion.

use crate::core::types::{RedteamReport, Severity};
use anyhow::Result;

fn sarif_level(sev: &Severity) -> &'static str {
    match sev {
        Severity::Critical | Severity::High => "error",
        Severity::Medium => "warning",
        Severity::Low | Severity::Informational => "note",
    }
}

pub fn render(report: &RedteamReport) -> Result<String> {
    // Distinct rules = distinct plugin ids among findings.
    let mut rules: Vec<serde_json::Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for v in report.findings() {
        if seen.insert(v.plugin.clone()) {
            rules.push(serde_json::json!({
                "id": v.plugin,
                "name": v.plugin,
                "shortDescription": { "text": format!("LLM red-team finding: {}", v.plugin) },
                "defaultConfiguration": { "level": sarif_level(&v.severity) },
            }));
        }
    }

    let results: Vec<serde_json::Value> = report
        .findings()
        .iter()
        .map(|v| {
            serde_json::json!({
                "ruleId": v.plugin,
                "level": sarif_level(&v.severity),
                "message": { "text": format!("{} — {}", v.title, v.description) },
                "properties": {
                    "severity": v.severity.label(),
                    "strategy": v.strategy,
                    "score": v.score,
                    "attackPrompt": v.attack_prompt,
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": "promptfooconfig.yaml" },
                        "region": { "startLine": 1 }
                    }
                }]
            })
        })
        .collect();

    let doc = serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "lancer-promptfoo",
                    "informationUri": "https://github.com/NHTanvir/lancer-promptfoo",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rules,
                }
            },
            "results": results,
        }]
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}
