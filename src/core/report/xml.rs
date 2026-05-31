//! Generic XML renderer for the report.

use crate::core::types::RedteamReport;
use std::fmt::Write as _;

pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn render(report: &RedteamReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let _ = writeln!(
        out,
        "<redteamReport target=\"{}\" timestamp=\"{}\">",
        xml_escape(&report.target),
        report.timestamp.to_rfc3339()
    );
    let s = &report.summary;
    let _ = writeln!(
        out,
        "  <summary totalTests=\"{}\" passRate=\"{:.4}\" critical=\"{}\" high=\"{}\" medium=\"{}\" low=\"{}\" info=\"{}\"/>",
        s.total_tests, s.pass_rate, s.critical, s.high, s.medium, s.low, s.informational
    );
    let _ = writeln!(out, "  <vulnerabilities>");
    for v in &report.vulnerabilities {
        let _ = writeln!(
            out,
            "    <vulnerability id=\"{}\" severity=\"{}\" pass=\"{}\" score=\"{:.4}\">",
            xml_escape(&v.id),
            v.severity.label(),
            v.pass,
            v.score
        );
        let _ = writeln!(out, "      <plugin>{}</plugin>", xml_escape(&v.plugin));
        if let Some(st) = &v.strategy {
            let _ = writeln!(out, "      <strategy>{}</strategy>", xml_escape(st));
        }
        let _ = writeln!(out, "      <title>{}</title>", xml_escape(&v.title));
        let _ = writeln!(out, "      <description>{}</description>", xml_escape(&v.description));
        let _ = writeln!(out, "      <attackPrompt>{}</attackPrompt>", xml_escape(&v.attack_prompt));
        let _ = writeln!(out, "      <modelResponse>{}</modelResponse>", xml_escape(&v.model_response));
        let _ = writeln!(out, "    </vulnerability>");
    }
    let _ = writeln!(out, "  </vulnerabilities>");
    let _ = writeln!(out, "</redteamReport>");
    out
}
