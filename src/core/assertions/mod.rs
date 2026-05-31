//! Assertion registry (~45 assertion types) + deterministic dispatch.
//!
//! The catalog is the const table in [`registry_data`]. Deterministic
//! assertions (equals, contains, regex, is-json, levenshtein, rouge/bleu/f1…)
//! are evaluated natively by [`deterministic`]. Model-graded ones (llm-rubric,
//! factuality, g-eval, pi…) are routed to `core::traits::Grader`. Custom ones
//! (javascript/python/ruby/webhook) shell out via the script transport.

pub mod deterministic;
pub mod registry_data;

use crate::core::traits::{AssertionKind, AssertionSpec};
use crate::core::types::{AssertionConfig, AssertionOutcome};

pub struct AssertionRegistry {
    specs: Vec<AssertionSpec>,
}

impl AssertionRegistry {
    pub fn new() -> Self {
        AssertionRegistry {
            specs: registry_data::all_assertion_specs(),
        }
    }

    pub fn specs(&self) -> &[AssertionSpec] {
        &self.specs
    }

    pub fn len(&self) -> usize {
        self.specs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    pub fn ids(&self) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = self.specs.iter().map(|s| s.id).collect();
        ids.sort_unstable();
        ids
    }

    pub fn resolve(&self, kind: &str) -> Option<&AssertionSpec> {
        let base = kind.trim_start_matches("not-");
        self.specs
            .iter()
            .find(|s| s.id == base || s.aliases.contains(&base))
    }

    pub fn kind_of(&self, kind: &str) -> Option<AssertionKind> {
        self.resolve(kind).map(|s| s.kind)
    }

    /// Evaluate an assertion. Deterministic ones are computed here; model-graded
    /// / custom ones return `None` (caller routes them to the Grader/script).
    /// Honors the `not-` prefix (inverts the outcome).
    pub fn evaluate(
        &self,
        kind: &str,
        output: &str,
        cfg: &AssertionConfig,
        latency_ms: u64,
        cost: f64,
    ) -> Option<AssertionOutcome> {
        let inverted = kind.starts_with("not-");
        let base = kind.trim_start_matches("not-");
        let mut base_cfg = cfg.clone();
        base_cfg.kind = base.to_string();
        let result = deterministic::evaluate(output, &base_cfg, latency_ms, cost)?;
        if inverted {
            Some(AssertionOutcome {
                kind: kind.to_string(),
                pass: !result.pass,
                score: 1.0 - result.score,
                reason: format!("(negated) {}", result.reason),
                metric: result.metric,
            })
        } else {
            Some(result)
        }
    }
}

impl Default for AssertionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
