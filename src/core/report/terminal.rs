//! Colored terminal report + a plain-text renderer.

use crate::core::types::{RedteamReport, ReportSummary, Severity, Vulnerability};
use colored::Colorize;
use std::fmt::Write as _;

pub fn print_report(report: &RedteamReport) {
    print!("{}", render_colored(report));
}

fn render_colored(report: &RedteamReport) -> String {
    let mut out = String::new();
    let bar = "═".repeat(63);
    let _ = writeln!(out, "\n{}", bar.as_str().cyan());
    let _ = writeln!(out, "{}", "  lancer-promptfoo  |  Red Team Report".cyan().bold());
    let _ = writeln!(out, "{}", bar.as_str().cyan());
    let _ = writeln!(out, "\n  Target  : {}", report.target.as_str().yellow());
    let _ = writeln!(
        out,
        "  Time    : {}",
        report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );
    if let Some(p) = &report.purpose {
        let _ = writeln!(out, "  Purpose : {}", p);
    }
    let _ = writeln!(out);
    out.push_str(&summary_block(&report.summary, true));

    let findings: Vec<&Vulnerability> = report.findings();
    if findings.is_empty() {
        let _ = writeln!(out, "\n  {} No exploitable vulnerabilities found.\n", "✓".green());
        return out;
    }

    let _ = writeln!(out, "\n{}\n", "  Findings".bold().underline());
    let mut sorted = findings.clone();
    sorted.sort_by(|a, b| a.severity.cmp(&b.severity));
    for v in &sorted {
        let _ = writeln!(
            out,
            "  {:<9} {:<28} {:<18} {:.2}  {}",
            colorize_sev(&v.severity),
            truncate(&v.plugin, 28),
            truncate(v.strategy.as_deref().unwrap_or("-"), 18),
            v.score,
            truncate(&v.title, 50),
        );
    }

    let critical_high: Vec<&&Vulnerability> = sorted
        .iter()
        .filter(|v| matches!(v.severity, Severity::Critical | Severity::High))
        .collect();
    if !critical_high.is_empty() {
        let _ = writeln!(out, "\n{}", "  Critical / High Detail".bold().underline());
        for v in &critical_high {
            let _ = writeln!(out, "\n  {} {}", colorize_sev(&v.severity), v.plugin.as_str().yellow());
            let _ = writeln!(out, "  {}", v.title.as_str().bold());
            let _ = writeln!(out, "  Description : {}", v.description);
            let _ = writeln!(out, "  Attack      : {}", truncate(&v.attack_prompt, 200).cyan());
            let _ = writeln!(out, "  Response    : {}", truncate(&v.model_response, 200).dimmed());
        }
    }
    let _ = writeln!(out, "\n{}\n", bar.as_str().cyan());
    out
}

/// Plain (no ANSI color) rendering for `--output report.txt` / piping.
pub fn render_plain(report: &RedteamReport) -> String {
    let mut out = String::new();
    let bar = "=".repeat(63);
    let _ = writeln!(out, "{}\n  lancer-promptfoo  |  Red Team Report\n{}", bar, bar);
    let _ = writeln!(out, "Target : {}", report.target);
    let _ = writeln!(out, "Time   : {}", report.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
    if let Some(p) = &report.purpose {
        let _ = writeln!(out, "Purpose: {}", p);
    }
    let _ = writeln!(out);
    out.push_str(&summary_block(&report.summary, false));
    let mut findings = report.findings();
    findings.sort_by(|a, b| a.severity.cmp(&b.severity));
    if findings.is_empty() {
        let _ = writeln!(out, "\nNo exploitable vulnerabilities found.");
        return out;
    }
    let _ = writeln!(out, "\nFindings:");
    for v in &findings {
        let _ = writeln!(
            out,
            "  {:<9} {:<30} {:<18} {:.2}  {}",
            v.severity.label(),
            truncate(&v.plugin, 30),
            truncate(v.strategy.as_deref().unwrap_or("-"), 18),
            v.score,
            truncate(&v.title, 50),
        );
    }
    out
}

fn summary_block(s: &ReportSummary, _color: bool) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "  Summary");
    let _ = writeln!(
        out,
        "  Total tests : {}  |  Pass rate : {:.1}%",
        s.total_tests,
        s.pass_rate * 100.0
    );
    let _ = writeln!(
        out,
        "  {} CRITICAL   {} HIGH   {} MEDIUM   {} LOW   {} INFO",
        s.critical, s.high, s.medium, s.low, s.informational
    );
    out
}

fn colorize_sev(s: &Severity) -> String {
    match s {
        Severity::Critical => "CRITICAL".red().bold().to_string(),
        Severity::High => "HIGH".bright_red().to_string(),
        Severity::Medium => "MEDIUM".yellow().to_string(),
        Severity::Low => "LOW".bright_yellow().to_string(),
        Severity::Informational => "INFO".dimmed().to_string(),
    }
}

fn truncate(s: &str, n: usize) -> String {
    let cleaned: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    if cleaned.chars().count() <= n {
        cleaned
    } else {
        cleaned.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
    }
}
