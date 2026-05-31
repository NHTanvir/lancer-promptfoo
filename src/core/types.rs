//! Core domain model for lancer-promptfoo.
//!
//! These types are the GUI-agnostic contract shared by the eval engine, the
//! red-team engine, every provider/plugin/strategy/assertion, the reporters,
//! the CLI and the GUI. Nothing here depends on `egui`, `clap`, or any
//! frontend — keeping `core` reusable from any frontend (parity mandate).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

/// Finding severity. Ordering: `Critical < High < Medium < Low < Informational`
/// so that `severity <= threshold` means "at least as severe as threshold"
/// (a CI gate of `--fail-on high` triggers on Critical and High).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    #[serde(alias = "info")]
    Informational,
}

impl Severity {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "critical" | "crit" => Severity::Critical,
            "high" => Severity::High,
            "medium" | "moderate" | "med" => Severity::Medium,
            "low" => Severity::Low,
            _ => Severity::Informational,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
            Severity::Low => "LOW",
            Severity::Informational => "INFO",
        }
    }

    /// True if `self` is at least as severe as `threshold`.
    pub fn is_at_least(&self, threshold: &Severity) -> bool {
        self <= threshold
    }

    pub const ALL: [Severity; 5] = [
        Severity::Critical,
        Severity::High,
        Severity::Medium,
        Severity::Low,
        Severity::Informational,
    ];
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Informational
    }
}

// ---------------------------------------------------------------------------
// Modality (text / image / audio / video / embedding) — 2026 multimodal targets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Modality {
    Text,
    Image,
    Audio,
    Video,
    Embedding,
}

impl Default for Modality {
    fn default() -> Self {
        Modality::Text
    }
}

// ---------------------------------------------------------------------------
// Token usage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(default)]
    pub prompt: u64,
    #[serde(default)]
    pub completion: u64,
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub cached: u64,
}

impl TokenUsage {
    pub fn add(&mut self, other: &TokenUsage) {
        self.prompt += other.prompt;
        self.completion += other.completion;
        self.total += other.total;
        self.cached += other.cached;
    }
}

// ---------------------------------------------------------------------------
// Provider response
// ---------------------------------------------------------------------------

/// What a provider returns from a single call. Mirrors upstream
/// `ProviderResponse = { output, tokenUsage?, cached?, error?, raw?, metadata? }`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub cached: bool,
    #[serde(default)]
    pub token_usage: TokenUsage,
    #[serde(default)]
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ProviderResponse {
    pub fn text(output: impl Into<String>) -> Self {
        ProviderResponse {
            output: output.into(),
            ..Default::default()
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        ProviderResponse {
            error: Some(msg.into()),
            ..Default::default()
        }
    }

    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

// ---------------------------------------------------------------------------
// Call context — per-call options threaded into a provider
// ---------------------------------------------------------------------------

/// Per-call context passed to `Provider::call`. Mirrors upstream
/// `CallApiContextParams` (vars + prior conversation + free-form options).
#[derive(Debug, Clone, Default)]
pub struct CallContext {
    /// Template variables in scope for this call.
    pub vars: HashMap<String, serde_json::Value>,
    /// Prior conversation turns (role, content) for multi-turn strategies.
    pub history: Vec<(String, String)>,
    /// Provider-specific options (max_tokens, temperature, headers…).
    pub options: HashMap<String, serde_json::Value>,
    /// Optional config-dir path for cache lookups.
    pub purpose: Option<String>,
}

impl CallContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_var(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.vars.insert(key.into(), value);
        self
    }
}

// ---------------------------------------------------------------------------
// Test case (an eval row or a red-team probe)
// ---------------------------------------------------------------------------

/// A single test case: the rendered prompt vars + the assertions to run + the
/// red-team provenance (plugin/strategy/severity) when this is an attack probe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestCase {
    /// The actual prompt text delivered to the target (post-strategy-transform).
    pub prompt: String,
    /// Template variables (for `{{var}}` substitution / multi-var prompts).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub vars: HashMap<String, serde_json::Value>,
    /// Assertions to evaluate against the response.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<AssertionConfig>,
    /// Free-text description / label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Red-team provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
    /// Grader rubric (model-graded red-team scoring).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grader_rubric: Option<String>,
    /// Whether this case is multi-turn (crescendo/goat/hydra).
    #[serde(default)]
    pub multi_turn: bool,
    /// Arbitrary metadata carried through to the finding.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TestCase {
    pub fn new(prompt: impl Into<String>) -> Self {
        TestCase {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    pub fn with_plugin(mut self, plugin: impl Into<String>) -> Self {
        self.plugin = Some(plugin.into());
        self
    }

    pub fn with_strategy(mut self, strategy: impl Into<String>) -> Self {
        self.strategy = Some(strategy.into());
        self
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }
}

// ---------------------------------------------------------------------------
// Assertion config + result
// ---------------------------------------------------------------------------

/// A configured assertion as it appears in `promptfooconfig.yaml`:
/// `{ type: "contains", value: "foo", threshold: 0.7, metric: "...", weight: 1 }`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssertionConfig {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionOutcome {
    pub kind: String,
    pub pass: bool,
    pub score: f64,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
}

// ---------------------------------------------------------------------------
// Vulnerability / Finding (a graded test result)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub severity: Severity,
    pub plugin: String,
    pub strategy: Option<String>,
    pub title: String,
    pub description: String,
    pub attack_prompt: String,
    pub model_response: String,
    /// `true` = the target behaved safely (refused / no vuln). `false` = a finding.
    pub pass: bool,
    pub score: f64,
    pub timestamp: DateTime<Utc>,
    pub provider: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<AssertionOutcome>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Vulnerability {
    pub fn is_finding(&self) -> bool {
        !self.pass
    }

    /// Stable dedup key: plugin + strategy + a hash-free attack signature.
    pub fn dedup_key(&self) -> String {
        format!(
            "{}::{}",
            self.plugin,
            self.strategy.as_deref().unwrap_or("-")
        )
    }
}

// ---------------------------------------------------------------------------
// Report summary
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReportSummary {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub informational: usize,
    pub total_tests: usize,
    pub total_pass: usize,
    pub pass_rate: f64,
    #[serde(default)]
    pub token_usage: TokenUsage,
}

impl ReportSummary {
    pub fn from_vulns(vulns: &[Vulnerability]) -> Self {
        let total = vulns.len();
        let passed = vulns.iter().filter(|v| v.pass).count();
        let mut s = ReportSummary {
            total_tests: total,
            total_pass: passed,
            pass_rate: if total > 0 {
                passed as f64 / total as f64
            } else {
                1.0
            },
            ..Default::default()
        };
        for v in vulns.iter().filter(|v| !v.pass) {
            match v.severity {
                Severity::Critical => s.critical += 1,
                Severity::High => s.high += 1,
                Severity::Medium => s.medium += 1,
                Severity::Low => s.low += 1,
                Severity::Informational => s.informational += 1,
            }
        }
        s
    }

    pub fn findings_at_or_above(&self, threshold: &Severity) -> usize {
        match threshold {
            Severity::Critical => self.critical,
            Severity::High => self.critical + self.high,
            Severity::Medium => self.critical + self.high + self.medium,
            Severity::Low => self.critical + self.high + self.medium + self.low,
            Severity::Informational => {
                self.critical + self.high + self.medium + self.low + self.informational
            }
        }
    }

    pub fn total_findings(&self) -> usize {
        self.critical + self.high + self.medium + self.low + self.informational
    }
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedteamReport {
    pub target: String,
    pub timestamp: DateTime<Utc>,
    pub vulnerabilities: Vec<Vulnerability>,
    pub summary: ReportSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

impl RedteamReport {
    pub fn new(target: impl Into<String>, vulnerabilities: Vec<Vulnerability>) -> Self {
        let summary = ReportSummary::from_vulns(&vulnerabilities);
        RedteamReport {
            target: target.into(),
            timestamp: Utc::now(),
            summary,
            vulnerabilities,
            config_path: None,
            output_path: None,
            purpose: None,
            tags: Vec::new(),
            author: None,
        }
    }

    pub fn findings(&self) -> Vec<&Vulnerability> {
        self.vulnerabilities.iter().filter(|v| !v.pass).collect()
    }

    pub fn findings_at_or_above(&self, threshold: &Severity) -> Vec<&Vulnerability> {
        self.findings()
            .into_iter()
            .filter(|v| v.severity.is_at_least(threshold))
            .collect()
    }

    pub fn recompute_summary(&mut self) {
        self.summary = ReportSummary::from_vulns(&self.vulnerabilities);
    }
}
