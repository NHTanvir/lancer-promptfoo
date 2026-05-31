//! JUnit XML renderer — for Jenkins / GitLab / Azure DevOps test-result gating.
//! Each test case is a probe; a finding (`pass == false`) becomes a JUnit failure.

use crate::core::report::xml::xml_escape;
use crate::core::types::RedteamReport;
use std::fmt::Write as _;

pub fn render(report: &RedteamReport) -> String {
    let mut out = String::new();
    let total = report.vulnerabilities.len();
    let failures = report.summary.total_findings();
    let _ = writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let _ = writeln!(
        out,
        "<testsuites name=\"lancer-promptfoo\" tests=\"{}\" failures=\"{}\">",
        total, failures
    );
    let _ = writeln!(
        out,
        "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" timestamp=\"{}\">",
        xml_escape(&report.target),
        total,
        failures,
        report.timestamp.to_rfc3339()
    );
    for v in &report.vulnerabilities {
        let classname = match &v.strategy {
            Some(s) => format!("{}.{}", v.plugin, s),
            None => v.plugin.clone(),
        };
        let _ = writeln!(
            out,
            "    <testcase classname=\"{}\" name=\"{}\">",
            xml_escape(&classname),
            xml_escape(&v.title)
        );
        if !v.pass {
            let _ = writeln!(
                out,
                "      <failure type=\"{}\" message=\"{}\">{}</failure>",
                v.severity.label(),
                xml_escape(&v.description),
                xml_escape(&format!(
                    "Attack: {}\nResponse: {}",
                    v.attack_prompt, v.model_response
                ))
            );
        }
        let _ = writeln!(out, "    </testcase>");
    }
    let _ = writeln!(out, "  </testsuite>");
    let _ = writeln!(out, "</testsuites>");
    out
}
