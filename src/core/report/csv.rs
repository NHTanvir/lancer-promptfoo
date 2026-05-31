//! CSV renderer (RFC 4180 quoting).

use crate::core::types::RedteamReport;
use std::fmt::Write as _;

fn esc(field: &str) -> String {
    let needs_quote = field.contains([',', '"', '\n', '\r']);
    let body = field.replace('"', "\"\"");
    if needs_quote {
        format!("\"{}\"", body)
    } else {
        body
    }
}

pub fn render(report: &RedteamReport) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "id,severity,plugin,strategy,pass,score,provider,title,attack_prompt,model_response"
    );
    for v in &report.vulnerabilities {
        let _ = writeln!(
            out,
            "{},{},{},{},{},{:.4},{},{},{},{}",
            esc(&v.id),
            esc(v.severity.label()),
            esc(&v.plugin),
            esc(v.strategy.as_deref().unwrap_or("")),
            v.pass,
            v.score,
            esc(&v.provider),
            esc(&v.title),
            esc(&v.attack_prompt),
            esc(&v.model_response),
        );
    }
    out
}
