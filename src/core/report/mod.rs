//! Report rendering + output formats.
//!
//! Parity mandate: every output format upstream emits must work — text, JSON,
//! YAML, CSV, HTML, XML, SARIF, JUnit. The format is inferred from the output
//! path extension (`report.json`, `report.sarif`, `report.junit.xml`, …) or
//! given explicitly.

pub mod csv;
pub mod html;
pub mod junit;
pub mod markdown;
pub mod sarif;
pub mod terminal;
pub mod xml;

use crate::core::types::{RedteamReport, Severity, Vulnerability};
use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Yaml,
    Csv,
    Html,
    Xml,
    Sarif,
    Junit,
    Markdown,
    Text,
}

impl OutputFormat {
    /// Infer the format from a file path's extension / suffix.
    pub fn from_path(path: &Path) -> OutputFormat {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        if name.ends_with(".junit.xml") || name.contains("junit") {
            return OutputFormat::Junit;
        }
        match path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "json" => OutputFormat::Json,
            "yaml" | "yml" => OutputFormat::Yaml,
            "csv" => OutputFormat::Csv,
            "html" | "htm" => OutputFormat::Html,
            "sarif" => OutputFormat::Sarif,
            "xml" => OutputFormat::Xml,
            "md" | "markdown" => OutputFormat::Markdown,
            _ => OutputFormat::Text,
        }
    }

    pub fn parse(s: &str) -> Option<OutputFormat> {
        Some(match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "yaml" | "yml" => OutputFormat::Yaml,
            "csv" => OutputFormat::Csv,
            "html" => OutputFormat::Html,
            "xml" => OutputFormat::Xml,
            "sarif" => OutputFormat::Sarif,
            "junit" => OutputFormat::Junit,
            "markdown" | "md" => OutputFormat::Markdown,
            "text" | "txt" => OutputFormat::Text,
            _ => return None,
        })
    }
}

/// Render a report to a string in the requested format.
pub fn render(report: &RedteamReport, fmt: OutputFormat) -> Result<String> {
    Ok(match fmt {
        OutputFormat::Json => serde_json::to_string_pretty(report)?,
        OutputFormat::Yaml => serde_yaml::to_string(report)?,
        OutputFormat::Csv => csv::render(report),
        OutputFormat::Html => html::render(report),
        OutputFormat::Xml => xml::render(report),
        OutputFormat::Sarif => sarif::render(report)?,
        OutputFormat::Junit => junit::render(report),
        OutputFormat::Markdown => markdown::render(report),
        OutputFormat::Text => terminal::render_plain(report),
    })
}

/// Write a report to a path, inferring the format from the extension.
pub fn write_output(report: &RedteamReport, path: &Path) -> Result<()> {
    let fmt = OutputFormat::from_path(path);
    write_output_fmt(report, path, fmt)
}

pub fn write_output_fmt(report: &RedteamReport, path: &Path, fmt: OutputFormat) -> Result<()> {
    let body = render(report, fmt)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).ok();
        }
    }
    std::fs::write(path, body).with_context(|| format!("Failed to write {}", path.display()))
}

/// True if the report has findings at or above `threshold` severity.
pub fn check_fail_on(report: &RedteamReport, threshold: &Severity) -> bool {
    report.summary.findings_at_or_above(threshold) > 0
}

/// Return only findings present in `current` but not in `baseline` (by dedup key).
pub fn diff_from_baseline(
    current: &RedteamReport,
    baseline: &RedteamReport,
) -> Vec<Vulnerability> {
    let baseline_keys: std::collections::HashSet<String> =
        baseline.findings().iter().map(|v| v.dedup_key()).collect();
    current
        .findings()
        .into_iter()
        .filter(|v| !baseline_keys.contains(&v.dedup_key()))
        .cloned()
        .collect()
}

// Convenience re-exports used across the codebase.
pub use markdown::write_markdown;
pub use terminal::print_report;
