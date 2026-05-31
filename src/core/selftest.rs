//! `--self-test`: run the parity matrix against a bundled offline (echo) target
//! and print pass/fail per upstream feature group. Fully deterministic — no
//! network, no API keys required.

use crate::core::config::{
    EvaluateOptions, GeneratorOptions, PromptfooConfig, ProviderConfig, RedteamSection, TestSpec,
};
use crate::core::engine::RunOptions;
use crate::core::report::OutputFormat;
use crate::core::strategies::encoders;
use crate::core::types::AssertionConfig;

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub group: String,
    pub detail: String,
    pub pass: bool,
}

#[derive(Debug, Default)]
pub struct SelfTestReport {
    pub checks: Vec<CheckResult>,
}

impl SelfTestReport {
    fn add(&mut self, group: &str, detail: impl Into<String>, pass: bool) {
        self.checks.push(CheckResult {
            group: group.to_string(),
            detail: detail.into(),
            pass,
        });
    }

    pub fn passed(&self) -> usize {
        self.checks.iter().filter(|c| c.pass).count()
    }

    pub fn failed(&self) -> usize {
        self.checks.iter().filter(|c| !c.pass).count()
    }

    pub fn all_passed(&self) -> bool {
        self.failed() == 0
    }
}

/// Run the full self-test matrix.
pub async fn run() -> SelfTestReport {
    let mut r = SelfTestReport::default();

    // 1. Catalog population.
    let c = crate::core::catalog_counts();
    r.add("catalog:providers", format!("{} provider specs", c.providers), c.providers >= 50);
    r.add("catalog:plugins", format!("{} plugin specs", c.plugins), c.plugins >= 150);
    r.add("catalog:strategies", format!("{} strategy specs", c.strategies), c.strategies >= 25);
    r.add("catalog:assertions", format!("{} assertion specs", c.assertions), c.assertions >= 40);
    r.add("catalog:presets", format!("{} framework presets", c.presets), c.presets >= 10);
    r.add("catalog:collections", format!("{} collections", c.collections), c.collections >= 5);

    // 2. Deterministic encoders.
    r.add("encoders:base64", "foobar→Zm9vYmFy", encoders::base64_encode("foobar") == "Zm9vYmFy");
    r.add("encoders:rot13", "roundtrip", encoders::rot13(&encoders::rot13("Hello")) == "Hello");
    r.add("encoders:morse", "SOS", encoders::morse_encode("SOS") == "... --- ...");
    r.add("encoders:hex", "AB→4142", encoders::hex_encode("AB") == "4142");

    // 3. Deterministic assertions.
    let pass_contains = crate::core::assertions::deterministic::evaluate(
        "hello world",
        &AssertionConfig { kind: "contains".into(), value: Some("world".into()), ..Default::default() },
        0,
        0.0,
    )
    .map(|o| o.pass)
    .unwrap_or(false);
    r.add("assertions:contains", "matches substring", pass_contains);
    let lev = crate::core::assertions::deterministic::levenshtein("kitten", "sitting");
    r.add("assertions:levenshtein", "kitten/sitting=3", lev == 3);

    // 4. Preset + collection expansion.
    let reg = crate::core::plugins::PluginRegistry::new();
    let expanded = reg.expand(&["owasp:llm".to_string(), "pii".to_string()]);
    r.add("expansion:preset+collection", format!("expanded to {} plugin ids", expanded.len()), !expanded.is_empty());

    // 5. End-to-end offline red-team against the echo target.
    let mut cfg = PromptfooConfig::default();
    cfg.targets = vec![ProviderConfig::Id("echo".to_string())];
    cfg.redteam = Some(RedteamSection {
        purpose: Some("self-test echo target".into()),
        num_tests: Some(2),
        plugins: vec![
            crate::core::config::PluginEntry::Id("sql-injection".into()),
            crate::core::config::PluginEntry::Id("pii:direct".into()),
        ],
        strategies: vec![
            crate::core::config::StrategyEntry::Id("base64".into()),
            crate::core::config::StrategyEntry::Id("crescendo".into()),
        ],
        ..Default::default()
    });
    let opts = RunOptions {
        offline: true,
        cache_enabled: false,
        max_concurrency: 4,
        ..Default::default()
    };
    match crate::core::redteam::run(&cfg, opts).await {
        Ok(report) => {
            r.add("engine:redteam", format!("{} probes graded", report.vulnerabilities.len()), !report.vulnerabilities.is_empty());
            // 6. Every output format renders without error.
            for (label, fmt) in [
                ("json", OutputFormat::Json),
                ("yaml", OutputFormat::Yaml),
                ("csv", OutputFormat::Csv),
                ("html", OutputFormat::Html),
                ("xml", OutputFormat::Xml),
                ("sarif", OutputFormat::Sarif),
                ("junit", OutputFormat::Junit),
                ("markdown", OutputFormat::Markdown),
                ("text", OutputFormat::Text),
            ] {
                let ok = crate::core::report::render(&report, fmt).map(|s| !s.is_empty()).unwrap_or(false);
                r.add("output", format!("format {} renders", label), ok);
            }
        }
        Err(e) => r.add("engine:redteam", format!("run failed: {}", e), false),
    }

    // 7. Eval engine offline.
    let mut eval_cfg = PromptfooConfig::default();
    eval_cfg.providers = vec![ProviderConfig::Id("echo".to_string())];
    eval_cfg.prompts = vec![serde_json::json!("Say {{topic}}")];
    eval_cfg.tests = vec![TestSpec {
        vars: [("topic".to_string(), serde_json::json!("hello"))].into_iter().collect(),
        assert: vec![AssertionConfig { kind: "contains".into(), value: Some("hello".into()), ..Default::default() }],
        ..Default::default()
    }];
    eval_cfg.evaluate_options = Some(EvaluateOptions { max_concurrency: Some(2), ..Default::default() });
    let eval_opts = RunOptions { offline: true, cache_enabled: false, ..Default::default() };
    match crate::core::eval::run(&eval_cfg, eval_opts).await {
        Ok(report) => {
            let passed = report.vulnerabilities.iter().all(|v| v.pass);
            r.add("engine:eval", "echo+contains assertion passes", passed && !report.vulnerabilities.is_empty());
        }
        Err(e) => r.add("engine:eval", format!("eval failed: {}", e), false),
    }

    // 8. Config generator round-trips through the parser.
    let gen = GeneratorOptions::for_web_app("https://example.com/api", "demo");
    let doc = gen.build();
    let yaml = doc.to_yaml().unwrap_or_default();
    let reparsed = PromptfooConfig::parse(&yaml).is_ok();
    r.add("config:roundtrip", "generated YAML re-parses", reparsed && !yaml.is_empty());

    r
}

/// Pretty-print the self-test report to stdout. Returns true if all passed.
pub fn print_report(report: &SelfTestReport) -> bool {
    use colored::Colorize;
    println!("\n  lancer-promptfoo self-test\n  {}", "─".repeat(40));
    for c in &report.checks {
        let mark = if c.pass { "✓".green() } else { "✗".red() };
        println!("  {} {:<28} {}", mark, c.group, c.detail);
    }
    println!("  {}", "─".repeat(40));
    println!(
        "  {} passed, {} failed\n",
        report.passed().to_string().green(),
        if report.failed() == 0 {
            "0".to_string().green()
        } else {
            report.failed().to_string().red()
        }
    );
    report.all_passed()
}
