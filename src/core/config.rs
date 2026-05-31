//! `promptfooconfig.yaml` schema model + loader + generator.
//!
//! Mirrors the upstream config surface: `description`, `providers`/`targets`,
//! `prompts`, `tests`/`defaultTest`, the `redteam` block (plugins, strategies,
//! provider, purpose, numTests…), `evaluateOptions`, `outputPath`, `sharing`,
//! `env`, `tags`, `author`, `notifications`, `policies`. Polymorphic entries
//! (a plugin can be a bare string or `{id, numTests, config}`) are modelled as
//! untagged enums so existing YAML round-trips.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptfooConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Eval providers (alias for the same concept as `targets`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<ProviderConfig>,
    /// Red-team targets (alias for `providers`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<ProviderConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tests: Vec<TestSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_test: Option<TestSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redteam: Option<RedteamSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluate_options: Option<EvaluateOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sharing: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notifications: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policies: Option<String>,
    /// Any unrecognized top-level keys are preserved verbatim.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl PromptfooConfig {
    /// Load from a YAML or JSON file.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read config {}", path.display()))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self> {
        // Try JSON first (a superset of nothing here, but cheap), then YAML.
        if let Ok(c) = serde_json::from_str::<PromptfooConfig>(text) {
            return Ok(c);
        }
        serde_yaml::from_str(text).context("Failed to parse config (not valid YAML or JSON)")
    }

    /// Resolved target list: `targets` if present, else `providers`.
    pub fn resolved_targets(&self) -> &[ProviderConfig] {
        if !self.targets.is_empty() {
            &self.targets
        } else {
            &self.providers
        }
    }

    pub fn is_redteam(&self) -> bool {
        self.redteam.is_some()
    }

    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self).context("Failed to serialize config to YAML")
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.to_yaml()?)
            .with_context(|| format!("Failed to write config to {}", path.display()))
    }
}

// ---------------------------------------------------------------------------
// Provider / target config (string or detailed map)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProviderConfig {
    Id(String),
    Detailed(ProviderConfigDetailed),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfigDetailed {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

impl ProviderConfig {
    pub fn id(&self) -> &str {
        match self {
            ProviderConfig::Id(s) => s,
            ProviderConfig::Detailed(d) => &d.id,
        }
    }

    pub fn label(&self) -> Option<&str> {
        match self {
            ProviderConfig::Id(_) => None,
            ProviderConfig::Detailed(d) => d.label.as_deref(),
        }
    }

    /// The full config block as JSON (`{}` if a bare id).
    pub fn config_json(&self) -> serde_json::Value {
        match self {
            ProviderConfig::Id(_) => serde_json::json!({}),
            ProviderConfig::Detailed(d) => {
                let mut v = serde_json::Map::new();
                v.insert("id".into(), serde_json::Value::String(d.id.clone()));
                if let Some(label) = &d.label {
                    v.insert("label".into(), serde_json::Value::String(label.clone()));
                }
                if let Some(cfg) = &d.config {
                    v.insert("config".into(), cfg.clone());
                }
                serde_json::Value::Object(v)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Red-team section
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedteamSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_tests: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<PluginEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strategies: Vec<StrategyEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inject_var: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginEntry {
    Id(String),
    Detailed(PluginEntryDetailed),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntryDetailed {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_tests: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
}

impl PluginEntry {
    pub fn id(&self) -> &str {
        match self {
            PluginEntry::Id(s) => s,
            PluginEntry::Detailed(d) => &d.id,
        }
    }
    pub fn num_tests(&self) -> Option<usize> {
        match self {
            PluginEntry::Id(_) => None,
            PluginEntry::Detailed(d) => d.num_tests,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StrategyEntry {
    Id(String),
    Detailed(StrategyEntryDetailed),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyEntryDetailed {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

impl StrategyEntry {
    pub fn id(&self) -> &str {
        match self {
            StrategyEntry::Id(s) => s,
            StrategyEntry::Detailed(d) => &d.id,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests + eval options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub vars: HashMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assert: Vec<crate::core::types::AssertionConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrency: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_progress_bar: Option<bool>,
}

// ===========================================================================
// Config generator (used by `init` / `redteam setup`)
// ===========================================================================

/// High-level generator inputs for scaffolding a `promptfooconfig.yaml`.
#[derive(Debug, Clone)]
pub struct GeneratorOptions {
    pub target_url: String,
    pub target_label: String,
    pub purpose: String,
    pub num_tests: usize,
    pub plugins: Vec<String>,
    pub strategies: Vec<String>,
    pub provider: String,
    pub judge_provider: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body_template: String,
    pub response_transform: String,
    pub max_concurrency: usize,
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".into(), "application/json".into());
        GeneratorOptions {
            target_url: "http://localhost:3000/api/chat".into(),
            target_label: "target-app".into(),
            purpose: "AI chatbot assistant for customer support".into(),
            num_tests: 5,
            plugins: web_app_plugins(),
            strategies: recommended_strategies(),
            provider: "anthropic:claude-opus-4-8".into(),
            judge_provider: "anthropic:claude-sonnet-4-6".into(),
            method: "POST".into(),
            headers,
            body_template: r#"{"message": "{{prompt}}"}"#.into(),
            response_transform: "json.reply".into(),
            max_concurrency: 4,
        }
    }
}

pub fn web_app_plugins() -> Vec<String> {
    [
        "owasp:llm",
        "pii:direct",
        "pii:session",
        "sql-injection",
        "ssrf",
        "bola",
        "bfla",
        "rbac",
        "prompt-extraction",
        "system-prompt-override",
        "indirect-prompt-injection",
        "debug-access",
        "excessive-agency",
        "hallucination",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

pub fn coding_agent_plugins() -> Vec<String> {
    [
        "owasp:agentic",
        "coding-agent:core",
        "coding-agent:secret-env-read",
        "coding-agent:secret-file-read",
        "coding-agent:sandbox-read-escape",
        "coding-agent:sandbox-write-escape",
        "coding-agent:steganographic-exfil",
        "coding-agent:repo-prompt-injection",
        "coding-agent:automation-poisoning",
        "indirect-prompt-injection",
        "prompt-extraction",
        "data-exfil",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

pub fn recommended_strategies() -> Vec<String> {
    ["meta", "hydra", "crescendo", "base64", "tree"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl GeneratorOptions {
    pub fn for_web_app(target_url: &str, purpose: &str) -> Self {
        GeneratorOptions {
            target_url: target_url.to_string(),
            purpose: purpose.to_string(),
            plugins: web_app_plugins(),
            strategies: recommended_strategies(),
            ..Default::default()
        }
    }

    pub fn for_coding_agent(target_url: &str, purpose: &str) -> Self {
        GeneratorOptions {
            target_url: target_url.to_string(),
            purpose: purpose.to_string(),
            plugins: coding_agent_plugins(),
            strategies: recommended_strategies(),
            ..Default::default()
        }
    }

    /// Build a full `PromptfooConfig` document from the generator inputs.
    pub fn build(&self) -> PromptfooConfig {
        let mut headers_map = serde_json::Map::new();
        for (k, v) in &self.headers {
            headers_map.insert(k.clone(), serde_json::Value::String(v.clone()));
        }
        let target = ProviderConfig::Detailed(ProviderConfigDetailed {
            id: "http".into(),
            label: Some(self.target_label.clone()),
            config: Some(serde_json::json!({
                "url": self.target_url,
                "method": self.method,
                "headers": headers_map,
                "body": self.body_template,
                "transformResponse": self.response_transform,
            })),
        });

        let redteam = RedteamSection {
            purpose: Some(self.purpose.clone()),
            num_tests: Some(self.num_tests),
            plugins: self.plugins.iter().cloned().map(PluginEntry::Id).collect(),
            strategies: self.strategies.iter().cloned().map(StrategyEntry::Id).collect(),
            provider: Some(ProviderConfig::Detailed(ProviderConfigDetailed {
                id: self.provider.clone(),
                label: None,
                config: Some(serde_json::json!({
                    "max_tokens": 4096,
                    "temperature": 0.8,
                })),
            })),
            ..Default::default()
        };

        let default_test = TestSpec {
            options: Some(serde_json::json!({
                "provider": {
                    "id": self.judge_provider,
                    "config": {
                        "extra_headers": { "anthropic-beta": "prompt-caching-2024-07-31" }
                    }
                }
            })),
            ..Default::default()
        };

        PromptfooConfig {
            description: Some(format!("lancer-promptfoo red-team config for {}", self.target_label)),
            targets: vec![target],
            redteam: Some(redteam),
            default_test: Some(default_test),
            evaluate_options: Some(EvaluateOptions {
                max_concurrency: Some(self.max_concurrency),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}
