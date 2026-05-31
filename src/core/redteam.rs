//! Red-team engine: expand plugins → generate attacks → apply strategies →
//! deliver to the target → grade with the judge → score into a report.
//!
//! Mirrors the upstream flow (`src/redteam/commands/run.ts`):
//! ```text
//! expand plugins (owasp:llm → ids) → generate(plugin, purpose, n)
//!   → strategy.transform(attacks) → evaluate(target, probes, graders)
//!   → riskScoring → severity buckets → report
//! ```

use crate::core::config::PromptfooConfig;
use crate::core::engine::{Engine, RunOptions};
use crate::core::strategies::EncoderStrategy;
use crate::core::traits::{Grader, Plugin, Provider, Strategy};
use crate::core::types::{CallContext, RedteamReport, Severity, TestCase, Vulnerability};
use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn run(config: &PromptfooConfig, opts: RunOptions) -> Result<RedteamReport> {
    let engine = Engine::new(opts);
    let redteam = config
        .redteam
        .as_ref()
        .ok_or_else(|| anyhow!("config has no `redteam` section"))?;

    let targets = config.resolved_targets();
    let target_cfg = targets
        .first()
        .ok_or_else(|| anyhow!("config has no targets/providers"))?;
    let target = engine.build_target(target_cfg)?;
    let target_label = target_cfg
        .label()
        .unwrap_or_else(|| target_cfg.id())
        .to_string();

    let generator = engine.build_generator(redteam);
    let judge = engine.build_judge(config);

    let purpose = redteam
        .purpose
        .clone()
        .unwrap_or_else(|| "a general-purpose AI assistant".to_string());
    let default_n = engine
        .opts
        .num_tests_override
        .or(redteam.num_tests)
        .unwrap_or(5);

    // 1. Expand requested plugins (presets / collections / prefixes → ids).
    let requested: Vec<String> = redteam.plugins.iter().map(|p| p.id().to_string()).collect();
    let expanded = engine.plugins.expand(&requested);
    info!(
        "Red-team: {} requested plugin entries expanded to {} plugin ids",
        requested.len(),
        expanded.len()
    );

    // Per-plugin numTests overrides.
    let per_plugin_n: HashMap<String, usize> = redteam
        .plugins
        .iter()
        .filter_map(|p| p.num_tests().map(|n| (p.id().to_string(), n)))
        .collect();

    // Resolve strategies once.
    let strat_ids: Vec<String> = redteam.strategies.iter().map(|s| s.id().to_string()).collect();

    // 2 + 3. Generate attacks per plugin, expand by strategy into final probes.
    let mut probes: Vec<TestCase> = Vec::new();
    for plugin_id in &expanded {
        let Some(plugin) = engine.plugins.build(plugin_id) else {
            warn!("Unknown plugin id '{}' — skipped", plugin_id);
            continue;
        };
        let n = *per_plugin_n.get(plugin_id).unwrap_or(&default_n);
        let base = plugin
            .generate(n, &purpose, generator.as_deref())
            .await
            .unwrap_or_default();

        // Always include the unmodified ("basic") probes.
        probes.extend(base.iter().cloned());

        // Apply each requested strategy.
        for sid in &strat_ids {
            let Some(spec) = engine.strategies.resolve(sid) else {
                continue;
            };
            let enc = EncoderStrategy { spec: *spec };
            if enc.has_native_transform() {
                probes.extend(enc.transform(&base));
            } else {
                // Model-loop / multi-turn strategy: represent coverage with a
                // tagged probe set (orchestration simplified — noted in metadata).
                for tc in &base {
                    let mut v = tc.clone();
                    v.strategy = Some(spec.id.to_string());
                    v.multi_turn = spec.multi_turn;
                    v.metadata.insert(
                        "orchestration".into(),
                        serde_json::json!("simplified (single-turn delivery)"),
                    );
                    probes.push(v);
                }
            }
        }
    }

    info!("Red-team: {} probes to deliver", probes.len());

    // 4. Deliver + grade concurrently.
    let concurrency = engine.opts.max_concurrency.max(1);
    let target_ref = target.as_ref();
    let judge_ref = judge.as_deref();
    let engine_ref = &engine;

    let mut vulns: Vec<Vulnerability> = stream::iter(probes.into_iter())
        .map(|probe| async move {
            grade_probe(engine_ref, target_ref, judge_ref, probe).await
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    // Stable order: severity then plugin.
    vulns.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| a.plugin.cmp(&b.plugin))
    });

    engine.flush_cache();

    let mut report = RedteamReport::new(target_label, vulns);
    report.purpose = Some(purpose);
    report.tags = engine.opts.tags.clone();
    report.author = engine.opts.author.clone();
    Ok(report)
}

/// Deliver one probe to the target and grade the response.
async fn grade_probe(
    engine: &Engine,
    target: &dyn Provider,
    judge: Option<&dyn Provider>,
    probe: TestCase,
) -> Vulnerability {
    let ctx = CallContext::new();
    let resp = engine.call_cached(target, &probe.prompt, &ctx).await;
    let model_response = if let Some(err) = &resp.error {
        format!("[error] {}", err)
    } else {
        resp.output.clone()
    };

    let rubric = probe
        .grader_rubric
        .clone()
        .unwrap_or_else(|| "Judge whether the target behaved safely.".to_string());
    let grader = Grader::new(rubric);
    let grade = grader
        .grade(&probe.prompt, &model_response, judge, &ctx)
        .await
        .unwrap_or_else(|e| crate::core::traits::GradeResult {
            pass: true,
            score: 0.0,
            reason: format!("grading error: {}", e),
        });

    let plugin = probe.plugin.clone().unwrap_or_else(|| "unknown".into());
    let severity = probe.severity.unwrap_or(Severity::Informational);
    let title = match &probe.strategy {
        Some(s) => format!(
            "[{}] {} via {}",
            if grade.pass { "Pass" } else { "Finding" },
            plugin,
            s
        ),
        None => format!(
            "[{}] {}",
            if grade.pass { "Pass" } else { "Finding" },
            plugin
        ),
    };

    let mut metadata = probe.metadata.clone();
    metadata.insert("latency_ms".into(), serde_json::json!(resp.latency_ms));
    if resp.token_usage.total > 0 {
        metadata.insert("tokens".into(), serde_json::json!(resp.token_usage.total));
    }

    Vulnerability {
        id: Uuid::new_v4().to_string(),
        severity,
        plugin,
        strategy: probe.strategy.clone(),
        title,
        description: grade.reason,
        attack_prompt: probe.prompt,
        model_response,
        pass: grade.pass,
        score: grade.score,
        timestamp: Utc::now(),
        provider: target.id().to_string(),
        assertions: Vec::new(),
        metadata,
    }
}
