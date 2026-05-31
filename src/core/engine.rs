//! Shared engine plumbing for both the eval and red-team runners: registry
//! ownership, provider construction from config, cached provider calls, and
//! concurrency control.

use crate::core::assertions::AssertionRegistry;
use crate::core::cache::Cache;
use crate::core::config::{PromptfooConfig, ProviderConfig, RedteamSection};
use crate::core::plugins::PluginRegistry;
use crate::core::providers::{EchoProvider, ProviderRegistry};
use crate::core::strategies::StrategyRegistry;
use crate::core::traits::Provider;
use crate::core::types::{CallContext, ProviderResponse};
use anyhow::Result;
use std::sync::Arc;

/// Knobs shared by eval + red-team runs (mostly from `evaluateOptions` + CLI).
#[derive(Debug, Clone)]
pub struct RunOptions {
    pub max_concurrency: usize,
    pub repeat: usize,
    pub delay_ms: u64,
    pub cache_enabled: bool,
    pub no_remote: bool,
    /// Force the deterministic offline Echo provider (used by `--self-test`).
    pub offline: bool,
    pub num_tests_override: Option<usize>,
    pub tags: Vec<String>,
    pub author: Option<String>,
}

impl Default for RunOptions {
    fn default() -> Self {
        RunOptions {
            max_concurrency: 4,
            repeat: 1,
            delay_ms: 0,
            cache_enabled: true,
            no_remote: crate::core::env::remote_generation_disabled(),
            offline: false,
            num_tests_override: None,
            tags: Vec::new(),
            author: None,
        }
    }
}

pub struct Engine {
    pub providers: ProviderRegistry,
    pub plugins: PluginRegistry,
    pub strategies: StrategyRegistry,
    pub assertions: AssertionRegistry,
    pub cache: Arc<Cache>,
    pub opts: RunOptions,
}

impl Engine {
    pub fn new(opts: RunOptions) -> Self {
        let cache = if opts.cache_enabled {
            Cache::load(crate::core::env::cache_path(), true)
        } else {
            Cache::disabled()
        };
        Engine {
            providers: ProviderRegistry::new(),
            plugins: PluginRegistry::new(),
            strategies: StrategyRegistry::new(),
            assertions: AssertionRegistry::new(),
            cache: Arc::new(cache),
            opts,
        }
    }

    /// Build a live target provider from a config entry. In offline mode every
    /// target collapses to the deterministic Echo provider.
    pub fn build_target(&self, cfg: &ProviderConfig) -> Result<Arc<dyn Provider>> {
        if self.opts.offline {
            return Ok(Arc::new(EchoProvider::new(cfg.id())));
        }
        self.providers.build(cfg.id(), &cfg.config_json())
    }

    /// Build the judge provider for model-graded scoring (from
    /// `defaultTest.options.provider`), if configured.
    pub fn build_judge(&self, config: &PromptfooConfig) -> Option<Arc<dyn Provider>> {
        if self.opts.offline {
            return Some(Arc::new(EchoProvider::new("judge:echo")));
        }
        let provider_val = config
            .default_test
            .as_ref()
            .and_then(|t| t.options.as_ref())
            .and_then(|o| o.get("provider"))?;
        let (id, cfg) = provider_id_and_config(provider_val)?;
        self.providers.build(&id, &cfg).ok()
    }

    /// Build the attack-generation provider (from `redteam.provider`).
    pub fn build_generator(&self, redteam: &RedteamSection) -> Option<Arc<dyn Provider>> {
        // Offline → no generator at all (TemplatePlugin emits deterministic seed
        // probes). `no_remote` only means "don't call the cloud generation
        // endpoint" — the locally-configured `redteam.provider` is still used.
        if self.opts.offline {
            return None;
        }
        let p = redteam.provider.as_ref()?;
        self.providers.build(p.id(), &p.config_json()).ok()
    }

    /// Call a provider, consulting/populating the cache.
    pub async fn call_cached(
        &self,
        provider: &dyn Provider,
        prompt: &str,
        ctx: &CallContext,
    ) -> ProviderResponse {
        let opts_sig = format!("{:?}", ctx.history);
        let key = Cache::key(provider.id(), prompt, &opts_sig);
        if let Some(hit) = self.cache.get(&key) {
            return hit;
        }
        let resp = match provider.call(prompt, ctx).await {
            Ok(r) => r,
            Err(e) => ProviderResponse::error(e.to_string()),
        };
        if !resp.is_error() {
            self.cache.put(key, resp.clone());
        }
        resp
    }

    pub fn flush_cache(&self) {
        let _ = self.cache.flush();
    }
}

/// Extract `(id, config)` from a provider value that may be a bare string or a
/// `{id, config}` map.
pub fn provider_id_and_config(val: &serde_json::Value) -> Option<(String, serde_json::Value)> {
    match val {
        serde_json::Value::String(s) => Some((s.clone(), serde_json::json!({}))),
        serde_json::Value::Object(_) => {
            let id = val.get("id").and_then(|i| i.as_str())?.to_string();
            Some((id, val.clone()))
        }
        _ => None,
    }
}
