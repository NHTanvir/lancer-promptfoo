//! Eval engine: render prompts → call providers → run assertions → report.
//!
//! Mirrors upstream `src/evaluator.ts`: for each test, render the prompt with
//! its vars, call every provider, evaluate each assertion (deterministic ones
//! natively, model-graded ones via the judge), persist a graded row.

use crate::core::config::PromptfooConfig;
use crate::core::engine::{Engine, RunOptions};
use crate::core::traits::{AssertionKind, Grader, Provider};
use crate::core::types::{
    AssertionConfig, AssertionOutcome, CallContext, RedteamReport, Severity, Vulnerability,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

struct WorkItem {
    provider: Arc<dyn Provider>,
    prompt_label: String,
    rendered: String,
    description: String,
    vars: HashMap<String, serde_json::Value>,
    assertions: Vec<AssertionConfig>,
}

pub async fn run(config: &PromptfooConfig, opts: RunOptions) -> Result<RedteamReport> {
    let engine = Engine::new(opts);
    let targets = config.resolved_targets();
    if targets.is_empty() {
        return Err(anyhow!("config has no providers/targets to evaluate"));
    }

    let providers: Vec<Arc<dyn Provider>> = targets
        .iter()
        .map(|t| engine.build_target(t))
        .collect::<Result<_>>()?;

    let judge = engine.build_judge(config);

    // Prompt templates (strings or {raw,label} objects). Default to "{{prompt}}".
    let prompts: Vec<(String, String)> = if config.prompts.is_empty() {
        vec![("prompt".to_string(), "{{prompt}}".to_string())]
    } else {
        config
            .prompts
            .iter()
            .enumerate()
            .map(|(i, p)| extract_prompt(p, i))
            .collect()
    };

    // defaultTest assertions are merged into every test.
    let default_asserts: Vec<AssertionConfig> = config
        .default_test
        .as_ref()
        .map(|t| t.assert.clone())
        .unwrap_or_default();

    let tests = if config.tests.is_empty() {
        vec![crate::core::config::TestSpec::default()]
    } else {
        config.tests.clone()
    };

    // Build the full work list (prompt × test × provider), honoring `repeat`.
    let mut work: Vec<WorkItem> = Vec::new();
    for (plabel, ptmpl) in &prompts {
        for test in &tests {
            let mut asserts = default_asserts.clone();
            asserts.extend(test.assert.clone());
            let rendered = render(ptmpl, &test.vars);
            for provider in &providers {
                for _ in 0..engine.opts.repeat.max(1) {
                    work.push(WorkItem {
                        provider: Arc::clone(provider),
                        prompt_label: plabel.clone(),
                        rendered: rendered.clone(),
                        description: test.description.clone().unwrap_or_default(),
                        vars: test.vars.clone(),
                        assertions: asserts.clone(),
                    });
                }
            }
        }
    }

    let concurrency = engine.opts.max_concurrency.max(1);
    let judge_ref = judge.as_deref();
    let engine_ref = &engine;

    let mut rows: Vec<Vulnerability> = stream::iter(work.into_iter())
        .map(|item| async move { eval_one(engine_ref, judge_ref, item).await })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    rows.sort_by(|a, b| a.pass.cmp(&b.pass).then_with(|| a.plugin.cmp(&b.plugin)));
    engine.flush_cache();

    let target_name = targets
        .iter()
        .map(|t| t.id())
        .collect::<Vec<_>>()
        .join(", ");
    let mut report = RedteamReport::new(target_name, rows);
    report.tags = engine.opts.tags.clone();
    report.author = engine.opts.author.clone();
    Ok(report)
}

async fn eval_one(
    engine: &Engine,
    judge: Option<&dyn Provider>,
    item: WorkItem,
) -> Vulnerability {
    let ctx = CallContext {
        vars: item.vars.clone(),
        ..Default::default()
    };
    let resp = engine
        .call_cached(item.provider.as_ref(), &item.rendered, &ctx)
        .await;
    let output = if let Some(e) = &resp.error {
        format!("[error] {}", e)
    } else {
        resp.output.clone()
    };
    let cost = resp.cost.unwrap_or(0.0);

    let mut outcomes: Vec<AssertionOutcome> = Vec::new();
    for a in &item.assertions {
        // Deterministic path.
        if let Some(o) = engine
            .assertions
            .evaluate(&a.kind, &output, a, resp.latency_ms, cost)
        {
            outcomes.push(o);
            continue;
        }
        // Model-graded path (rubric = assertion value string).
        match engine.assertions.kind_of(&a.kind) {
            Some(AssertionKind::ModelGraded) => {
                let rubric = a
                    .value
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or("Grade whether the output satisfies the test intent.")
                    .to_string();
                let grader = Grader::new(rubric).with_threshold(a.threshold.unwrap_or(0.5));
                let g = grader
                    .grade(&item.rendered, &output, judge, &ctx)
                    .await
                    .unwrap_or(crate::core::traits::GradeResult {
                        pass: false,
                        score: 0.0,
                        reason: "grading failed".into(),
                    });
                outcomes.push(AssertionOutcome {
                    kind: a.kind.clone(),
                    pass: g.pass,
                    score: g.score,
                    reason: g.reason,
                    metric: a.metric.clone(),
                });
            }
            _ => {
                // Custom/unknown — record as inconclusive (does not fail the row).
                outcomes.push(AssertionOutcome {
                    kind: a.kind.clone(),
                    pass: true,
                    score: 1.0,
                    reason: format!("assertion '{}' not natively evaluated (skipped)", a.kind),
                    metric: a.metric.clone(),
                });
            }
        }
    }

    let pass = resp.error.is_none() && outcomes.iter().all(|o| o.pass);
    let score = if outcomes.is_empty() {
        if pass {
            1.0
        } else {
            0.0
        }
    } else {
        outcomes.iter().map(|o| o.score).sum::<f64>() / outcomes.len() as f64
    };

    let title = if item.description.is_empty() {
        format!("{} @ {}", item.prompt_label, item.provider.id())
    } else {
        item.description.clone()
    };
    let description = outcomes
        .iter()
        .filter(|o| !o.pass)
        .map(|o| format!("{}: {}", o.kind, o.reason))
        .collect::<Vec<_>>()
        .join("; ");

    Vulnerability {
        id: Uuid::new_v4().to_string(),
        severity: Severity::Informational,
        plugin: item.prompt_label,
        strategy: None,
        title,
        description: if description.is_empty() {
            "All assertions passed.".into()
        } else {
            description
        },
        attack_prompt: item.rendered,
        model_response: output,
        pass,
        score,
        timestamp: Utc::now(),
        provider: item.provider.id().to_string(),
        assertions: outcomes,
        metadata: HashMap::new(),
    }
}

/// Replace `{{var}}` placeholders with the test's vars (string-coerced).
fn render(template: &str, vars: &HashMap<String, serde_json::Value>) -> String {
    let mut out = template.to_string();
    for (k, v) in vars {
        let val = match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        out = out
            .replace(&format!("{{{{{}}}}}", k), &val)
            .replace(&format!("{{{{ {} }}}}", k), &val);
    }
    out
}

fn extract_prompt(p: &serde_json::Value, idx: usize) -> (String, String) {
    match p {
        serde_json::Value::String(s) => (format!("prompt[{}]", idx), s.clone()),
        serde_json::Value::Object(_) => {
            let raw = p
                .get("raw")
                .or_else(|| p.get("content"))
                .and_then(|v| v.as_str())
                .unwrap_or("{{prompt}}")
                .to_string();
            let label = p
                .get("label")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("prompt[{}]", idx));
            (label, raw)
        }
        _ => (format!("prompt[{}]", idx), "{{prompt}}".to_string()),
    }
}
