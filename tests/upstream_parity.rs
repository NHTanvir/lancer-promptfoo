//! Upstream parity matrix (see `tests/upstream_parity/`).
//!
//! Fails CI if a documented upstream flag is unsupported, a catalog shrinks
//! below its parity floor, an output format diverges, or a legacy strategy
//! alias stops resolving. These are the guard-rails for the parity mandate.

use clap::Parser;
use lancer_promptfoo::cli::Cli;
use lancer_promptfoo::core;

// ---------------------------------------------------------------------------
// Catalog floors — none of these may regress.
// ---------------------------------------------------------------------------

#[test]
fn catalog_floors() {
    let c = core::catalog_counts();
    assert!(c.providers >= 50, "providers regressed: {}", c.providers);
    assert!(c.plugins >= 150, "plugins regressed: {}", c.plugins);
    assert!(c.strategies >= 25, "strategies regressed: {}", c.strategies);
    assert!(c.assertions >= 40, "assertions regressed: {}", c.assertions);
    assert!(c.presets >= 10, "presets regressed: {}", c.presets);
    assert!(c.collections >= 5, "collections regressed: {}", c.collections);
}

// ---------------------------------------------------------------------------
// Every documented CLI subcommand + flag must parse.
// ---------------------------------------------------------------------------

fn parses(args: &[&str]) {
    let mut full = vec!["lancer-promptfoo"];
    full.extend_from_slice(args);
    assert!(
        Cli::try_parse_from(&full).is_ok(),
        "flags failed to parse: {:?}",
        args
    );
}

#[test]
fn eval_flags_parse() {
    parses(&["eval", "--config", "x.yaml", "-o", "out.json", "-o", "out.sarif"]);
    parses(&["eval", "--max-concurrency", "8", "--delay", "100", "--repeat", "2"]);
    parses(&["eval", "--watch", "--resume", "--retry-errors", "--suggest-prompts"]);
    parses(&["eval", "--filter-first-n", "10", "--filter-range", "#10-50"]);
    parses(&["eval", "--filter-pattern", "foo", "--filter-metadata", "k=v"]);
    parses(&["eval", "--filter-prompts", "p", "--filter-providers", "openai"]);
    parses(&["eval", "--tag", "bb-2026", "--tag", "staging", "--author", "jane@acme.com"]);
    parses(&["eval", "--share", "--no-cache", "--csv-metrics", "latency,cost"]);
    parses(&["eval", "--no-share", "--grader", "anthropic:claude-sonnet-4-6"]);
}

#[test]
fn redteam_flags_parse() {
    parses(&["redteam", "run", "--config", "c.yaml", "--no-progress-bar"]);
    parses(&["redteam", "run", "--max-concurrency", "4", "--no-remote", "--num-tests", "10"]);
    parses(&["redteam", "run", "--baseline", "base.json", "--fail-on", "high"]);
    parses(&["redteam", "run", "--tag", "bb-2026", "--share", "--plugins", "owasp:llm,pii:direct"]);
    parses(&["redteam", "run", "--strategies", "base64,crescendo", "--offline"]);
    parses(&["redteam", "init", "--target", "https://x.com", "--preset", "coding-agent"]);
    parses(&["redteam", "setup", "--target", "https://x.com"]);
    parses(&["redteam", "plugins", "--filter", "coding-agent"]);
    parses(&["redteam", "strategies"]);
    parses(&["redteam", "generate", "--config", "c.yaml"]);
}

#[test]
fn other_subcommands_parse() {
    parses(&["optimize", "--config", "c.yaml"]);
    parses(&["code-scans", "run", ".", "-o", "scan.sarif"]);
    parses(&["scan-model", "model.pkl", "--model-audit-scanners", "protectai,modelscan"]);
    parses(&["mcp"]);
    parses(&["validate", "--config", "c.yaml"]);
    parses(&["logs"]);
    parses(&["delete", "eval-abc"]);
    parses(&["retry", "eval-abc"]);
    parses(&["show", "eval-abc"]);
    parses(&["auth", "login", "--api-key", "k"]);
    parses(&["auth", "logout"]);
    parses(&["auth", "whoami"]);
    parses(&["auth", "teams", "--switch", "acme"]);
    parses(&["auth", "can-create-targets"]);
    parses(&["config", "set", "email", "jane@acme.com"]);
    parses(&["feedback", "great tool"]);
    parses(&["debug"]);
    parses(&["view", "--port", "15500", "--legacy"]);
    parses(&["share", "--team", "acme"]);
    parses(&["import-openai-evals", "eval-123"]);
    parses(&["list", "evals"]);
    parses(&["export", "eval-abc", "-o", "out.json"]);
    parses(&["init", "--target", "https://x.com"]);
    parses(&["self-test"]);
    parses(&["serve", "--target", "https://x.com"]);
    parses(&["report", "report.json", "--fail-on", "critical"]);
}

// ---------------------------------------------------------------------------
// Legacy strategy aliases must keep resolving (old YAML compatibility).
// ---------------------------------------------------------------------------

#[test]
fn legacy_strategy_aliases_resolve() {
    let reg = core::strategies::StrategyRegistry::new();
    for legacy in ["jailbreak:meta", "jailbreak:hydra", "camelCase", "morse", "emoji", "composite"] {
        assert!(reg.resolve(legacy).is_some(), "legacy alias '{}' lost", legacy);
    }
    // Canonical ids resolve too.
    for canonical in ["meta", "hydra", "base64", "crescendo", "goat"] {
        assert!(reg.resolve(canonical).is_some(), "strategy '{}' missing", canonical);
    }
}

// ---------------------------------------------------------------------------
// Framework presets + collections expand to real plugin ids.
// ---------------------------------------------------------------------------

#[test]
fn preset_and_collection_expansion() {
    let reg = core::plugins::PluginRegistry::new();
    let expanded = reg.expand(&[
        "owasp:llm".to_string(),
        "pii".to_string(),
        "coding-agent".to_string(),
    ]);
    assert!(!expanded.is_empty(), "expansion produced nothing");
    // Every expanded id must be a real plugin.
    for id in &expanded {
        assert!(reg.resolve(id).is_some(), "expanded id '{}' is not a real plugin", id);
    }
}

// ---------------------------------------------------------------------------
// Encoders — known vectors.
// ---------------------------------------------------------------------------

#[test]
fn encoder_vectors() {
    use core::strategies::encoders::*;
    assert_eq!(base64_encode("foobar"), "Zm9vYmFy");
    assert_eq!(hex_encode("AB"), "4142");
    assert_eq!(rot13("Hello"), "Uryyb");
    assert_eq!(morse_encode("SOS"), "... --- ...");
}

// ---------------------------------------------------------------------------
// All output formats render an offline report without error.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn all_output_formats_render() {
    use core::config::{PluginEntry, PromptfooConfig, ProviderConfig, RedteamSection};
    use core::engine::RunOptions;
    use core::report::OutputFormat;

    let mut cfg = PromptfooConfig::default();
    cfg.targets = vec![ProviderConfig::Id("echo".into())];
    cfg.redteam = Some(RedteamSection {
        purpose: Some("parity test".into()),
        num_tests: Some(1),
        plugins: vec![PluginEntry::Id("sql-injection".into())],
        ..Default::default()
    });
    let opts = RunOptions { offline: true, cache_enabled: false, ..Default::default() };
    let report = core::redteam::run(&cfg, opts).await.expect("redteam run");

    for fmt in [
        OutputFormat::Json,
        OutputFormat::Yaml,
        OutputFormat::Csv,
        OutputFormat::Html,
        OutputFormat::Xml,
        OutputFormat::Sarif,
        OutputFormat::Junit,
        OutputFormat::Markdown,
        OutputFormat::Text,
    ] {
        let body = core::report::render(&report, fmt).expect("render");
        assert!(!body.is_empty(), "format {:?} produced empty output", fmt);
    }
}

// ---------------------------------------------------------------------------
// Full self-test matrix must pass.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_test_matrix_passes() {
    let report = core::selftest::run().await;
    assert!(
        report.all_passed(),
        "self-test failures: {:?}",
        report
            .checks
            .iter()
            .filter(|c| !c.pass)
            .map(|c| format!("{}: {}", c.group, c.detail))
            .collect::<Vec<_>>()
    );
}
