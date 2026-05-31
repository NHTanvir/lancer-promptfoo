//! Provider registry + live transports.
//!
//! The exhaustive *catalog* of provider ids (OpenAI, Anthropic, Bedrock, all
//! the 2026 gateways/inference hosts, multimodal targets, script/MCP/browser
//! shims…) lives as a const table in [`registry_data`]. This module wires each
//! registered spec to a concrete transport so it can actually issue calls.

pub mod registry_data;

use crate::core::traits::{Provider, ProviderKind, ProviderSpec};
use crate::core::types::{CallContext, Modality, ProviderResponse, TokenUsage};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Lookup table over every registered provider spec, with alias resolution.
pub struct ProviderRegistry {
    specs: Vec<ProviderSpec>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        ProviderRegistry {
            specs: registry_data::all_provider_specs(),
        }
    }

    pub fn specs(&self) -> &[ProviderSpec] {
        &self.specs
    }

    pub fn len(&self) -> usize {
        self.specs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    /// Resolve a provider id or alias to its canonical spec. A target id like
    /// `openai:gpt-5.5` resolves on its `openai` prefix.
    pub fn resolve(&self, id: &str) -> Option<&ProviderSpec> {
        let prefix = id.split(':').next().unwrap_or(id);
        self.specs.iter().find(|s| {
            s.id == id
                || s.id == prefix
                || s.aliases.contains(&id)
                || s.aliases.contains(&prefix)
        })
    }

    pub fn contains(&self, id: &str) -> bool {
        self.resolve(id).is_some()
    }

    /// Build a live provider from a target id and its config block. Unknown ids
    /// are still accepted as a generic HTTP target if a `url` is present.
    pub fn build(&self, id: &str, config: &serde_json::Value) -> Result<Arc<dyn Provider>> {
        let spec = self.resolve(id);
        let kind = spec.map(|s| s.kind);

        match kind {
            Some(ProviderKind::Http) | Some(ProviderKind::Webhook) | Some(ProviderKind::Gateway) => {
                build_http(id, config)
            }
            Some(ProviderKind::Script) => Ok(Arc::new(ScriptProvider::from_config(id, config)?)),
            Some(ProviderKind::Chat) | Some(ProviderKind::Image) | Some(ProviderKind::Audio)
            | Some(ProviderKind::Video) | Some(ProviderKind::Embedding) => {
                build_chat(id, config)
            }
            Some(ProviderKind::WebSocket) => Ok(Arc::new(WebSocketProvider::from_config(id, config))),
            Some(ProviderKind::Mcp) | Some(ProviderKind::Browser) => {
                // Registered, but live calls require the dedicated subsystem
                // (`core::mcp` / a browser driver). Surface a clear error rather
                // than silently dropping the target.
                Ok(Arc::new(UnwiredProvider::new(id, spec.map(|s| s.kind))))
            }
            None => {
                // Not in the catalog — accept as a raw HTTP target if a url exists.
                if config.get("url").is_some() {
                    build_http(id, config)
                } else {
                    Err(anyhow!(
                        "Unknown provider id '{}' and no `url` provided for a generic HTTP target.",
                        id
                    ))
                }
            }
        }
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Base URL / API key resolution for OpenAI-compatible & Anthropic providers
// ---------------------------------------------------------------------------

/// Map a provider prefix to its OpenAI-compatible chat base URL (where it has
/// one). Anthropic and the genuinely-bespoke ones are special-cased in `call`.
fn openai_compatible_base(prefix: &str) -> Option<&'static str> {
    Some(match prefix {
        "openai" => "https://api.openai.com/v1",
        "groq" => "https://api.groq.com/openai/v1",
        "openrouter" => "https://openrouter.ai/api/v1",
        "together" => "https://api.together.xyz/v1",
        "deepseek" => "https://api.deepseek.com/v1",
        "mistral" => "https://api.mistral.ai/v1",
        "xai" => "https://api.x.ai/v1",
        "perplexity" => "https://api.perplexity.ai",
        "fireworks" => "https://api.fireworks.ai/inference/v1",
        "cerebras" => "https://api.cerebras.ai/v1",
        "hyperbolic" => "https://api.hyperbolic.xyz/v1",
        "nvidia-nim" => "https://integrate.api.nvidia.com/v1",
        "sambanova" => "https://api.sambanova.ai/v1",
        "novita" => "https://api.novita.ai/v3/openai",
        "ollama" => "http://localhost:11434/v1",
        "localai" => "http://localhost:8080/v1",
        "vllm" => "http://localhost:8000/v1",
        "llama-cpp" => "http://localhost:8080/v1",
        _ => return None,
    })
}

fn env_key_for(prefix: &str) -> &'static str {
    match prefix {
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "groq" => "GROQ_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        "together" => "TOGETHER_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "cohere" => "COHERE_API_KEY",
        "xai" => "XAI_API_KEY",
        "perplexity" => "PERPLEXITY_API_KEY",
        "fireworks" => "FIREWORKS_API_KEY",
        "cerebras" => "CEREBRAS_API_KEY",
        "google" | "vertex" | "gemini" => "GOOGLE_API_KEY",
        _ => "OPENAI_API_KEY",
    }
}

fn build_chat(id: &str, config: &serde_json::Value) -> Result<Arc<dyn Provider>> {
    let prefix = id.split(':').next().unwrap_or(id).to_string();
    let model = id.splitn(2, ':').nth(1).unwrap_or("default").to_string();
    if prefix == "anthropic" {
        return Ok(Arc::new(AnthropicProvider::new(id, model, config)));
    }
    if let Some(base) = openai_compatible_base(&prefix) {
        return Ok(Arc::new(OpenAiCompatProvider::new(id, base, model, &prefix, config)));
    }
    // Catalogued but no built-in REST transport yet → register, error on call.
    Ok(Arc::new(UnwiredProvider::new(id, Some(ProviderKind::Chat))))
}

fn build_http(id: &str, config: &serde_json::Value) -> Result<Arc<dyn Provider>> {
    let url = config
        .get("url")
        .and_then(|u| u.as_str())
        .ok_or_else(|| anyhow!("HTTP provider '{}' requires a `url` in config", id))?
        .to_string();
    Ok(Arc::new(HttpProvider::new(id, url, config)))
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("reqwest client")
}

// ---------------------------------------------------------------------------
// EchoProvider — deterministic, offline; used by --self-test and tests.
// ---------------------------------------------------------------------------

pub struct EchoProvider {
    id: String,
}

impl EchoProvider {
    pub fn new(id: impl Into<String>) -> Self {
        EchoProvider { id: id.into() }
    }
}

#[async_trait]
impl Provider for EchoProvider {
    fn id(&self) -> &str {
        &self.id
    }
    async fn call(&self, prompt: &str, _ctx: &CallContext) -> Result<ProviderResponse> {
        Ok(ProviderResponse::text(prompt.to_string()))
    }
}

// ---------------------------------------------------------------------------
// OpenAI-compatible chat provider
// ---------------------------------------------------------------------------

pub struct OpenAiCompatProvider {
    id: String,
    base_url: String,
    model: String,
    api_key: Option<String>,
    extra_headers: HashMap<String, String>,
    max_tokens: u64,
    temperature: f64,
}

impl OpenAiCompatProvider {
    pub fn new(
        id: &str,
        base_url: &str,
        model: String,
        prefix: &str,
        config: &serde_json::Value,
    ) -> Self {
        let cfg = config.get("config").unwrap_or(config);
        let api_key = cfg
            .get("apiKey")
            .and_then(|k| k.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var(env_key_for(prefix)).ok());
        let mut extra_headers = HashMap::new();
        if let Some(h) = cfg.get("extra_headers").and_then(|h| h.as_object()) {
            for (k, v) in h {
                if let Some(vs) = v.as_str() {
                    extra_headers.insert(k.clone(), vs.to_string());
                }
            }
        }
        OpenAiCompatProvider {
            id: id.to_string(),
            base_url: base_url.to_string(),
            model,
            api_key,
            extra_headers,
            max_tokens: cfg.get("max_tokens").and_then(|m| m.as_u64()).unwrap_or(1024),
            temperature: cfg.get("temperature").and_then(|t| t.as_f64()).unwrap_or(0.0),
        }
    }
}

#[async_trait]
impl Provider for OpenAiCompatProvider {
    fn id(&self) -> &str {
        &self.id
    }

    async fn call(&self, prompt: &str, ctx: &CallContext) -> Result<ProviderResponse> {
        let start = Instant::now();
        let mut messages: Vec<serde_json::Value> = ctx
            .history
            .iter()
            .map(|(role, content)| serde_json::json!({"role": role, "content": content}))
            .collect();
        messages.push(serde_json::json!({"role": "user", "content": prompt}));

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
        });

        let mut req = client()
            .post(format!("{}/chat/completions", self.base_url))
            .json(&body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        for (k, v) in &self.extra_headers {
            req = req.header(k, v);
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => return Ok(ProviderResponse::error(format!("request failed: {}", e))),
        };
        let status = resp.status();
        let json: serde_json::Value = match resp.json().await {
            Ok(j) => j,
            Err(e) => return Ok(ProviderResponse::error(format!("bad JSON ({}): {}", status, e))),
        };
        let output = json
            .pointer("/choices/0/message/content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let usage = json.get("usage");
        let token_usage = TokenUsage {
            prompt: usage.and_then(|u| u.get("prompt_tokens")).and_then(|v| v.as_u64()).unwrap_or(0),
            completion: usage.and_then(|u| u.get("completion_tokens")).and_then(|v| v.as_u64()).unwrap_or(0),
            total: usage.and_then(|u| u.get("total_tokens")).and_then(|v| v.as_u64()).unwrap_or(0),
            cached: 0,
        };
        Ok(ProviderResponse {
            output,
            error: if status.is_success() { None } else { Some(format!("HTTP {}", status)) },
            cached: false,
            token_usage,
            latency_ms: start.elapsed().as_millis() as u64,
            cost: None,
            raw: Some(json),
            metadata: HashMap::new(),
        })
    }
}

// ---------------------------------------------------------------------------
// Anthropic Messages API provider
// ---------------------------------------------------------------------------

pub struct AnthropicProvider {
    id: String,
    model: String,
    api_key: Option<String>,
    extra_headers: HashMap<String, String>,
    max_tokens: u64,
    temperature: f64,
}

impl AnthropicProvider {
    pub fn new(id: &str, model: String, config: &serde_json::Value) -> Self {
        let cfg = config.get("config").unwrap_or(config);
        let api_key = cfg
            .get("apiKey")
            .and_then(|k| k.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());
        let mut extra_headers = HashMap::new();
        if let Some(h) = cfg.get("extra_headers").and_then(|h| h.as_object()) {
            for (k, v) in h {
                if let Some(vs) = v.as_str() {
                    extra_headers.insert(k.clone(), vs.to_string());
                }
            }
        }
        AnthropicProvider {
            id: id.to_string(),
            model,
            api_key,
            extra_headers,
            max_tokens: cfg.get("max_tokens").and_then(|m| m.as_u64()).unwrap_or(2048),
            temperature: cfg.get("temperature").and_then(|t| t.as_f64()).unwrap_or(0.0),
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn id(&self) -> &str {
        &self.id
    }

    async fn call(&self, prompt: &str, ctx: &CallContext) -> Result<ProviderResponse> {
        let start = Instant::now();
        let mut messages: Vec<serde_json::Value> = ctx
            .history
            .iter()
            .map(|(role, content)| serde_json::json!({"role": role, "content": content}))
            .collect();
        messages.push(serde_json::json!({"role": "user", "content": prompt}));

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
        });

        let mut req = client()
            .post("https://api.anthropic.com/v1/messages")
            .header("anthropic-version", "2023-06-01")
            .json(&body);
        if let Some(key) = &self.api_key {
            req = req.header("x-api-key", key);
        }
        for (k, v) in &self.extra_headers {
            req = req.header(k, v);
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => return Ok(ProviderResponse::error(format!("request failed: {}", e))),
        };
        let status = resp.status();
        let json: serde_json::Value = match resp.json().await {
            Ok(j) => j,
            Err(e) => return Ok(ProviderResponse::error(format!("bad JSON ({}): {}", status, e))),
        };
        let output = json
            .pointer("/content/0/text")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let usage = json.get("usage");
        let token_usage = TokenUsage {
            prompt: usage.and_then(|u| u.get("input_tokens")).and_then(|v| v.as_u64()).unwrap_or(0),
            completion: usage.and_then(|u| u.get("output_tokens")).and_then(|v| v.as_u64()).unwrap_or(0),
            total: 0,
            cached: usage.and_then(|u| u.get("cache_read_input_tokens")).and_then(|v| v.as_u64()).unwrap_or(0),
        };
        Ok(ProviderResponse {
            output,
            error: if status.is_success() { None } else { Some(format!("HTTP {}", status)) },
            cached: false,
            token_usage,
            latency_ms: start.elapsed().as_millis() as u64,
            cost: None,
            raw: Some(json),
            metadata: HashMap::new(),
        })
    }
}

// ---------------------------------------------------------------------------
// Generic HTTP custom provider (body template + response transform)
// ---------------------------------------------------------------------------

pub struct HttpProvider {
    id: String,
    url: String,
    method: String,
    headers: HashMap<String, String>,
    body_template: String,
    transform: Option<String>,
}

impl HttpProvider {
    pub fn new(id: &str, url: String, config: &serde_json::Value) -> Self {
        let cfg = config.get("config").unwrap_or(config);
        let method = cfg.get("method").and_then(|m| m.as_str()).unwrap_or("POST").to_string();
        let mut headers = HashMap::new();
        if let Some(h) = cfg.get("headers").and_then(|h| h.as_object()) {
            for (k, v) in h {
                if let Some(vs) = v.as_str() {
                    headers.insert(k.clone(), vs.to_string());
                }
            }
        }
        headers.entry("Content-Type".to_string()).or_insert_with(|| "application/json".to_string());
        let body_template = match cfg.get("body") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => r#"{"prompt": "{{prompt}}"}"#.to_string(),
        };
        let transform = cfg
            .get("transformResponse")
            .or_else(|| cfg.get("responseParser"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());
        HttpProvider {
            id: id.to_string(),
            url,
            method,
            headers,
            body_template,
            transform,
        }
    }

    fn render_body(&self, prompt: &str) -> String {
        // JSON-escape the prompt so it is safe to splice into a JSON body.
        let escaped = serde_json::to_string(prompt).unwrap_or_else(|_| "\"\"".to_string());
        let escaped = escaped.trim_matches('"');
        self.body_template
            .replace("{{prompt}}", escaped)
            .replace("{{ prompt }}", escaped)
    }
}

#[async_trait]
impl Provider for HttpProvider {
    fn id(&self) -> &str {
        &self.id
    }

    async fn call(&self, prompt: &str, _ctx: &CallContext) -> Result<ProviderResponse> {
        let start = Instant::now();
        let body = self.render_body(prompt);
        let method = reqwest::Method::from_bytes(self.method.as_bytes())
            .unwrap_or(reqwest::Method::POST);
        let mut req = client().request(method, &self.url).body(body);
        for (k, v) in &self.headers {
            req = req.header(k, v);
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => return Ok(ProviderResponse::error(format!("request failed: {}", e))),
        };
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        let output = apply_transform(&text, self.transform.as_deref());
        Ok(ProviderResponse {
            output,
            error: if (200..400).contains(&status) { None } else { Some(format!("HTTP {}", status)) },
            cached: false,
            token_usage: TokenUsage::default(),
            latency_ms: start.elapsed().as_millis() as u64,
            cost: None,
            raw: Some(serde_json::Value::String(text)),
            metadata: HashMap::new(),
        })
    }
}

/// Apply a `transformResponse` expression. Supports the common `json.a.b`
/// dotted-path form and a bare passthrough; anything else returns the raw body.
pub fn apply_transform(body: &str, transform: Option<&str>) -> String {
    let Some(t) = transform else {
        return body.to_string();
    };
    let path = t.trim().trim_start_matches("json.").trim_start_matches("json");
    if path.is_empty() {
        return body.to_string();
    }
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return body.to_string();
    };
    let mut cur: &serde_json::Value = &v;
    for seg in path.split('.') {
        if seg.is_empty() {
            continue;
        }
        // support array indices like reply[0]
        let next = if let Some((field, idx)) = parse_index(seg) {
            cur.get(field).and_then(|c| c.get(idx))
        } else {
            cur.get(seg)
        };
        match next {
            Some(n) => cur = n,
            None => return body.to_string(), // path miss → return the raw body
        }
    }
    match cur {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => body.to_string(),
        other => other.to_string(),
    }
}

fn parse_index(seg: &str) -> Option<(&str, usize)> {
    let open = seg.find('[')?;
    let close = seg.find(']')?;
    let field = &seg[..open];
    let idx: usize = seg[open + 1..close].parse().ok()?;
    Some((field, idx))
}

// ---------------------------------------------------------------------------
// Script provider — shells out to a python/node/ruby file
// ---------------------------------------------------------------------------

pub struct ScriptProvider {
    id: String,
    interpreter: String,
    script_path: String,
}

impl ScriptProvider {
    pub fn from_config(id: &str, config: &serde_json::Value) -> Result<Self> {
        let prefix = id.split(':').next().unwrap_or("python");
        let interpreter = match prefix {
            "python" => "python",
            "javascript" | "js" | "node" => "node",
            "ruby" => "ruby",
            other => other,
        }
        .to_string();
        let script_path = id
            .splitn(2, ':')
            .nth(1)
            .map(|s| s.trim_start_matches("file://").to_string())
            .or_else(|| {
                config
                    .get("config")
                    .and_then(|c| c.get("path"))
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| anyhow!("script provider '{}' requires a file path", id))?;
        Ok(ScriptProvider {
            id: id.to_string(),
            interpreter,
            script_path,
        })
    }
}

#[async_trait]
impl Provider for ScriptProvider {
    fn id(&self) -> &str {
        &self.id
    }
    async fn call(&self, prompt: &str, _ctx: &CallContext) -> Result<ProviderResponse> {
        let start = Instant::now();
        let out = tokio::process::Command::new(&self.interpreter)
            .arg(&self.script_path)
            .arg(prompt)
            .output()
            .await;
        match out {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
                Ok(ProviderResponse {
                    output: stdout,
                    error: if o.status.success() {
                        None
                    } else {
                        Some(String::from_utf8_lossy(&o.stderr).to_string())
                    },
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                })
            }
            Err(e) => Ok(ProviderResponse::error(format!("script spawn failed: {}", e))),
        }
    }
}

// ---------------------------------------------------------------------------
// WebSocket provider (minimal — connect, send, recv one frame)
// ---------------------------------------------------------------------------

pub struct WebSocketProvider {
    id: String,
    #[allow(dead_code)]
    url: String,
}

impl WebSocketProvider {
    pub fn from_config(id: &str, config: &serde_json::Value) -> Self {
        let url = config
            .get("config")
            .and_then(|c| c.get("url"))
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        WebSocketProvider {
            id: id.to_string(),
            url,
        }
    }
}

#[async_trait]
impl Provider for WebSocketProvider {
    fn id(&self) -> &str {
        &self.id
    }
    async fn call(&self, _prompt: &str, _ctx: &CallContext) -> Result<ProviderResponse> {
        Ok(ProviderResponse::error(
            "WebSocket transport is registered but requires the tokio-tungstenite feature.",
        ))
    }
}

// ---------------------------------------------------------------------------
// UnwiredProvider — catalogued but no built-in live transport yet
// ---------------------------------------------------------------------------

pub struct UnwiredProvider {
    id: String,
    kind: Option<ProviderKind>,
}

impl UnwiredProvider {
    pub fn new(id: &str, kind: Option<ProviderKind>) -> Self {
        UnwiredProvider {
            id: id.to_string(),
            kind,
        }
    }
}

#[async_trait]
impl Provider for UnwiredProvider {
    fn id(&self) -> &str {
        &self.id
    }
    fn modality(&self) -> Modality {
        match self.kind {
            Some(ProviderKind::Image) => Modality::Image,
            Some(ProviderKind::Audio) => Modality::Audio,
            Some(ProviderKind::Video) => Modality::Video,
            Some(ProviderKind::Embedding) => Modality::Embedding,
            _ => Modality::Text,
        }
    }
    async fn call(&self, _prompt: &str, _ctx: &CallContext) -> Result<ProviderResponse> {
        Ok(ProviderResponse::error(format!(
            "Provider '{}' is registered in the catalog but has no built-in live transport yet \
             (configure it via the generic `http` provider, or use a gateway like litellm/openrouter).",
            self.id
        )))
    }
}
